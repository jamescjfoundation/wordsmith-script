// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF export and printing of the paginated script.
//!
//! The PDF is drawn from the same paginated layout the page view shows, so the pages match the
//! screen exactly, with real (selectable, searchable) text in embedded Courier Prime subsets.
//! Printing sends that PDF to the system: CUPS (`lp`) on macOS and Linux; on Windows the PDF
//! opens in the system's PDF viewer to print from there (`docs/decisions/0013-printing.md`).

use std::io::Write;

use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, file_access, p};
use wordsmith_pdf::PdfOptions;

use crate::paginate::{Paginated, paginate};

/// Most copies one print job may ask for.
pub const MAX_COPIES: u32 = 999;

/// The PDF of an already paginated script.
pub fn pdf_of(pag: &Paginated) -> Result<Vec<u8>, String> {
    wordsmith_pdf::export_layout(&pag.doc, &pag.layout, &PdfOptions::default()).map_err(|e| e.to_string())
}

/// The PDF of `doc`: its title page, when it has one, then the paginated script.
pub fn pdf(doc: &Document) -> Result<Vec<u8>, String> {
    printed(doc, &paginate(doc))
}

/// The printed PDF of `doc`, already paginated as `pag`: the title page (if any) first, as a
/// section of its own, then the script numbered from page 1.
pub fn printed(doc: &Document, pag: &Paginated) -> Result<Vec<u8>, String> {
    let title = crate::title_page::TitlePage::of_doc(doc);
    if title.is_empty() {
        return pdf_of(pag);
    }
    let with = crate::title_page::with_title_page(&pag.doc, &title);
    let layout = crate::paginate::measure::print_layout(&with, &mut wordsmith_layout::LayoutCache::new());
    wordsmith_pdf::export_layout(&with, &layout, &PdfOptions::default()).map_err(|e| e.to_string())
}

/// Write `bytes` to `path`, replacing it only once the new file is complete (craft-atomic: the
/// old file stays whole if anything fails). The path must be one automation may write
/// (`file_access`).
pub fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    file_access::check_write(path)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        craft_atomic::atomic_write(std::path::Path::new(path), bytes).map_err(|e| format!("{path}: {e}"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = bytes;
        Err(format!("{path}: the web build cannot write files"))
    }
}

/// What to print or export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrintOptions {
    /// Only the pages with revised text (no title page).
    pub revised_only: bool,
    /// The revision marks in the margins.
    pub marks: bool,
}

impl Default for PrintOptions {
    fn default() -> Self {
        PrintOptions { revised_only: false, marks: true }
    }
}

impl PrintOptions {
    fn of(v: &Value) -> Self {
        PrintOptions {
            revised_only: v.get("revisedOnly").and_then(Value::as_bool).unwrap_or(false),
            marks: v.get("marks").and_then(Value::as_bool).unwrap_or(true),
        }
    }
}

/// The PDF of `doc` as `opts` ask, and how many pages it has.
pub fn pdf_with(doc: &Document, opts: PrintOptions) -> Result<(Vec<u8>, usize), String> {
    let doc = if opts.marks {
        std::borrow::Cow::Borrowed(doc)
    } else {
        let mut copy = doc.clone();
        let mut r = crate::revisions::Revisions::of_doc(&copy);
        r.show_marks = false;
        r.store(&mut copy);
        std::borrow::Cow::Owned(copy)
    };
    let pag = paginate(&doc);
    if !opts.revised_only {
        let bytes = printed(&doc, &pag)?;
        return Ok((bytes, pag.pages.len()));
    }
    let pages: Vec<usize> = pag.page_revisions.iter().enumerate().filter(|(_, r)| r.is_some()).map(|(i, _)| i).collect();
    if pages.is_empty() {
        return Err("no page has revised text".into());
    }
    let n = pages.len();
    let bytes = wordsmith_pdf::export_layout(&pag.doc, &pag.layout, &PdfOptions { pages: Some(pages), ..PdfOptions::default() })
        .map_err(|e| e.to_string())?;
    Ok((bytes, n))
}

fn export_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let path = p::req_str(v, "path")?.to_string();
    if !path.to_lowercase().ends_with(".pdf") {
        return Err(CmdError::Params("`path` must end in .pdf".into()));
    }
    let (bytes, pages) = pdf_with(&s.doc, PrintOptions::of(v)).map_err(CmdError::Failed)?;
    write_file(&path, &bytes).map_err(CmdError::Failed)?;
    Ok(json!({"path": path, "pages": pages, "bytes": bytes.len()}))
}

/// `file.exportRevisedPdf {path}`: export only the revised pages.
fn export_revised_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let mut v = v.clone();
    v["revisedOnly"] = json!(true);
    export_pdf(s, &v)
}

/// `file.printRevised`: print only the revised pages.
fn print_revised(s: &mut Session, v: &Value) -> CmdResult {
    let mut v = if v.is_object() { v.clone() } else { json!({}) };
    v["revisedOnly"] = json!(true);
    print(s, &v)
}

/// The `lp` arguments for a print job: the printer (else the default), copies and title; the
/// PDF goes in on standard input.
pub fn lp_args(printer: Option<&str>, copies: u32, title: &str) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(p) = printer.filter(|p| !p.is_empty()) {
        args.push("-d".into());
        args.push(p.into());
    }
    args.push("-n".into());
    args.push(copies.clamp(1, MAX_COPIES).to_string());
    // A title is shown in the print queue; keep it short and printable.
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    args.push("-t".into());
    args.push(if title.is_empty() { "Script".into() } else { title });
    args.push("--".into());
    args.push("-".into());
    args
}

#[cfg(not(windows))]
fn send_to_printer(pdf: &[u8], printer: Option<&str>, copies: u32, title: &str) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("lp")
        .args(lp_args(printer, copies, title))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("printing needs CUPS (`lp`): {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(pdf).map_err(|e| format!("lp: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("lp: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!("lp: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

#[cfg(windows)]
fn send_to_printer(pdf: &[u8], _printer: Option<&str>, _copies: u32, title: &str) -> Result<String, String> {
    // Open the PDF in the system's viewer, where the person prints it.
    let name: String = title.chars().filter(|c| c.is_alphanumeric() || *c == ' ').take(40).collect();
    let path = std::env::temp_dir().join(format!("{} {}.pdf", if name.trim().is_empty() { "Script" } else { name.trim() }, stamp()));
    std::fs::write(&path, pdf).map_err(|e| format!("{}: {e}", path.display()))?;
    std::process::Command::new("cmd").args(["/C", "start", ""]).arg(&path).spawn().map_err(|e| format!("opening the PDF: {e}"))?;
    Ok(format!("opened {} to print", path.display()))
}

#[cfg(windows)]
fn stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn print(s: &mut Session, v: &Value) -> CmdResult {
    let copies = p::u64(v, "copies").unwrap_or(1);
    if !(1..=u64::from(MAX_COPIES)).contains(&copies) {
        return Err(CmdError::Params(format!("`copies` is 1 to {MAX_COPIES}")));
    }
    let printer = p::str(v, "printer");
    let (bytes, pages) = pdf_with(&s.doc, PrintOptions::of(v)).map_err(CmdError::Failed)?;
    let title = if s.doc.core.title.is_empty() { "Script".to_string() } else { s.doc.core.title.clone() };
    let job = send_to_printer(&bytes, printer, copies as u32, &title).map_err(CmdError::Failed)?;
    Ok(json!({"job": job, "pages": pages}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.exportPdf", "Export PDF", "File", export_pdf)
            .params(r#"{"path": string ending in .pdf, "revisedOnly"?: bool, "marks"?: bool}"#)
            .pure(),
        CommandSpec::new("file.exportRevisedPdf", "Export Revised Pages", "File", export_revised_pdf)
            .params(r#"{"path": string ending in .pdf, "marks"?: bool}"#)
            .pure(),
        CommandSpec::new("file.print", "Print", "File", print)
            .params(r#"{"printer"?: string, "copies"?: 1-999, "revisedOnly"?: bool, "marks"?: bool}"#)
            .key("Mod+P")
            .pure(),
        CommandSpec::new("file.printRevised", "Print Revised Pages", "File", print_revised)
            .params(r#"{"printer"?: string, "copies"?: 1-999, "marks"?: bool}"#)
            .pure(),
    ]
}

#[cfg(test)]
pub(crate) mod tests;
