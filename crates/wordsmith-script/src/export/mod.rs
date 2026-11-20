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

/// The PDF of `doc`.
pub fn pdf(doc: &Document) -> Result<Vec<u8>, String> {
    pdf_of(&paginate(doc))
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

fn export_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let path = p::req_str(v, "path")?.to_string();
    if !path.to_lowercase().ends_with(".pdf") {
        return Err(CmdError::Params("`path` must end in .pdf".into()));
    }
    let pag = paginate(&s.doc);
    let bytes = pdf_of(&pag).map_err(CmdError::Failed)?;
    write_file(&path, &bytes).map_err(CmdError::Failed)?;
    Ok(json!({"path": path, "pages": pag.pages.len(), "bytes": bytes.len()}))
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
    let pag = paginate(&s.doc);
    let bytes = pdf_of(&pag).map_err(CmdError::Failed)?;
    let title = if s.doc.core.title.is_empty() { "Script".to_string() } else { s.doc.core.title.clone() };
    let job = send_to_printer(&bytes, printer, copies as u32, &title).map_err(CmdError::Failed)?;
    Ok(json!({"job": job, "pages": pag.pages.len()}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.exportPdf", "Export PDF", "File", export_pdf).params(r#"{"path": string ending in .pdf}"#).pure(),
        CommandSpec::new("file.print", "Print", "File", print).params(r#"{"printer"?: string, "copies"?: 1-999}"#).key("Mod+P").pure(),
    ]
}

#[cfg(test)]
mod tests;
