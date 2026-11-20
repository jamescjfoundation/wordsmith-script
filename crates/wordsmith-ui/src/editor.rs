// SPDX-License-Identifier: MIT OR Apache-2.0
//! The script editor: pages drawn from the layout, the caret and selection, and every key press,
//! click and menu choice turned into a command run through the session's registry.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Color32, Event, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordsmith_doc::StoryRef;
use wordsmith_engine::Session;
use wordsmith_script::paginate::Paginated;
use wordsmith_script::{ElementKind, Elements};

use crate::autosave::AutoSave;
use crate::dialogs::{Palette, RecoveryPrompt, SettingsDialog};
use crate::keys;

/// What to do once the writer has dealt with unsaved changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    New,
    Open,
    Import,
    /// Close the window.
    Close,
}

/// The writer's answer to "save changes first?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedAnswer {
    Save,
    DontSave,
    Cancel,
}

/// "Is this a script?", asked when importing plain text or RTF, with a guess for the default.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportQuestion {
    pub path: String,
    pub guess: bool,
}

/// Backup copies made on every save.
#[derive(Clone, Debug, PartialEq)]
pub struct Backups {
    pub enabled: bool,
    /// Backups kept per script.
    pub keep: usize,
    /// The backups folder; none means no backups are made.
    pub dir: Option<PathBuf>,
}

impl Default for Backups {
    fn default() -> Self {
        Backups { enabled: true, keep: 1_000, dir: None }
    }
}

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

/// Whether the script has a native file of its own to save to.
fn has_native_file(s: &Session) -> bool {
    s.path.as_deref().and_then(|p| p.extension()).and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("wss"))
}

/// Space between pages and around them, in points on screen.
const GAP: f32 = 16.0;
/// Zoom limits.
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.5..=3.0;

/// A tile's raster, for one document revision, scale and theme.
struct PageTexture {
    rev: u64,
    scale_bits: u32,
    dark: bool,
    texture: TextureHandle,
}

/// How the script is shown (SPEC "Views": normal, page and speed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum View {
    /// Pages as they print: (MORE), (CONT'D), page numbers.
    #[default]
    Page,
    /// The script in one continuous column, no page edges.
    Normal,
    /// Continuous and plain: every element at the left margin, its name in the gutter.
    Speed,
}

/// What a view shows: its document and layout, and, for the page view, the map back to the
/// script (the other views show the script itself).
#[derive(Clone)]
pub struct Shown {
    /// The document shown, when it is not the paginated script's (the continuous views).
    own: Option<Arc<wordsmith_doc::Document>>,
    pub layout: Arc<wordsmith_layout::DocLayout>,
    pub pag: Option<Arc<Paginated>>,
}

impl Shown {
    /// The document this view draws.
    pub fn doc(&self) -> &wordsmith_doc::Document {
        match (&self.own, &self.pag) {
            (Some(d), _) => d,
            (None, Some(p)) => &p.doc,
            (None, None) => empty_document(),
        }
    }

    pub fn to_output(&self, pos: &wordsmith_doc::Pos) -> Option<wordsmith_doc::Pos> {
        match &self.pag {
            Some(p) => p.to_output(pos),
            None => Some(pos.clone()),
        }
    }
    pub fn to_source(&self, pos: &wordsmith_doc::Pos) -> Option<wordsmith_doc::Pos> {
        match &self.pag {
            Some(p) => p.to_source(pos),
            None => Some(pos.clone()),
        }
    }
}

/// Height of one drawn tile of a tall page, in document points. A continuous view is one very
/// tall page; only its tiles in view are drawn. A page no taller than [`WHOLE_PAGE`] is drawn
/// as one tile (every printed page), since each tile costs a renderer set-up.
const TILE: f32 = 600.0;
const WHOLE_PAGE: f32 = 1_200.0;

/// The tile height for a page `h` points tall.
fn tile_height(h: f32) -> f32 {
    if h <= WHOLE_PAGE { h.max(1.0) } else { TILE }
}

/// The find bar's fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FindBar {
    pub query: String,
    pub with: String,
    pub match_case: bool,
    pub whole_word: bool,
    /// Matches found for the current query (shown in the bar).
    pub count: Option<usize>,
}

impl FindBar {
    pub fn params(&self) -> Value {
        json!({"query": self.query, "with": self.with, "matchCase": self.match_case, "wholeWord": self.whole_word})
    }
}

/// Autocomplete suggestions showing at the caret.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestions {
    pub candidates: Vec<String>,
    /// The highlighted one, which Tab or Enter accepts.
    pub selected: usize,
}

/// One script window's editor.
pub struct Editor {
    pub session: Session,
    pages: HashMap<(View, usize, usize), PageTexture>,
    /// Which view is showing.
    pub view: View,
    /// Dark interface and dark pages.
    pub dark: bool,
    /// Keep the caret's line at the middle of the window.
    pub typewriter: bool,
    /// Hide menus, toolbar and status bar.
    pub focus_mode: bool,
    /// The normal view's copy of the script, for one revision.
    normal_doc: Option<(u64, Arc<wordsmith_doc::Document>)>,
    /// The speed view's document and layout, for one revision.
    speed: Option<(u64, Shown)>,
    speed_cache: wordsmith_layout::LayoutCache,
    /// Points on screen per document point.
    pub zoom: f32,
    /// Apply macOS key conventions.
    pub mac: bool,
    pub palette: Option<Palette>,
    pub settings: Option<SettingsDialog>,
    pub recovery: Option<RecoveryPrompt>,
    pub show_about: bool,
    /// Scroll the caret into view on the next frame (after a keyboard command).
    scroll_to_caret: bool,
    /// The last error a command reported, for the status bar.
    pub message: String,
    /// Text a copy or cut asked to put on the clipboard.
    clipboard: Option<String>,
    /// Pagination, kept between edits so only changed paragraphs are measured again.
    paginator: wordsmith_script::paginate::Paginator,
    /// The paginated script for the session's current revision.
    paginated: Option<(u64, Arc<Paginated>)>,
    /// The find bar, while it is open.
    pub find: Option<FindBar>,
    /// Underline misspelled words.
    pub spelling: bool,
    spell_cache: wordsmith_script::spelling::SpellCache,
    /// The misspelling a right-click opened the menu on, with its suggestions.
    spell_menu: Option<(wordsmith_script::spelling::Misspelling, Vec<String>)>,
    /// Autocomplete suggestions at the caret, if any are showing.
    pub suggestions: Option<Suggestions>,
    /// Where suggestions were hidden with Escape: (revision, caret). They stay hidden there.
    dismissed: Option<(u64, wordsmith_doc::Pos)>,
    /// The caret's rectangle on screen in the last frame, for placing the suggestion list.
    caret_screen: Option<Rect>,
    /// Native file dialogs; each answer runs the command named with it, with the chosen path.
    #[cfg(not(target_arch = "wasm32"))]
    pub dialogs: craft_dialogs::Dialogs<&'static str>,
    /// "Save changes first?", while it is asked, and what follows the answer.
    pub unsaved: Option<Then>,
    /// "Is this a script?", while it is asked.
    pub import_question: Option<ImportQuestion>,
    /// What to do once the save under way completes (after "Save" in the unsaved prompt).
    after_save: Option<Then>,
    /// The writer chose not to save: the next open or import may replace the changes.
    discard_ok: bool,
    /// The writer agreed to close the window; the app closes it.
    pub close_confirmed: bool,
    pub autosave: AutoSave,
    pub backups: Backups,
    /// The preferences window is open.
    pub show_prefs: bool,
    /// The title page fields being edited, while the title page window is open.
    pub title_page: Option<Value>,
    /// The header and footer settings being edited, while their window is open.
    pub header_footer: Option<Value>,
    /// The index cards or the scene view, when one shows instead of the script.
    pub structure: Option<crate::structure::Structure>,
    /// The Tables panel's table, while the panel is open.
    pub tables: Option<crate::tables::Table>,
    /// The table cell being edited.
    pub table_edit: Option<crate::tables::CellEdit>,
    /// The scene listing, for one revision.
    scene_cache: Option<(u64, Arc<Vec<Value>>)>,
    /// The revision auto-save last saw, to notice changes.
    seen_rev: u64,
}

impl Editor {
    pub fn new(session: Session) -> Self {
        Editor {
            session,
            pages: HashMap::new(),
            view: View::Page,
            dark: false,
            typewriter: false,
            focus_mode: false,
            normal_doc: None,
            speed: None,
            speed_cache: Default::default(),
            zoom: 1.0,
            mac: cfg!(target_os = "macos"),
            palette: None,
            settings: None,
            recovery: None,
            show_about: false,
            scroll_to_caret: true,
            message: String::new(),
            clipboard: None,
            find: None,
            spelling: true,
            spell_cache: Default::default(),
            spell_menu: None,
            suggestions: None,
            dismissed: None,
            caret_screen: None,
            paginator: Default::default(),
            paginated: None,
            #[cfg(not(target_arch = "wasm32"))]
            dialogs: craft_dialogs::Dialogs::new(Arc::new(|| {})),
            unsaved: None,
            import_question: None,
            after_save: None,
            discard_ok: false,
            close_confirmed: false,
            autosave: AutoSave::default(),
            backups: Backups::default(),
            show_prefs: false,
            title_page: None,
            header_footer: None,
            structure: None,
            tables: None,
            table_edit: None,
            scene_cache: None,
            seen_rev: 0,
        }
    }

    /// Run a command; failures go to the status bar, never panic. New, Open and Import over
    /// unsaved changes ask first ([`Editor::unsaved`]).
    pub fn run(&mut self, id: &str, params: Value) {
        let then = match id {
            "file.new" => Some(Then::New),
            "file.open" => Some(Then::Open),
            "file.import" => Some(Then::Import),
            _ => None,
        };
        if let Some(then) = then
            && self.session.dirty
        {
            self.unsaved = Some(then);
            return;
        }
        self.run_now(id, params);
    }

    /// The script's scenes as `scene.list` gives them, kept for one revision.
    pub fn scene_list(&mut self) -> Arc<Vec<Value>> {
        let rev = self.session.rev();
        if let Some((r, list)) = &self.scene_cache
            && *r == rev
        {
            return list.clone();
        }
        let list: Vec<Value> = self.session.run("scene.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let list = Arc::new(list);
        self.scene_cache = Some((rev, list.clone()));
        list
    }

    /// Run a command without asking about unsaved changes.
    fn run_now(&mut self, id: &str, params: Value) {
        match self.session.run(id, &params) {
            Ok(_) => self.message.clear(),
            Err(e) => self.message = e.to_string(),
        }
        self.scroll_to_caret = true;
        self.take_ui_requests();
        self.refresh_suggestions();
    }

    /// Ask the script what is offered at the caret now. Suggestions hidden with Escape stay
    /// hidden until the text or the caret changes.
    pub fn refresh_suggestions(&mut self) {
        let here = (self.session.rev(), self.session.sel.focus.clone());
        if self.dismissed.as_ref() == Some(&here) {
            self.suggestions = None;
            return;
        }
        self.dismissed = None;
        let offered = self.session.run("autocomplete.suggest", &json!({})).ok();
        let candidates: Vec<String> =
            offered.and_then(|v| v.get("candidates").cloned()).and_then(|c| serde_json::from_value(c).ok()).unwrap_or_default();
        self.suggestions = match (&self.suggestions, candidates.is_empty()) {
            (_, true) => None,
            // Keep the highlight on the same entry while it is still offered.
            (Some(old), false) => {
                let keep = old.candidates.get(old.selected).and_then(|c| candidates.iter().position(|n| n == c)).unwrap_or(0);
                Some(Suggestions { candidates, selected: keep })
            }
            (None, false) => Some(Suggestions { candidates, selected: 0 }),
        };
    }

    /// Keys the suggestion list takes while it shows: Tab and Enter accept, Up and Down move,
    /// Escape hides. Returns whether the key was taken.
    fn suggestion_key(&mut self, shortcut: &str) -> bool {
        let Some(sug) = self.suggestions.clone() else { return false };
        match shortcut {
            "Tab" | "Enter" => {
                if let Some(c) = sug.candidates.get(sug.selected) {
                    self.run("autocomplete.accept", json!({"candidate": c}));
                }
                true
            }
            "Down" => {
                self.suggestions = Some(Suggestions { selected: (sug.selected + 1) % sug.candidates.len().max(1), ..sug });
                true
            }
            "Up" => {
                let n = sug.candidates.len().max(1);
                self.suggestions = Some(Suggestions { selected: (sug.selected + n - 1) % n, ..sug });
                true
            }
            "Escape" => {
                self.dismissed = Some((self.session.rev(), self.session.sel.focus.clone()));
                self.suggestions = None;
                true
            }
            _ => false,
        }
    }

    /// Escape with no suggestions showing leaves focus mode.
    fn leave_focus(&mut self, shortcut: &str) -> bool {
        if shortcut == "Escape" && self.focus_mode {
            self.focus_mode = false;
            return true;
        }
        false
    }

    /// Ask about closing the window; true when it may close now (no unsaved changes, or the
    /// writer already answered).
    pub fn may_close(&mut self) -> bool {
        if self.close_confirmed || !self.session.dirty {
            return true;
        }
        self.unsaved = Some(Then::Close);
        false
    }

    /// The writer's answer to "save changes first?".
    pub fn answer_unsaved(&mut self, answer: UnsavedAnswer) {
        let Some(then) = self.unsaved.take() else { return };
        match answer {
            UnsavedAnswer::Save => {
                self.after_save = Some(then);
                self.run_now("file.save", json!({}));
                if !self.message.is_empty() {
                    // The save failed: stay put.
                    self.after_save = None;
                }
            }
            UnsavedAnswer::DontSave => {
                self.discard_ok = true;
                self.session.discard_recovery_copy();
                self.proceed(then);
            }
            UnsavedAnswer::Cancel => {}
        }
    }

    /// Carry on with what waited for unsaved changes.
    fn proceed(&mut self, then: Then) {
        match then {
            Then::New => {
                self.run_now("file.new", json!({}));
                self.discard_ok = false;
            }
            Then::Open => self.run_now("file.open", json!({})),
            Then::Import => self.run_now("file.import", json!({})),
            Then::Close => self.close_confirmed = true,
        }
    }

    /// The writer's answer to "is this a script?" (`None`: cancel the import).
    pub fn answer_import(&mut self, as_script: Option<bool>) {
        let Some(q) = self.import_question.take() else { return };
        if let Some(as_script) = as_script {
            let discard = std::mem::take(&mut self.discard_ok);
            self.run_now("file.openPath", json!({"path": q.path, "asScript": as_script, "discard": discard}));
        } else {
            self.discard_ok = false;
        }
    }

    /// A save finished: make the backup copy, restart the auto-save clock, and carry on with
    /// whatever waited for the save.
    fn on_saved(&mut self, path: &str) {
        self.autosave.saved();
        #[cfg(not(target_arch = "wasm32"))]
        if self.backups.enabled
            && let Some(dir) = self.backups.dir.clone()
            && let Err(e) = wordsmith_format::files::backup(std::path::Path::new(path), &dir, self.backups.keep, std::time::SystemTime::now())
        {
            self.message = format!("Saved, but the backup copy failed: {e}");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = path;
        if let Some(then) = self.after_save.take() {
            self.proceed(then);
        }
    }

    /// Auto-save: notice changes, and save when one is due. Call every frame with the time;
    /// returns how long until the next save is due, to wake the window then.
    pub fn tick(&mut self, now: Instant) -> Option<Duration> {
        let rev = self.session.rev();
        if rev != self.seen_rev {
            self.seen_rev = rev;
            if self.session.dirty {
                self.autosave.edited(now);
            }
        }
        let has_file = has_native_file(&self.session);
        if self.autosave.due(now, self.session.dirty, has_file) {
            self.run_now("file.save", json!({}));
            if self.session.dirty {
                // It failed: try again after another interval, not every frame.
                self.message = format!("Auto-save failed: {}", self.message);
                self.autosave.saved();
                self.autosave.edited(now);
            }
        }
        self.autosave.due_at(self.session.dirty, has_file).map(|at| at.saturating_duration_since(now))
    }

    /// The window title: the script's name, marked while it has unsaved changes.
    pub fn window_title(&self) -> String {
        let name = script_name(&self.session);
        if self.session.dirty { format!("{name} • — Wordsmith Script") } else { format!("{name} — Wordsmith Script") }
    }

    /// Act on what commands asked of the interface (open a dialog, copy text).
    fn take_ui_requests(&mut self) {
        for req in std::mem::take(&mut self.session.ui_requests) {
            if let Some(text) = req.get("copy").and_then(Value::as_str) {
                self.clipboard = Some(text.to_string());
            }
            if let Some(path) = req.get("saved").and_then(Value::as_str) {
                self.on_saved(path);
            }
            match req.get("open").and_then(Value::as_str) {
                Some("palette") => self.palette = Some(Palette::default()),
                Some("elementSettings") => {
                    let current = Elements::of_doc(&self.session.doc).of_para_at(&self.session).to_string();
                    self.settings = Some(SettingsDialog::new(&mut self.session, &current));
                }
                Some("about") => self.show_about = true,
                Some("recover") => self.recovery = Some(RecoveryPrompt),
                Some("exportPdf") => self.ask_export_path(),
                Some("open" | "import" | "saveAs" | "export") => self.ask_file(&req),
                Some("importQuestion") => {
                    let path = req.get("path").and_then(Value::as_str).unwrap_or("").to_string();
                    let guess = req.get("guess").and_then(Value::as_bool).unwrap_or(true);
                    self.import_question = Some(ImportQuestion { path, guess });
                }
                Some("preferences") => self.show_prefs = true,
                Some("titlePage") => self.title_page = self.session.run("titlePage.get", &json!({})).ok(),
                Some("headerFooter") => self.header_footer = self.session.run("headerFooter.get", &json!({})).ok(),
                Some("view.page") => self.set_view(View::Page),
                Some("view.normal") => self.set_view(View::Normal),
                Some("view.speed") => self.set_view(View::Speed),
                Some("view.tables") => {
                    self.tables = if self.tables.is_some() { None } else { Some(crate::tables::Table::Scenes) };
                    self.table_edit = None;
                }
                Some("view.cards") => self.structure = Some(crate::structure::Structure::Cards),
                Some("view.scenes") => self.structure = Some(crate::structure::Structure::Scenes),
                Some("view.dark") => self.dark = !self.dark,
                Some("view.typewriter") => self.typewriter = !self.typewriter,
                Some("view.focus") => self.focus_mode = !self.focus_mode,
                Some("find") => {
                    let query = self.session.selected_text();
                    let bar = self.find.take().unwrap_or_default();
                    self.find = Some(FindBar { query: if query.is_empty() || query.contains('\n') { bar.query.clone() } else { query }, ..bar });
                }
                _ => {}
            }
        }
    }

    /// Text waiting for the clipboard (for tests).
    pub fn pending_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ask_export_path(&mut self) {
        let dialog = craft_dialogs::FileDialog::save().title("Export PDF").filter("PDF", &["pdf"]).file_name("Script.pdf");
        self.dialogs.ask(dialog, "file.exportPdf");
    }

    #[cfg(target_arch = "wasm32")]
    fn ask_export_path(&mut self) {
        self.message = "Export PDF is not available in the web build yet.".into();
    }

    /// A native file dialog for a file command; its answer runs the command that takes the path.
    #[cfg(not(target_arch = "wasm32"))]
    fn ask_file(&mut self, req: &Value) {
        use craft_dialogs::FileDialog;
        let name = req.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or("Untitled");
        let (dialog, then) = match req.get("open").and_then(Value::as_str) {
            Some("open") => (
                FileDialog::open()
                    .title("Open Script")
                    .filter("Scripts", &["wss", "fountain", "spmd"])
                    .filter("Wordsmith Script", &["wss"])
                    .filter("Fountain", &["fountain", "spmd"]),
                "file.openPath",
            ),
            Some("import") => (
                FileDialog::open()
                    .title("Import")
                    .filter("Text, RTF, PDF or Fountain", &["txt", "text", "rtf", "pdf", "fountain", "spmd"])
                    .filter("Plain Text", &["txt", "text"])
                    .filter("Rich Text (RTF)", &["rtf"])
                    .filter("PDF", &["pdf"]),
                "file.openPath",
            ),
            Some("saveAs") => {
                (FileDialog::save().title("Save Script").filter("Wordsmith Script", &["wss"]).file_name(format!("{name}.wss")), "file.saveAsPath")
            }
            Some("export") => {
                let (label, ext, then) = match req.get("format").and_then(Value::as_str) {
                    Some("csv-scenes") => ("CSV", "csv", "tables.exportScenesCsv"),
                    Some("csv-characters") => ("CSV", "csv", "tables.exportCharactersCsv"),
                    Some("fountain") => ("Fountain", "fountain", "file.exportFountain"),
                    Some("rtf") => ("Rich Text (RTF)", "rtf", "file.exportRtf"),
                    Some("html") => ("Web Page (HTML)", "html", "file.exportHtml"),
                    _ => ("Plain Text", "txt", "file.exportText"),
                };
                (FileDialog::save().title(format!("Export {label}")).filter(label, &[ext]).file_name(format!("{name}.{ext}")), then)
            }
            _ => return,
        };
        self.dialogs.ask(dialog, then);
    }

    #[cfg(target_arch = "wasm32")]
    fn ask_file(&mut self, _req: &Value) {
        self.message = "Files are not available in the web build yet.".into();
    }

    /// Show a waiting native dialog over `parent` (the window), and act on answers: each runs the
    /// command it was asked for, with the chosen path. Call once a frame, after [`Editor::ui`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_dialogs<W: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle>(&mut self, parent: Option<&W>) {
        self.dialogs.show_pending(parent);
        for (then, answer) in self.dialogs.take_answers() {
            match answer {
                craft_dialogs::Answer::Paths(paths) => {
                    if let Some(path) = paths.first() {
                        let params = if then == "file.openPath" {
                            json!({"path": path, "discard": std::mem::take(&mut self.discard_ok)})
                        } else {
                            json!({"path": path})
                        };
                        self.run_now(then, params);
                        if then == "file.saveAsPath" && self.session.dirty {
                            // The save failed: nothing waits on it any more.
                            self.after_save = None;
                        }
                    }
                }
                craft_dialogs::Answer::Cancelled => {
                    // Nothing waits on a dialog the writer closed.
                    self.discard_ok = false;
                    self.after_save = None;
                }
                craft_dialogs::Answer::Message(_) => {}
                other => self.message = format!("{other:?}"),
            }
        }
    }

    /// Whether the editor should take keyboard input now (no dialog or text field has it).
    fn has_keyboard(&self, ui: &Ui) -> bool {
        self.palette.is_none() && self.settings.is_none() && !ui.ctx().text_edit_focused()
    }

    /// Turn this frame's keyboard events into commands.
    pub fn handle_keys(&mut self, ui: &Ui) {
        if !self.has_keyboard(ui) {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.palette = None;
                self.settings = None;
            }
            return;
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            // In the card and scene views, typing does not reach the script: only shortcuts with
            // a modifier run, and Escape goes back to the script.
            if self.structure.is_some() {
                match &event {
                    Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                        self.structure = None;
                        continue;
                    }
                    Event::Key { modifiers, .. } if modifiers.command || modifiers.ctrl || modifiers.alt => {}
                    _ => continue,
                }
            }
            match event {
                Event::Text(text) => self.run("text.insert", json!({"text": text})),
                Event::Paste(text) => self.run("text.insert", json!({"text": text})),
                Event::Copy => self.run("edit.copy", json!({})),
                Event::Cut => self.run("edit.cut", json!({})),
                Event::Key { key, pressed: true, modifiers, .. } => {
                    let Some(shortcut) = keys::shortcut(key, modifiers, self.mac) else { continue };
                    if !self.suggestion_key(&shortcut) && !self.leave_focus(&shortcut) {
                        self.press(&shortcut);
                    }
                }
                _ => {}
            }
        }
        // Copy and cut put text on the clipboard through the commands' requests.
        if let Some(text) = self.clipboard.take() {
            ui.ctx().copy_text(text);
        }
        // Tab and arrows are ours, not egui's focus navigation.
        ui.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { key: egui::Key::Tab, .. })));
    }

    /// Run the command bound to `shortcut`: a custom element's own shortcut first, then the
    /// registry's.
    pub fn press(&mut self, shortcut: &str) {
        let elements = Elements::of_doc(&self.session.doc);
        if let Some(def) =
            elements.all().iter().find(|d| d.kind == ElementKind::Custom && d.shortcut.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(shortcut)))
        {
            let id = def.id.clone();
            self.run("element.set", json!({"element": id}));
            return;
        }
        let id = self.session.registry.by_shortcut(shortcut).map(|spec| spec.id);
        if let Some(id) = id {
            self.run(id, json!({}));
        }
    }

    /// The paginated script for the current revision, paginating again after an edit.
    pub fn paginated(&mut self) -> Arc<Paginated> {
        let rev = self.session.rev();
        if let Some((r, p)) = &self.paginated
            && *r == rev
        {
            return p.clone();
        }
        let p = Arc::new(self.paginator.paginate(&self.session.doc));
        self.paginated = Some((rev, p.clone()));
        p
    }

    /// Switch views. Up and Down follow the view's lines.
    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.structure = None;
        self.session.view_mode = match view {
            View::Page => wordsmith_layout::ViewMode::Print,
            View::Normal | View::Speed => wordsmith_layout::ViewMode::Web,
        };
        self.session.view_width = text_width(&self.session.doc);
        self.scroll_to_caret = true;
    }

    /// What the current view shows.
    pub fn shown(&mut self) -> Shown {
        match self.view {
            View::Page => {
                let pag = self.paginated();
                Shown { own: None, layout: pag.layout.clone(), pag: Some(pag) }
            }
            View::Normal => {
                self.session.view_mode = wordsmith_layout::ViewMode::Web;
                self.session.view_width = text_width(&self.session.doc);
                let rev = self.session.rev();
                let doc = match &self.normal_doc {
                    Some((r, d)) if *r == rev => d.clone(),
                    _ => {
                        let d = Arc::new(self.session.doc.clone());
                        self.normal_doc = Some((rev, d.clone()));
                        d
                    }
                };
                Shown { own: Some(doc), layout: self.session.layout(), pag: None }
            }
            View::Speed => {
                let rev = self.session.rev();
                if let Some((r, s)) = &self.speed
                    && *r == rev
                {
                    return s.clone();
                }
                let doc = speed_document(&self.session.doc);
                let opts =
                    wordsmith_layout::LayoutOptions { view: wordsmith_layout::ViewMode::Web, web_width: text_width(&doc), ..Default::default() };
                let layout = Arc::new(wordsmith_layout::layout(&doc, &mut self.speed_cache, &opts));
                let s = Shown { own: Some(Arc::new(doc)), layout, pag: None };
                self.speed = Some((rev, s.clone()));
                s
            }
        }
    }

    /// A page's texture at `scale` pixels per document point, drawn again when the document or
    /// scale changed.
    fn texture(
        &mut self,
        ctx: &egui::Context,
        doc: &wordsmith_doc::Document,
        index: usize,
        tile: usize,
        page: &wordsmith_layout::Page,
        scale: f32,
    ) -> TextureHandle {
        let rev = self.session.rev();
        let bits = scale.to_bits();
        let key = (self.view, index, tile);
        if let Some(t) = self.pages.get(&key)
            && t.rev == rev
            && t.scale_bits == bits
            && t.dark == self.dark
        {
            return t.texture.clone();
        }
        let opts = wordsmith_render::RenderOptions { dark: self.dark, ..Default::default() };
        let th = tile_height(page.h);
        let top = tile as f32 * th;
        let area = wordsmith_geom::Rect { x: 0.0, y: top, w: page.w, h: (page.h - top).clamp(1.0, th) };
        let r = wordsmith_render::render_area(doc, page, area, scale, &opts);
        let image = egui::ColorImage::from_rgba_premultiplied([r.width as usize, r.height as usize], &r.pixels);
        let texture = match self.pages.remove(&key) {
            Some(mut t) => {
                t.texture.set(image, TextureOptions::LINEAR);
                t.texture
            }
            None => ctx.load_texture(format!("page-{index}-{tile}"), image, TextureOptions::LINEAR),
        };
        self.pages.insert(key, PageTexture { rev, scale_bits: bits, dark: self.dark, texture: texture.clone() });
        texture
    }

    /// The pages, caret and selection, with mouse handling. The pages are the paginated script
    /// (with its (MORE) and (CONT'D) lines and page numbers); the caret and clicks map to the
    /// script itself.
    pub fn page_view(&mut self, ui: &mut Ui) {
        let pag = self.shown();
        let layout = pag.layout.clone();
        let zoom = self.zoom;
        let ppp = ui.ctx().pixels_per_point();
        let widest = layout.pages.iter().map(|p| p.w).fold(0.0f32, f32::max) * zoom;
        let height: f32 = layout.pages.iter().map(|p| p.h * zoom + GAP).sum::<f32>() + GAP;
        let width = (widest + 2.0 * GAP).max(ui.available_width());
        let (area, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        // The page view keeps keyboard focus, and Tab and the arrows stay ours instead of moving
        // egui's focus between widgets.
        if ui.memory(|m| m.focused().is_none()) || response.clicked() {
            response.request_focus();
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(response.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false })
        });
        let painter = ui.painter_at(area);
        let (desk, paper) = if self.dark { (Color32::from_gray(24), Color32::from_gray(40)) } else { (Color32::from_gray(96), Color32::WHITE) };
        painter.rect_filled(area, 0.0, desk);

        let mut origins: Vec<Rect> = Vec::with_capacity(layout.pages.len());
        let mut y = area.top() + GAP;
        for page in &layout.pages {
            let size = vec2(page.w * zoom, page.h * zoom);
            origins.push(Rect::from_min_size(pos2(area.center().x - size.x / 2.0, y), size));
            y += size.y + GAP;
        }
        for (i, page) in layout.pages.iter().enumerate() {
            let Some(rect) = origins.get(i).copied() else { continue };
            if !ui.is_rect_visible(rect) {
                continue;
            }
            // Draw the page in tiles, only those in view.
            let th = tile_height(page.h);
            let tiles = (page.h / th).ceil().max(1.0) as usize;
            for tile in 0..tiles {
                let top = tile as f32 * th;
                let h = (page.h - top).min(th);
                let tile_rect = Rect::from_min_size(rect.min + vec2(0.0, top * zoom), vec2(rect.width(), h * zoom));
                if ui.is_rect_visible(tile_rect) {
                    let tex = self.texture(ui.ctx(), pag.doc(), i, tile, page, zoom * ppp);
                    painter.image(tex.id(), tile_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                } else {
                    painter.rect_filled(tile_rect, 0.0, paper);
                }
            }
        }
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        self.draw_gutter(ui, &painter, &pag, &origins, zoom);
        self.draw_find_matches(&painter, &pag, &origins, zoom);

        // Selection, then the caret.
        if !self.session.sel.is_collapsed() {
            let (a, b) = self.session.sel.ordered();
            let (a, b) = (pag.to_output(&a).unwrap_or(a), pag.to_output(&b).unwrap_or(b));
            for (page, r) in layout.selection_rects(pag.doc(), &a, &b, 0) {
                if let Some(min) = to_screen(page, r.x, r.y) {
                    painter.rect_filled(
                        Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)),
                        0.0,
                        Color32::from_rgba_unmultiplied(64, 128, 255, 70),
                    );
                }
            }
        }
        self.draw_misspellings(ui, &painter, &pag, &origins, zoom);
        let mut caret_rect = None;
        if let Some(c) = pag.to_output(&self.session.sel.focus).and_then(|at| layout.caret(&at))
            && let Some(top) = to_screen(c.page, c.x, c.top)
        {
            let r = Rect::from_min_size(top, vec2(1.5, c.height * zoom));
            let ink = if self.dark { Color32::from_gray(230) } else { Color32::BLACK };
            painter.line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.5, ink));
            caret_rect = Some(r.expand(24.0));
            self.caret_screen = Some(r);
        }
        if std::mem::take(&mut self.scroll_to_caret)
            && let Some(r) = caret_rect
        {
            // Typewriter scrolling keeps the caret's line at the middle of the window.
            ui.scroll_to_rect(r, self.typewriter.then_some(egui::Align::Center));
        }

        // Mouse: a click places the caret, Shift+click or a drag extends, a double-click selects
        // a word.
        let hit = |p: Pos2| -> Option<Value> {
            let page = origins.iter().position(|r| r.contains(p)).or_else(|| nearest_page(&origins, p))?;
            let r = origins.get(page)?;
            let (x, y) = ((p.x - r.min.x) / zoom, (p.y - r.min.y) / zoom);
            layout.hit(page, x, y, StoryRef::Body).and_then(|pos| pag.to_source(&pos)).and_then(|pos| serde_json::to_value(pos).ok())
        };
        // Right-click on a misspelled word: its suggestions, Ignore, Add to Dictionary.
        if response.secondary_clicked()
            && let Some(p) = response.interact_pointer_pos()
            && let Some(pos) = hit(p).and_then(|v| serde_json::from_value::<wordsmith_doc::Pos>(v).ok())
        {
            let found = wordsmith_script::spelling::check(&self.session.doc, &mut self.spell_cache);
            self.spell_menu = found.into_iter().find(|m| m.block == pos.path.last() && (m.start..=m.end).contains(&pos.off)).map(|m| {
                let s = wordsmith_script::spelling::suggest(&m.word, 6);
                (m, s)
            });
        }
        let mut menu_action: Option<(&'static str, Value)> = None;
        if let Some((m, suggestions)) = self.spell_menu.clone() {
            response.context_menu(|ui| {
                for s in &suggestions {
                    if ui.button(s.as_str()).clicked() {
                        menu_action =
                            Some(("spelling.replace", json!({"block": m.block, "start": m.start, "end": m.end, "word": m.word, "with": s})));
                        ui.close();
                    }
                }
                if suggestions.is_empty() {
                    ui.weak("No suggestions");
                }
                ui.separator();
                if ui.button("Ignore in This Script").clicked() {
                    menu_action = Some(("spelling.ignore", json!({"word": m.word})));
                    ui.close();
                }
                if ui.button("Add to Dictionary").clicked() {
                    menu_action = Some(("spelling.addWord", json!({"word": m.word})));
                    ui.close();
                }
            });
        }
        if let Some((id, params)) = menu_action {
            self.spell_menu = None;
            self.run(id, params);
        }
        if let Some(p) = response.interact_pointer_pos().filter(|_| !response.secondary_clicked()) {
            let shift = ui.input(|i| i.modifiers.shift);
            if response.double_clicked() {
                if let Some(pos) = hit(p) {
                    self.run("select.wordAt", json!({"pos": pos}));
                }
            } else if response.drag_started() || response.clicked() {
                if let Some(pos) = hit(p) {
                    self.run("caret.to", json!({"pos": pos, "extend": shift}));
                }
            } else if response.dragged()
                && let Some(pos) = hit(p)
            {
                self.run("caret.to", json!({"pos": pos, "extend": true}));
            }
            self.scroll_to_caret = false;
        }
    }

    /// The speed view names each paragraph's element in the left gutter.
    fn draw_gutter(&self, ui: &Ui, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        if self.view != View::Speed {
            return;
        }
        let elements = Elements::of_doc(&self.session.doc);
        let font = egui::FontId::proportional(10.0 * zoom.max(0.8));
        let colour = if self.dark { Color32::from_gray(150) } else { Color32::from_gray(220) };
        for (i, b) in self.session.doc.body.iter().enumerate() {
            let Some(p) = b.as_para() else { continue };
            let at = wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(i), off: 0 };
            let Some(c) = pag.layout.caret(&at) else { continue };
            let Some(page) = origins.get(c.page) else { continue };
            let y = page.min.y + c.top * zoom;
            if !ui.is_rect_visible(Rect::from_min_size(pos2(page.min.x - 120.0, y), vec2(120.0, c.height * zoom))) {
                continue;
            }
            painter.text(pos2(page.min.x - 8.0, y), egui::Align2::RIGHT_TOP, elements.of_para(p).name.as_str(), font.clone(), colour);
        }
    }

    /// Every match of the find bar's query, on the pages in view.
    fn draw_find_matches(&self, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        if let Some(bar) = self.find.as_ref().filter(|b| !b.query.is_empty()) {
            let q = wordsmith_script::commands::find::Query { text: bar.query.clone(), match_case: bar.match_case, whole_word: bar.whole_word };
            let fill = Color32::from_rgba_unmultiplied(255, 210, 0, 90);
            for (a, b) in wordsmith_script::commands::find::find_all(&self.session, &q).into_iter().take(5_000) {
                let (Some(a), Some(b)) = (pag.to_output(&a), pag.to_output(&b)) else { continue };
                for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &b, 0) {
                    if let Some(min) = to_screen(page, r.x, r.y) {
                        painter.rect_filled(Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)), 0.0, fill);
                    }
                }
            }
        }
    }

    /// Misspelled words: a dotted red line under each, on the pages in view.
    fn draw_misspellings(&mut self, ui: &Ui, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        if self.spelling {
            let found = wordsmith_script::spelling::check(&self.session.doc, &mut self.spell_cache);
            let red = Stroke::new(1.5, Color32::from_rgb(220, 40, 40));
            for m in &found {
                let at = |off| wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(m.block), off };
                let (Some(a), Some(b)) = (pag.to_output(&at(m.start)), pag.to_output(&at(m.end))) else { continue };
                for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &b, 0) {
                    let Some(min) = to_screen(page, r.x, r.y + r.h - 1.0) else { continue };
                    if !ui.is_rect_visible(Rect::from_min_size(min, vec2(r.w * zoom, 2.0))) {
                        continue;
                    }
                    let (x0, x1, y) = (min.x, min.x + r.w * zoom, min.y);
                    let mut x = x0;
                    while x < x1 {
                        painter.line_segment([pos2(x, y), pos2((x + 2.0).min(x1), y)], red);
                        x += 4.0;
                    }
                }
            }
        }
    }

    /// The whole window: menus, toolbar, status bar, pages and dialogs.
    pub fn ui(&mut self, ui: &mut Ui) {
        let theme = if self.dark { egui::Theme::Dark } else { egui::Theme::Light };
        if ui.ctx().theme() != theme {
            ui.ctx().set_theme(theme);
        }
        self.handle_keys(ui);
        if !self.focus_mode {
            crate::chrome::menu_bar(self, ui);
            crate::chrome::toolbar(self, ui);
            crate::chrome::find_bar(self, ui);
            crate::chrome::status_bar(self, ui);
            crate::tables::panel(self, ui);
        }
        egui::CentralPanel::default().show(ui, |ui| match self.structure {
            Some(crate::structure::Structure::Cards) => crate::structure::cards(self, ui),
            Some(crate::structure::Structure::Scenes) => crate::structure::scene_view(self, ui),
            None => {
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| self.page_view(ui));
            }
        });
        self.suggestion_list(ui.ctx());
        crate::dialogs::show(self, ui.ctx());
    }

    /// The suggestion list, just below the caret. Clicking an entry accepts it.
    fn suggestion_list(&mut self, ctx: &egui::Context) {
        let (Some(sug), Some(caret)) = (self.suggestions.clone(), self.caret_screen) else { return };
        let mut chosen = None;
        egui::Area::new(egui::Id::new("autocomplete")).order(egui::Order::Foreground).fixed_pos(caret.left_bottom() + vec2(0.0, 4.0)).show(
            ctx,
            |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (i, c) in sug.candidates.iter().enumerate() {
                        if ui.selectable_label(i == sug.selected, c.as_str()).clicked() {
                            chosen = Some(c.clone());
                        }
                    }
                    ui.weak("Tab or Enter to accept · Esc to hide");
                });
            },
        );
        if let Some(c) = chosen {
            self.run("autocomplete.accept", json!({"candidate": c}));
        }
    }
}

/// An empty document, for a view with nothing to show.
fn empty_document() -> &'static wordsmith_doc::Document {
    static EMPTY: std::sync::LazyLock<wordsmith_doc::Document> = std::sync::LazyLock::new(wordsmith_doc::Document::new);
    &EMPTY
}

/// The text column's width, points: what the continuous views lay the script out at, so lines
/// break as on paper.
fn text_width(doc: &wordsmith_doc::Document) -> f32 {
    let s = &doc.last_section;
    (s.page_w - s.margin_left - s.margin_right).max(72.0)
}

/// The speed view's document: the script with every element at the left margin, left-aligned,
/// with no space before except before Scene Headings. Paragraphs, text and casing are the
/// script's own, so positions are the same.
pub fn speed_document(doc: &wordsmith_doc::Document) -> wordsmith_doc::Document {
    let mut out = doc.clone();
    let elements = Elements::of_doc(doc);
    let sheet = Arc::make_mut(&mut out.styles);
    for st in &mut sheet.styles {
        let heading = elements.get(&st.id).is_some_and(|d| d.kind == ElementKind::SceneHeading);
        st.para.indent_left = Some(0.0);
        st.para.indent_right = Some(0.0);
        st.para.indent_first = Some(0.0);
        st.para.align = Some(wordsmith_doc::props::Align::Left);
        st.para.space_before = Some(if heading { wordsmith_script::template::LINE } else { 0.0 });
        st.para.page_break_before = Some(false);
    }
    out
}

/// The page whose vertical span is nearest `p` (a click in the gap between pages).
fn nearest_page(origins: &[Rect], p: Pos2) -> Option<usize> {
    origins
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.center().y - p.y).abs();
            let db = (b.center().y - p.y).abs();
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

/// Lookups the editor needs on the registry.
trait ElementsExt {
    fn of_para_at(&self, s: &Session) -> &str;
}

impl ElementsExt for Elements {
    fn of_para_at(&self, s: &Session) -> &str {
        match s.doc.para_at(&s.sel.focus) {
            Some(p) => &self.of_para(p).id,
            None => self.id_of(ElementKind::General),
        }
    }
}
