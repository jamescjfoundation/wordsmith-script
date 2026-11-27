// SPDX-License-Identifier: MIT OR Apache-2.0
//! The editor driven headlessly through egui input inside the shell, the way a window drives it.

use std::ops::{Deref, DerefMut};

use egui::{Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};
use jcj_shell::files::{FileAnswer, FileDialogs, FileKind};
use jcj_shell::workspace::{Layout, Side};
use jcj_shell::{Page, Shell};
use serde_json::{Value, json};

use crate::dialogs::SettingsDialog;
use crate::editor::Editor;
use crate::ribbon::tab;

/// A window: the shell around the editor. It reads as the editor (`w.session`, `w.find`), with
/// the shell beside it (`w.shell`).
pub(crate) struct Window {
    pub shell: Shell,
    pub ed: Editor,
}

impl Deref for Window {
    type Target = Editor;
    fn deref(&self) -> &Editor {
        &self.ed
    }
}

impl DerefMut for Window {
    fn deref_mut(&mut self) -> &mut Editor {
        &mut self.ed
    }
}

impl Window {
    /// Run a command as the window's controls do, and pass on what it asks of the shell.
    pub fn run(&mut self, id: &str, params: Value) {
        self.ed.run_cmd(&jcj_shell::Cmd::with(id, params));
        self.ed.flush(&mut self.shell);
    }

    /// Show panel `id` on its side.
    fn show_panel(&mut self, id: &str) {
        self.shell.layout.show(&crate::sidebar::panels(), id);
    }

    /// The panel open on `side`.
    fn open_on(&self, side: Side) -> Option<String> {
        self.shell.layout.open_on(side).map(str::to_string)
    }
}

/// A window on `ed`, with portable (Windows and Linux) key conventions, so tests read the same on
/// every platform.
pub(crate) fn window_on(mut ed: Editor) -> Window {
    ed.mac = false;
    Window { shell: crate::shell(), ed }
}

/// A window on a new script.
pub(crate) fn window() -> Window {
    window_on(Editor::new(crate::new_session()))
}

fn editor() -> Window {
    window()
}

/// A window on `doc`, with every command.
fn with_script(doc: jcj_doc::Document) -> Window {
    let mut session = crate::new_session();
    session.set_document(doc);
    window_on(Editor::new(session))
}

fn ctx() -> egui::Context {
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    ctx
}

/// A frame in a window `width` × `height`, with `events`.
fn frame_out(ctx: &egui::Context, w: &mut Window, width: f32, height: f32, events: Vec<Event>) -> egui::FullOutput {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, height))), events, ..Default::default() };
    let mut out = ctx.run_ui(input, |ui| w.shell.ui(ui, &mut w.ed));
    // No GPU here to take the page textures.
    out.textures_delta.clear();
    out
}

fn frame(ctx: &egui::Context, w: &mut Window, events: Vec<Event>) {
    frame_out(ctx, w, 1200.0, 900.0, events);
}

/// A frame in a window `width` wide and 800 high.
fn frame_at(ctx: &egui::Context, w: &mut Window, width: f32, events: Vec<Event>) {
    frame_out(ctx, w, width, 800.0, events);
}

fn key(k: Key, modifiers: Modifiers) -> Event {
    Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn text(t: &str) -> Event {
    Event::Text(t.to_string())
}

fn lines(ed: &Editor) -> Vec<(String, String)> {
    wordsmith_script_model::commands::text::body_lines(&ed.session)
}

fn l(e: &str, t: &str) -> (String, String) {
    (e.to_string(), t.to_string())
}

/// The modifiers of a `Mod` key press, as the system sends them: Command on macOS, Control
/// elsewhere. On macOS Control is a key of its own, so a press that carried it would be
/// `Mod+Ctrl+…` there and reach nothing.
fn cmd() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers { command: true, mac_cmd: true, ..Modifiers::NONE }
    } else {
        Modifiers { command: true, ctrl: true, ..Modifiers::NONE }
    }
}

/// Every main tab, and the contextual ones with their views.
fn all_tabs() -> [(&'static str, Option<crate::structure::Structure>); 8] {
    use crate::structure::Structure;
    [
        (tab::WRITE, None),
        (tab::INSERT, None),
        (tab::PLAN, None),
        (tab::PRODUCTION, None),
        (tab::REVIEW, None),
        (tab::VIEW, None),
        (tab::BOARD, Some(Structure::Board)),
        (tab::CARDS, Some(Structure::Cards)),
    ]
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
    ed.session.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+Y"})).unwrap();
    frame(&ctx, &mut ed, vec![key(Key::Y, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(lines(&ed)[0].0, "Custom1");
}

#[test]
fn dialogs_open_through_commands_and_take_the_keyboard() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::P, Modifiers { shift: true, ..cmd() })]);
    assert!(ed.shell.dialogs.palette.is_some());
    // Typing goes to the palette, not the script.
    frame(&ctx, &mut ed, vec![text("x")]);
    frame(&ctx, &mut ed, vec![text("y")]);
    assert_eq!(lines(&ed), vec![l("SceneHeading", "")]);
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(ed.shell.dialogs.palette.is_none());
    ed.run("view.elementSettings", json!({}));
    assert_eq!(ed.settings.as_ref().map(|d| d.element.as_str()), Some("SceneHeading"));
    ed.run("help.about", json!({}));
    assert_eq!(ed.shell.chrome.file_page, Some(Page::About), "About is a page of the File page");
    frame(&ctx, &mut ed, vec![]);
}

#[test]
fn the_palette_finds_commands_by_any_word() {
    use jcj_shell::Commands;
    let ed = editor();
    let all = ed.all_commands();
    let ids = |q: &str| jcj_shell::commands::palette_matches(&all, q, 40).into_iter().map(|m| m.id).collect::<Vec<_>>();
    assert!(ids("scene heading").contains(&"element.sceneHeading".to_string()));
    assert!(ids("undo").contains(&"edit.undo".to_string()));
    assert!(!ids("").contains(&"element.set".to_string()), "commands that need parameters are left out");
    assert!(ids("zzzz no such").is_empty());
    assert!(ids("mail merge").is_empty(), "the engine's word-processor catalog stays out of sight");
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
    for id in [
        "edit.undo",
        "file.recover",
        "text.tab",
        "element.dialogue",
        "element.configure",
        "view.commandPalette",
        "edit.copy",
        "help.about",
        "caret.home",
        "find.replace",
    ] {
        assert!(r.get(id).is_some(), "{id}");
    }
    let mut seen = std::collections::HashSet::new();
    for spec in r.all() {
        assert!(seen.insert(spec.id), "{} registered twice", spec.id);
    }
    // The lists themselves, before the registry is built: only a command below Script's (the
    // engine's, the core's, the shared features') may be replaced.
    let mut lists = wordsmith_script_model::commands::specs();
    lists.extend(crate::commands::ui_specs());
    lists.extend(wordsmith_script_format::commands::specs());
    assert!(jcj_engine::duplicate_ids(&lists).is_empty(), "registered twice: {:?}", jcj_engine::duplicate_ids(&lists));
}

#[test]
fn about_carries_the_credit() {
    use jcj_shell::App;
    let ed = editor();
    assert_eq!(ed.about().credit, "Based on WordCraft by the ArtCraft team.");
    let text = crate::about_text();
    assert!(text.starts_with("Wordsmith Script "));
    assert!(text.contains("Based on WordCraft by the ArtCraft team."));
    assert!(text.contains("\n\nAcknowledgments\n"));
    // Release packages carry the libraries' license texts in this file and the fonts' in this
    // folder beside it (`cargo xtask package`, the core's).
    assert!(text.contains("THIRD-PARTY-LICENSES.txt") && text.contains("licenses folder"));
}

#[test]
fn the_tour_says_where_the_update_check_is_turned_off() {
    let (_, last) = crate::ribbon::TOUR[crate::ribbon::TOUR.len() - 1];
    assert!(last.contains("GitHub") && last.contains("Preferences › Starting"));
}

/// A script of `scenes` scenes, about half a page each.
fn long_script(scenes: usize) -> jcj_doc::Document {
    let mut doc = wordsmith_script_model::new_script();
    let mut body = Vec::new();
    for n in 0..scenes {
        let mut push = |e: &str, t: String| body.push(jcj_doc::para_block(wordsmith_script_model::template::paragraph(e, &t)));
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
    let mut ed = with_script(long_script(6));
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
/// paginates again and draws the visible pages. Record the number in docs/performance.md when
/// it changes.
#[test]
#[ignore = "timing, run with --release -- --ignored --nocapture"]
fn typing_in_a_150_page_script() {
    let ctx = ctx();
    let mut ed = with_script(long_script(300));
    frame(&ctx, &mut ed, vec![]);
    let pages = ed.paginated().pages.len();
    ed.run("caret.docEnd", json!({}));
    frame(&ctx, &mut ed, vec![]);
    // WS_TIMING_PARTS=1 also times the frame without each part of the chrome.
    let variants: Vec<&str> = if std::env::var("WS_TIMING_PARTS").is_ok() { vec!["all", "no-panels", "focus"] } else { vec!["all"] };
    for v in variants {
        ed.focus_mode = v == "focus";
        ed.shell.layout = if v == "all" { Layout::with_open(&crate::sidebar::panels(), &["scenes"]) } else { Layout::default() };
        let mut times = Vec::new();
        for _ in 0..20 {
            let t = std::time::Instant::now();
            frame(&ctx, &mut ed, vec![text("x")]);
            times.push(t.elapsed());
        }
        times.sort();
        eprintln!("typing, {pages} pages ({v}): median {:?}, worst {:?} per keystroke frame", times[times.len() / 2], times[times.len() - 1]);
    }
}

#[test]
fn export_pdf_asks_for_a_path_and_writes_the_file() {
    let dir = std::env::temp_dir().join(format!("wordsmith-script-ui-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("chosen.pdf");
    let answer = target.clone();
    let mut ed = editor();
    // The editor's own `run`: the window's would hand the dialog on to the shell.
    ed.ed.run("file.exportPdfAs", json!({}));
    let mut asks = std::mem::take(&mut ed.file_asks);
    assert_eq!(asks.len(), 1, "a file dialog was asked for");
    let ask = asks.remove(0);
    assert!(ask.dialog.kind == FileKind::Save && ask.dialog.filters[0].1 == ["pdf"]);
    ed.answer_file(ask, Some(answer));
    assert!(std::fs::read(&target).unwrap().starts_with(b"%PDF-"), "{}", ed.message);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The window's part of the shell's file dialogs, with `answer` standing in for the system's.
fn dialogs(answer: impl FnMut(&jcj_shell::files::FileAsk) -> FileAnswer + 'static) -> FileDialogs {
    let mut files = FileDialogs::new();
    files.answer_with(Some(Box::new(answer)));
    files
}

/// What a window does between two frames: show the dialogs the editor asked the shell for, then
/// run the commands their answers queued.
fn answer_dialogs(ctx: &egui::Context, w: &mut Window, files: &mut FileDialogs) {
    w.ed.flush(&mut w.shell);
    files.tick(&mut w.shell, ctx);
    files.tick(&mut w.shell, ctx);
    w.shell.run_queue(&mut w.ed);
    assert!(!files.is_open() && files.waiting() == 0 && w.shell.file_asks_waiting() == 0, "every dialog was answered");
}

#[test]
fn save_as_asks_the_shell_for_a_dialog_and_saves_where_it_answers() {
    let dir = temp_dir("save-as");
    let target = dir.join("mine.wss");
    let ctx = ctx();
    let mut w = editor();
    w.run("text.insert", json!({"text": "int. hall"}));
    w.run("file.saveAs", json!({}));
    assert_eq!(w.shell.file_asks_waiting(), 1, "the shell holds the dialog for the window to show");
    assert!(w.file_asks.is_empty());
    let answer = target.clone();
    let mut files = dialogs(move |ask| {
        assert_eq!((ask.kind, ask.title.as_str(), ask.file_name.as_deref()), (FileKind::Save, "Save Script", Some("Untitled.wss")));
        assert_eq!(ask.filters, [("Wordsmith Script".to_owned(), vec!["wss".to_owned()])]);
        FileAnswer::Chosen(vec![answer.clone()])
    });
    answer_dialogs(&ctx, &mut w, &mut files);
    assert!(std::fs::read_to_string(&target).unwrap().starts_with("Wordsmith Script "), "{}", w.message);
    assert_eq!(w.session.path.as_deref(), Some(target.as_path()));
    assert!(!w.session.dirty);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dialogs_answer_goes_to_the_tab_that_asked() {
    let dir = temp_dir("tab-answer");
    let ctx = ctx();
    let mut w = editor();
    w.run("text.insert", json!({"text": "int. first"}));
    w.run("file.saveAs", json!({}));
    // The dialog does not stop the window: another script is started while it is up.
    w.new_tab();
    w.run("text.insert", json!({"text": "int. second"}));
    assert_eq!(w.active_tab, 1);
    let answer = dir.join("first.wss");
    let mut files = dialogs(move |_| FileAnswer::Chosen(vec![answer.clone()]));
    answer_dialogs(&ctx, &mut w, &mut files);
    assert_eq!(w.active_tab, 0, "the answer shows the script it is for");
    assert!(std::fs::read_to_string(dir.join("first.wss")).unwrap().contains("INT. FIRST"));
    assert_eq!(w.tab_names(), [("first".to_string(), false), ("Untitled".to_string(), true)]);

    // A dialog whose script was closed meanwhile does nothing, and says so.
    w.switch_tab(1);
    w.run("file.saveAs", json!({}));
    assert!(w.close_tab(true));
    let answer = dir.join("second.wss");
    let mut files = dialogs(move |_| FileAnswer::Chosen(vec![answer.clone()]));
    answer_dialogs(&ctx, &mut w, &mut files);
    assert!(!dir.join("second.wss").exists());
    assert!(w.shell.message.contains("is closed"), "{}", w.shell.message);
    assert_eq!(w.session.path.as_deref(), Some(dir.join("first.wss").as_path()), "the open script is left alone");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_step_that_waits_on_a_save_dialog_goes_with_the_dialog() {
    let dir = temp_dir("waiting-step");
    let ctx = ctx();
    // Closing the window with unsaved changes, answered "Save", in a script with no file yet:
    // the close waits on the Save dialog.
    let closing = |w: &mut Window| {
        w.run("text.insert", json!({"text": "int. hall"}));
        assert!(!w.may_close());
        w.answer_unsaved(crate::editor::UnsavedAnswer::Save);
        w.ed.flush(&mut w.shell);
        assert_eq!(w.shell.file_asks_waiting(), 1);
    };
    let save_to = |name: &str| {
        let path = dir.join(name);
        dialogs(move |_| FileAnswer::Chosen(vec![path.clone()]))
    };

    // A place chosen: the script is saved and the window may close.
    let mut w = editor();
    closing(&mut w);
    answer_dialogs(&ctx, &mut w, &mut save_to("saved.wss"));
    assert!(dir.join("saved.wss").is_file() && w.close_confirmed);

    // The dialog closed without a choice, or not shown at all: the close is given up, and a
    // later Save As saves without closing the window.
    for (name, lost) in [("closed.wss", FileAnswer::Cancelled), ("unshown.wss", FileAnswer::Unavailable("no dialog here".into()))] {
        let mut w = editor();
        closing(&mut w);
        let unshown = lost != FileAnswer::Cancelled;
        answer_dialogs(&ctx, &mut w, &mut dialogs(move |_| lost.clone()));
        assert!(!w.close_confirmed && w.session.dirty, "{name}");
        assert_eq!(w.shell.message.contains("could not be shown"), unshown, "{name}: {}", w.shell.message);
        w.run("file.saveAs", json!({}));
        answer_dialogs(&ctx, &mut w, &mut save_to(name));
        assert!(dir.join(name).is_file() && !w.session.dirty, "{name}");
        assert!(!w.close_confirmed, "{name}: the window stays open");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_dialog_carries_the_options_it_was_asked_with() {
    let mut w = editor();
    w.run("text.insert", json!({"text": "int. hall"}));
    // A report's dialog keeps the report's options, even when another dialog is asked for
    // before it is answered.
    w.dialog_params = Some(json!({"kind": "scenes"}));
    w.ed.run("report.export", json!({"kind": "scenes"}));
    w.ed.run("file.saveAs", json!({}));
    assert!(w.dialog_params.is_none());
    let asks: Vec<(&str, Value)> = w.file_asks.iter().map(|a| (a.then, Value::Object(a.params.clone()))).collect();
    assert_eq!(asks, [("report.export", json!({"kind": "scenes"})), ("file.saveAsPath", json!({}))]);
    assert!(w.file_asks.iter().all(|a| a.tab == w.tab_id()));
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
    ed.run("find.replaceAll", bar.params());
    assert_eq!(lines(&ed)[0].1, "The cat and the cat.");
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.find.as_ref().and_then(|b| b.count), Some(0));
}

#[test]
fn the_three_views_show_the_same_script() {
    use crate::editor::View;
    let ctx = ctx();
    let mut ed = with_script(long_script(6));
    frame(&ctx, &mut ed, vec![]);
    let page = ed.shown();
    assert!(page.layout.pages.len() >= 3);
    frame(&ctx, &mut ed, vec![key(Key::Num1, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(ed.view, View::Normal);
    let normal = ed.shown();
    assert_eq!(normal.layout.pages.len(), 1, "one continuous column");
    assert!(normal.pag.is_none(), "the script itself, no (MORE)/(CONT'D)");
    frame(&ctx, &mut ed, vec![key(Key::Num3, Modifiers { alt: true, ..cmd() })]);
    assert_eq!(ed.view, View::Draft);
    let draft = ed.shown();
    // Every element sits at the left margin in the draft view.
    let x = |s: &crate::editor::Shown, block: usize| {
        let at = jcj_doc::Pos { story: jcj_doc::StoryRef::Body, path: jcj_doc::Path::top(block), off: 0 };
        s.layout.caret(&at).map(|c| c.x).unwrap_or(f32::NAN)
    };
    assert!((x(&draft, 2) - x(&draft, 1)).abs() < 0.5, "Character and Action line up");
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
    let dir = std::env::temp_dir().join(format!("wordsmith-script-ui-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Type a little script with the keyboard.
fn type_scene(ctx: &egui::Context, ed: &mut Window) {
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
    wordsmith_script_format::files::save(&other, &wordsmith_script_model::new_script()).unwrap();
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
    use jcj_shell::App;
    use jcj_shell::autosave::Tick;
    let ctx = ctx();
    let dir = temp_dir("autosave");
    let mut ed = editor();
    ed.backups.dir = Some(dir.join("Backups"));
    type_scene(&ctx, &mut ed);
    ed.run("file.saveAsPath", json!({"path": dir.join("Draft.wss")}));
    assert_eq!(wordsmith_script_format::files::backups_of(&dir.join("Backups"), "Draft").len(), 1);
    frame(&ctx, &mut ed, vec![]);
    // The shell's schedule with Script's save: due 15 minutes after the change, once typing
    // pauses.
    let w = &mut ed;
    let before = std::time::Instant::now();
    frame(&ctx, w, vec![text(" Typed.")]);
    let (dirty, file) = (w.ed.is_dirty(), w.ed.has_file());
    let due = w.shell.autosave.due_at(dirty, file).expect("a save is waiting");
    let fifteen = std::time::Duration::from_secs(15 * 60);
    assert!(due >= before + fifteen && due <= std::time::Instant::now() + fifteen);
    let tick = |w: &mut Window, at| {
        let (dirty, file) = (w.ed.is_dirty(), w.ed.has_file());
        let ed = &mut w.ed;
        w.shell.autosave.tick(at, dirty, file, || ed.autosave())
    };
    assert!(matches!(tick(w, due - std::time::Duration::from_secs(60)), Tick::Waiting(_)));
    assert!(w.ed.session.dirty);
    assert_eq!(tick(w, due + std::time::Duration::from_secs(1)), Tick::Saved);
    assert!(!w.ed.session.dirty, "auto-saved");
    assert_eq!(wordsmith_script_format::files::backups_of(&dir.join("Backups"), "Draft").len(), 2);
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
    ed.set_view(crate::editor::View::Draft);
    ed.shell.mode = jcj_shell::theme::Mode::Dark;
    ed.shell.autosave.every = std::time::Duration::from_secs(5 * 60);
    ed.backups.keep = 50;
    let t0 = std::time::Instant::now();
    let of = |w: &Window, p: &Prefs| Prefs { shell: w.shell.prefs(), app: p.app.of(&w.ed) };
    {
        let (mut file, shown, problem) = PrefsFile::open(&dir);
        assert!(problem.is_none() && shown == Prefs::default(), "no file yet: the defaults");
        assert_eq!(file.save(&of(&ed, &shown), t0, false), Ok(None));
        // A second change within the interval waits; forcing writes it.
        ed.zoom = 2.0;
        assert!(file.save(&of(&ed, &shown), t0, false).unwrap().is_some());
        ed.zoom = 1.0;
        ed.backups.keep = 51;
        assert_eq!(file.save(&of(&ed, &shown), t0, true), Ok(None));
    }
    let (_, read, problem) = PrefsFile::open(&dir);
    assert!(problem.is_none());
    let mut fresh = editor();
    fresh.shell.apply_prefs(&read.shell, &crate::sidebar::panels());
    read.app.apply(&mut fresh);
    assert_eq!(fresh.view, crate::editor::View::Draft);
    assert_eq!(fresh.shell.mode, jcj_shell::theme::Mode::Dark);
    assert_eq!(fresh.shell.autosave.every, std::time::Duration::from_secs(5 * 60));
    assert_eq!(fresh.backups.keep, 51);
    // Values out of range are brought back in.
    let mut wild = Prefs::default();
    wild.shell.autosave.minutes = 0;
    wild.app.backups.keep = 99_999;
    wild.app.view.zoom = f32::NAN;
    fresh.shell.apply_prefs(&wild.shell, &crate::sidebar::panels());
    wild.app.apply(&mut fresh);
    assert_eq!(fresh.shell.autosave.every, std::time::Duration::from_secs(60));
    assert_eq!(fresh.backups.keep, 10_000);
    assert_eq!(fresh.zoom, 1.0);
    // A missing key takes its default; a damaged file is moved aside.
    std::fs::write(dir.join("preferences.json"), r#"{"app": {"view": {"typewriter": true}}}"#).unwrap();
    let (_, partial, problem) = PrefsFile::open(&dir);
    assert!(problem.is_none() && partial.app.view.typewriter && partial.shell.autosave == jcj_shell::prefs::AutoSavePrefs::default());
    // A file from before decision 0041 names the draft view "speed"; it still opens in it, and
    // is written back with the new name.
    std::fs::write(dir.join("preferences.json"), r#"{"view": {"view": "speed"}}"#).unwrap();
    let (_, old, problem) = PrefsFile::open(&dir);
    assert!(problem.is_none());
    let mut reopened = editor();
    old.app.apply(&mut reopened);
    assert_eq!(reopened.view, crate::editor::View::Draft);
    assert_eq!(old.app.of(&reopened).view.view, "draft");
    std::fs::write(dir.join("preferences.json"), "{ not json").unwrap();
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
    assert_eq!(wordsmith_script_model::title_page::TitlePage::of_doc(&ed.session.doc).authors, "Anna Writer");
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
    assert_eq!(wordsmith_script_model::header_footer::HeaderFooter::of_doc(&ed.session.doc).footer.center, "{title}");
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
    ed.run("scene.move", json!({"scene": 1, "before": jcj_board::drop_target(1, 0, 2)}));
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
    // The beats are the canvas's cards.
    let sb = wordsmith_script_model::storyboard::Storyboard::of_doc(&ed.session.doc);
    let board = crate::board::board_of(&sb, ed.board.current);
    assert_eq!(board.cards.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["Anna arrives", "The fight"]);
    assert_eq!(board.links.len(), 1);
    // Editing: Tab moves between the fields; Save writes them.
    ed.board.selection.select(Some(jcj_board::CardId(1)));
    let b = sb.beat(1).cloned().unwrap();
    ed.board.edit = Some(crate::board::edit_of(&b));
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
    let board = wordsmith_script_model::storyboard::Storyboard::of_doc(&ed.session.doc);
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
    ed.session.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 3, path: jcj_doc::Path::top(0), ..ed.session.sel.focus.clone() });
    frame(&ctx, &mut ed, vec![]);
    let spot = ed.caret_rect().expect("the caret is drawn").center();
    ed.run("caret.docEnd", json!({}));
    assert_eq!(ed.session.sel.focus.path.last(), 1);
    let press = |pressed| Event::PointerButton { pos: spot, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    frame(&ctx, &mut ed, vec![Event::PointerMoved(spot), press(true), press(false)]);
    assert_eq!(ed.session.sel.focus.path.last(), 0, "the caret went where the right-click was");
    frame(&ctx, &mut ed, vec![]);
}

/// The keys page of the user guide (`docs/guide/keys.md`), made from the commands the window
/// shows, so it never disagrees with the app. It is a golden file (`docs/golden.md`), rewritten
/// with the one switch every golden file has:
/// `UPDATE_GOLDEN=1 cargo test -p wordsmith-script-ui the_keyboard_guide_matches_the_commands`.
///
/// The page holds the keys of Windows and Linux. On macOS some general commands have other keys
/// (the system keeps the usual ones for itself, or has a convention of its own), so the page is
/// neither compared nor rewritten there.
#[test]
fn the_keyboard_guide_matches_the_commands() {
    if cfg!(target_os = "macos") {
        return;
    }
    let registry = crate::registry();
    let mut rows: Vec<(&str, &str, &str)> =
        registry.all().iter().filter(|s| !s.shortcut.is_empty() && crate::commands::shown(s.id)).map(|s| (s.location, s.label, s.shortcut)).collect();
    rows.sort_by(|a, b| a.0.cmp(b.0).then(a.2.cmp(b.2)));
    let mut out = String::from(
        "# Keyboard\n\nEvery key that runs a command, by menu. `Mod` is Command on macOS and Control elsewhere.\nThe keys are those of Windows and Linux; on macOS a few general commands have other keys, which\nthe command's tooltip and the Keys page of the File page show.\nThis page is made from the app's command list (`the_keyboard_guide_matches_the_commands` keeps it\ncurrent). Custom elements can have keys of their own (Format › Element Settings).\n",
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
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/keys.md");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &out).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), out, "run with UPDATE_GOLDEN=1 and review the change");
}

/// Keys Script had before 0.2 that its commands no longer declare: they come from the shared
/// features and the core now, without Script's keys, and hidden engine commands hold the keys.
#[test]
fn script_s_keys_are_kept() {
    let ed = editor();
    let map = ed.key_map(&Default::default());
    for (key, id) in [("Mod+M", "bookmark.add"), ("Mod+Shift+C", "track.toggle"), ("Mod+Shift+T", "format.plain")] {
        assert_eq!(map.bound_to(key), Some(id), "{key}");
    }
}

#[test]
fn shift_extends_caret_movement_and_shift_backspace_deletes() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("Hello world")]);
    frame(&ctx, &mut ed, vec![key(Key::Home, Modifiers::SHIFT)]);
    assert_eq!(ed.session.selected_text(), "Hello world");
    // By word: Mod with the arrow. Not pressed on macOS, where Option with the arrow should do
    // it and goes to the line's end instead: the shell and the engine both translate that
    // system's text keys (open-source-core#48).
    if !cfg!(target_os = "macos") {
        frame(&ctx, &mut ed, vec![key(Key::End, Modifiers::NONE), key(Key::ArrowLeft, Modifiers { shift: true, ..cmd() })]);
        assert_eq!(ed.session.selected_text(), "world");
    }
    frame(&ctx, &mut ed, vec![key(Key::End, Modifiers::NONE), key(Key::Backspace, Modifiers::SHIFT)]);
    assert_eq!(lines(&ed)[0].1, "Hello worl");
}

#[test]
fn a_shifted_caret_key_that_was_turned_off_does_nothing() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("Hello world")]);
    // Shift+Left turned off on the Keys page: it runs nothing, the rule for shifted keys included.
    assert!(ed.shell.keys.bind("Shift+Left", ""));
    frame(&ctx, &mut ed, vec![key(Key::ArrowLeft, Modifiers::SHIFT)]);
    assert_eq!(ed.session.selected_text(), "");
    // The other shifted caret keys still extend the selection.
    frame(&ctx, &mut ed, vec![key(Key::Home, Modifiers::SHIFT)]);
    assert_eq!(ed.session.selected_text(), "Hello world");
}

#[test]
fn the_go_to_checker_and_reformat_windows_work_through_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("hall"), key(Key::Enter, Modifiers::NONE), text("Rain.")]);
    // Mod+G opens Go To; a bookmark is listed there.
    ed.run("bookmark.add", json!({}));
    frame(&ctx, &mut ed, vec![key(Key::G, cmd())]);
    assert!(ed.tools.goto.is_some());
    let marks = ed.session.run("bookmark.list", &json!({})).unwrap();
    assert_eq!(marks["bookmarks"].as_array().map(Vec::len), Some(1));
    // An empty paragraph is one format issue; Fix deletes it.
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE)]);
    ed.run("view.checker", json!({}));
    frame(&ctx, &mut ed, vec![]);
    // The checker is the Inspector's Check Format panel now (v0.3).
    assert_eq!(ed.open_on(Side::Right).as_deref(), Some("checker"));
    let issues = ed.session.run("check.run", &json!({})).ok().and_then(|v| v["count"].as_u64());
    assert_eq!(issues, Some(1));
    ed.run("check.fix", json!({"block": 2, "rule": "empty"}));
    assert_eq!(lines(&ed).len(), 2);
    // The reformat tool: a digit changes the paragraph's element and moves on.
    ed.tools.checker = None;
    ed.run("view.reformat", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.tools.reformat);
    ed.session.sel = jcj_engine::Selection::caret(wordsmith_script_model::data::body_pos(1, 0));
    frame(&ctx, &mut ed, vec![key(Key::Num0, Modifiers::NONE)]);
    assert_eq!(lines(&ed)[1].0, "General");
}

#[test]
fn macro_keys_and_style_keys_work_from_the_keyboard() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    let macro_key = Modifiers { alt: true, shift: true, ..cmd() };
    frame(&ctx, &mut ed, vec![key(Key::Num2, macro_key), text("yard"), key(Key::Num3, macro_key), text("Sun.")]);
    assert_eq!(lines(&ed), vec![l("SceneHeading", "EXT. YARD - DAY"), l("Action", "Sun.")]);
    // Mod+B makes what is typed next bold.
    frame(&ctx, &mut ed, vec![key(Key::B, cmd()), text(" Hot.")]);
    let p = ed.session.doc.para_at(&wordsmith_script_model::data::body_pos(1, 0)).cloned().unwrap_or_default();
    assert_eq!(p.props_at(6).bold, Some(true));
    assert_eq!(p.props_at(2).bold, None);
    // The macros window opens from its command.
    ed.run("view.macros", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.tools.macros.is_some());
}

#[test]
fn reports_scene_properties_and_tables_open_and_work() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(
        &ctx,
        &mut ed,
        vec![
            text("int"),
            key(Key::Tab, Modifiers::NONE),
            text("hall"),
            key(Key::Enter, Modifiers::NONE),
            key(Key::Tab, Modifiers::NONE),
            text("anna"),
        ],
    );
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE), text("Hello.")]);
    // Reports (Mod+Shift+J) runs a report straight away.
    frame(&ctx, &mut ed, vec![key(Key::J, Modifiers { shift: true, ..cmd() })]);
    frame(&ctx, &mut ed, vec![]);
    let title = ed.tools.reports.as_ref().and_then(|r| r.result.as_ref()).map(|r| r["title"].clone());
    assert_eq!(title, Some(json!("Scene Report")));
    // Scene properties for the caret's scene.
    ed.run("view.sceneProperties", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.open_on(Side::Right).as_deref(), Some("scene"), "scene properties are the Inspector's Scene panel");
    ed.run("scene.setProperties", json!({"scene": 0, "title": "Arrival", "color": "Green"}));
    // The Tables panel shows the title, sorted and filtered without changing the script.
    ed.run("view.tables", json!({}));
    ed.table_view.sort = Some(("heading".into(), true));
    ed.table_view.filter = "arr".into();
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.scene_list()[0]["title"], "Arrival");
    // Highlighted speeches are listed for drawing.
    ed.run("character.set", json!({"name": "ANNA", "highlightOn": true}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(wordsmith_script_model::cast::highlighted_speeches(&ed.session.doc).len(), 1);
}

#[test]
fn boards_strip_and_cards_draw_and_act() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("hall"), key(Key::Enter, Modifiers::NONE), text("Rain.")]);
    // A second board: beats added there stay there.
    ed.run("view.board", json!({}));
    frame(&ctx, &mut ed, vec![]);
    let id = ed.session.run("storyboard.addBoard", &json!({"name": "Ideas"})).unwrap()["id"].as_u64().unwrap() as u32;
    ed.board.current = id;
    ed.run("beat.add", json!({"board": id, "title": "Twist"}));
    frame(&ctx, &mut ed, vec![]);
    let b = wordsmith_script_model::storyboard::Storyboard::of_doc(&ed.session.doc);
    assert_eq!(b.beats.iter().filter(|x| x.board == id).count(), 1);
    // The structure strip shows above the script and goes to a scene on a click.
    ed.structure = None;
    ed.run("view.strip", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.tools.strip);
    // The index cards, five to a row, showing script text.
    ed.run("view.cards", json!({}));
    ed.tools.cards.per_row = 5;
    ed.tools.cards.script_text = true;
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Cards));
}

#[test]
fn production_windows_dual_dialogue_and_tracking() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(
        &ctx,
        &mut ed,
        vec![
            text("int"),
            key(Key::Tab, Modifiers::NONE),
            text("hall"),
            key(Key::Enter, Modifiers::NONE),
            key(Key::Tab, Modifiers::NONE),
            text("anna"),
        ],
    );
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE), text("Go."), key(Key::Enter, Modifiers::NONE), key(Key::Tab, Modifiers::NONE)]);
    frame(&ctx, &mut ed, vec![text("ben"), key(Key::Enter, Modifiers::NONE), text("Stay.")]);
    // Mod+Shift+D sets Ben's speech beside Anna's; the page view still draws.
    frame(&ctx, &mut ed, vec![key(Key::D, Modifiers { shift: true, ..cmd() })]);
    assert_eq!(wordsmith_script_model::paginate::dual::pairs(&ed.session.doc).len(), 1);
    frame(&ctx, &mut ed, vec![]);
    // Tracking: typed text is marked, and the status bar says so.
    ed.run("track.toggle", json!({"on": true}));
    frame(&ctx, &mut ed, vec![text(" Now.")]);
    assert_eq!(wordsmith_script_model::tracking::changes(&ed.session.doc).len(), 1);
    // Print is a page of the File page; the watermark and scene number windows open.
    for id in ["view.watermark", "view.sceneNumbers"] {
        ed.run(id, json!({}));
    }
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.tools.watermark.is_some() && ed.tools.scene_numbers.is_some());
    ed.run("view.print", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.shell.chrome.file_page, Some(Page::Print));
    let p = ed.print.clone();
    let params = crate::production::PrintChoices { range: "scenes", from: "1".into(), to: "2".into(), which: "revised", ..p }.params();
    assert_eq!((params["scenes"].clone(), params["revisedOnly"].clone()), (json!([0, 1]), json!(true)));
}

#[test]
fn tabs_split_keys_templates_and_statistics() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![text("int"), key(Key::Tab, Modifiers::NONE), text("hall"), key(Key::Enter, Modifiers::NONE), text("Rain falls.")]);
    // Writing statistics count the words typed today.
    frame(&ctx, &mut ed, vec![text(" Hard and long.")]);
    let words: u64 = ed.stats.data.days.values().map(|d| d.words).sum();
    assert!(words >= 3, "counted {words}");
    // Mod+T: a second script in a tab; the first keeps its text.
    frame(&ctx, &mut ed, vec![key(Key::T, cmd())]);
    assert_eq!((ed.tabs.len(), ed.active_tab), (2, 1));
    frame(&ctx, &mut ed, vec![text("ext"), key(Key::Tab, Modifiers::NONE), text("yard")]);
    ed.switch_tab(0);
    assert_eq!(lines(&ed)[1].1, "Rain falls. Hard and long.");
    // Closing a tab with unsaved changes asks first.
    ed.session.dirty = true;
    frame(&ctx, &mut ed, vec![key(Key::W, cmd())]);
    assert_eq!(ed.unsaved, Some(crate::editor::Then::CloseTab));
    ed.answer_unsaved(crate::editor::UnsavedAnswer::DontSave);
    assert_eq!(ed.tabs.len(), 1);
    assert_eq!(lines(&ed)[0].1, "EXT. YARD");
    // The tab strip: closing another script's tab leaves the writer where they were; closing
    // one with unsaved changes shows it and asks first.
    ed.new_tab();
    ed.new_tab();
    assert_eq!((ed.tabs.len(), ed.active_tab), (3, 2));
    crate::workspace::tab_event(&mut ed.ed, jcj_shell::split::TabEvent::Close(1));
    assert_eq!((ed.tabs.len(), ed.active_tab), (2, 1));
    crate::workspace::tab_event(&mut ed.ed, jcj_shell::split::TabEvent::Select(0));
    assert_eq!(lines(&ed)[0].1, "EXT. YARD");
    ed.session.dirty = true;
    ed.switch_tab(1);
    crate::workspace::tab_event(&mut ed.ed, jcj_shell::split::TabEvent::Close(0));
    assert_eq!((ed.tabs.len(), ed.active_tab, ed.unsaved), (2, 0, Some(crate::editor::Then::CloseTab)));
    ed.answer_unsaved(crate::editor::UnsavedAnswer::Cancel);
    ed.session.dirty = false;
    crate::workspace::tab_event(&mut ed.ed, jcj_shell::split::TabEvent::Close(1));
    assert_eq!((ed.tabs.len(), ed.active_tab), (1, 0));
    assert_eq!(lines(&ed)[0].1, "EXT. YARD");
    // A key of the writer's own runs its command; "" turns a key off.
    ed.shell.keys.bind("Mod+Shift+Y", "element.action");
    ed.shell.keys.bind("Mod+1", "");
    frame(&ctx, &mut ed, vec![key(Key::Y, Modifiers { shift: true, ..cmd() }), key(Key::Num1, cmd())]);
    assert_eq!(lines(&ed)[0].0, "Action");
    // Split: the cards beside the script; swap; unsplit.
    ed.run("view.splitCards", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.split.map(|s| s.pane), Some(Some(crate::structure::Structure::Cards)));
    ed.run("view.splitSwap", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.structure, Some(crate::structure::Structure::Cards));
    ed.run("view.unsplit", json!({}));
    // A template from the chooser, in a tab of its own when there is unsaved work.
    ed.structure = None;
    ed.session.dirty = true;
    ed.new_tab();
    ed.run("file.newFromTemplate", json!({"template": "stage"}));
    assert_eq!(ed.tabs.len(), 2);
    // A sprint starts and shows its time.
    ed.run("view.sprint", json!({}));
    assert!(crate::workspace::sprint_left(&ed.ed).is_some());
    ed.run("view.stats", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.stats.window.is_some());
}

/// Every control drawn with every tab, the menu bar and every File page showing (drop-downs
/// unfolded, so their items are drawn too): where each was drawn and the command it runs.
fn every_control_drawn() -> Vec<jcj_shell::ribbon::Drawn> {
    let ctx = ctx();
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    ed.shell.recent = vec!["Example.wss".into()];
    ed.shell.chrome.unfold_menus = true;
    ed.menu_bar = true;
    let mut all = Vec::new();
    for (tab, view) in all_tabs() {
        ed.structure = view;
        ed.shell.chrome.tab = tab.into();
        frame_at(&ctx, &mut ed, 1920.0, vec![]);
        all.extend(ed.shell.chrome.drawn.iter().cloned());
    }
    for page in
        [Page::New, Page::Open, Page::Save, Page::Export, Page::Print, Page::App("compare".into()), Page::Preferences, Page::Keys, Page::About]
    {
        ed.shell.open_file_page(page);
        frame_at(&ctx, &mut ed, 1920.0, vec![]);
        all.extend(ed.shell.chrome.drawn.iter().cloned());
    }
    all
}

#[test]
fn every_drawn_control_runs_a_registered_command() {
    use jcj_shell::Commands;
    use jcj_shell::ribbon::Place;
    let all = every_control_drawn();
    let ed = editor();
    let known = |id: &str| id.starts_with("shell.") || ed.command(id).is_some();
    let missing: Vec<&str> = all.iter().filter_map(|d| d.id.as_deref()).filter(|id| !known(id)).collect();
    assert!(missing.is_empty(), "controls run commands that are not registered: {missing:?}");
    let on = |places: &[Place]| -> std::collections::BTreeSet<&str> {
        all.iter().filter(|d| places.contains(&d.place)).filter_map(|d| d.id.as_deref()).collect()
    };
    let ribbon = on(&[Place::Ribbon]);
    assert!(ribbon.len() > 100, "every tab was drawn: {}", ribbon.len());
    for id in ["format.strikethrough", "format.changeCase", "view.reports", "cards.print", "view.spelling", "view.menuBar"] {
        assert!(ribbon.contains(id), "{id} is on the ribbon");
    }
    assert!(ribbon.iter().any(|id| id.starts_with("macro.slot.")), "the macros are on the ribbon");
    // The menu bar (drawn in the document area) offers nothing the ribbon (with its quick-access
    // row) or the File page does not.
    let elsewhere = on(&[Place::Ribbon, Place::Quick, Place::FilePage]);
    let menus = on(&[Place::Other]);
    assert!(menus.len() > 100, "every menu was drawn: {}", menus.len());
    // The shell's own controls reach some of them: the File page's entries (and the File
    // button), the search box (the palette), the About page's update check.
    let shell_s = |id: &str| match id {
        "app.preferences" | "help.about" | "view.keys" | "view.start" | "view.templates" | "help.checkForUpdates" => Some("shell.filePage"),
        "view.commandPalette" => Some("shell.palette"),
        _ => None,
    };
    let only_in_menus: Vec<&&str> =
        menus.iter().filter(|id| !elsewhere.contains(*id) && !shell_s(id).is_some_and(|s| elsewhere.contains(s))).collect();
    assert!(only_in_menus.is_empty(), "menu commands with no place on the ribbon or the File page: {only_in_menus:?}");
}

/// The outlines egui draws around each control that takes clicks, when asked to (debug builds).
#[cfg(debug_assertions)]
fn clickable_rects(out: &egui::FullOutput) -> Vec<Rect> {
    let click = [egui::Color32::from_rgb(0x88, 0, 0), egui::Color32::from_rgb(0x88, 0, 0x88)];
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r) if click.contains(&r.stroke.color) && r.rect.is_positive() => Some(r.rect),
            _ => None,
        })
        .collect()
}

#[cfg(debug_assertions)]
#[test]
fn every_ribbon_and_status_bar_control_is_a_command() {
    use jcj_shell::ribbon::Place;
    let ctx = ctx();
    ctx.all_styles_mut(|s| s.debug.show_interactive_widgets = true);
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    // The status bar's chips that open panels.
    ed.run("revision.mode", json!({}));
    ed.run("track.toggle", json!({}));
    for (tab, view) in all_tabs() {
        ed.structure = view;
        ed.shell.chrome.tab = tab.into();
        for w in [1920.0, 1280.0] {
            let out = frame_out(&ctx, &mut ed, w, 800.0, vec![]);
            let areas: Vec<Rect> =
                ed.shell.chrome.areas.iter().filter(|(p, _)| matches!(p, Place::Quick | Place::Ribbon | Place::Status)).map(|(_, r)| *r).collect();
            assert_eq!(areas.len(), 3, "the quick-access row, the ribbon's groups and the status bar were drawn");
            let strays: Vec<Rect> = clickable_rects(&out)
                .into_iter()
                .filter(|r| areas.iter().any(|a| a.contains(r.center())))
                .filter(|r| !ed.shell.chrome.drawn.iter().any(|d| d.rect.expand(1.0).contains(r.center())))
                .collect();
            assert!(strays.is_empty(), "{tab} at {w}: controls that run no command at {strays:?}");
        }
    }
}

#[test]
fn the_ribbon_draws_every_tab_at_three_widths_and_in_every_theme() {
    use jcj_shell::theme::Mode;
    for (w, mode) in [(800.0, Mode::Light), (1280.0, Mode::Dark), (1920.0, Mode::ExtraDark)] {
        let ctx = ctx();
        let mut ed = editor();
        ed.shell.mode = mode;
        for (tab, _) in &all_tabs()[..6] {
            ed.shell.chrome.tab = (*tab).into();
            frame_at(&ctx, &mut ed, w, vec![]);
        }
        assert_eq!(ed.dark, mode.is_dark(), "the pages follow the theme");
        // The contextual tabs come with their views.
        ed.structure = Some(crate::structure::Structure::Board);
        assert!(crate::ribbon::tabs_now(&ed).iter().any(|t| t.id == tab::BOARD && t.contextual));
        ed.shell.chrome.tab = tab::BOARD.into();
        frame(&ctx, &mut ed, vec![]);
        ed.structure = None;
        frame(&ctx, &mut ed, vec![]);
        assert_eq!(ed.shell.chrome.tab, tab::WRITE, "the contextual tab went with its view");
    }
}

#[test]
fn key_tips_open_a_tab_and_run_a_control() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![key(Key::F10, Modifiers::NONE)]);
    assert!(ed.shell.chrome.keytips);
    // V opens the View tab, whose first control (Page) has the letter A.
    frame(&ctx, &mut ed, vec![text("v")]);
    assert_eq!(ed.shell.chrome.tab, tab::VIEW);
    frame(&ctx, &mut ed, vec![]);
    ed.set_view(crate::editor::View::Draft);
    frame(&ctx, &mut ed, vec![text("a")]);
    assert_eq!(ed.view, crate::editor::View::Page);
    assert!(!ed.shell.chrome.keytips);
}

/// Each letter of `letters` typed, in one frame.
fn typed(letters: &str) -> Vec<Event> {
    letters.chars().map(|c| text(&c.to_string())).collect()
}

/// Show key tips on `tab` (its letter `letter`), open the drop-downs in `opened` by their
/// letters, and give the controls showing with their letters.
fn keytips_on(
    ctx: &egui::Context,
    ed: &mut Window,
    w: f32,
    (tab, letter): (&str, &str),
    opened: &[jcj_shell::ribbon::Keytip],
) -> Vec<(String, jcj_shell::ribbon::Keytip)> {
    frame_at(ctx, ed, w, vec![key(Key::F10, Modifiers::NONE)]);
    assert!(ed.shell.chrome.keytips);
    frame_at(ctx, ed, w, typed(&letter.to_lowercase()));
    assert_eq!(ed.shell.chrome.tab, tab);
    frame_at(ctx, ed, w, vec![]);
    for what in opened {
        let letters = letters_of(&ed.shell.chrome.keytip_targets, what);
        frame_at(ctx, ed, w, typed(&letters));
        assert!(ed.shell.chrome.keytips, "a drop-down's letters keep the key tips showing");
        frame_at(ctx, ed, w, vec![]);
        frame_at(ctx, ed, w, vec![]);
    }
    let targets = ed.shell.chrome.keytip_targets.clone();
    for (i, (a, _)) in targets.iter().enumerate() {
        for (j, (b, _)) in targets.iter().enumerate() {
            assert!(i == j || !b.starts_with(a.as_str()), "{tab}: the letters {a} begin {b}: {targets:?}");
        }
    }
    targets
}

fn letters_of(targets: &[(String, jcj_shell::ribbon::Keytip)], what: &jcj_shell::ribbon::Keytip) -> String {
    targets.iter().find(|(_, t)| t == what).map(|(l, _)| l.clone()).unwrap_or_else(|| panic!("no key tip for {what:?} in {targets:?}"))
}

#[test]
fn key_tips_reach_every_control_and_run_it_with_its_parameters() {
    use jcj_shell::ribbon::Keytip;
    // As wide as a laptop's screen, and narrow enough that groups fold into drop-downs.
    for w in [1280.0, 800.0] {
        let ctx = ctx();
        let mut ed = editor();
        type_scene(&ctx, &mut ed);
        for (tab, view) in all_tabs() {
            // Whatever the last control opened goes, so each one starts from the same window.
            let reset = |ed: &mut Window| {
                ed.shell.chrome.keytips = false;
                ed.shell.chrome.file_page = None;
                ed.shell.dialogs = Default::default();
                ed.focus_mode = false;
                ed.structure = view;
                ed.tools.reformat = false;
                ed.unsaved = None;
                egui::Popup::close_all(&ctx);
            };
            reset(&mut ed);
            let letter = crate::ribbon::tabs_now(&ed).into_iter().find(|t| t.id == tab).map(|t| t.letter).unwrap_or_default();
            // The groups settle to the window's width over a few frames.
            ed.shell.chrome.tab = tab.into();
            for _ in 0..4 {
                frame_at(&ctx, &mut ed, w, vec![]);
            }
            let top = keytips_on(&ctx, &mut ed, w, (tab, &letter), &[]);
            assert!(!top.is_empty(), "{tab} shows key tips");
            let mut todo: Vec<(Vec<Keytip>, Keytip)> = top.iter().map(|(_, t)| (Vec::new(), t.clone())).collect();
            let mut ran = 0;
            while let Some((path, what)) = todo.pop() {
                reset(&mut ed);
                let targets = keytips_on(&ctx, &mut ed, w, (tab, &letter), &path);
                let letters = letters_of(&targets, &what);
                ed.ran.clear();
                frame_at(&ctx, &mut ed, w, typed(&letters));
                match &what {
                    Keytip::Run(c) => {
                        if !c.is_shell() {
                            assert_eq!(ed.ran.first(), Some(&(c.id.clone(), c.params.clone())), "{tab}: {letters} runs {c:?}");
                        }
                        assert!(!ed.shell.chrome.keytips, "key tips end once a control runs");
                        ran += 1;
                    }
                    Keytip::Open(popup) => {
                        assert!(ed.ran.is_empty());
                        assert!(egui::Popup::is_id_open(&ctx, *popup), "{tab}: {letters} opens its drop-down");
                        // Its items show letters too: run each of them from the start.
                        if path.is_empty() {
                            frame_at(&ctx, &mut ed, w, vec![]);
                            frame_at(&ctx, &mut ed, w, vec![]);
                            let inner: Vec<Keytip> =
                                ed.shell.chrome.keytip_targets.iter().map(|(_, t)| t.clone()).filter(|t| !top.iter().any(|(_, x)| x == t)).collect();
                            assert!(!inner.is_empty(), "{tab}: the drop-down {letters} shows its items' letters");
                            todo.extend(inner.into_iter().map(|t| (vec![what.clone()], t)));
                        }
                    }
                }
            }
            assert!(ran > 0, "{tab}");
        }
    }
}

#[test]
fn every_side_panel_draws_and_acts_through_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    frame(
        &ctx,
        &mut ed,
        vec![
            text("int"),
            key(Key::Tab, Modifiers::NONE),
            text("hall"),
            key(Key::Enter, Modifiers::NONE),
            key(Key::Tab, Modifiers::NONE),
            text("anna"),
        ],
    );
    frame(&ctx, &mut ed, vec![key(Key::Enter, Modifiers::NONE), text("Hello there.")]);
    ed.run("bookmark.add", json!({"name": "here"}));
    for (left, right) in [
        ("scenes", "element"),
        ("outline", "scene"),
        ("bookmarks", "revisions"),
        ("notes", "page"),
        ("characters", "changes"),
        ("find", "checker"),
        ("scenes", "goals"),
    ] {
        ed.show_panel(left);
        ed.show_panel(right);
        frame(&ctx, &mut ed, vec![]);
        assert_eq!((ed.open_on(Side::Left).as_deref(), ed.open_on(Side::Right).as_deref()), (Some(left), Some(right)));
    }
    // The Find panel with nothing typed in it leaves the find state alone.
    assert!(ed.find.is_none());
    ed.shell.layout = Layout::default();
    frame(&ctx, &mut ed, vec![]);
    // Characters: the panel opens a character's details by name.
    ed.session.push_ui_request(json!({"open": "tool.character", "name": "ANNA"}));
    ed.run("view.character", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.open_on(Side::Left).as_deref(), Some("characters"));
    assert!(ed.panels.open.iter().any(|n| n == "ANNA"));
}

#[test]
fn every_file_page_draws_and_its_buttons_run_commands() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    for page in
        [Page::New, Page::Open, Page::Save, Page::Export, Page::Print, Page::App("compare".into()), Page::Preferences, Page::Keys, Page::About]
    {
        ed.shell.open_file_page(page.clone());
        frame(&ctx, &mut ed, vec![]);
        assert_eq!(ed.shell.chrome.file_page, Some(page));
    }
    // Escape goes back to the script.
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert_eq!(ed.shell.chrome.file_page, None);
    // A template from the New page (as its card does).
    ed.shell.open_file_page(Page::New);
    ed.run("file.newFromTemplate", json!({"template": "stage"}));
    assert_eq!(crate::editor::script_name(&ed.session), "Stage play");
    // Welcome and the templates are the New page; Print uses the page's choices.
    ed.shell.chrome.file_page = None;
    ed.run("view.start", json!({}));
    assert_eq!(ed.shell.chrome.file_page, Some(Page::New));
    ed.print.copies = 3;
    ed.ran.clear();
    ed.run("file.print", json!({}));
    assert_eq!(ed.ran.get(1).map(|r| r.1["copies"].clone()), Some(json!(3)), "{:?}", ed.ran);
}

#[test]
fn the_window_layout_and_the_tour_keep_through_preferences() {
    let ctx = ctx();
    let mut ed = editor();
    let panels = crate::sidebar::panels();
    // A first run: the tour shows; the Scenes panel is open.
    let fresh = crate::prefs::Prefs::default();
    ed.shell.apply_prefs(&fresh.shell, &panels);
    assert_eq!(ed.shell.chrome.tour, Some(0));
    assert_eq!(ed.open_on(Side::Left).as_deref(), Some("scenes"));
    frame(&ctx, &mut ed, vec![]);
    ed.shell.chrome.tour = None;
    ed.shell.chrome.tour_done = true;
    ed.show_panel("notes");
    ed.show_panel("revisions");
    ed.shell.layout.set_size(Side::Right, 333.0);
    ed.shell.chrome.collapsed = true;
    let saved = ed.shell.prefs();
    assert!(saved.tour_seen);
    let mut again = editor();
    again.shell.apply_prefs(&saved, &panels);
    assert_eq!(
        (again.open_on(Side::Left), again.open_on(Side::Right), again.shell.chrome.collapsed, again.shell.chrome.tour),
        (Some("notes".into()), Some("revisions".into()), true, None)
    );
    assert!((again.shell.layout.size.right - 333.0).abs() < 0.01);
}

#[test]
fn check_for_updates_asks_the_shell_and_the_about_page_shows_every_status() {
    use jcj_shell::updates::{Offer, Request, Status};
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![]);
    ed.run("help.checkForUpdates", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.shell.updates.status, Status::Checking);
    assert!(ed.shell.updates.prompt);
    assert_eq!(ed.shell.updates.take_requests(), vec![Request::Check]);
    let offer = |installs| Status::Available(Offer { version: "9.0.0".into(), page: "https://github.com/x/y/releases/tag/v9.0.0".into(), installs });
    for status in [
        Status::Idle,
        Status::Current,
        offer(true),
        offer(false),
        Status::Installing("Downloading…".into()),
        Status::Ready("9.0.0".into()),
        Status::Failed("offline".into()),
        Status::Updated("9.0.0".into()),
    ] {
        ed.shell.updates.status = status;
        ed.shell.open_file_page(Page::About);
        frame(&ctx, &mut ed, vec![]);
    }
    assert!(ed.shell.updates.describe("Wordsmith Script", "0.9.0").0.contains("9.0.0"));
    // The preference round-trips.
    ed.shell.updates.check_at_start = false;
    assert!(!ed.shell.prefs().check_updates);
    assert!(crate::prefs::Prefs::default().shell.check_updates, "on by default");
}

/// The regression guard for typing speed (docs/performance.md): a keystroke in a 150-page script
/// paginates once, lays out again only the paragraph typed in, and reads no element registry
/// from the script's settings; a frame without input does none of that. It counts work rather
/// than timing it, so it gives the same answer on any machine and in any build.
#[test]
fn a_keystroke_in_a_long_script_does_only_its_own_work() {
    let ctx = ctx();
    let mut ed = with_script(long_script(300));
    frame(&ctx, &mut ed, vec![]);
    let pages = ed.paginated().pages.len();
    assert!(pages >= 140, "{pages} pages");
    ed.run("caret.docEnd", json!({}));
    frame(&ctx, &mut ed, vec![]);
    for panel in [Some("scenes"), None] {
        ed.shell.layout = Layout::with_open(&crate::sidebar::panels(), &panel.into_iter().collect::<Vec<_>>());
        frame(&ctx, &mut ed, vec![]);
        for k in 0..3 {
            let before = wordsmith_script_model::work::so_far();
            frame(&ctx, &mut ed, vec![text("x")]);
            let w = wordsmith_script_model::work::so_far().since(before);
            assert_eq!(w.paginations, 1, "keystroke {k} with {panel:?}: {w:?}");
            assert!(w.paragraphs_measured <= 2, "keystroke {k} with {panel:?}: {w:?}");
            assert!(w.element_parses <= 1, "keystroke {k} with {panel:?}: {w:?}");
        }
        let before = wordsmith_script_model::work::so_far();
        frame(&ctx, &mut ed, vec![]);
        let idle = wordsmith_script_model::work::so_far().since(before);
        assert_eq!((idle.paginations, idle.paragraphs_measured), (0, 0), "a frame without input with {panel:?}: {idle:?}");
    }
}

#[test]
fn the_window_s_own_switches_are_commands() {
    use crate::structure::Structure;
    use jcj_shell::theme::Mode;
    let ctx = ctx();
    let mut ed = editor();
    ed.shell.layout = Layout::with_open(&crate::sidebar::panels(), &["scenes"]);
    frame(&ctx, &mut ed, vec![]);
    let spelling = ed.spelling;
    ed.run("view.spelling", json!({}));
    assert_eq!(ed.spelling, !spelling);
    // The side panels: shown or hidden, or a panel named.
    ed.run("view.scriptPanel", json!({}));
    assert_eq!(ed.open_on(Side::Left), None);
    ed.run("view.scriptPanel", json!({}));
    assert_eq!(ed.open_on(Side::Left).as_deref(), Some("scenes"));
    ed.run("view.scriptPanel", json!({"panel": "notes"}));
    assert_eq!(ed.open_on(Side::Left).as_deref(), Some("notes"));
    ed.run("view.scriptPanel", json!({"panel": "revisions"}));
    assert_eq!(ed.open_on(Side::Left), None, "a panel of the other side is not named, so the side toggles");
    ed.run("view.inspector", json!({"panel": "changes"}));
    assert_eq!(ed.open_on(Side::Right).as_deref(), Some("changes"));
    ed.run("view.inspector", json!({}));
    assert_eq!(ed.open_on(Side::Right), None);
    // The themes and the menu bar.
    ed.run("view.extraDark", json!({}));
    assert_eq!(ed.shell.mode, Mode::ExtraDark);
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.dark && ed.extra_dark);
    ed.run("view.lightTheme", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(!ed.dark && !ed.extra_dark);
    ed.run("view.menuBar", json!({}));
    assert!(ed.menu_bar);
    frame(&ctx, &mut ed, vec![]);
    // Zoom to a size.
    ed.run("view.zoomTo", json!({"percent": 150}));
    assert!((ed.zoom - 1.5).abs() < 0.001);
    // A report tile opens the Reports window on its report.
    ed.run("view.reports", json!({"kind": "locations"}));
    assert_eq!(ed.tools.reports.as_ref().map(|r| r.kind), Some("locations"));
    // Index cards: printing opens the cards view with the print choices.
    ed.run("cards.print", json!({}));
    assert_eq!(ed.structure, Some(Structure::Cards));
    assert!(ed.tools.cards.printing);
    ed.run("cards.print", json!({}));
    assert!(!ed.tools.cards.printing);
    ed.run("cards.perRow", json!({"count": 4}));
    assert_eq!(ed.tools.cards.per_row, 4);
    ed.run("cards.scriptText", json!({"show": "script"}));
    assert!(ed.tools.cards.script_text);
    frame(&ctx, &mut ed, vec![]);
    // Focus mode hides the ribbon, the panels and the status bar.
    ed.run("view.focusMode", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(!ed.shell.options.ribbon && !ed.shell.options.status_bar);
    assert!(jcj_shell::App::panels(&ed.ed).is_empty());
}

/// Whether the frame asked the window to paste.
fn asks_paste(out: &egui::FullOutput) -> bool {
    out.viewport_output.values().any(|v| v.commands.contains(&egui::ViewportCommand::RequestPaste))
}

#[test]
fn paste_asks_the_window_and_the_paste_goes_in_once() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("Hello")]);
    // The command (the ribbon's Paste, the palette) asks the window for the clipboard…
    ed.run("edit.paste", json!({}));
    assert!(asks_paste(&frame_out(&ctx, &mut ed, 1200.0, 900.0, vec![])));
    // …whose text comes back as a paste.
    frame(&ctx, &mut ed, vec![Event::Paste(" there".into())]);
    assert_eq!(lines(&ed), vec![l("Action", "Hello there")]);
    // The paste key: the window sends the paste itself, so the command does not ask again.
    let out = frame_out(&ctx, &mut ed, 1200.0, 900.0, vec![key(Key::V, cmd()), Event::Paste("!".into())]);
    assert!(!asks_paste(&out));
    assert_eq!(lines(&ed), vec![l("Action", "Hello there!")]);
    // The Paste button is on the Write tab, with its key in the tooltip, as this system writes it.
    let paste_key = jcj_shell::keys::display("Mod+V", ed.shell.mac);
    assert!(jcj_shell::widgets::tip(&ed.shell, &ed.ed, "edit.paste", "").contains(&paste_key), "{paste_key}");
}

#[test]
fn the_find_bar_and_the_find_panel_share_one_query() {
    let ctx = ctx();
    let mut ed = editor();
    frame(&ctx, &mut ed, vec![key(Key::Num2, cmd()), text("The dog and the dog.")]);
    // The bar opens; the panel shows its query and matches.
    frame(&ctx, &mut ed, vec![key(Key::F, cmd())]);
    assert!(ed.find_bar);
    ed.find = Some(crate::editor::FindBar { query: "dog".into(), ..Default::default() });
    ed.show_panel("find");
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![]);
    assert_eq!(ed.find.as_ref().and_then(|b| b.count), Some(2));
    // Escape with nothing else open closes the bar; the panel keeps the query.
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(!ed.find_bar);
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![]);
    assert!(!ed.find_bar, "the panel does not open the bar again");
    assert_eq!(ed.find.as_ref().map(|b| b.query.as_str()), Some("dog"));
    // Escape that closes something above the bar leaves the bar open.
    ed.run("view.find", json!({}));
    assert!(ed.find_bar);
    ed.run("view.commandPalette", json!({}));
    frame(&ctx, &mut ed, vec![]);
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(ed.shell.dialogs.palette.is_none());
    assert!(ed.find_bar, "Escape closed the palette, not the bar");
    ed.run("view.cards", json!({}));
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(ed.structure.is_none());
    assert!(ed.find_bar, "Escape left the cards, not the bar");
    // With the bar closed and the panel gone, nothing marks matches.
    frame(&ctx, &mut ed, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(!ed.find_bar);
    ed.show_panel("scenes");
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.find.is_none());
}

#[test]
fn unsaved_changes_ask_through_the_shell_and_its_answer_comes_back() {
    let ctx = ctx();
    let mut ed = editor();
    type_scene(&ctx, &mut ed);
    frame(&ctx, &mut ed, vec![key(Key::N, cmd())]);
    frame(&ctx, &mut ed, vec![]);
    let q = ed.shell.dialogs.unsaved.clone().expect("the shell asks");
    assert_eq!((q.name.as_str(), q.tag.as_str()), ("Untitled", "new"));
    // The shell's answer runs as a command.
    ed.run(jcj_shell::commands::ids::UNSAVED_ANSWER, json!({"answer": "dontSave", "then": "new"}));
    assert!(!ed.session.dirty);
    assert_eq!(lines(&ed).len(), 1, "a new script");
}

#[test]
fn custom_element_and_macro_keys_are_commands_of_their_own() {
    use jcj_shell::Commands;
    let mut ed = editor();
    ed.session.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+Y"})).unwrap();
    let all = ed.all_commands();
    let lyrics = all.iter().find(|c| c.id == crate::commands::element_id("Custom1")).expect("listed");
    assert_eq!((lyrics.label.as_str(), lyrics.shortcut.as_str()), ("Lyrics", "Mod+Alt+Y"));
    assert!(all.iter().any(|c| crate::commands::macro_slot(&c.id).is_some()), "the macros are listed");
    ed.run(&crate::commands::element_id("Custom1"), json!({}));
    assert_eq!(lines(&ed)[0].0, "Custom1");
}

#[test]
fn recovery_copies_are_offered_through_the_shell() {
    let ctx = ctx();
    let mut ed = editor();
    ed.run("file.recoverPrompt", json!({}));
    frame(&ctx, &mut ed, vec![]);
    assert!(ed.shell.dialogs.show_recovery, "the shell's recovery window opens");
    assert!(ed.shell.dialogs.can_restore, "a new script can take a copy");
}
