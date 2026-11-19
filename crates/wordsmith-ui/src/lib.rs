// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface layer for Wordsmith Script: the script editor, its menus and dialogs. Only the app
//! crates may depend on it; `cargo xtask ci` enforces that with its layering check.
//!
//! The interface owns no behavior. Every action, its own included (opening the command palette,
//! copying), is a command in the session's registry, run by id.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod chrome;
pub mod dialogs;
pub mod editor;
pub mod fonts;
pub mod keys;

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
        CommandSpec::new("view.elementSettings", "Element Settings…", "Format › Elements", |s, _| open(s, "elementSettings")).pure(),
        CommandSpec::new("help.about", "About Wordsmith Script", "Help", |s, _| open(s, "about")).pure(),
        CommandSpec::new("file.recoverPrompt", "Recover Unsaved Scripts…", "File", |s, _| open(s, "recover")).pure(),
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
    wordsmith_engine::cmd::registry_with(specs)
}

/// A session on a new script with every command, for a window.
pub fn new_session() -> Session {
    Session::with_registry(wordsmith_script::new_script(), Arc::new(registry()))
}

#[cfg(test)]
mod tests;
