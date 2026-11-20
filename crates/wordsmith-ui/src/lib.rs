// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface layer for Wordsmith Script: the script editor, its menus and dialogs. Only the app
//! crates may depend on it; `cargo xtask ci` enforces that with its layering check.
//!
//! The interface owns no behavior. Every action, its own included (opening the command palette,
//! copying), is a command in the session's registry, run by id.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod autosave;
pub mod board;
pub mod chrome;
pub mod dialogs;
pub mod editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod files;
pub mod fonts;
pub mod keys;
pub mod prefs;
pub mod structure;
pub mod tables;

use std::sync::Arc;

use serde_json::{Value, json};
use wordsmith_engine::{CmdError, CommandSpec, Registry, Session};

pub use editor::Editor;

/// Plain-text credit required by the reuse rules. Shown in the About window.
pub const UPSTREAM_CREDIT: &str = "based on WordCraft by the ArtCraft team";

/// Text for the About window.
pub fn about_text() -> String {
    format!(
        "{} {}\n{}\nLicensed under MIT OR Apache-2.0. Courier Prime, Inter and JetBrains Mono are under the SIL Open Font License 1.1.",
        wordsmith_script::PRODUCT_NAME,
        wordsmith_script::VERSION,
        UPSTREAM_CREDIT,
    )
}

fn open(s: &mut Session, what: &str) -> Result<Value, CmdError> {
    s.push_ui_request(json!({"open": what}));
    Ok(Value::Null)
}

/// The interface's own commands. They ask the window for something through a UI request, so the
/// palette, menus, keys and agents reach them the same way.
pub fn ui_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("view.commandPalette", "Command Palette", "View", |s, _| open(s, "palette")).key("Mod+Shift+P").pure(),
        CommandSpec::new("view.normal", "Normal View", "View", |s, _| open(s, "view.normal")).key("Mod+Alt+1").pure(),
        CommandSpec::new("view.page", "Page View", "View", |s, _| open(s, "view.page")).key("Mod+Alt+2").pure(),
        CommandSpec::new("view.speed", "Speed View", "View", |s, _| open(s, "view.speed")).key("Mod+Alt+3").pure(),
        CommandSpec::new("view.note", "Note…", "Edit", |s, _| open(s, "note")).key("Mod+Alt+N").pure(),
        CommandSpec::new("view.board", "Story Board", "View", |s, _| open(s, "view.board")).key("Mod+Alt+6").pure(),
        CommandSpec::new("view.tables", "Tables", "View", |s, _| open(s, "view.tables")).key("Mod+Alt+T").pure(),
        CommandSpec::new("view.cards", "Index Cards", "View", |s, _| open(s, "view.cards")).key("Mod+Alt+4").pure(),
        CommandSpec::new("view.scenes", "Scene View", "View", |s, _| open(s, "view.scenes")).key("Mod+Alt+5").pure(),
        CommandSpec::new("view.darkTheme", "Dark Theme", "View", |s, _| open(s, "view.dark")).pure(),
        CommandSpec::new("view.typewriter", "Typewriter Scrolling", "View", |s, _| open(s, "view.typewriter")).pure(),
        CommandSpec::new("view.focusMode", "Focus Mode", "View", |s, _| open(s, "view.focus")).key("Mod+Shift+F").pure(),
        CommandSpec::new("view.find", "Find and Replace…", "Edit", |s, _| open(s, "find")).key("Mod+F").pure(),
        CommandSpec::new("view.elementSettings", "Element Settings…", "Format › Elements", |s, _| open(s, "elementSettings")).pure(),
        CommandSpec::new("view.titlePage", "Title Page…", "Format", |s, _| open(s, "titlePage")).pure(),
        CommandSpec::new("view.revisions", "Revision Sets…", "Production", |s, _| open(s, "revisions")).pure(),
        CommandSpec::new("view.headerFooter", "Header and Footer…", "Format", |s, _| open(s, "headerFooter")).pure(),
        CommandSpec::new("help.about", "About Wordsmith Script", "Help", |s, _| open(s, "about")).pure(),
        CommandSpec::new("file.exportPdfAs", "Export PDF…", "File", |s, _| open(s, "exportPdf")).key("Mod+Shift+E").pure(),
        CommandSpec::new("file.recoverPrompt", "Recover Unsaved Scripts…", "File", |s, _| open(s, "recover")).pure(),
        CommandSpec::new("app.preferences", "Preferences…", "File", |s, _| open(s, "preferences")).key("Mod+,").pure(),
        CommandSpec::new("edit.copy", "Copy", "Edit", |s, _| {
            let text = s.selected_text();
            s.push_ui_request(json!({"copy": text}));
            Ok(Value::Null)
        })
        .key("Mod+C")
        .when(|s| if s.sel.is_collapsed() { Some("nothing is selected") } else { None })
        .pure(),
        CommandSpec::new("edit.cut", "Cut", "Edit", |s, v| {
            let text = s.selected_text();
            s.push_ui_request(json!({"copy": text}));
            wordsmith_script::commands::text::backspace(s, v)
        })
        .key("Mod+X")
        .when(|s| if s.sel.is_collapsed() { Some("nothing is selected") } else { None }),
    ]
}

/// Every command: engine, screenplay and interface.
pub fn registry() -> Registry {
    let mut specs = wordsmith_script::commands::specs();
    specs.extend(ui_specs());
    #[cfg(not(target_arch = "wasm32"))]
    specs.extend(files::specs());
    wordsmith_engine::cmd::registry_with(specs)
}

/// A session on a new script with every command, for a window.
pub fn new_session() -> Session {
    Session::with_registry(wordsmith_script::new_script(), Arc::new(registry()))
}

#[cfg(test)]
mod tests;
