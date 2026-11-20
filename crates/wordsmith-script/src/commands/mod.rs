// SPDX-License-Identifier: MIT OR Apache-2.0
//! The screenplay commands. Every user action is a command with an id in one registry; the
//! menus, shortcuts, command palette, CLI and MCP server all run them through
//! [`wordsmith_engine::Session::run`].

use std::sync::Arc;

use serde_json::{Value, json};
use wordsmith_engine::{CmdResult, CommandSpec, Registry, Session, cmd::pos_json};

pub mod autocomplete;
pub mod caret;
pub mod character;
pub mod element;
pub mod find;
pub mod scene;
pub mod settings;
pub mod spelling;
pub mod text;

use crate::element::Elements;

/// Most paragraphs `document.inspect` lists. Bounds replies for agents.
pub const MAX_INSPECT_PARAGRAPHS: usize = 20_000;

/// Every command: the engine's (undo, redo, recovery) and the screenplay layer's.
pub fn registry() -> Registry {
    wordsmith_engine::cmd::registry_with(specs())
}

/// A session on a new, empty script with every command registered.
pub fn new_session() -> Session {
    Session::with_registry(crate::template::new_script(), Arc::new(registry()))
}

/// A session on `doc` with every command registered.
pub fn session_for(doc: wordsmith_doc::Document) -> Session {
    Session::with_registry(doc, Arc::new(registry()))
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
        CommandSpec::new("document.text", "Script Text", "Script", |s, _| Ok(json!(s.doc.plain_text(wordsmith_doc::StoryRef::Body)))).pure(),
        CommandSpec::new("file.new", "New Script", "File", |s, _| {
            s.set_document(crate::template::new_script());
            Ok(Value::Null)
        })
        .key("Mod+N"),
    ];
    all.extend(element::specs());
    all.extend(settings::specs());
    all.extend(caret::specs());
    all.extend(crate::export::specs());
    all.extend(crate::paginate::specs());
    all.extend(autocomplete::specs());
    all.extend(spelling::specs());
    all.extend(find::specs());
    all.extend(crate::title_page::specs());
    all.extend(crate::header_footer::specs());
    all.extend(scene::specs());
    all.extend(character::specs());
    all.extend(crate::notes::specs());
    all.extend(crate::revisions::specs());
    all.extend(crate::scene_numbers::specs());
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
