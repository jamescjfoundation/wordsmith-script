// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF export and printing of the paginated script.
//!
//! The PDF is drawn from the same paginated layout the page view shows, so the pages match the
//! screen exactly, with real (selectable, searchable) text in embedded Courier Prime subsets.
//! Printing sends that PDF to the system: CUPS (`lp`) on macOS and Linux; on Windows the PDF
//! opens in the system's PDF viewer to print from there (`docs/decisions/0013-printing.md`).

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintOptions {
    /// Only the pages with revised text (no title page).
    pub revised_only: bool,
    /// The revision marks in the margins.
    pub marks: bool,
    /// Only the pages without revised text (SPEC-2 11.3).
    pub unrevised_only: bool,
    /// Only the pages with text revised in this set (its index).
    pub set: Option<u32>,
    /// Only these pages (1-based, inclusive).
    pub pages: Option<(usize, usize)>,
    /// Only the pages these scenes are on (scene indices, inclusive).
    pub scenes: Option<(usize, usize)>,
    /// Last page first.
    pub reverse: bool,
    /// The title page, when there is one and every page prints.
    pub title_page: bool,
    /// Sides: only the scenes this character is in, with the name on every page.
    pub sides: Option<String>,
    /// A name at the top of every page.
    pub page_name: Option<String>,
}

impl Default for PrintOptions {
    fn default() -> Self {
        PrintOptions {
            revised_only: false,
            marks: true,
            unrevised_only: false,
            set: None,
            pages: None,
            scenes: None,
            reverse: false,
            title_page: true,
            sides: None,
            page_name: None,
        }
    }
}

impl PrintOptions {
    pub fn of(v: &Value) -> Self {
        let range = |k: &str| {
            let a = v.get(k)?.as_array()?;
            let lo = a.first()?.as_u64()? as usize;
            let hi = a.get(1).and_then(Value::as_u64).map_or(lo, |x| x as usize);
            Some((lo.min(hi), lo.max(hi)))
        };
        let text =
            |k: &str| v.get(k).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()).map(|t| t.chars().take(80).collect::<String>());
        PrintOptions {
            revised_only: v.get("revisedOnly").and_then(Value::as_bool).unwrap_or(false),
            marks: v.get("marks").and_then(Value::as_bool).unwrap_or(true),
            unrevised_only: v.get("unrevisedOnly").and_then(Value::as_bool).unwrap_or(false),
            set: v.get("set").and_then(Value::as_u64).and_then(|x| u32::try_from(x).ok()),
            pages: range("pages"),
            scenes: range("scenes"),
            reverse: v.get("reverse").and_then(Value::as_bool).unwrap_or(false),
            title_page: v.get("titlePage").and_then(Value::as_bool).unwrap_or(true),
            sides: text("sides"),
            page_name: text("pageName"),
        }
    }

    /// Whether every page prints, in order (so the title page goes first).
    fn whole(&self) -> bool {
        !self.revised_only && !self.unrevised_only && self.set.is_none() && self.pages.is_none() && self.scenes.is_none() && !self.reverse
    }
}

/// `doc` reduced to the scenes `who` is in, each keeping the number it has in the whole script.
pub fn sides_of(doc: &Document, who: &str) -> Document {
    let who = who.trim().to_uppercase();
    let elements = crate::element::Elements::of_doc(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let mut out = doc.clone();
    let mut keep = Vec::new();
    for sc in crate::scenes::scenes(doc) {
        let (_, cast) = crate::scene_data::cast(doc, &elements, &sc);
        if !cast.contains(&who) {
            continue;
        }
        for i in sc.start..sc.end {
            if let Some(b) = doc.body.get(i) {
                let mut b = b.clone();
                if i == sc.start
                    && let wordsmith_doc::Block::Para(p) = std::sync::Arc::make_mut(&mut b)
                {
                    p.props.scene_number = numbers.get(sc.index).cloned();
                }
                keep.push(b);
            }
        }
    }
    out.body = keep;
    out.ensure_nonempty();
    let mut n = crate::scene_numbers::SceneNumbers::of_doc(&out);
    n.locked = true;
    n.store(&mut out);
    out
}

/// The PDF of `doc` as `opts` ask, and how many pages it has.
pub fn pdf_with(doc: &Document, opts: PrintOptions) -> Result<(Vec<u8>, usize), String> {
    let mut doc = std::borrow::Cow::Borrowed(doc);
    if !opts.marks {
        let copy = doc.to_mut();
        let mut r = crate::revisions::Revisions::of_doc(copy);
        r.show_marks = false;
        r.store(copy);
    }
    if let Some(who) = &opts.sides {
        let reduced = sides_of(&doc, who);
        *doc.to_mut() = reduced;
    }
    if let Some(name) = opts.page_name.clone().or_else(|| opts.sides.clone()) {
        let copy = doc.to_mut();
        let mut w = crate::watermark::Watermark::of_doc(copy);
        w.page_name = name.to_uppercase();
        w.store(copy);
    }
    let pag = paginate(&doc);
    if opts.whole() {
        let bytes = if opts.title_page { printed(&doc, &pag)? } else { pdf_of(&pag)? };
        return Ok((bytes, pag.pages.len()));
    }
    let n_pages = pag.pages.len();
    let mut pages: Vec<usize> = (0..n_pages).collect();
    if opts.revised_only {
        pages.retain(|i| pag.page_revisions.get(*i).is_some_and(Option::is_some));
    }
    if opts.unrevised_only {
        pages.retain(|i| pag.page_revisions.get(*i).is_some_and(Option::is_none));
    }
    if let Some(set) = opts.set {
        // Pages with any text marked in that set.
        let on: std::collections::HashSet<usize> = pag
            .pages
            .iter()
            .enumerate()
            .filter(|(_, pg)| {
                pg.iter().any(|pc| match pc {
                    crate::paginate::Piece::Para { block, start, end, .. } => doc.body.get(*block).and_then(|b| b.as_para()).is_some_and(|p| {
                        p.mark.revised == Some(set) || p.run_ranges().any(|(r, c)| c.revised == Some(set) && r.end > *start && r.start < *end)
                    }),
                    _ => false,
                })
            })
            .map(|(i, _)| i)
            .collect();
        pages.retain(|i| on.contains(i));
    }
    if let Some((a, b)) = opts.pages {
        pages.retain(|i| (a..=b).contains(&(i + 1)));
    }
    if let Some((a, b)) = opts.scenes {
        let all = crate::scenes::scenes(&doc);
        let blocks: std::collections::HashSet<usize> = all.iter().filter(|s| (a..=b).contains(&s.index)).flat_map(|s| s.start..s.end).collect();
        pages.retain(|i| {
            pag.pages.get(*i).is_some_and(|pg| pg.iter().any(|pc| matches!(pc, crate::paginate::Piece::Para { block, .. } if blocks.contains(block))))
        });
    }
    if opts.reverse {
        pages.reverse();
    }
    if pages.is_empty() {
        return Err(if opts.revised_only { "no page has revised text".into() } else { "no page matches what was asked to print".into() });
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
fn send_to_printer(pdf: &[u8], printer: Option<&str>, copies: u32, title: &str, collate: bool) -> Result<String, String> {
    use std::process::{Command, Stdio};
    use std::io::Write;
    let mut args = lp_args(printer, copies, title);
    if collate && copies > 1 {
        args.splice(0..0, ["-o".to_string(), "collate=true".to_string()]);
    }
    let mut child = Command::new("lp")
        .args(args)
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
fn send_to_printer(pdf: &[u8], _printer: Option<&str>, _copies: u32, title: &str, _collate: bool) -> Result<String, String> {
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
    let collate = v.get("collate").and_then(Value::as_bool).unwrap_or(true);
    let job = send_to_printer(&bytes, printer, copies as u32, &title, collate).map_err(CmdError::Failed)?;
    Ok(json!({"job": job, "pages": pages}))
}

/// `file.printPreview {…print options}`: the pages as they will print, opened in the system's
/// PDF viewer.
fn print_preview(s: &mut Session, v: &Value) -> CmdResult {
    let (bytes, pages) = pdf_with(&s.doc, PrintOptions::of(v)).map_err(CmdError::Failed)?;
    let secs = s.now;
    let path = std::env::temp_dir().join(format!("Wordsmith Script preview {secs}-{}.pdf", bytes.len()));
    std::fs::write(&path, &bytes).map_err(|e| CmdError::Failed(format!("{}: {e}", path.display())))?;
    open_with_system(&path).map_err(CmdError::Failed)?;
    Ok(json!({"path": path, "pages": pages}))
}

/// Open `path` in the program the system uses for it.
fn open_with_system(path: &std::path::Path) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    cmd.arg(path).spawn().map(|_| ()).map_err(|e| format!("opening the preview: {e}"))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.exportPdf", "Export PDF", "File", export_pdf)
            .params(r#"{"path": string ending in .pdf, "revisedOnly"?: bool, "unrevisedOnly"?: bool, "set"?: index, "pages"?: [from, to], "scenes"?: [from, to], "reverse"?: bool, "titlePage"?: bool, "sides"?: character, "pageName"?: string, "marks"?: bool}"#)
            .pure(),
        CommandSpec::new("file.exportRevisedPdf", "Export Revised Pages", "File", export_revised_pdf)
            .params(r#"{"path": string ending in .pdf, "marks"?: bool}"#)
            .pure(),
        CommandSpec::new("file.print", "Print", "File", print)
            .params(r#"{"printer"?: string, "copies"?: 1-999, "collate"?: bool, "revisedOnly"?: bool, "unrevisedOnly"?: bool, "set"?: index, "pages"?: [from, to], "scenes"?: [from, to], "reverse"?: bool, "titlePage"?: bool, "sides"?: character, "marks"?: bool}"#)
            .key("Mod+P")
            .pure(),
        CommandSpec::new("file.printPreview", "Print Preview", "File", print_preview).params(r#"{…the print options}"#).key("Mod+Alt+P").pure(),
        CommandSpec::new("file.printRevised", "Print Revised Pages", "File", print_revised)
            .params(r#"{"printer"?: string, "copies"?: 1-999, "marks"?: bool}"#)
            .pure(),
    ]
}

#[cfg(test)]
pub(crate) mod tests;
