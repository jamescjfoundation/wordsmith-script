// SPDX-License-Identifier: MIT OR Apache-2.0
//! File commands: open, import, save, save as and the exports (decisions 0020, 0029). The ones
//! without a path ask the window for a file dialog (a UI request; headless callers just get
//! `{"asks": …}` back); the dialog's answer runs the command that takes the path, so the window,
//! the CLI, the MCP server and agents reach the same code.
//!
//! Opening or importing over unsaved changes is refused unless the caller says to discard them
//! (`"discard": true`); the editor asks the writer first. A save tells the window (`{"saved":
//! path}`), which makes the backup copy and restarts the auto-save clock.

use std::path::{Path, PathBuf};

use crate::files::{self, Kind};
use serde_json::{Value, json};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

/// The script's name: its file's name, the name it was given, or "Untitled".
pub fn script_name(s: &Session) -> String {
    s.path
        .as_deref()
        .and_then(|p| p.file_stem())
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .or_else(|| s.file_name.clone())
        .unwrap_or_else(|| "Untitled".into())
}

fn ask(s: &mut Session, what: &str, extra: Value) -> CmdResult {
    let mut req = json!({"open": what});
    if let (Some(r), Value::Object(e)) = (req.as_object_mut(), extra) {
        r.extend(e);
    }
    s.push_ui_request(req);
    Ok(json!({"asks": what}))
}

fn path_of(v: &Value) -> Option<PathBuf> {
    v.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).map(PathBuf::from)
}

fn stem(path: &Path) -> Option<String> {
    path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).map(str::to_string)
}

fn failed(e: String) -> CmdError {
    CmdError::Failed(e)
}

/// Put `doc` in the session: a native file keeps its path; anything else is a new, unsaved
/// script named after the file it came from.
fn replace(s: &mut Session, doc: wordsmith_doc::Document, path: &Path, native: bool, dirty: bool) {
    s.set_document(doc);
    if native {
        s.path = Some(path.to_path_buf());
    } else {
        s.path = None;
        s.file_name = stem(path);
    }
    s.dirty = dirty;
}

/// `file.openPath {path, asScript?, discard?}`: open a script (`.wss`, Fountain) or import a
/// file (plain text, RTF, PDF). Plain text and RTF need `asScript`, the answer to "is this a
/// script?"; without it the window is asked to put the question, with a guess.
fn open_path(s: &mut Session, v: &Value) -> CmdResult {
    let path = PathBuf::from(p::req_str(v, "path")?);
    if s.dirty && v.get("discard").and_then(Value::as_bool) != Some(true) {
        return Err(failed("the script has unsaved changes: save it first, or pass \"discard\": true".into()));
    }
    let kind = Kind::of(&path).ok_or_else(|| failed(format!("{}: not a file this app opens", path.display())))?;
    match kind {
        Kind::Native | Kind::Fountain => {
            let doc = files::open(&path).map_err(failed)?;
            replace(s, doc, &path, kind == Kind::Native, false);
        }
        Kind::Text | Kind::Rtf => {
            let Some(as_script) = v.get("asScript").and_then(Value::as_bool) else {
                let guess = files::guess_is_script(&path).map_err(failed)?;
                return ask(s, "importQuestion", json!({"path": path.display().to_string(), "guess": guess}));
            };
            let doc = files::import_file(&path, as_script).map_err(failed)?;
            replace(s, doc, &path, false, true);
        }
        Kind::Pdf => {
            let doc = files::import_file(&path, true).map_err(failed)?;
            replace(s, doc, &path, false, true);
        }
        Kind::Html => return Err(failed(format!("{}: HTML is exported, not opened", path.display()))),
    }
    Ok(json!({"opened": path.display().to_string()}))
}

fn saved(s: &mut Session, path: &Path) -> CmdResult {
    s.path = Some(path.to_path_buf());
    s.file_name = None;
    s.dirty = false;
    let shown = path.display().to_string();
    s.push_ui_request(json!({"saved": shown}));
    Ok(json!({"saved": shown}))
}

/// `file.save`: save to the script's file; a script without one asks for a file.
fn save(s: &mut Session, _: &Value) -> CmdResult {
    match s.path.clone().filter(|p| Kind::of(p) == Some(Kind::Native)) {
        Some(path) => {
            files::save(&path, &s.doc).map_err(failed)?;
            saved(s, &path)
        }
        None => ask(s, "saveAs", json!({"name": script_name(s)})),
    }
}

/// `file.saveAsPath {path}`: save to `path` (`.wss` is added when missing); the script's file is
/// that one from now on.
fn save_as_path(s: &mut Session, v: &Value) -> CmdResult {
    let path = files::with_extension(&PathBuf::from(p::req_str(v, "path")?), Kind::Native);
    files::save(&path, &s.doc).map_err(failed)?;
    saved(s, &path)
}

/// Export as `kind`: with a path, write it; without one, ask for a file.
fn export(s: &mut Session, v: &Value, kind: Kind) -> CmdResult {
    match path_of(v) {
        Some(path) => {
            let path = files::with_extension(&path, kind);
            files::export_to(&path, &s.doc, kind).map_err(failed)?;
            Ok(json!({"exported": path.display().to_string()}))
        }
        None => ask(s, "export", json!({"format": kind.extension(), "name": script_name(s)})),
    }
}

fn export_fountain(s: &mut Session, v: &Value) -> CmdResult {
    export(s, v, Kind::Fountain)
}

fn export_text(s: &mut Session, v: &Value) -> CmdResult {
    export(s, v, Kind::Text)
}

fn export_rtf(s: &mut Session, v: &Value) -> CmdResult {
    export(s, v, Kind::Rtf)
}

fn export_html(s: &mut Session, v: &Value) -> CmdResult {
    export(s, v, Kind::Html)
}

/// Write a table as CSV: with a path, write it; without one, ask for a file.
fn export_csv(s: &mut Session, v: &Value, table: &str) -> CmdResult {
    match path_of(v) {
        Some(path) => {
            let path = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv")) { path } else { path.with_extension("csv") };
            let text = match table {
                "scenes" => wordsmith_script::commands::character::scenes_csv(&s.doc),
                "report" => wordsmith_script::reports::character_report_csv(&s.doc),
                _ => wordsmith_script::commands::character::characters_csv(&s.doc),
            };
            files::write_atomic(&path, text.as_bytes()).map_err(failed)?;
            Ok(json!({"exported": path.display().to_string()}))
        }
        None => ask(
            s,
            "export",
            json!({"format": format!("csv-{table}"), "name": format!("{} {}", script_name(s), if table == "scenes" { "Scenes" } else { "Characters" })}),
        ),
    }
}

fn export_character_report(s: &mut Session, v: &Value) -> CmdResult {
    export_csv(s, v, "report")
}

/// `file.exportRevisedPdfAs`: ask where to export the revised pages.
fn export_revised_as(s: &mut Session, _: &Value) -> CmdResult {
    ask(s, "export", json!({"format": "pdf-revised", "name": format!("{} Revised", script_name(s))}))
}

fn export_scenes_csv(s: &mut Session, v: &Value) -> CmdResult {
    export_csv(s, v, "scenes")
}

fn export_characters_csv(s: &mut Session, v: &Value) -> CmdResult {
    export_csv(s, v, "characters")
}

/// The file commands.
/// `report.export {kind, …report options, path, csv?}`: write a report as text (or CSV, by
/// `csv` or a `.csv` path). Without a path, ask for a file; the window keeps the options.
fn export_report(s: &mut Session, v: &Value) -> CmdResult {
    let csv = v.get("csv").and_then(Value::as_bool).unwrap_or(false);
    let Some(path) = path_of(v) else {
        return ask(s, "export", json!({"format": if csv { "report-csv" } else { "report-text" }, "name": script_name(s)}));
    };
    let csv = csv || path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    let report = wordsmith_script::reports_more::report(&s.doc, v)?;
    let text = if csv { report.to_csv() } else { report.to_text() };
    std::fs::write(&path, text).map_err(|e| failed(format!("{}: {e}", path.display())))?;
    Ok(json!({"path": path}))
}

/// `report.exportFilteredPdf {elements, path}`: the script reduced to some elements, as a PDF.
fn export_filtered_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let keep: Vec<String> =
        v.get("elements").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let Some(path) = path_of(v) else {
        return ask(s, "export", json!({"format": "pdf-filtered", "name": format!("{} Filtered", script_name(s))}));
    };
    let doc = wordsmith_script::reports_more::filtered_script(&s.doc, &keep);
    let bytes = wordsmith_script::export::pdf(&doc).map_err(failed)?;
    std::fs::write(&path, bytes).map_err(|e| failed(format!("{}: {e}", path.display())))?;
    Ok(json!({"path": path}))
}

fn write_pdf(path: &Path, doc: &wordsmith_doc::Document) -> CmdResult {
    let bytes = wordsmith_script::planning_pdf::pdf(doc).map_err(failed)?;
    std::fs::write(path, bytes).map_err(|e| failed(format!("{}: {e}", path.display())))?;
    Ok(json!({"path": path}))
}

/// `storyboard.exportPdf {board?, path?}`: a story board as a one-page PDF.
fn export_board_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let Some(path) = path_of(v) else {
        return ask(s, "export", json!({"format": "pdf-board", "name": format!("{} Story Board", script_name(s))}));
    };
    let board = v.get("board").and_then(Value::as_u64).and_then(|b| u32::try_from(b).ok()).unwrap_or(0);
    write_pdf(&path, &wordsmith_script::planning_pdf::storyboard_document(&s.doc, board))
}

/// `cards.exportPdf {stock?, columns?, rows?, frames?, grey?, scenes?, path?}`: index cards.
fn export_cards_pdf(s: &mut Session, v: &Value) -> CmdResult {
    let Some(path) = path_of(v) else {
        return ask(s, "export", json!({"format": "pdf-cards", "name": format!("{} Cards", script_name(s))}));
    };
    let opts = wordsmith_script::planning_pdf::CardOptions::from_params(v);
    write_pdf(&path, &wordsmith_script::planning_pdf::cards_document(&s.doc, &opts))
}

/// `macro.copyFromPath {path}`: replace this script's macros with another script's.
fn copy_macros_from(s: &mut Session, v: &Value) -> CmdResult {
    let path = path_of(v).ok_or_else(|| CmdError::Params("`path` is required".into()))?;
    let other = files::open(&path).map_err(failed)?;
    let macros = wordsmith_script::macros::Macros::of_doc(&other);
    s.run("macro.copyFrom", &json!({"macros": macros}))
}

pub fn specs() -> Vec<CommandSpec> {
    const PATH: &str = r#"{"path"?: string (asks for a file when left out)}"#;
    vec![
        CommandSpec::new("file.open", "Open…", "File", |s, _| ask(s, "open", json!({}))).key("Mod+O").pure(),
        CommandSpec::new("file.openPath", "Open File", "File", open_path).params(r#"{"path": string, "asScript"?: bool, "discard"?: bool}"#),
        CommandSpec::new("macro.copyFromFile", "Copy Macros from a Script…", "Tools › Macros", |s, _| ask(s, "macrosFrom", json!({}))).pure(),
        CommandSpec::new("macro.copyFromPath", "Copy Macros from File", "Tools › Macros", copy_macros_from).params(r#"{"path": string}"#),
        CommandSpec::new("report.export", "Save Report…", "Reports", export_report)
            .params(r#"{"kind": string, "path"?: string, "csv"?: bool, …report options}"#)
            .pure(),
        CommandSpec::new("report.exportFilteredPdf", "Filtered Script as PDF…", "Reports", export_filtered_pdf)
            .params(r#"{"elements": [elementId], "path"?: string}"#)
            .pure(),
        CommandSpec::new("storyboard.exportPdf", "Story Board as PDF…", "File › Export", export_board_pdf)
            .params(r#"{"board"?: number, "path"?: string}"#)
            .pure(),
        CommandSpec::new("cards.exportPdf", "Index Cards as PDF…", "File › Export", export_cards_pdf)
            .params(
                r#"{"stock"?: "card"|"grid", "columns"?: 1-6, "rows"?: 1-6, "frames"?: bool, "grey"?: bool, "scenes"?: [index], "path"?: string}"#,
            )
            .pure(),
        CommandSpec::new("file.import", "Import…", "File", |s, _| ask(s, "import", json!({}))).pure(),
        CommandSpec::new("file.save", "Save", "File", save).key("Mod+S").pure(),
        CommandSpec::new("file.saveAs", "Save As…", "File", |s, _| ask(s, "saveAs", json!({"name": script_name(s)}))).key("Mod+Shift+S").pure(),
        CommandSpec::new("file.saveAsPath", "Save to File", "File", save_as_path).params(r#"{"path": string}"#).pure(),
        CommandSpec::new("file.exportFountain", "Export Fountain…", "File › Export", export_fountain).params(PATH).pure(),
        CommandSpec::new("file.exportText", "Export Plain Text…", "File › Export", export_text).params(PATH).pure(),
        CommandSpec::new("file.exportRtf", "Export RTF…", "File › Export", export_rtf).params(PATH).pure(),
        CommandSpec::new("tables.exportScenesCsv", "Export Scenes Table…", "File › Export", export_scenes_csv).params(PATH).pure(),
        CommandSpec::new("tables.exportCharactersCsv", "Export Characters Table…", "File › Export", export_characters_csv).params(PATH).pure(),
        CommandSpec::new("report.exportCharacters", "Character Report…", "Production", export_character_report).params(PATH).pure(),
        CommandSpec::new("file.exportRevisedPdfAs", "Export Revised Pages…", "File › Export", export_revised_as).pure(),
        CommandSpec::new("file.exportHtml", "Export HTML…", "File › Export", export_html).params(PATH).pure(),
    ]
}
