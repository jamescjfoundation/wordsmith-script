// SPDX-License-Identifier: MIT OR Apache-2.0
//! The screenplay commands. Every user action is a command with an id in one registry; the
//! menus, shortcuts, command palette, CLI and MCP server all run them through
//! [`jcj_engine::Session::run`].

use std::sync::Arc;

use jcj_engine::{CmdResult, CommandSpec, Registry, Session, cmd::pos_json};
use serde_json::{Value, json};

pub mod aliases;
pub mod autocomplete;
pub mod character;
pub mod editing;
pub mod element;
pub mod find;
pub mod format;
pub mod scene;
pub mod settings;
pub mod spelling;
pub mod text;

use crate::element::Elements;

/// Most paragraphs `document.inspect` lists. Bounds replies for agents.
pub const MAX_INSPECT_PARAGRAPHS: usize = 20_000;

/// Every command: the engine's, the core's general commands and shared document features
/// (`jcj-features`), and the screenplay layer's, which replace any of theirs with the same id.
///
/// Script's keys win: a command below Script's whose shortcut a Script command also uses loses
/// that shortcut (it stays reachable by id and from the palette). Script's element keys (Mod+0 to
/// Mod+9) and the rest of its keyboard are as they were before 0.2.
pub fn registry() -> Registry {
    registry_plus(Vec::new())
}

/// [`registry`] with more of the app's own commands (the file commands of `wordsmith-script-format`),
/// which, like Script's, replace commands below them with the same id and win their keys.
pub fn registry_plus(extra: Vec<CommandSpec>) -> Registry {
    let mut own = specs();
    own.extend(extra);
    let mut app: Vec<CommandSpec> = jcj_features::commands().into_iter().filter(|f| !own.iter().any(|o| o.id == f.id)).collect();
    app.extend(own.iter().cloned());
    let app = jcj_engine::commands::merged(app);
    let ours: std::collections::HashSet<&str> = own
        .iter()
        .flat_map(|s| s.shortcut.split(" / "))
        .chain(SCRIPT_KEYS.iter().map(|(_, k)| *k))
        .filter(|k| !k.is_empty())
        .map(|k| k.trim())
        .collect();
    let ours: std::collections::HashSet<String> = ours.into_iter().map(str::to_ascii_lowercase).collect();
    let script_ids: std::collections::HashSet<&str> = own.iter().map(|s| s.id).collect();
    let mut all: Vec<CommandSpec> = jcj_engine::engine_specs().into_iter().filter(|e| !app.iter().any(|a| a.id == e.id)).collect();
    all.extend(app);
    for spec in &mut all {
        if !script_ids.contains(spec.id) && spec.shortcut.split(" / ").any(|k| ours.contains(&k.trim().to_ascii_lowercase())) {
            let kept: Vec<&str> = spec.shortcut.split(" / ").filter(|k| !ours.contains(&k.trim().to_ascii_lowercase())).collect();
            spec.shortcut = leak_shortcut(kept.join(" / "));
        }
    }
    for spec in &mut all {
        if let Some((_, key)) = SCRIPT_KEYS.iter().find(|(id, _)| *id == spec.id) {
            spec.shortcut = key;
        }
    }
    Registry::new(all)
}

/// Keys Script gives commands that are not its own (the shared features', the core's and the
/// engine's): the keys these had in 0.1, when Script registered the commands itself.
pub const SCRIPT_KEYS: [(&str, &str); 6] = [
    ("bookmark.add", "Mod+M"),
    ("track.toggle", "Mod+Shift+C"),
    ("format.plain", "Mod+Shift+T"),
    ("format.toggleCapitals", "Mod+Shift+K"),
    ("note.next", "Mod+Alt+Down"),
    ("note.previous", "Mod+Alt+Up"),
];

/// A shortcut string that lives as long as the registry's specs (`&'static str`). Built once per
/// distinct text, so the few strings this makes are not repeated.
fn leak_shortcut(text: String) -> &'static str {
    static MADE: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    if text.is_empty() {
        return "";
    }
    let Ok(mut made) = MADE.lock() else { return "" };
    if let Some(s) = made.iter().find(|s| **s == text) {
        return s;
    }
    let s: &'static str = Box::leak(text.into_boxed_str());
    made.push(s);
    s
}

/// The shared features' commands Script does not replace, then Script's own.
pub fn app_specs() -> Vec<CommandSpec> {
    let ours = specs();
    let mut all: Vec<CommandSpec> = jcj_features::commands().into_iter().filter(|f| !ours.iter().any(|o| o.id == f.id)).collect();
    all.extend(ours);
    all
}

/// The registry every session shares (built once).
pub fn shared_registry() -> Arc<Registry> {
    static REGISTRY: std::sync::OnceLock<Arc<Registry>> = std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            crate::paginate::plan_pages_for_features();
            Arc::new(registry())
        })
        .clone()
}

/// A session on a new, empty script with every command registered.
pub fn new_session() -> Session {
    session_for(crate::template::new_script())
}

/// A session on `doc` with every command registered.
pub fn session_for(doc: jcj_doc::Document) -> Session {
    crate::fonts::ensure();
    jcj_engine::session_with(doc, shared_registry())
}

/// The screenplay layer's commands.
pub fn specs() -> Vec<CommandSpec> {
    let mut all = vec![
        CommandSpec::new("text.insert", "Typing", "Edit", text::insert).params(r#"{"text": string (line breaks act as Return)}"#),
        CommandSpec::new("text.newParagraph", "New Paragraph", "Edit", text::new_paragraph).key("Enter"),
        CommandSpec::new("text.tab", "Tab", "Edit", text::tab).key("Tab"),
        CommandSpec::new("text.backspace", "Delete Back", "Edit", text::backspace).key("Backspace"),
        CommandSpec::new("text.delete", "Delete", "Edit", text::delete).key("Delete"),
        CommandSpec::new("document.inspect", "Inspect Script", "Script", inspect).pure(),
        CommandSpec::new("document.text", "Script Text", "Script", |s, _| Ok(json!(s.doc.plain_text(jcj_doc::StoryRef::Body)))).pure(),
        CommandSpec::new("document.pages", "Page Count", "Script", |s, _| {
            let pag = crate::paginate::paginate(&s.doc);
            Ok(json!({"pages": pag.pages.len(), "labels": pag.labels}))
        })
        .pure(),
        CommandSpec::new("file.new", "New Script", "File", |s, _| {
            s.set_document(crate::template::new_script());
            Ok(Value::Null)
        })
        .key("Mod+N"),
    ];
    all.extend(element::specs());
    all.extend(settings::specs());
    all.extend(crate::export::specs());
    all.extend(crate::paginate::specs());
    all.extend(autocomplete::specs());
    all.extend(spelling::specs());
    all.extend(find::specs());
    all.extend(crate::title_page::specs());
    all.extend(scene::specs());
    all.extend(character::specs());
    all.extend(crate::scene_numbers::specs());
    all.extend(crate::reports::specs());
    all.extend(crate::storyboard::specs());
    all.extend(crate::outline::specs());
    all.extend(editing::specs());
    all.extend(crate::checker::specs());
    all.extend(crate::typing::specs());
    all.extend(crate::macros::specs());
    all.extend(format::specs());
    all.extend(crate::scene_data::specs());
    all.extend(crate::cast::specs());
    all.extend(crate::reports_more::specs());
    all.extend(character::table_tab_specs());
    all.extend(crate::strip::specs());
    all.extend(editing::dual_specs());
    all.extend(crate::templates::specs());
    all.extend(aliases::specs());
    all
}

/// `document.inspect`: the script's paragraphs (element and text), scenes and selection.
fn inspect(s: &mut Session, _: &Value) -> CmdResult {
    let lines = text::body_lines(s);
    let truncated = lines.len() > MAX_INSPECT_PARAGRAPHS;
    let paragraphs: Vec<Value> =
        lines.into_iter().take(MAX_INSPECT_PARAGRAPHS).map(|(element, text)| json!({"element": element, "text": text})).collect();
    let scenes: Vec<Value> =
        crate::scenes::scenes(&s.doc).into_iter().map(|sc| json!({"heading": sc.heading, "start": sc.start, "end": sc.end})).collect();
    let current = element::element_of_para_at(&s.doc, &Elements::of_doc(&s.doc), &s.sel.focus).id.clone();
    Ok(json!({
        "paragraphs": paragraphs,
        "truncated": truncated,
        "scenes": scenes,
        "element": current,
        "selection": {"anchor": pos_json(&s.sel.anchor), "focus": pos_json(&s.sel.focus)},
    }))
}

#[cfg(test)]
mod tests;
