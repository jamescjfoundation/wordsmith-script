// SPDX-License-Identifier: MIT OR Apache-2.0
//! The editor driven headlessly through egui input, the way a window drives it.

use egui::{Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};
use serde_json::json;

use crate::dialogs::{SettingsDialog, palette_matches};
use crate::editor::Editor;

fn ctx() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::fonts::definitions());
    ctx
}

fn frame(ctx: &egui::Context, ed: &mut Editor, events: Vec<Event>) {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), events, ..Default::default() };
    let mut out = ctx.run_ui(input, |ui| ed.ui(ui));
    // No GPU here to take the page textures.
    out.textures_delta.clear();
}

fn key(k: Key, modifiers: Modifiers) -> Event {
    Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn text(t: &str) -> Event {
    Event::Text(t.to_string())
}

fn lines(ed: &Editor) -> Vec<(String, String)> {
    wordsmith_script::commands::text::body_lines(&ed.session)
}

fn l(e: &str, t: &str) -> (String, String) {
    (e.to_string(), t.to_string())
}

/// An editor with portable (Windows and Linux) key conventions, so tests read the same on every
/// platform.
fn editor() -> Editor {
    let mut ed = Editor::new(crate::new_session());
    ed.mac = false;
    ed
}

fn cmd() -> Modifiers {
    Modifiers { command: true, ctrl: true, ..Modifiers::NONE }
}

#[test]
fn a_scene_typed_on_the_keyboard() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("kitchen"), key(Key::Tab, Modifiers::NONE), text("night")]);
    frame(
        &ctx,
        &mut ed,
        vec![key(Key::Enter, Modifiers::NONE), text("Rain."), key(Key::Enter, Modifiers::NONE), key(Key::Tab, Modifiers::NONE), text("anna")],
    );
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE), text("Hello.")]);
    assert_eq!(lines(&ed), vec![l("SceneHeading", "INT. KITCHEN - NIGHT"), l("Action", "Rain."), l("Character", "ANNA"), l("Dialogue", "Hello.")]);
    // Mod+Z undoes the typing; Mod+3 makes the paragraph a Character.
    frame(&ctx, &mut ed, vec![key(Key::Z, cmd())]);
    assert_eq!(lines(&ed).last(), Some(&l("Dialogue", "")));
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd())]);
    assert_eq!(lines(&ed).last(), Some(&l("Action", "")));
}

#[test]
fn copy_and_cut_go_through_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("Hello world")]);
    frame(&ctx, &mut ed, vec![key(Key::A, cmd()), Event::Copy]);
    assert_eq!(lines(&ed), vec![l("Action", "Hello world")]);
    frame(&ctx, &mut ed, vec![Event::Cut]);
    assert_eq!(lines(&ed), vec![l("Action", "")]);
    frame(&ctx, &mut ed, vec![Event::Paste("Back again".into())]);
    assert_eq!(lines(&ed), vec![l("Action", "Back again")]);
    // Copy with nothing selected is refused and says why.
    frame(&ctx, &mut ed, vec![Event::Copy]);
    assert!(ed.message.contains("nothing is selected"), "{}", ed.message);
}

#[test]
fn a_custom_element_shortcut_reaches_it() {
    let ctx = ctx();
    let mut ed = editor();
    ed.session.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+L"})).unwrap();
    frame(&ctx, &mut ed, vec![key(Key::L, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(lines(&ed)[0].0, "Custom1");
}

#[test]
fn dialogs_open_through_commands_and_take_the_keyboard() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::P, Modifiers { shift: true, ..cmd() })]);
    assert!(ed.palette.is_some());
    // Typing goes to the palette, not the script.
    frame(&ctx, &mut ed, vec![text("x")]);
    frame(&ctx, &mut ed, vec![text("y")]);
    assert_eq!(lines(&ed), vec![l("SceneHeading", "")]);
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(ed.palette.is_none());
    ed.run("view.elementSettings", json!({}));
    assert_eq!(ed.settings.as_ref().map(|d| d.element.as_str()), Some("SceneHeading"));
    ed.run("help.about", json!({}));
    assert!(ed.show_about);
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn the_palette_finds_commands_by_any_word() {
    let r = crate::registry();
    let ids = |q: &str| palette_matches(&r, q).into_iter().map(|m| m.0).collect::<Vec<_>>();
    assert!(ids("scene heading").contains(&"element.sceneHeading"));
    assert!(ids("undo").contains(&"edit.undo"));
    assert!(!ids("").contains(&"element.set"), "commands that need parameters are left out");
    assert!(ids("zzzz no such").is_empty());
}

#[test]
fn the_settings_dialog_sends_only_what_changed() {
    let mut s = crate::new_session();
    let mut d = SettingsDialog::new(&mut s, "Dialogue");
    assert_eq!(d.changes(), json!({"element": "Dialogue"}));
    d.form["spaceBefore"] = json!(1.0);
    d.form["bold"] = json!(true);
    assert_eq!(d.changes(), json!({"element": "Dialogue", "spaceBefore": 1.0, "bold": true}));
    s.run("element.configure", &d.changes()).unwrap();
    assert_eq!(s.run("element.settings", &json!({"element": "Dialogue"})).unwrap()["bold"], true);
}

#[test]
fn every_screenplay_and_ui_command_is_in_one_registry() {
    let r = crate::registry();
    for id in ["edit.undo", "file.recover", "text.tab", "element.dialogue", "element.configure", "view.commandPalette", "edit.copy", "help.about"] {
        assert!(r.get(id).is_some(), "{id}");
    }
    let mut seen = std::collections::HashSet::new();
    for spec in r.all() {
        assert!(seen.insert(spec.id), "{} registered twice", spec.id);
    }
}

#[test]
fn about_carries_the_credit() {
    let text = crate::about_text();
    assert!(text.starts_with("Wordsmith Script "));
    assert!(text.contains(crate::UPSTREAM_CREDIT));
}
