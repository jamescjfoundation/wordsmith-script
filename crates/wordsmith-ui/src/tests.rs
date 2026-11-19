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

/// A script of `scenes` scenes, about half a page each.
fn long_script(scenes: usize) -> wordsmith_doc::Document {
    let mut doc = wordsmith_script::new_script();
    let mut body = Vec::new();
    for n in 0..scenes {
        let mut push = |e: &str, t: String| body.push(wordsmith_doc::para_block(wordsmith_script::template::paragraph(e, &t)));
        push("SceneHeading", format!("INT. ROOM {n} - DAY"));
        push("Action", "The light is low and the room is quiet. Somebody has left the radio on, very softly.".into());
        for k in 0..4 {
            push("Character", if k % 2 == 0 { "ANNA".into() } else { "BOB".into() });
            push("Dialogue", "I keep thinking we missed something obvious. Tell me again what you saw.".into());
        }
        push("Transition", "CUT TO:".into());
    }
    doc.body = body;
    doc
}

#[test]
fn the_page_view_shows_the_paginated_script() {
    let ctx = ctx();
    let mut ed = Editor::new(wordsmith_script::session_for(long_script(6)));
    ed.session.registry = std::sync::Arc::new(crate::registry());
    frame(&ctx, &mut ed, vec![]);
    let pag = ed.paginated();
    assert!(pag.pages.len() >= 3, "{} pages", pag.pages.len());
    assert!(pag.matches_plan);
    // The caret at the end of the script shows on the last page.
    ed.run("caret.docEnd", json!({}));
    let at = pag.to_output(&ed.session.sel.focus).unwrap();
    assert_eq!(pag.layout.caret(&at).map(|c| c.page), Some(pag.pages.len() - 1));
}

/// Typing in a 150-page script: one frame with a typed character, which runs the command,
/// paginates again and draws the visible pages. The kickoff asks for the number to be recorded
/// (docs/performance.md).
#[test]
#[ignore = "timing, run with --release -- --ignored --nocapture"]
fn typing_in_a_150_page_script() {
    let ctx = ctx();
    let mut ed = Editor::new(wordsmith_script::session_for(long_script(300)));
    ed.session.registry = std::sync::Arc::new(crate::registry());
    ed.mac = false;
    frame(&ctx, &mut ed, vec![]);
    let pages = ed.paginated().pages.len();
    ed.run("caret.docEnd", json!({}));
    frame(&ctx, &mut ed, vec![]);
    let mut times = Vec::new();
    for _ in 0..20 {
        let t = std::time::Instant::now();
        frame(&ctx, &mut ed, vec![text("x")]);
        times.push(t.elapsed());
    }
    times.sort();
    eprintln!("typing, {pages} pages: median {:?}, worst {:?} per keystroke frame", times[times.len() / 2], times[times.len() - 1]);
}

/// A window that has no handle: native dialogs are answered by an override in tests.
struct NoWindow;
impl raw_window_handle::HasWindowHandle for NoWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Err(raw_window_handle::HandleError::Unavailable)
    }
}
impl raw_window_handle::HasDisplayHandle for NoWindow {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Err(raw_window_handle::HandleError::Unavailable)
    }
}

#[test]
fn export_pdf_asks_for_a_path_and_writes_the_file() {
    let dir = std::env::temp_dir().join(format!("wordsmith-ui-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("chosen.pdf");
    let answer = target.clone();
    let mut ed = editor();
    ed.dialogs.set_override(Some(Box::new(move |_| craft_dialogs::Answer::Paths(vec![answer.clone()]))));
    ed.run("file.exportPdfAs", json!({}));
    for _ in 0..50 {
        ed.native_dialogs(Some(&NoWindow));
        if target.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(std::fs::read(&target).unwrap().starts_with(b"%PDF-"), "{}", ed.message);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn suggestions_show_and_tab_or_enter_accepts_but_typing_ignores() {
    let ctx = ctx();
    let mut ed = editor();
    ed.session.run("autocomplete.add", &json!({"list": "characters", "term": "BOB"})).unwrap();
    frame(&ctx, &mut ed, vec![key(Key::Num3, cmd()), text("b")]);
    assert_eq!(ed.suggestions.as_ref().map(|s| s.candidates.clone()), Some(vec!["BOB".to_string()]));
    // Typing on narrows to nothing: the list goes, the typed text stays.
    frame(&ctx, &mut ed, vec![text("x")]);
    assert!(ed.suggestions.is_none());
    assert_eq!(lines(&ed)[0], l("Character", "BX"));
    // Back to "B": Enter accepts instead of starting a new paragraph.
    frame(&ctx, &mut ed, vec![key(Key::Backspace, Modifiers::NONE)]);
    assert!(ed.suggestions.is_some());
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE)]);
    assert_eq!(lines(&ed), vec![l("Character", "BOB")]);
    // Escape hides it; Enter then is Return again.
    frame(
        &ctx,
        &mut ed,
        vec![key(Key::Enter, Modifiers::NONE), text("Hi."), key(Key::Enter, Modifiers::NONE), key(Key::Tab, Modifiers::NONE), text("b")],
    );
    assert!(ed.suggestions.is_some());
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(ed.suggestions.is_none());
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE)]);
    assert_eq!(lines(&ed).last(), Some(&l("Dialogue", "")), "Return after the Character");
    assert_eq!(lines(&ed)[lines(&ed).len() - 2], l("Character", "B"));
}

#[test]
fn up_and_down_move_through_suggestions() {
    let ctx = ctx();
    let mut ed = editor();
    for n in ["ANNA", "ABE", "ALEX"] {
        ed.session.run("autocomplete.add", &json!({"list": "characters", "term": n})).unwrap();
    }
    frame(&ctx, &mut ed, vec![key(Key::Num3, cmd()), text("a")]);
    assert_eq!(ed.suggestions.as_ref().map(|s| s.candidates.len()), Some(3));
    frame(&ctx, &mut ed, vec![key(Key::ArrowDown, Modifiers::NONE), key(Key::ArrowDown, Modifiers::NONE), key(Key::Tab, Modifiers::NONE)]);
    assert_eq!(lines(&ed)[0], l("Character", "ANNA"), "third alphabetically: ABE, ALEX, ANNA");
}

#[test]
fn misspellings_are_checked_in_the_page_view() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("A dragn sleeps.")]);
    let found = ed.session.run("spelling.check", &json!({})).unwrap();
    assert_eq!(found["misspellings"][0]["word"], "dragn");
    // Drawing the underlines runs every frame without trouble; switching them off too.
    frame(&ctx, &mut ed, vec![]);
    ed.spelling = false;
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn the_find_bar_opens_with_the_selection_and_runs_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("The dog and the dog.")]);
    ed.run("caret.docStart", json!({}));
    ed.run("select.wordRight", json!({}));
    frame(&ctx, &mut ed, vec![key(Key::F, cmd())]);
    assert_eq!(ed.find.as_ref().map(|b| b.query.as_str()), Some("The "));
    let bar = crate::editor::FindBar { query: "dog".into(), with: "cat".into(), ..Default::default() };
    ed.find = Some(bar.clone());
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.find.as_ref().and_then(|b| b.count), Some(2));
    ed.run("replace.all", bar.params());
    assert_eq!(lines(&ed)[0].1, "The cat and the cat.");
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.find.as_ref().and_then(|b| b.count), Some(0));
}
