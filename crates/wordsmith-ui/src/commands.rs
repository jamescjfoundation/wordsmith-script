// SPDX-License-Identifier: MIT OR Apache-2.0
//! The commands as the window knows them: the registry (the engine's, the core's, the shared
//! features', Script's and these interface commands), the ones the window shows (in the command
//! palette, on the Keys page, bound to keys), and what the shell shows of each
//! ([`jcj_shell::Commands`]).
//!
//! The registry also holds the whole word-processor catalog of the engine the core builds on;
//! Script shows only its own commands and the general ones it uses ([`shown`]). Custom elements'
//! and macros' own keys are commands of their own here (`element.custom.<id>`,
//! `macro.slot.<n>`), so the shell binds their keys and lists them like any other.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use jcj_engine::{CmdError, CommandSpec, Registry, Session};
use jcj_shell::{CommandInfo, Commands};
use serde_json::{Value, json};
use wordsmith_script::{ElementKind, Elements};

use crate::editor::Editor;

fn open(s: &mut Session, what: &str) -> Result<Value, CmdError> {
    s.push_ui_request(json!({"open": what}));
    Ok(Value::Null)
}

/// Ask the window for `what`, passing on the parameters named in `keep` that the command was
/// given.
fn open_with(s: &mut Session, what: &str, params: &Value, keep: &[&str]) -> Result<Value, CmdError> {
    let mut req = json!({"open": what});
    for k in keep {
        if let Some(v) = params.get(*k) {
            req[*k] = v.clone();
        }
    }
    s.push_ui_request(req);
    Ok(Value::Null)
}

fn zoom(s: &mut Session, by: &str) -> Result<Value, CmdError> {
    s.push_ui_request(json!({"open": "tool.zoom", "by": by}));
    Ok(Value::Null)
}

/// The interface's own commands. They ask the window for something through a UI request, so the
/// palette, menus, keys and agents reach them the same way.
pub fn ui_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("view.commandPalette", "Command Palette", "View", |s, _| open(s, "palette")).key("Mod+Shift+P").pure(),
        CommandSpec::new("view.normal", "Normal View", "View", |s, _| open(s, "view.normal")).key("Mod+Alt+1").pure(),
        CommandSpec::new("view.page", "Page View", "View", |s, _| open(s, "view.page")).key("Mod+Alt+2").pure(),
        CommandSpec::new("view.speed", "Draft View", "View", |s, _| open(s, "view.speed")).key("Mod+Alt+3").pure(),
        CommandSpec::new("view.note", "Note…", "Edit", |s, _| open(s, "note")).key("Mod+Alt+N").pure(),
        CommandSpec::new("view.board", "Story Board", "View", |s, _| open(s, "view.board")).key("Mod+Alt+6").pure(),
        CommandSpec::new("view.outline", "Outline", "View", |s, _| open(s, "view.outline")).key("Mod+Alt+7").pure(),
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
        CommandSpec::new("help.checkForUpdates", "Check for Updates…", "Help", |s, _| open(s, "tool.checkUpdates")).pure(),
        CommandSpec::new("file.exportPdfAs", "Export PDF…", "File", |s, _| open(s, "exportPdf")).key("Mod+Shift+E").pure(),
        CommandSpec::new("file.recoverPrompt", "Recover Unsaved Scripts…", "File", |s, _| open(s, "recover")).pure(),
        CommandSpec::new("view.goto", "Go To…", "Go", |s, _| open(s, "tool.goto")).key("Mod+G").pure(),
        CommandSpec::new("view.bookmarks", "Bookmarks…", "Go", |s, _| open(s, "tool.bookmarks")).key("Mod+Shift+M").pure(),
        CommandSpec::new("view.checker", "Check Format…", "Tools", |s, _| open(s, "tool.checker")).pure(),
        CommandSpec::new("view.macros", "Macros…", "Tools", |s, _| open(s, "tool.macros")).pure(),
        CommandSpec::new("view.reports", "Reports…", "Tools", |s, v| open_with(s, "tool.reports", v, &["kind"])).key("Mod+Shift+J").pure(),
        CommandSpec::new("view.sceneProperties", "Scene Properties…", "Script", |s, _| open(s, "tool.sceneProperties")).pure(),
        CommandSpec::new("view.character", "Character Details…", "Script", |s, _| open(s, "tool.character")).pure(),
        CommandSpec::new("view.highlights", "Show Character Highlights", "View", |s, _| open(s, "tool.highlights")).pure(),
        CommandSpec::new("view.strip", "Structure Strip", "View", |s, _| open(s, "tool.strip")).key("Mod+Alt+S").pure(),
        CommandSpec::new("view.print", "Print…", "File", |s, _| open(s, "tool.print")).key("Mod+Alt+Shift+P").pure(),
        CommandSpec::new("view.watermark", "Watermark…", "Production", |s, _| open(s, "tool.watermark")).pure(),
        CommandSpec::new("view.sceneNumbers", "Scene Numbers…", "Production", |s, _| open(s, "tool.sceneNumbers")).pure(),
        CommandSpec::new("view.start", "Welcome…", "File", |s, _| open(s, "tool.start")).pure(),
        CommandSpec::new("view.templates", "New from Template…", "File", |s, _| open(s, "tool.templates")).key("Mod+Shift+O").pure(),
        CommandSpec::new("view.keys", "Keys…", "File", |s, _| open(s, "tool.keys")).pure(),
        CommandSpec::new("view.stats", "Writing Statistics…", "Tools", |s, _| open(s, "tool.stats")).pure(),
        CommandSpec::new("view.sprint", "Start or Stop a Sprint", "Tools", |s, _| open(s, "tool.sprint")).pure(),
        CommandSpec::new("view.extraDark", "Extra Dark Theme", "View", |s, _| open(s, "tool.extraDark")).pure(),
        CommandSpec::new("file.newTab", "New Tab", "File", |s, _| open(s, "tool.newTab")).key("Mod+T").pure(),
        CommandSpec::new("file.closeTab", "Close Tab", "File", |s, _| open(s, "tool.closeTab")).key("Mod+W").pure(),
        CommandSpec::new("view.nextTab", "Next Tab", "View", |s, _| open(s, "tool.nextTab")).key("Mod+Alt+Right").pure(),
        CommandSpec::new("view.zoomIn", "Zoom In", "View", |s, _| zoom(s, "in")).pure(),
        CommandSpec::new("view.zoomOut", "Zoom Out", "View", |s, _| zoom(s, "out")).pure(),
        CommandSpec::new("view.zoomReset", "Actual Size", "View", |s, _| zoom(s, "reset")).pure(),
        CommandSpec::new("view.zoomTo", "Zoom To", "View", |s, v| {
            s.push_ui_request(json!({"open": "tool.zoom", "by": "to", "percent": v.get("percent").cloned().unwrap_or(json!(100))}));
            Ok(Value::Null)
        })
        .params(r#"{"percent": number}"#)
        .pure(),
        CommandSpec::new("view.spelling", "Spelling Underlines", "View", |s, _| open(s, "tool.spelling")).pure(),
        CommandSpec::new("view.scriptPanel", "Script Panel", "View", |s, v| open_with(s, "tool.scriptPanel", v, &["panel"])).pure(),
        CommandSpec::new("view.inspector", "Inspector", "View", |s, v| open_with(s, "tool.inspector", v, &["panel"])).pure(),
        CommandSpec::new("view.lightTheme", "Light Theme", "View", |s, _| open(s, "tool.lightTheme")).pure(),
        CommandSpec::new("view.menuBar", "Menu Bar", "View", |s, _| open(s, "tool.menuBar")).pure(),
        CommandSpec::new("cards.print", "Print Index Cards…", "File", |s, _| open(s, "tool.cardsPrint")).pure(),
        CommandSpec::new("cards.perRow", "Index Cards per Row", "View › Index Cards", |s, v| open_with(s, "tool.cardsPerRow", v, &["count"]))
            .params(r#"{"count": 0-9}"#)
            .pure(),
        CommandSpec::new("cards.scriptText", "Script Text on Index Cards", "View › Index Cards", |s, v| {
            open_with(s, "tool.cardsText", v, &["show"])
        })
        .pure(),
        CommandSpec::new("view.splitCards", "Split: Index Cards", "View › Split", |s, _| open(s, "tool.split.cards")).pure(),
        CommandSpec::new("view.splitBoard", "Split: Story Board", "View › Split", |s, _| open(s, "tool.split.board")).pure(),
        CommandSpec::new("view.splitOutline", "Split: Outline", "View › Split", |s, _| open(s, "tool.split.outline")).pure(),
        CommandSpec::new("view.splitScenes", "Split: Scene View", "View › Split", |s, _| open(s, "tool.split.scenes")).pure(),
        CommandSpec::new("view.splitScript", "Split: Script", "View › Split", |s, _| open(s, "tool.split.script")).pure(),
        CommandSpec::new("view.splitBelow", "Split Below or Beside", "View › Split", |s, _| open(s, "tool.split.below")).pure(),
        CommandSpec::new("view.splitSwap", "Swap Panes", "View › Split", |s, _| open(s, "tool.split.swap")).pure(),
        CommandSpec::new("view.unsplit", "Unsplit", "View › Split", |s, _| open(s, "tool.split.off")).pure(),
        CommandSpec::new("view.reformat", "Reformat…", "Tools", |s, _| open(s, "tool.reformat")).pure(),
        CommandSpec::new("app.preferences", "Preferences…", "File", |s, _| open(s, "preferences")).key("Mod+,").pure(),
        CommandSpec::new("edit.copy", "Copy", "Edit", |s, _| {
            let text = s.selected_text();
            s.push_ui_request(json!({"copy": text}));
            Ok(Value::Null)
        })
        .key("Mod+C")
        .when(|s| if s.sel.is_collapsed() { Some("nothing is selected") } else { None })
        .pure(),
        // The window answers by reading the clipboard and sending its text as a paste, which
        // goes in like any paste from the keyboard.
        CommandSpec::new("edit.paste", "Paste", "Edit", |s, _| {
            s.push_ui_request(json!({"paste": true}));
            Ok(Value::Null)
        })
        .key("Mod+V")
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

/// The interface's commands and, on the desktop, the file commands.
fn window_specs() -> Vec<CommandSpec> {
    let mut specs = ui_specs();
    #[cfg(not(target_arch = "wasm32"))]
    specs.extend(wordsmith_format::commands::specs());
    specs
}

/// Every command: the engine's, the core's, the shared features', Script's and the interface's.
pub fn registry() -> Registry {
    wordsmith_script::commands::registry_plus(window_specs())
}

/// The registry every window's sessions share (built once).
pub fn shared_registry() -> Arc<Registry> {
    static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            wordsmith_script::paginate::plan_pages_for_features();
            Arc::new(registry())
        })
        .clone()
}

/// A session on a new script with every command, for a window.
pub fn new_session() -> Session {
    wordsmith_script::fonts::ensure();
    jcj_engine::session_with(wordsmith_script::new_script(), shared_registry())
}

/// The general commands of the engine Script's interface uses (besides every `caret.` and
/// `select.` command): the rest of the engine's word-processor catalog stays out of sight.
const ENGINE: [&str; 14] = [
    "edit.undo",
    "edit.redo",
    "file.recover",
    "format.bold",
    "format.italic",
    "format.underline",
    "format.strikethrough",
    "format.doubleStrikethrough",
    "format.superscript",
    "format.subscript",
    "format.smallCaps",
    "format.allCaps",
    "format.changeCase",
    "review.addToDictionary",
];

/// Whether the window shows command `id`: in the palette, on the Keys page, and on a key.
pub fn shown(id: &str) -> bool {
    static SHOWN: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let set = SHOWN.get_or_init(|| {
        let mut set: HashSet<&'static str> = wordsmith_script::commands::app_specs().iter().map(|s| s.id).collect();
        set.extend(window_specs().iter().map(|s| s.id));
        set.extend(jcj_engine::commands::specs().iter().map(|s| s.id));
        set.extend(ENGINE);
        set
    });
    set.contains(id) || ((id.starts_with("caret.") || id.starts_with("select.")) && !id.contains("Cell") && !id.contains("Column"))
}

/// The id of the command that runs macro `slot` (1-based).
pub fn macro_id(slot: usize) -> String {
    format!("macro.slot.{slot}")
}

/// The macro slot (1-based) command `id` runs, if it is a macro's command.
pub fn macro_slot(id: &str) -> Option<usize> {
    id.strip_prefix("macro.slot.")?.parse().ok().filter(|n| *n >= 1)
}

/// The id of the command that sets custom element `element`.
pub fn element_id(element: &str) -> String {
    format!("element.custom.{element}")
}

/// The custom element command `id` sets, if it is one.
pub fn custom_element(id: &str) -> Option<&str> {
    id.strip_prefix("element.custom.").filter(|e| !e.is_empty())
}

/// What the shell shows of `spec`.
pub fn info(spec: &CommandSpec) -> CommandInfo {
    CommandInfo {
        id: spec.id.into(),
        label: spec.label.into(),
        shortcut: spec.shortcut.into(),
        location: spec.location.into(),
        icon: crate::script_icons::generic(crate::script_icons::for_command(spec.id)).into(),
        takes_params: spec.params != "{}",
    }
}

impl Editor {
    /// Custom elements' and macros' commands, with their own keys.
    fn own_key_commands(&self) -> Vec<CommandInfo> {
        let mut out = Vec::new();
        for d in Elements::of_doc(&self.session.doc).all().iter().filter(|d| d.kind == ElementKind::Custom) {
            out.push(CommandInfo {
                location: "Format › Elements".into(),
                icon: "styles".into(),
                ..CommandInfo::new(element_id(&d.id), d.name.clone(), d.shortcut.clone().unwrap_or_default())
            });
        }
        let macros = wordsmith_script::macros::Macros::of_doc(&self.session.doc);
        for (i, m) in macros.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
            out.push(CommandInfo {
                location: "Tools › Macros".into(),
                icon: "macros".into(),
                ..CommandInfo::new(macro_id(i + 1), format!("{}. {}", i + 1, m.name), m.shortcut.clone().unwrap_or_default())
            });
        }
        out
    }
}

impl Commands for Editor {
    fn command(&self, id: &str) -> Option<CommandInfo> {
        if let Some(spec) = self.session.registry.get(id) {
            return Some(info(spec));
        }
        if macro_slot(id).is_some() || custom_element(id).is_some() {
            return self.own_key_commands().into_iter().find(|c| c.id == id);
        }
        None
    }

    fn why_disabled(&self, id: &str) -> Option<String> {
        self.session.why_disabled(id).map(str::to_string)
    }

    fn all_commands(&self) -> Vec<CommandInfo> {
        // Custom elements' and macros' keys come first: they win a key they share.
        let mut all = self.own_key_commands();
        all.extend(self.session.registry.all().iter().filter(|s| shown(s.id)).map(info));
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_key_commands_name_their_targets() {
        assert_eq!(macro_slot(&macro_id(3)), Some(3));
        assert_eq!(macro_slot("macro.slot.0"), None);
        assert_eq!(custom_element(&element_id("Custom1")), Some("Custom1"));
        assert_eq!(custom_element("element.custom."), None);
    }

    #[test]
    fn the_window_shows_script_s_commands_not_the_whole_engine() {
        assert!(shown("element.sceneHeading") && shown("file.save") && shown("view.commandPalette") && shown("edit.undo"));
        assert!(shown("caret.home") && shown("find.replace") && shown("bookmark.add"));
        assert!(!shown("table.insertRowAbove") && !shown("mailings.startMerge"));
    }
}
