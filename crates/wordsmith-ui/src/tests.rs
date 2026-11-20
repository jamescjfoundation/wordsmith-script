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

#[test]
fn the_three_views_show_the_same_script() {
    use crate::editor::View;
    let ctx = ctx();
    let mut ed = Editor::new(wordsmith_script::session_for(long_script(6)));
    ed.session.registry = std::sync::Arc::new(crate::registry());
    ed.mac = false;
    frame(&ctx, &mut ed, vec![]);
    let page = ed.shown();
    assert!(page.layout.pages.len() >= 3);
    frame(&ctx, &mut ed, vec![key(Key::Num1, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(ed.view, View::Normal);
    let normal = ed.shown();
    assert_eq!(normal.layout.pages.len(), 1, "one continuous column");
    assert!(normal.pag.is_none(), "the script itself, no (MORE)/(CONT'D)");
    frame(&ctx, &mut ed, vec![key(Key::Num3, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(ed.view, View::Speed);
    let speed = ed.shown();
    // Every element sits at the left margin in the speed view.
    let x = |s: &crate::editor::Shown, block: usize| {
        let at = wordsmith_doc::Pos { story: wordsmith_doc::StoryRef::Body, path: wordsmith_doc::Path::top(block), off: 0 };
        s.layout.caret(&at).map(|c| c.x).unwrap_or(f32::NAN)
    };
    assert!((x(&speed, 2) - x(&speed, 1)).abs() < 0.5, "Character and Action line up");
    assert!((x(&normal, 2) - x(&normal, 1)).abs() > 100.0, "but not in the normal view");
    // Typing works the same in each view.
    ed.run("caret.docEnd", json!({}));
    frame(&ctx, &mut ed, vec![text("!")]);
    assert!(lines(&ed).last().map(|l| l.1.ends_with('!')).unwrap_or(false));
    frame(&ctx, &mut ed, vec![key(Key::Num2, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(ed.view, View::Page);
}

#[test]
fn dark_theme_typewriter_and_focus_mode() {
    let ctx = ctx();
    let mut ed = editor();
    ed.run("view.darkTheme", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.dark && ctx.theme() == egui::Theme::Dark);
    ed.run("view.typewriter", json!({}));
    assert!(ed.typewriter);
    frame(&ctx, &mut ed, vec![key(Key::F, Modifiers { shift: true, ..cmd() })]);
    assert!(ed.focus_mode);
    // Typing still works in focus mode; Escape leaves it.
    frame(&ctx, &mut ed, vec![text("int")]);
    assert_eq!(lines(&ed)[0].1, "INT");
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE), key(Key::Escape, Modifiers::NONE)]);
    assert!(!ed.focus_mode);
}

/// A fresh folder for one test.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("wordsmith-ui-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Type a little script with the keyboard.
fn type_scene(ctx: &egui::Context, ed: &mut Editor) {
    frame(ctx, ed, vec![]);
    frame(ctx, ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("kitchen"), key(Key::Enter, Modifiers::NONE), text("Rain.")]);
}

#[test]
fn save_as_open_and_the_window_title() {
    let ctx = ctx();
    let dir = temp_dir("save");
    let mut ed = editor();
    assert_eq!(ed.window_title(), "Untitled — Wordsmith Script");
    type_scene(&ctx, &mut ed);
    assert_eq!(ed.window_title(), "Untitled • — Wordsmith Script");
    // Save with no file asks for one.
    frame(&ctx, &mut ed, vec![key(Key::S, cmd())]);
    assert!(ed.session.dirty, "nothing saved yet: a dialog was asked for");
    let path = dir.join("Kitchen");
    ed.run("file.saveAsPath", json!({"path": path}));
    assert_eq!(ed.message, "");
    assert!(!ed.session.dirty);
    assert_eq!(ed.window_title(), "Kitchen — Wordsmith Script");
    assert!(dir.join("Kitchen.wss").exists(), ".wss is added");
    let saved = lines(&ed);
    // Ctrl+S now saves to the file without asking.
    frame(&ctx, &mut ed, vec![text(" More.")]);
    frame(&ctx, &mut ed, vec![key(Key::S, cmd())]);
    assert!(!ed.session.dirty);
    let mut other = editor();
    other.run("file.openPath", json!({"path": dir.join("Kitchen.wss")}));
    assert_eq!(other.message, "");
    assert_eq!(lines(&other), lines(&ed));
    assert_ne!(lines(&other), saved);
    assert_eq!(other.window_title(), "Kitchen — Wordsmith Script");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unsaved_changes_are_never_lost_without_asking() {
    let ctx = ctx();
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    let before = lines(&ed);
    // New over unsaved changes asks; Cancel keeps everything.
    frame(&ctx, &mut ed, vec![key(Key::N, cmd())]);
    assert_eq!(ed.unsaved, Some(crate::editor::Then::New));
    assert_eq!(lines(&ed), before);
    ed.answer_unsaved(crate::editor::UnsavedAnswer::Cancel);
    assert_eq!(lines(&ed), before);
    assert!(ed.session.dirty);
    // Opening over them by command is refused unless told to discard.
    let dir = temp_dir("unsaved");
    let other = dir.join("Other.wss");
    wordsmith_format::files::save(&other, &wordsmith_script::new_script()).unwrap();
    ed.run("file.openPath", json!({"path": other}));
    assert!(ed.message.contains("unsaved changes"), "{}", ed.message);
    assert_eq!(lines(&ed), before);
    // Closing asks too; Don't Save lets it close.
    assert!(!ed.may_close());
    assert_eq!(ed.unsaved, Some(crate::editor::Then::Close));
    ed.answer_unsaved(crate::editor::UnsavedAnswer::DontSave);
    assert!(ed.close_confirmed);
    // Don't Save before New starts the new script.
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    ed.run("file.new", json!({}));
    ed.answer_unsaved(crate::editor::UnsavedAnswer::DontSave);
    assert!(!ed.session.dirty);
    assert_eq!(lines(&ed).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_in_the_prompt_saves_then_carries_on() {
    let ctx = ctx();
    let dir = temp_dir("prompt-save");
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    ed.run("file.saveAsPath", json!({"path": dir.join("Draft.wss")}));
    frame(&ctx, &mut ed, vec![text(" Again.")]);
    let typed = lines(&ed);
    ed.run("file.new", json!({}));
    ed.answer_unsaved(crate::editor::UnsavedAnswer::Save);
    assert_eq!(lines(&ed).len(), 1, "saved, then a new script");
    let mut back = editor();
    back.run("file.openPath", json!({"path": dir.join("Draft.wss")}));
    assert_eq!(lines(&back), typed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn importing_text_asks_whether_it_is_a_script() {
    let dir = temp_dir("import");
    let path = dir.join("notes.txt");
    std::fs::write(&path, "INT. KITCHEN - NIGHT\n\nRain.\n\nANNA\nHello.\n").unwrap();
    let mut ed = editor();
    ed.run("file.openPath", json!({"path": path}));
    let q = ed.import_question.clone().unwrap();
    assert!(q.guess, "it looks like a script");
    assert_eq!(lines(&ed).len(), 1, "nothing imported before the answer");
    ed.answer_import(Some(true));
    assert_eq!(lines(&ed), [l("SceneHeading", "INT. KITCHEN - NIGHT"), l("Action", "Rain."), l("Character", "ANNA"), l("Dialogue", "Hello.")]);
    assert!(ed.session.dirty, "an import is a new, unsaved script");
    assert_eq!(ed.window_title(), "notes • — Wordsmith Script");
    assert!(ed.session.path.is_none(), "Save asks where: the text file is not overwritten");
    let mut plain = editor();
    plain.run("file.openPath", json!({"path": path}));
    plain.answer_import(Some(false));
    assert!(lines(&plain).iter().all(|(e, _)| e == "Action"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exports_write_each_format() {
    let ctx = ctx();
    let dir = temp_dir("export");
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    for (id, ext) in [("file.exportFountain", "fountain"), ("file.exportText", "txt"), ("file.exportRtf", "rtf"), ("file.exportHtml", "html")] {
        ed.run(id, json!({"path": dir.join("Out")}));
        assert_eq!(ed.message, "", "{id}");
        assert!(dir.join(format!("Out.{ext}")).exists(), "{id}");
    }
    assert!(ed.session.dirty, "exporting is not saving");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saves_make_backups_and_auto_save_waits_for_a_pause() {
    let ctx = ctx();
    let dir = temp_dir("autosave");
    let mut ed = editor();
    ed.backups.dir = Some(dir.join("Backups"));
    type_scene(&ctx, &mut ed);
    ed.run("file.saveAsPath", json!({"path": dir.join("Draft.wss")}));
    assert_eq!(wordsmith_format::files::backups_of(&dir.join("Backups"), "Draft").len(), 1);
    let t0 = std::time::Instant::now();
    ed.tick(t0);
    frame(&ctx, &mut ed, vec![text(" Typed.")]);
    let wait = ed.tick(t0);
    assert_eq!(wait, Some(std::time::Duration::from_secs(15 * 60)));
    ed.tick(t0 + std::time::Duration::from_secs(14 * 60));
    assert!(ed.session.dirty);
    ed.tick(t0 + std::time::Duration::from_secs(15 * 60 + 1));
    assert!(!ed.session.dirty, "auto-saved");
    assert_eq!(wordsmith_format::files::backups_of(&dir.join("Backups"), "Draft").len(), 2);
    let mut back = editor();
    back.run("file.openPath", json!({"path": dir.join("Draft.wss")}));
    assert_eq!(lines(&back), lines(&ed));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preferences_round_trip_through_their_file() {
    use crate::prefs::{Prefs, PrefsFile};
    let dir = temp_dir("prefs");
    let mut ed = editor();
    ed.set_view(crate::editor::View::Speed);
    ed.dark = true;
    ed.autosave.every = std::time::Duration::from_secs(5 * 60);
    ed.backups.keep = 50;
    let t0 = std::time::Instant::now();
    {
        let (mut file, shown, problem) = PrefsFile::open(&dir);
        assert!(problem.is_none() && shown == Prefs::default(), "no file yet: the defaults");
        assert_eq!(file.save(&shown.of(&ed), t0, false), Ok(None));
        // A second change within the interval waits; forcing writes it.
        ed.dark = false;
        assert!(file.save(&shown.of(&ed), t0, false).unwrap().is_some());
        ed.dark = true;
        assert_eq!(file.save(&shown.of(&ed), t0, true), Ok(None));
    }
    let (_, read, problem) = PrefsFile::open(&dir);
    assert!(problem.is_none());
    let mut fresh = editor();
    read.apply(&mut fresh);
    assert_eq!(fresh.view, crate::editor::View::Speed);
    assert!(fresh.dark);
    assert_eq!(fresh.autosave.every, std::time::Duration::from_secs(5 * 60));
    assert_eq!(fresh.backups.keep, 50);
    // Values out of range are brought back in.
    let mut wild = Prefs::default();
    wild.autosave.minutes = 0;
    wild.backups.keep = 99_999;
    wild.view.zoom = f32::NAN;
    wild.apply(&mut fresh);
    assert_eq!(fresh.autosave.every, std::time::Duration::from_secs(60));
    assert_eq!(fresh.backups.keep, 10_000);
    assert_eq!(fresh.zoom, 1.0);
    // A missing key takes its default; a damaged file is moved aside.
    std::fs::write(dir.join(PrefsFile::NAME), r#"{"view": {"dark": true}}"#).unwrap();
    let (_, partial, problem) = PrefsFile::open(&dir);
    assert!(problem.is_none() && partial.view.dark && partial.autosave == crate::prefs::AutoSavePrefs::default());
    std::fs::write(dir.join(PrefsFile::NAME), "{ not json").unwrap();
    let (_, damaged, problem) = PrefsFile::open(&dir);
    assert!(problem.is_some() && damaged == Prefs::default());
    assert!(dir.join("preferences.json.damaged").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_title_page_window_edits_the_fields() {
    let ctx = ctx();
    let mut ed = editor();
    ed.run("view.titlePage", json!({}));
    let form = ed.title_page.as_mut().unwrap();
    form["title"] = json!("THE TEST");
    form["authors"] = json!("Anna Writer");
    let form = form.clone();
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.title_page.is_some(), "the window stays open");
    ed.run("titlePage.set", form);
    assert_eq!(ed.session.doc.core.title, "THE TEST");
    assert_eq!(wordsmith_script::title_page::TitlePage::of_doc(&ed.session.doc).authors, "Anna Writer");
    assert!(ed.session.dirty);
}

#[test]
fn the_header_and_footer_window_sets_the_bands() {
    let ctx = ctx();
    let mut ed = editor();
    ed.run("view.headerFooter", json!({}));
    let form = ed.header_footer.as_mut().unwrap();
    assert_eq!(form["header"]["right"], "{page}.");
    form["footer"]["center"] = json!("{title}");
    let form = form.clone();
    frame(&ctx, &mut ed, vec![]);
    ed.run("headerFooter.set", form);
    assert_eq!(ed.message, "");
    assert_eq!(wordsmith_script::header_footer::HeaderFooter::of_doc(&ed.session.doc).footer.center, "{title}");
}

#[test]
fn index_cards_and_the_scene_view_show_the_script_and_keep_typing_out() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(
        &ctx,
        &mut ed,
        vec![
            text("int"),
            key(Key::Tab, Modifiers::NONE),
            text("kitchen"),
            key(Key::Enter, Modifiers::NONE),
            text("Rain."),
            key(Key::Enter, Modifiers::NONE),
            key(Key::Enter, Modifiers::NONE),
        ],
    );
    ed.run("element.sceneHeading", json!({}));
    frame(&ctx, &mut ed, vec![text("ext. yard"), key(Key::Enter, Modifiers::NONE), text("Sun.")]);
    let before = lines(&ed);
    let alt = Modifiers { alt: true, ..cmd() };
    frame(&ctx, &mut ed, vec![key(Key::Num4, alt)]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Cards));
    assert_eq!(ed.scene_list().len(), 2);
    frame(&ctx, &mut ed, vec![text("typed"), key(Key::Enter, Modifiers::NONE), key(Key::Backspace, Modifiers::NONE)]);
    assert_eq!(lines(&ed), before, "typing in the cards does not reach the script");
    // A card dropped on the first moves its scene there; undo puts it back.
    ed.run("scene.move", json!({"scene": 1, "before": crate::structure::drop_target(1, 0, 2)}));
    assert_eq!(ed.scene_list()[0]["heading"], "EXT. YARD");
    frame(&ctx, &mut ed, vec![key(Key::Num5, alt)]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Scenes));
    frame(&ctx, &mut ed, vec![key(Key::Z, cmd())]);
    assert_eq!(lines(&ed), before, "undo works from the scene view");
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert_eq!(ed.structure, None);
}

#[test]
fn the_tables_panel_edits_the_script_and_exports_csv() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(
        &ctx,
        &mut ed,
        vec![
            text("int"),
            key(Key::Tab, Modifiers::NONE),
            text("kitchen"),
            key(Key::Enter, Modifiers::NONE),
            text("Anna waits."),
            key(Key::Enter, Modifiers::NONE),
        ],
    );
    ed.run("element.character", json!({}));
    frame(&ctx, &mut ed, vec![text("anna"), key(Key::Enter, Modifiers::NONE), text("Hello.")]);
    ed.run("view.tables", json!({}));
    assert_eq!(ed.tables, Some(crate::tables::Table::Scenes));
    frame(&ctx, &mut ed, vec![]);
    // A finished heading edit sets the heading; a finished name edit renames everywhere.
    let mut e = crate::tables::CellEdit::new(crate::tables::Table::Scenes, 0, "heading", "INT. KITCHEN");
    e.text = "ext. garden".into();
    let (id, params) = crate::tables::commit(&e).unwrap();
    ed.run(id, params);
    assert_eq!(ed.scene_list()[0]["heading"], "EXT. GARDEN");
    ed.tables = Some(crate::tables::Table::Characters);
    frame(&ctx, &mut ed, vec![]);
    ed.run("character.rename", json!({"from": "ANNA", "to": "Zoe"}));
    assert_eq!(lines(&ed)[1].1, "Zoe waits.");
    assert_eq!(lines(&ed)[2].1, "ZOE");
    let dir = temp_dir("csv");
    ed.run("tables.exportCharactersCsv", json!({"path": dir.join("Cast")}));
    assert_eq!(std::fs::read_to_string(dir.join("Cast.csv")).unwrap(), "Character,Speeches,Scenes,First scene\r\nZOE,1,1,1\r\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn notes_in_the_editor() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    ed.run("element.action", json!({}));
    frame(&ctx, &mut ed, vec![text("Rain hits the window.")]);
    // Select "Rain" and open a new note.
    frame(
        &ctx,
        &mut ed,
        vec![
            key(Key::Home, Modifiers::NONE),
            key(Key::ArrowRight, Modifiers::SHIFT),
            key(Key::ArrowRight, Modifiers::SHIFT),
            key(Key::ArrowRight, Modifiers::SHIFT),
            key(Key::ArrowRight, Modifiers::SHIFT),
        ],
    );
    let alt = Modifiers { alt: true, ..cmd() };
    frame(&ctx, &mut ed, vec![key(Key::N, alt)]);
    assert_eq!(ed.note_popover, Some((None, String::new())), "a new note");
    frame(&ctx, &mut ed, vec![]);
    ed.note_popover = None;
    ed.run("note.add", json!({"text": "Snow?"}));
    // With the caret in the noted text, the same command opens that note.
    frame(&ctx, &mut ed, vec![key(Key::Home, Modifiers::NONE), key(Key::ArrowRight, Modifiers::NONE)]);
    frame(&ctx, &mut ed, vec![key(Key::N, alt)]);
    assert_eq!(ed.note_popover.as_ref().map(|p| p.1.as_str()), Some("Snow?"));
    frame(&ctx, &mut ed, vec![]);
    // The Notes table lists it; editing its cell edits the note.
    ed.tables = Some(crate::tables::Table::Notes);
    frame(&ctx, &mut ed, vec![]);
    let mut e = crate::tables::CellEdit::new(crate::tables::Table::Notes, 1, "text", "Snow?");
    e.text = "Hail.".into();
    let (id, params) = crate::tables::commit(&e).unwrap();
    ed.run(id, params);
    assert_eq!(ed.session.run("note.list", &json!({})).unwrap()[0]["text"], "Hail.");
}

#[test]
fn revision_mode_from_the_editor() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    ed.run("element.action", json!({}));
    frame(&ctx, &mut ed, vec![text("Rain.")]);
    ed.run("revision.next", json!({}));
    ed.run("view.revisions", json!({}));
    assert!(ed.show_revisions);
    frame(&ctx, &mut ed, vec![text(" Snow.")]);
    let pag = ed.paginated();
    assert_eq!(pag.page_revisions, [Some(0)], "the page is revised in Blue");
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn revised_pages_and_the_character_report_export() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    ed.run("element.character", json!({}));
    frame(&ctx, &mut ed, vec![text("anna"), key(Key::Enter, Modifiers::NONE), text("Hello there.")]);
    let dir = temp_dir("revised");
    ed.run("file.exportRevisedPdf", json!({"path": dir.join("R.pdf").display().to_string()}));
    assert!(ed.message.contains("no page has revised text"), "{}", ed.message);
    ed.run("revision.next", json!({}));
    frame(&ctx, &mut ed, vec![text(" Again.")]);
    ed.run("file.exportRevisedPdf", json!({"path": dir.join("R.pdf").display().to_string()}));
    assert_eq!(ed.message, "");
    assert!(dir.join("R.pdf").exists());
    ed.run("report.exportCharacters", json!({"path": dir.join("Cast")}));
    assert_eq!(
        std::fs::read_to_string(dir.join("Cast.csv")).unwrap(),
        "Character,Speeches,Words,Scenes,First scene,Last scene,Pages\r\nANNA,1,3,0,,,1\r\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_story_board_adds_edits_and_sends_beats() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    let alt = Modifiers { alt: true, ..cmd() };
    frame(&ctx, &mut ed, vec![key(Key::Num6, alt)]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Board));
    ed.run("beat.add", json!({"title": "Anna arrives"}));
    ed.run("beat.add", json!({"title": "The fight", "x": 400, "y": 40}));
    ed.run("beat.link", json!({"from": 1, "to": 2}));
    frame(&ctx, &mut ed, vec![]);
    // Editing: Tab moves between the fields; Save writes them.
    ed.board.selected = Some(1);
    let b = wordsmith_script::storyboard::Storyboard::of_doc(&ed.session.doc).beat(1).cloned().unwrap();
    ed.board.edit = Some(crate::board::BeatEdit::of(&b));
    frame(&ctx, &mut ed, vec![key(Key::Tab, Modifiers::NONE)]);
    assert!(ed.board.edit.as_ref().is_some_and(|e| e.on_body));
    frame(&ctx, &mut ed, vec![text("Late and wet.")]);
    assert_eq!(ed.board.edit.as_ref().map(|e| e.body.as_str()), Some("Late and wet."));
    ed.run("beat.update", json!({"id": 1, "body": "Late and wet."}));
    ed.board.edit = None;
    ed.run("beat.toScript", json!({"id": 1}));
    assert_eq!(lines(&ed)[0], l("Outline1", "Anna arrives"));
    assert_eq!(lines(&ed)[1], l("Summary", "Late and wet."));
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn the_outline_view_shows_lanes_beats_and_the_script() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("kitchen")]);
    let alt = Modifiers { alt: true, ..cmd() };
    frame(&ctx, &mut ed, vec![key(Key::Num7, alt)]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Outline));
    ed.run("outline.template", json!({"name": "three-act"}));
    ed.run("beat.add", json!({"title": "Setup", "lane": 1}));
    ed.run("beat.add", json!({"title": "Loose"}));
    frame(&ctx, &mut ed, vec![]);
    // The same beat, moved in the outline, is moved for the story board too.
    ed.run("beat.place", json!({"id": 2, "lane": 1, "index": 0}));
    let board = wordsmith_script::storyboard::Storyboard::of_doc(&ed.session.doc);
    assert_eq!(board.lane_beats(Some(1)).iter().map(|b| b.title.as_str()).collect::<Vec<_>>(), ["Loose", "Setup"]);
    frame(&ctx, &mut ed, vec![]);
    ed.run("view.board", json!({}));
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn right_click_moves_the_caret_for_the_element_menu() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    ed.run("element.action", json!({}));
    frame(&ctx, &mut ed, vec![text("First paragraph."), key(Key::Enter, Modifiers::NONE), text("Second.")]);
    // Where the first paragraph is on screen: put the caret there and read its rectangle.
    ed.session.sel =
        wordsmith_engine::Selection::caret(wordsmith_doc::Pos { off: 3, path: wordsmith_doc::Path::top(0), ..ed.session.sel.focus.clone() });
    frame(&ctx, &mut ed, vec![]);
    let spot = ed.caret_rect().expect("the caret is drawn").center();
    ed.run("caret.docEnd", json!({}));
    assert_eq!(ed.session.sel.focus.path.last(), 1);
    let press = |pressed| Event::PointerButton { pos: spot, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    frame(&ctx, &mut ed, vec![Event::PointerMoved(spot), press(true), press(false)]);
    assert_eq!(ed.session.sel.focus.path.last(), 0, "the caret went where the right-click was");
    frame(&ctx, &mut ed, vec![]);
}

/// The keyboard page of the user guide, made from the registry, so it never disagrees with the
/// app. `UPDATE_GUIDE=1 cargo test -p wordsmith-ui keyboard_guide` rewrites it.
#[test]
fn the_keyboard_guide_matches_the_commands() {
    let registry = crate::registry();
    let mut rows: Vec<(&str, &str, &str)> =
        registry.all().iter().filter(|s| !s.shortcut.is_empty()).map(|s| (s.location, s.label, s.shortcut)).collect();
    rows.sort_by(|a, b| a.0.cmp(b.0).then(a.2.cmp(b.2)));
    let mut out = String::from(
        "# Keyboard\n\nEvery key that runs a command, by menu. `Mod` is Command on macOS and Control elsewhere.\nThis page is made from the app's command list (`the_keyboard_guide_matches_the_commands` keeps it\ncurrent). Custom elements can have keys of their own (Format › Element Settings).\n",
    );
    let mut last = "";
    for (location, label, keys) in rows {
        if location != last {
            out.push_str(&format!("\n## {location}\n\n| Command | Keys |\n| --- | --- |\n"));
            last = location;
        }
        let keys = keys.split(" / ").map(|k| format!("`{k}`")).collect::<Vec<_>>().join(" or ");
        out.push_str(&format!("| {label} | {keys} |\n"));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/keyboard.md");
    if std::env::var_os("UPDATE_GUIDE").is_some() {
        std::fs::write(&path, &out).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), out, "run with UPDATE_GUIDE=1 and review the change");
}

#[test]
fn the_go_to_checker_and_reformat_windows_work_through_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("hall"), key(Key::Enter, Modifiers::NONE), text("Rain.")]);
    // Mod+G opens Go To; a bookmark made with Mod+M is listed there.
    frame(&ctx, &mut ed, vec![key(Key::M, cmd()), key(Key::G, cmd())]);
    assert!(ed.tools.goto.is_some());
    let marks = ed.session.run("bookmark.list", &json!({})).unwrap();
    assert_eq!(marks["bookmarks"].as_array().map(Vec::len), Some(1));
    // An empty paragraph is one format issue; Fix deletes it.
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE)]);
    ed.run("view.checker", json!({}));
    frame(&ctx, &mut ed, vec![]);
    let issues = ed.tools.checker.as_ref().map(|c| c.issues.len());
    assert_eq!(issues, Some(1));
    ed.run("check.fix", json!({"block": 2, "rule": "empty"}));
    assert_eq!(lines(&ed).len(), 2);
    // The reformat tool: a digit changes the paragraph's element and moves on.
    ed.tools.checker = None;
    ed.run("view.reformat", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.tools.reformat);
    ed.session.sel = wordsmith_engine::Selection::caret(wordsmith_script::data::body_pos(1, 0));
    frame(&ctx, &mut ed, vec![key(Key::Num0, Modifiers::NONE)]);
    assert_eq!(lines(&ed)[1].0, "General");
}
