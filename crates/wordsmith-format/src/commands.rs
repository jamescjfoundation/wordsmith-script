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
use wordsmith_script::keys::ScriptPara;

use crate::files::{self, Kind};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde_json::{Value, json};

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
fn replace(s: &mut Session, doc: jcj_doc::Document, path: &Path, native: bool, dirty: bool) {
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
            let template = doc.ext.contains(wordsmith_script::templates::TEMPLATE_KEY);
            let read_only = std::fs::metadata(&path).is_ok_and(|m| m.permissions().readonly());
            if template {
                // A template starts a new, untitled script.
                replace(s, wordsmith_script::templates::from_template(doc), &path, false, false);
                s.file_name = Some("Untitled".into());
            } else if read_only && kind == Kind::Native {
                // A file that cannot be written opens as a copy: Save asks where to keep it.
                replace(s, doc, &path, false, false);
                s.push_ui_request(
                    json!({"notice": format!("{} is read-only; it is open as a copy, so Save asks where to keep it.", path.display())}),
                );
            } else {
                replace(s, doc, &path, kind == Kind::Native, false);
            }
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
    files::write_atomic(&path, text.as_bytes()).map_err(failed)?;
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
    files::write_atomic(&path, &bytes).map_err(failed)?;
    Ok(json!({"path": path}))
}

fn write_pdf(path: &Path, doc: &jcj_doc::Document) -> CmdResult {
    let bytes = wordsmith_script::planning_pdf::pdf(doc).map_err(failed)?;
    files::write_atomic(path, &bytes).map_err(failed)?;
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

/// `compare.withFile {path, open?, discard?}`: compare this script (the newer version) with the
/// one at `path` (the older). Without `open`, report what changed; with it, the marked-up copy
/// becomes the script in the window (an unsaved one), refused over unsaved changes unless
/// `discard`.
fn compare_with(s: &mut Session, v: &Value) -> CmdResult {
    let Some(path) = path_of(v) else { return ask(s, "compare", json!({})) };
    let old = files::open(&path).map_err(failed)?;
    let (marked, summary) = wordsmith_script::compare::compare(&old, &s.doc);
    if v.get("open").and_then(Value::as_bool) == Some(true) {
        if s.dirty && v.get("discard").and_then(Value::as_bool) != Some(true) {
            return Err(failed("save the script first: the comparison replaces it in the window".into()));
        }
        let name = format!("{} compared", script_name(s));
        s.set_document(marked);
        s.path = None;
        s.file_name = Some(name);
        s.dirty = true;
    }
    Ok(summary.to_json())
}

/// `file.saveAsTemplate {path?}`: save a copy of the script as a template (`.wst`); opening it
/// later starts a new script from it.
fn save_as_template(s: &mut Session, v: &Value) -> CmdResult {
    let Some(path) = path_of(v) else { return ask(s, "export", json!({"format": "template", "name": script_name(s)})) };
    let path = path.with_extension("wst");
    let mut doc = s.doc.clone();
    let _ = doc.ext.set(wordsmith_script::templates::TEMPLATE_KEY, json!(true));
    files::save(&path, &doc).map_err(failed)?;
    Ok(json!({"saved": path.display().to_string()}))
}

/// `file.revert {discard?}`: put back the script as last saved.
fn revert(s: &mut Session, _: &Value) -> CmdResult {
    let path = s.path.clone().ok_or_else(|| failed("the script has not been saved yet".into()))?;
    open_path(s, &json!({"path": path.display().to_string(), "discard": true}))
}

/// `file.merge {paths}`: a new script made of several scripts one after another (the first
/// one's elements and settings).
fn merge(s: &mut Session, v: &Value) -> CmdResult {
    let paths: Vec<PathBuf> =
        v.get("paths").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(PathBuf::from).collect()).unwrap_or_default();
    if paths.len() < 2 {
        return Err(CmdError::Params("`paths` names at least two scripts".into()));
    }
    if s.dirty && v.get("discard").and_then(Value::as_bool) != Some(true) {
        return Err(failed("save the script first: the merged script replaces it in the window".into()));
    }
    let mut out: Option<jcj_doc::Document> = None;
    for path in &paths {
        let doc = files::open(path).map_err(failed)?;
        match out.as_mut() {
            None => out = Some(doc),
            Some(o) => {
                let elements = wordsmith_script::Elements::of_doc(o);
                let theirs = wordsmith_script::Elements::of_doc(&doc);
                for b in &doc.body {
                    if let Some(p) = b.as_para() {
                        let mut q = p.clone();
                        // An element the first script lacks reads as General.
                        let id = theirs.of_para(p).id.clone();
                        q.props.style =
                            Some(if elements.get(&id).is_some() { id } else { elements.id_of(wordsmith_script::ElementKind::General).to_string() });
                        q.props.set_scene_id(None);
                        q.props.set_bookmark(None);
                        o.body.push(jcj_doc::para_block(q));
                    }
                }
            }
        }
    }
    let doc = out.ok_or_else(|| failed("nothing to merge".into()))?;
    s.set_document(doc);
    s.path = None;
    s.file_name = Some("Merged".into());
    s.dirty = true;
    Ok(json!({"merged": paths.len()}))
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
        CommandSpec::new("compare.withFile", "Compare with Another Version…", "Production", compare_with)
            .params(r#"{"path"?: string, "open"?: bool, "discard"?: bool}"#)
            .pure(),
        CommandSpec::new("file.saveAsTemplate", "Save as Template…", "File", save_as_template).params(r#"{"path"?: string}"#).pure(),
        CommandSpec::new("file.revert", "Revert to Saved", "File", revert)
            .when(|s| if s.path.is_none() { Some("the script has not been saved yet") } else { None }),
        CommandSpec::new("file.merge", "Merge Scripts", "File", merge).params(r#"{"paths": [string], "discard"?: bool}"#),
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

/// Every command a session without a window needs: the engine's, the screenplay's and these
/// file commands. The command line and the MCP server use it.
pub fn headless_registry() -> jcj_engine::Registry {
    wordsmith_script::commands::registry_plus(specs())
}

/// A session on a new script with [`headless_registry`].
pub fn headless_session() -> Session {
    static REGISTRY: std::sync::OnceLock<std::sync::Arc<jcj_engine::Registry>> = std::sync::OnceLock::new();
    let registry = REGISTRY.get_or_init(|| {
        wordsmith_script::paginate::plan_pages_for_features();
        std::sync::Arc::new(headless_registry())
    });
    jcj_engine::session_with(wordsmith_script::new_script(), registry.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A session with the screenplay commands and these file commands.
    fn session() -> Session {
        headless_session()
    }

    #[test]
    fn the_file_keys_of_0_1_reach_their_commands() {
        let r = headless_registry();
        for (key, id) in [("Mod+O", "file.open"), ("Mod+Alt+P", "file.printPreview"), ("Mod+S", "file.save"), ("Mod+Shift+S", "file.saveAs")] {
            assert_eq!(r.by_shortcut(key).map(|c| c.id), Some(id), "{key}");
        }
    }

    #[test]
    fn every_headless_command_id_is_unique() {
        let mut all = wordsmith_script::commands::specs();
        all.extend(specs());
        // The screenplay layer may replace a command of the engine, the core or the shared
        // features, never one of its own or these.
        let dupes: Vec<&str> = jcj_engine::duplicate_ids(&all);
        assert!(dupes.is_empty(), "registered twice: {dupes:?}");
        assert!(jcj_engine::duplicate_ids(headless_registry().all()).is_empty());
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wordsmith-commands-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn templates_revert_and_merge() {
        let d = dir("templates");
        let mut s = session();
        s.run("text.insert", &json!({"text": "int. hall\nRain."})).unwrap();
        let t = d.join("Mine.wst");
        s.run("file.saveAsTemplate", &json!({"path": t.display().to_string()})).unwrap();
        // Opening the template starts a new, untitled script.
        let mut o = session();
        o.run("file.openPath", &json!({"path": t.display().to_string()})).unwrap();
        assert!(o.path.is_none());
        assert!(!o.doc.ext.contains(wordsmith_script::templates::TEMPLATE_KEY));
        // Revert puts back the saved script.
        let a = d.join("A.wss");
        s.run("file.saveAsPath", &json!({"path": a.display().to_string()})).unwrap();
        s.run("text.insert", &json!({"text": " More."})).unwrap();
        s.run("file.revert", &json!({})).unwrap();
        assert_eq!(wordsmith_script::commands::text::body_lines(&s)[1].1, "Rain.");
        // Merge two scripts.
        let b = d.join("B.wss");
        let mut other = session();
        other.run("text.insert", &json!({"text": "ext. yard\nSun."})).unwrap();
        other.run("file.saveAsPath", &json!({"path": b.display().to_string()})).unwrap();
        s.run("file.merge", &json!({"paths": [a.display().to_string(), b.display().to_string()]})).unwrap();
        assert_eq!(wordsmith_script::scenes(&s.doc).len(), 2);
        assert!(s.path.is_none() && s.dirty);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Every command that takes a path keeps to the automation roots: given a path outside them it
    /// refuses through the file gateway and touches nothing there. Printing is refused outright.
    #[test]
    fn confined_commands_keep_to_their_folders() {
        use jcj_engine::file_access::{AutomationRoots, confine};
        let d = dir("confined");
        let (inside, outside) = (d.join("inside"), d.join("outside"));
        std::fs::create_dir_all(&inside).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // A real script outside, so commands that read find a file to refuse.
        let mut other = session();
        other.run("text.insert", &json!({"text": "int. hall\nRain."})).unwrap();
        let script = outside.join("Other.wss");
        other.run("file.saveAsPath", &json!({"path": script.display().to_string()})).unwrap();
        let listing = || {
            let mut l: Vec<_> = std::fs::read_dir(&outside).unwrap().map(|e| e.unwrap().path()).collect();
            l.sort();
            l.into_iter().map(|p| (p.clone(), std::fs::read(&p).unwrap())).collect::<Vec<_>>()
        };
        let before = listing();
        let roots = AutomationRoots::new(Some(&inside), Some(&inside)).unwrap().unwrap();
        let mut s = session();
        s.run("text.insert", &json!({"text": "int. hall\nRain."})).unwrap();
        // Revised text, so the revised-pages exports have pages to write.
        s.run("select.all", &json!({})).unwrap();
        s.run("revision.markRevised", &json!({})).unwrap();
        let ids: Vec<&'static str> = s
            .registry
            .all()
            .iter()
            .filter(|c| c.params.contains("{\"path\"") || c.params.contains(", \"path\"") || c.params.contains("\"paths\""))
            // Script's own commands (the engine's file commands are confined by the engine).
            .filter(|c| specs().iter().chain(wordsmith_script::commands::specs().iter()).any(|o| o.id == c.id))
            .map(|c| c.id)
            .collect();
        for id in ["report.export", "report.exportFilteredPdf", "storyboard.exportPdf", "cards.exportPdf", "file.exportPdf", "file.saveAsPath"] {
            assert!(ids.contains(&id), "{id} is checked");
        }
        for id in &ids {
            let target = if id.contains("open") || id.contains("compare") || id.contains("copyFrom") || id.contains("merge") {
                script.clone()
            } else {
                outside.join(format!("{id}.pdf"))
            };
            let t = target.display().to_string();
            let params = json!({"path": t, "paths": [t, t], "kind": "scenes", "elements": ["action"], "discard": true});
            let result = confine(Some(&roots), || s.run(id, &params));
            let e = result.expect_err(id).to_string();
            assert!(e.contains("automation path rejected"), "{id}: {e}");
        }
        for id in ["file.print", "file.printPreview"] {
            let e = confine(Some(&roots), || s.run(id, &json!({}))).expect_err(id).to_string();
            assert!(e.contains("confined"), "{id}: {e}");
        }
        assert_eq!(listing(), before, "nothing outside the roots changed");
        let _ = std::fs::remove_dir_all(&d);
    }
}
