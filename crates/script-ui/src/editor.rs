// SPDX-License-Identifier: MIT OR Apache-2.0
//! The script editor: pages drawn from the layout, the caret and selection, and every key press,
//! click and menu choice turned into a command run through the session's registry. The window
//! around it (ribbon, panels, File page, dialogs, preferences, auto-save) is the shell's
//! (`jcj_shell`); what the editor asks of it goes through [`ToShell`].

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Color32, Event, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, pos2, vec2};
use jcj_doc::StoryRef;
use jcj_engine::Session;
use jcj_shell::Page;
use jcj_shell::files::FileAsk as ShellFileAsk;
use jcj_shell::theme::Mode;
use jcj_shell::workspace::Side;
use serde_json::{Value, json};
use wordsmith_script_model::keys::ScriptRun;
use wordsmith_script_model::paginate::Paginated;
use wordsmith_script_model::{ElementKind, Elements};

pub use jcj_shell::dialogs::UnsavedAnswer;

use crate::dialogs::SettingsDialog;

/// What to do once the writer has dealt with unsaved changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    New,
    Open,
    Import,
    /// Close the window.
    Close,
    /// Close the script's tab.
    CloseTab,
}

impl Then {
    /// The name the shell's "save changes first?" hands back.
    pub fn key(self) -> &'static str {
        match self {
            Then::New => "new",
            Then::Open => "open",
            Then::Import => "import",
            Then::Close => "close",
            Then::CloseTab => "closeTab",
        }
    }

    /// What is about to happen, as words after "before".
    fn doing(self) -> &'static str {
        match self {
            Then::New => "starting a new script",
            Then::Open => "opening another script",
            Then::Import => "importing",
            Then::Close => "closing",
            Then::CloseTab => "closing its tab",
        }
    }
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

/// The command the shell runs when a file was chosen in a dialog the editor asked for:
/// `{"ask": number, "path": the choice}`. It is the shell's answer, not a command of the app's.
pub(crate) const FILE_CHOSEN: &str = "file.dialogChosen";
/// The command the shell runs when such a dialog was closed without a choice: `{"ask": number}`.
pub(crate) const FILE_CLOSED: &str = "file.dialogClosed";
/// Most file dialogs whose answers the editor waits for; the oldest beyond that is forgotten.
const MAX_FILES_OUT: usize = 64;

/// A file dialog the editor needs. The shell shows it ([`Editor::flush`] hands it over) and its
/// answer comes back as a command; [`Editor::answer_file`] then runs the command `then` with
/// the chosen path.
///
/// A dialog does not stop the window: while one is up the writer may switch tabs or ask for
/// another. So an ask carries what its answer needs, taken when it was asked: the tab it is
/// for, the options of the command it runs, and the step that waits on its save.
#[derive(Clone, Debug, PartialEq)]
pub struct FileAsk {
    /// The dialog: open or save, its title, the file types offered, the name it suggests.
    pub dialog: ShellFileAsk,
    /// The command the chosen path runs.
    pub then: &'static str,
    /// The parameters that command gets beside the path.
    pub params: serde_json::Map<String, Value>,
    /// The tab the dialog is for ([`Editor::tab_id`]).
    pub tab: u64,
    /// What to do once the save this dialog asks about completes.
    after_save: Option<Then>,
}

impl FileAsk {
    fn open(title: &str, then: &'static str) -> Self {
        FileAsk::of(ShellFileAsk::open().title(title), then)
    }

    fn save(title: &str, file_name: String, then: &'static str) -> Self {
        FileAsk::of(ShellFileAsk::save().title(title).file_name(file_name), then)
    }

    fn of(dialog: ShellFileAsk, then: &'static str) -> Self {
        FileAsk { dialog, then, params: serde_json::Map::new(), tab: 0, after_save: None }
    }

    fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.dialog = self.dialog.filter(name, extensions);
        self
    }
}

/// What the editor asks of the shell around it: passed on once a frame ([`Editor::flush`]).
#[derive(Clone, Debug, PartialEq)]
pub enum ToShell {
    /// Open the File page at this page.
    FilePage(Page),
    /// Open the command palette.
    Palette,
    /// Check for a newer release now.
    CheckForUpdates,
    /// Use this theme.
    Theme(Mode),
    /// Show this panel (on its side).
    ShowPanel(String),
    /// Show or hide this side's panel.
    ToggleSide(Side),
    /// Show the copies crash recovery kept.
    Recovery,
    /// This script was opened or saved: first among the recent scripts.
    Recent(String),
    /// Run one of the shell's own commands (a key bound to it).
    Run(jcj_shell::Cmd),
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
pub fn has_native_file(s: &Session) -> bool {
    s.path.as_deref().and_then(|p| p.extension()).and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("wss"))
}

/// Space between pages and around them, in points on screen.
const GAP: f32 = 16.0;
/// Zoom limits.
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.5..=3.0;
/// The zoom steps the view menu and status bar step through (SPEC-2 8: 75 % to 300 %).
pub const ZOOM_STEPS: [f32; 8] = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

/// A tile's raster, for one document revision, scale and theme.
struct PageTexture {
    rev: u64,
    scale_bits: u32,
    dark: bool,
    texture: TextureHandle,
}

/// How the script is shown: the page, normal and draft views (decisions 0016 and 0041).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum View {
    /// Pages as they print: (MORE), (CONT'D), page numbers.
    #[default]
    Page,
    /// The script in one continuous column, no page edges.
    Normal,
    /// Continuous and plain: every element at the left margin, its name in the gutter.
    Draft,
}

/// What a view shows: its document and layout, and, for the page view, the map back to the
/// script (the other views show the script itself).
#[derive(Clone)]
pub struct Shown {
    /// The document shown, when it is not the paginated script's (the continuous views).
    own: Option<Arc<jcj_doc::Document>>,
    pub layout: Arc<jcj_layout::DocLayout>,
    pub pag: Option<Arc<Paginated>>,
}

impl Shown {
    /// The document this view draws.
    pub fn doc(&self) -> &jcj_doc::Document {
        match (&self.own, &self.pag) {
            (Some(d), _) => d,
            (None, Some(p)) => &p.doc,
            (None, None) => empty_document(),
        }
    }

    pub fn to_output(&self, pos: &jcj_doc::Pos) -> Option<jcj_doc::Pos> {
        match &self.pag {
            Some(p) => p.to_output(pos),
            None => Some(pos.clone()),
        }
    }
    pub fn to_source(&self, pos: &jcj_doc::Pos) -> Option<jcj_doc::Pos> {
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
    /// Look only in these elements (ids); empty for all.
    pub elements: Vec<String>,
    /// Matches found for the current query (shown in the bar).
    pub count: Option<usize>,
}

impl FindBar {
    /// The find and replace commands' parameters (elements are the paragraphs' styles).
    pub fn params(&self) -> Value {
        json!({"query": self.query, "with": self.with, "matchCase": self.match_case, "wholeWord": self.whole_word, "styles": self.elements})
    }

    /// The query, as the core's find takes it.
    pub fn query(&self) -> jcj_engine::commands::find::Query {
        jcj_engine::commands::find::Query {
            text: self.query.clone(),
            match_case: self.match_case,
            whole_word: self.whole_word,
            styles: self.elements.clone(),
        }
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
    /// The theme is dark (the shell's theme, kept here for the pages).
    pub dark: bool,
    /// The theme is the darker of the two dark ones.
    pub extra_dark: bool,
    /// Crash recovery's copies from an earlier run have been offered.
    pub(crate) offered_recovery: bool,
    /// Keep the caret's line at the middle of the window.
    pub typewriter: bool,
    /// Hide the ribbon, side panels and status bar.
    pub focus_mode: bool,
    /// The normal view's copy of the script, for one revision.
    normal_doc: Option<(u64, Arc<jcj_doc::Document>)>,
    /// The draft view's document and layout, for one revision.
    draft: Option<(u64, Shown)>,
    draft_cache: jcj_layout::LayoutCache,
    /// Points on screen per document point.
    pub zoom: f32,
    /// Apply macOS key conventions.
    pub mac: bool,
    /// The element settings window, while it shows.
    pub settings: Option<SettingsDialog>,
    /// Scroll the caret into view on the next frame (after a keyboard command).
    scroll_to_caret: bool,
    /// The last error a command reported, for the status bar.
    pub message: String,
    /// The message last handed to the shell's status bar.
    shown_message: String,
    /// Text a copy or cut asked to put on the clipboard.
    clipboard: Option<String>,
    /// The Paste command asked the window to paste the clipboard.
    paste_asked: bool,
    /// Pagination, kept between edits so only changed paragraphs are measured again.
    paginator: wordsmith_script_model::paginate::Paginator,
    /// The paginated script for the session's current revision.
    paginated: Option<(u64, Arc<Paginated>)>,
    /// What the find bar and the Find panel look for, while either has a query; its matches are
    /// marked on the pages.
    pub find: Option<FindBar>,
    /// The find bar shows.
    pub find_bar: bool,
    /// Escape was pressed this frame with nothing above the find bar to take it.
    pub escape_for_find_bar: bool,
    /// Underline misspelled words.
    pub spelling: bool,
    spell_cache: wordsmith_script_model::spelling::SpellCache,
    /// The misspelling a right-click opened the menu on, with its suggestions.
    spell_menu: Option<(wordsmith_script_model::spelling::Misspelling, Vec<String>)>,
    /// Autocomplete suggestions at the caret, if any are showing.
    pub suggestions: Option<Suggestions>,
    /// Where suggestions were hidden with Escape: (revision, caret). They stay hidden there.
    dismissed: Option<(u64, jcj_doc::Pos)>,
    /// The caret's rectangle on screen in the last frame, for placing the suggestion list.
    caret_screen: Option<Rect>,
    /// File dialogs asked for and not yet handed to the shell ([`Editor::flush`]).
    pub file_asks: Vec<FileAsk>,
    /// File dialogs the shell has, whose answers have not come back, each under the number its
    /// answer names.
    files_out: Vec<(u64, FileAsk)>,
    next_file_ask: u64,
    /// "Save changes first?", while it is asked, and what follows the answer.
    pub unsaved: Option<Then>,
    /// The shell shows the question in [`Editor::unsaved`].
    unsaved_asked: bool,
    /// "Is this a script?", while it is asked.
    pub import_question: Option<ImportQuestion>,
    /// What to do once the save under way completes (after "Save" in the unsaved prompt).
    after_save: Option<Then>,
    /// The writer chose not to save: the next open or import may replace the changes.
    discard_ok: bool,
    /// The writer agreed to close the window; the app closes it.
    pub close_confirmed: bool,
    /// The settings folder, when the desktop app has one: statistics, the personal dictionary,
    /// crash recovery and the default backups folder are kept there.
    pub settings_dir: Option<std::path::PathBuf>,
    pub backups: Backups,
    /// The title page fields being edited, while the title page window is open.
    pub title_page: Option<Value>,
    /// The revision sets window is open.
    pub show_revisions: bool,
    /// The header and footer settings being edited, while their window is open.
    pub header_footer: Option<Value>,
    /// The index cards or the scene view, when one shows instead of the script.
    pub structure: Option<crate::structure::Structure>,
    /// The Tables panel's table, while the panel is open.
    pub tables: Option<crate::tables::Table>,
    /// The note popover: the note's id (none for a new note) and its text being edited.
    pub note_popover: Option<(Option<u32>, String)>,
    /// The table cell being edited.
    pub table_edit: Option<crate::tables::CellEdit>,
    /// The Tables panel's sort, filter, hidden columns and open tab.
    pub table_view: crate::tables::View,
    /// The story board's selection, drags and beat window.
    pub board: crate::board::BoardState,
    /// The index cards' selection.
    pub cards_selection: jcj_board::Selection,
    /// The scene listing, for one revision.
    scene_cache: Option<(u64, Arc<Vec<Value>>)>,
    /// Tool windows added after v0.1 (Go To, bookmarks, checker, reformat…).
    pub tools: crate::tools::Tools,
    /// Parameters a window adds to the command its file dialog runs.
    pub dialog_params: Option<Value>,
    /// Draw the highlighted characters' speeches.
    pub show_highlights: bool,
    /// Open scripts as tabs: the one being edited is `session` and its slot here is empty.
    pub tabs: Vec<Option<Session>>,
    pub active_tab: usize,
    /// A number for each tab, in tab order, that stays the tab's own while others open and
    /// close.
    tab_ids: Vec<u64>,
    next_tab_id: u64,
    /// A second view beside or below the main one.
    pub split: Option<crate::workspace::Split>,
    /// Where the split view's divider sits.
    pub split_share: jcj_shell::split::SplitState,
    /// Writing statistics, the sprint timer and goals.
    pub stats: crate::workspace::Stats,
    /// The menu bar under the quick-access row (off by default).
    pub menu_bar: bool,
    /// The File page's print choices.
    pub print: crate::production::PrintChoices,
    /// What the side panels keep while text is typed in them.
    pub panels: crate::sidebar::PanelState,
    /// What the editor asks of the shell, passed on at the end of the frame.
    pub to_shell: Vec<ToShell>,
    /// A dialog, page or key tips of the shell has the keyboard this frame.
    shell_busy: bool,
    /// The frame [`Editor::frame_sync`] last ran in.
    synced_frame: Option<u64>,
    lengths_cache: Option<(u64, Arc<Vec<u32>>)>,
    panel_cache: Option<(u64, Instant, Arc<Vec<Value>>, Arc<Vec<u32>>)>,
    ribbon_cache: Option<((u64, jcj_doc::Pos), Arc<crate::ribbon::tabs::RibbonData>)>,
    /// The commands asked to run, with their parameters (for tests).
    #[cfg(test)]
    pub ran: Vec<(String, Value)>,
}

impl Editor {
    pub fn new(session: Session) -> Self {
        Editor {
            session,
            pages: HashMap::new(),
            view: View::Page,
            dark: false,
            extra_dark: false,
            offered_recovery: false,
            typewriter: false,
            focus_mode: false,
            normal_doc: None,
            draft: None,
            draft_cache: Default::default(),
            zoom: 1.0,
            mac: cfg!(target_os = "macos"),
            settings: None,
            scroll_to_caret: true,
            message: String::new(),
            shown_message: String::new(),
            clipboard: None,
            paste_asked: false,
            find: None,
            find_bar: false,
            escape_for_find_bar: false,
            spelling: true,
            spell_cache: Default::default(),
            spell_menu: None,
            suggestions: None,
            dismissed: None,
            caret_screen: None,
            paginator: Default::default(),
            paginated: None,
            file_asks: Vec::new(),
            files_out: Vec::new(),
            next_file_ask: 0,
            unsaved: None,
            unsaved_asked: false,
            import_question: None,
            after_save: None,
            discard_ok: false,
            close_confirmed: false,
            backups: Backups::default(),
            title_page: None,
            header_footer: None,
            show_revisions: false,
            structure: None,
            tables: None,
            table_edit: None,
            table_view: Default::default(),
            note_popover: None,
            scene_cache: None,
            board: Default::default(),
            cards_selection: Default::default(),
            tools: Default::default(),
            dialog_params: None,
            show_highlights: true,
            tabs: vec![None],
            active_tab: 0,
            tab_ids: vec![0],
            next_tab_id: 1,
            split: None,
            split_share: Default::default(),
            stats: Default::default(),
            settings_dir: None,
            menu_bar: false,
            print: Default::default(),
            panels: Default::default(),
            to_shell: Vec::new(),
            shell_busy: false,
            synced_frame: None,
            lengths_cache: None,
            panel_cache: None,
            ribbon_cache: None,
            #[cfg(test)]
            ran: Vec::new(),
        }
    }

    /// Run a command; failures go to the status bar, never panic. New, Open and Import over
    /// unsaved changes ask first ([`Editor::unsaved`]).
    pub fn run(&mut self, id: &str, params: Value) {
        #[cfg(test)]
        self.ran.push((id.to_string(), params.clone()));
        let then = match id {
            "file.new" => Some(Then::New),
            "file.open" => Some(Then::Open),
            "file.import" => Some(Then::Import),
            _ => None,
        };
        if let Some(then) = then
            && self.session.dirty
        {
            self.ask_unsaved(then);
            return;
        }
        self.run_now(id, params);
    }

    /// Ask "save changes first?" before `then`.
    fn ask_unsaved(&mut self, then: Then) {
        self.unsaved = Some(then);
        self.unsaved_asked = false;
    }

    /// The caret's rectangle on screen in the last frame drawn.
    pub fn caret_rect(&self) -> Option<Rect> {
        self.caret_screen
    }

    /// The scene list and lengths for the side panels: refreshed when the script changes, but no
    /// more than every 300 ms while typing goes on, so a long script types as fast with the
    /// Script panel open. Clicks go by the script as it is now, never by these.
    pub fn panel_scenes(&mut self) -> (Arc<Vec<Value>>, Arc<Vec<u32>>) {
        let rev = self.session.rev();
        let fresh = |at: &Instant| at.elapsed() < Duration::from_millis(300);
        if let Some((r, at, list, lens)) = &self.panel_cache
            && (*r == rev || fresh(at))
        {
            return (list.clone(), lens.clone());
        }
        let list = self.scene_list();
        let lens = self.scene_lengths();
        self.panel_cache = Some((rev, Instant::now(), list.clone(), lens.clone()));
        (list, lens)
    }

    /// Each scene's length in eighths, kept for one revision.
    pub fn scene_lengths(&mut self) -> Arc<Vec<u32>> {
        let rev = self.session.rev();
        if let Some((r, v)) = &self.lengths_cache
            && *r == rev
        {
            return v.clone();
        }
        let pag = self.paginated();
        let v = Arc::new(wordsmith_script_model::reports::scene_eighths(&self.session.doc, &pag));
        self.lengths_cache = Some((rev, v.clone()));
        v
    }

    /// What the ribbon shows that depends on the script, kept for one revision.
    pub fn ribbon_data(&mut self) -> Arc<crate::ribbon::tabs::RibbonData> {
        let key = (self.session.rev(), self.session.sel.focus.clone());
        if let Some((k, v)) = &self.ribbon_cache
            && *k == key
        {
            return v.clone();
        }
        let v = Arc::new(crate::ribbon::tabs::RibbonData::of(self));
        self.ribbon_cache = Some((key, v.clone()));
        v
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
        let path_before = self.session.path.clone();
        match self.session.run(id, &params) {
            Ok(_) => self.message.clear(),
            Err(e) => self.message = e.to_string(),
        }
        // A script opened or saved under a new name joins the recent list.
        if self.session.path != path_before
            && let Some(p) = self.session.path.as_ref().map(|p| p.display().to_string())
        {
            self.to_shell.push(ToShell::Recent(p));
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
        if self.close_confirmed {
            return true;
        }
        if !self.session.dirty {
            match self.tabs.iter().position(|t| t.as_ref().is_some_and(|s| s.dirty)) {
                None => return true,
                Some(i) => self.switch_tab(i),
            }
        }
        self.ask_unsaved(Then::Close);
        false
    }

    /// The writer's answer to "save changes first?".
    pub fn answer_unsaved(&mut self, answer: UnsavedAnswer) {
        let Some(then) = self.unsaved.take() else { return };
        self.unsaved_asked = false;
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
                if matches!(then, Then::Close | Then::CloseTab) {
                    // Closing: this script's changes are let go.
                    self.session.dirty = false;
                }
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
            Then::Close => {
                // Another tab with unsaved changes asks next.
                if let Some(i) = self.tabs.iter().position(|t| t.as_ref().is_some_and(|s| s.dirty)) {
                    self.switch_tab(i);
                    self.ask_unsaved(Then::Close);
                } else {
                    self.close_confirmed = true;
                }
            }
            Then::CloseTab => {
                self.close_tab(true);
            }
        }
    }

    /// Forget the drawn page images (they belong to one egui context; the offscreen renderer
    /// starts a new one for each picture).
    pub fn forget_drawn_pages(&mut self) {
        self.pages.clear();
    }

    /// Forget everything shown for the script before (after a tab switch).
    fn reset_caches(&mut self) {
        self.paginated = None;
        self.pages.clear();
        self.normal_doc = None;
        self.draft = None;
        self.scene_cache = None;
        self.suggestions = None;
        self.dismissed = None;
        self.table_edit = None;
        self.board = Default::default();
        self.cards_selection = Default::default();
        self.structure = None;
        self.scroll_to_caret = true;
        self.stats.forget_counts();
    }

    /// Show tab `i`'s script.
    pub fn switch_tab(&mut self, i: usize) {
        if i == self.active_tab || i >= self.tabs.len() {
            return;
        }
        let Some(mut other) = self.tabs.get_mut(i).and_then(Option::take) else { return };
        std::mem::swap(&mut self.session, &mut other);
        if let Some(slot) = self.tabs.get_mut(self.active_tab) {
            *slot = Some(other);
        }
        self.active_tab = i;
        self.reset_caches();
    }

    /// A new, empty script in a tab of its own.
    pub fn new_tab(&mut self) {
        let mut s = crate::new_session();
        s.author = self.session.author.clone();
        self.tabs.push(Some(s));
        self.tab_ids.push(self.next_tab_id);
        self.next_tab_id += 1;
        self.switch_tab(self.tabs.len() - 1);
    }

    /// Close the script's tab (the last one stays). With unsaved changes, ask first unless
    /// `force`. Returns whether it closed.
    pub fn close_tab(&mut self, force: bool) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        if self.session.dirty && !force {
            self.ask_unsaved(Then::CloseTab);
            return false;
        }
        self.session.discard_recovery_copy();
        self.tabs.remove(self.active_tab);
        if self.active_tab < self.tab_ids.len() {
            self.tab_ids.remove(self.active_tab);
        }
        let j = self.active_tab.min(self.tabs.len() - 1);
        if let Some(s) = self.tabs.get_mut(j).and_then(Option::take) {
            self.session = s;
        }
        self.active_tab = j;
        self.reset_caches();
        true
    }

    /// The number of the tab being edited: its own for as long as the tab is open.
    pub fn tab_id(&self) -> u64 {
        self.tab_ids.get(self.active_tab).copied().unwrap_or_default()
    }

    /// The files of the open scripts that have one, in tab order.
    pub fn open_paths(&self) -> Vec<String> {
        self.tabs
            .iter()
            .enumerate()
            .filter_map(|(i, t)| match t {
                Some(s) => s.path.as_ref(),
                None if i == self.active_tab => self.session.path.as_ref(),
                None => None,
            })
            .map(|p| p.display().to_string())
            .collect()
    }

    /// Each tab's name and whether it has unsaved changes.
    pub fn tab_names(&self) -> Vec<(String, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| match t {
                Some(s) => (script_name(s), s.dirty),
                None if i == self.active_tab => (script_name(&self.session), self.session.dirty),
                None => (String::new(), false),
            })
            .collect()
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

    /// A save finished: make the backup copy and carry on with whatever waited for the save.
    /// (The shell's auto-save clock restarts by itself once the script is no longer dirty.)
    fn on_saved(&mut self, path: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.backups.enabled
            && let Some(dir) = self.backups.dir.clone()
            && let Err(e) = wordsmith_script_format::files::backup(std::path::Path::new(path), &dir, self.backups.keep, std::time::SystemTime::now())
        {
            self.message = format!("Saved, but the backup copy failed: {e}");
        }
        #[cfg(target_arch = "wasm32")]
        let _ = path;
        if let Some(then) = self.after_save.take() {
            self.proceed(then);
        }
    }

    /// Save for auto-save: an error says why it failed.
    pub fn autosave_now(&mut self) -> Result<(), String> {
        self.run_now("file.save", json!({}));
        if self.session.dirty {
            let why = if self.message.is_empty() { "the script could not be saved".to_string() } else { std::mem::take(&mut self.message) };
            return Err(why);
        }
        Ok(())
    }

    /// Nothing that Escape closes or leaves is open above the find bar: no list, window, page or
    /// drop-down, and no other field has the keyboard.
    pub(crate) fn nothing_above_find_bar(&self, ctx: &egui::Context) -> bool {
        !self.shell_busy
            && self.settings.is_none()
            && self.suggestions.is_none()
            && self.note_popover.is_none()
            && self.board.edit.is_none()
            && self.structure.is_none()
            && !crate::tools::has_keyboard(self)
            && !egui::Popup::is_any_open(ctx)
            && ctx.memory(|m| m.focused()).is_none()
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
            if req.get("paste").and_then(Value::as_bool) == Some(true) {
                self.paste_asked = true;
            }
            if let Some(path) = req.get("saved").and_then(Value::as_str) {
                self.on_saved(path);
            }
            if let Some(notice) = req.get("notice").and_then(Value::as_str) {
                self.message = notice.to_string();
            }
            match req.get("open").and_then(Value::as_str) {
                Some("palette") => self.to_shell.push(ToShell::Palette),
                Some("elementSettings") => {
                    let current = Elements::of_doc(&self.session.doc).of_para_at(&self.session).to_string();
                    self.settings = Some(SettingsDialog::new(&mut self.session, &current));
                }
                Some("about") => self.to_shell.push(ToShell::FilePage(Page::About)),
                Some("recover") => self.to_shell.push(ToShell::Recovery),
                Some("exportPdf") => self.ask_export_path(),
                Some("open" | "import" | "saveAs" | "export" | "macrosFrom" | "compare") => {
                    if req["open"] == "compare" {
                        // The comparison opens in the window.
                        self.dialog_params = Some(json!({"open": true, "discard": self.discard_ok}));
                    }
                    self.ask_file(&req)
                }
                Some("importQuestion") => {
                    let path = req.get("path").and_then(Value::as_str).unwrap_or("").to_string();
                    let guess = req.get("guess").and_then(Value::as_bool).unwrap_or(true);
                    self.import_question = Some(ImportQuestion { path, guess });
                }
                Some("preferences") => self.to_shell.push(ToShell::FilePage(Page::Preferences)),
                Some("revisions") => self.show_revisions = true,
                Some("titlePage") => self.title_page = self.session.run("titlePage.get", &json!({})).ok(),
                Some("headerFooter") => self.header_footer = self.session.run("headerFooter.get", &json!({})).ok(),
                Some("view.page") => self.set_view(View::Page),
                Some("view.normal") => self.set_view(View::Normal),
                Some("view.speed") => self.set_view(View::Draft),
                Some("note") => {
                    // The note at the caret, or a new one for the selection.
                    let here = self.session.run("note.at", &json!({})).ok().filter(|v| !v.is_null());
                    self.note_popover = Some(match here {
                        Some(v) => (v["id"].as_u64().and_then(|i| u32::try_from(i).ok()), v["text"].as_str().unwrap_or("").to_string()),
                        None => (None, String::new()),
                    });
                }
                Some("view.tables") => {
                    self.tables = if self.tables.is_some() { None } else { Some(crate::tables::Table::Scenes) };
                    self.table_edit = None;
                }
                Some("view.cards") => self.structure = Some(crate::structure::Structure::Cards),
                Some("view.scenes") => self.structure = Some(crate::structure::Structure::Scenes),
                Some("view.board") => self.structure = Some(crate::structure::Structure::Board),
                Some("view.outline") => self.structure = Some(crate::structure::Structure::Outline),
                Some("view.dark") => self.to_shell.push(ToShell::Theme(if self.dark { Mode::Light } else { Mode::Dark })),
                Some("view.typewriter") => self.typewriter = !self.typewriter,
                Some("view.focus") => self.focus_mode = !self.focus_mode,
                Some("find") => {
                    let query = self.session.selected_text();
                    let bar = self.find.take().unwrap_or_default();
                    self.find = Some(FindBar { query: if query.is_empty() || query.contains('\n') { bar.query.clone() } else { query }, ..bar });
                    self.find_bar = true;
                }
                Some(other) => {
                    crate::tools::open(self, other, &req);
                }
                None => {}
            }
        }
    }

    /// Text waiting for the clipboard (for tests).
    pub fn pending_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    fn ask_export_path(&mut self) {
        self.push_file_ask(FileAsk::save("Export PDF", "Script.pdf".into(), "file.exportPdf").filter("PDF", &["pdf"]));
    }

    /// Queue a file dialog for the tab being edited. What its answer needs is taken now and
    /// travels with it, because the window goes on while the dialog is up: the writer's leave
    /// to replace unsaved changes, the options a window set for the command, and the step that
    /// waits on the save.
    fn push_file_ask(&mut self, mut ask: FileAsk) {
        ask.tab = self.tab_id();
        if ask.then == "file.openPath" {
            ask.params.insert("discard".into(), json!(std::mem::take(&mut self.discard_ok)));
        }
        // Options a window set for the command the dialog runs (a report's kind).
        if let Some(Value::Object(extra)) = self.dialog_params.take() {
            for (k, x) in extra {
                ask.params.entry(k).or_insert(x);
            }
        }
        if ask.then == "file.saveAsPath" {
            ask.after_save = self.after_save.take();
        }
        self.file_asks.push(ask);
    }

    /// Hand the file dialogs asked for to the shell, which shows one at a time and answers each
    /// with [`FILE_CHOSEN`] or [`FILE_CLOSED`], naming the ask by its number.
    pub(crate) fn show_file_asks(&mut self, shell: &mut jcj_shell::Shell) {
        for ask in std::mem::take(&mut self.file_asks) {
            let id = self.next_file_ask;
            self.next_file_ask += 1;
            let tag = json!({"ask": id});
            if !shell.ask_file_or(ask.dialog.clone(), jcj_shell::Cmd::with(FILE_CHOSEN, tag.clone()), jcj_shell::Cmd::with(FILE_CLOSED, tag)) {
                self.message = "Too many file dialogs are waiting. Close one and try again.".into();
                continue;
            }
            // An ask whose dialog could not be shown is never answered: it stays here, and what
            // it carries is never run. Only so many are kept.
            if self.files_out.len() >= MAX_FILES_OUT {
                self.files_out.remove(0);
            }
            self.files_out.push((id, ask));
        }
    }

    /// The shell's answer to a file dialog: `chosen` with the choice as `"path"` in `params`,
    /// or closed without one. An answer to an ask the editor does not hold does nothing.
    pub(crate) fn file_answered(&mut self, params: &Value, chosen: bool) {
        let id = params.get("ask").and_then(Value::as_u64);
        let Some(at) = self.files_out.iter().position(|(n, _)| Some(*n) == id) else { return };
        let (_, ask) = self.files_out.remove(at);
        let path = params.get("path").and_then(Value::as_str).filter(|_| chosen).map(PathBuf::from);
        self.answer_file(ask, path);
    }

    /// A file dialog for a file command; its answer runs the command that takes the path.
    fn ask_file(&mut self, req: &Value) {
        let name = req.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or("Untitled");
        let ask = match req.get("open").and_then(Value::as_str) {
            Some("open") => FileAsk::open("Open Script", "file.openPath")
                .filter("Scripts", &["wss", "wst", "fountain", "spmd"])
                .filter("Wordsmith Script", &["wss"])
                .filter("Templates", &["wst"])
                .filter("Fountain", &["fountain", "spmd"]),
            Some("import") => FileAsk::open("Import", "file.openPath")
                .filter("Text, RTF, PDF or Fountain", &["txt", "text", "rtf", "pdf", "fountain", "spmd"])
                .filter("Plain Text", &["txt", "text"])
                .filter("Rich Text (RTF)", &["rtf"])
                .filter("PDF", &["pdf"]),
            Some("macrosFrom") => FileAsk::open("Copy Macros from a Script", "macro.copyFromPath").filter("Scripts", &["wss", "fountain", "spmd"]),
            Some("compare") => {
                FileAsk::open("Compare with Another Version", "compare.withFile").filter("Scripts", &["wss", "wst", "fountain", "spmd"])
            }
            Some("saveAs") => FileAsk::save("Save Script", format!("{name}.wss"), "file.saveAsPath").filter("Wordsmith Script", &["wss"]),
            Some("export") => {
                let (label, ext, then) = match req.get("format").and_then(Value::as_str) {
                    Some("csv-scenes") => ("CSV", "csv", "tables.exportScenesCsv"),
                    Some("csv-characters") => ("CSV", "csv", "tables.exportCharactersCsv"),
                    Some("csv-report") => ("CSV", "csv", "report.exportCharacters"),
                    Some("pdf-revised") => ("PDF", "pdf", "file.exportRevisedPdf"),
                    Some("fountain") => ("Fountain", "fountain", "file.exportFountain"),
                    Some("rtf") => ("Rich Text (RTF)", "rtf", "file.exportRtf"),
                    Some("html") => ("Web Page (HTML)", "html", "file.exportHtml"),
                    Some("report-text") => ("Plain Text", "txt", "report.export"),
                    Some("report-csv") => ("CSV", "csv", "report.export"),
                    Some("pdf-filtered") => ("PDF", "pdf", "report.exportFilteredPdf"),
                    Some("pdf-board") => ("PDF", "pdf", "storyboard.exportPdf"),
                    Some("template") => ("Wordsmith Script Template", "wst", "file.saveAsTemplate"),
                    Some("pdf-cards") => ("PDF", "pdf", "cards.exportPdf"),
                    _ => ("Plain Text", "txt", "file.exportText"),
                };
                FileAsk::save(&format!("Export {label}"), format!("{name}.{ext}"), then).filter(label, &[ext])
            }
            _ => return,
        };
        self.push_file_ask(ask);
    }

    /// The answer to a file dialog the editor asked for (`None`: it was closed without a
    /// choice): runs the command it was asked for, with the chosen path, in the tab that asked.
    pub fn answer_file(&mut self, ask: FileAsk, path: Option<PathBuf>) {
        // Nothing waits on a dialog the writer closed: what the ask carried goes with it.
        let Some(path) = path else { return };
        let Some(tab) = self.tab_ids.iter().position(|id| *id == ask.tab) else {
            self.message = "The script that file dialog was for is closed, so nothing was done.".into();
            return;
        };
        self.switch_tab(tab);
        let FileAsk { then, mut params, after_save, .. } = ask;
        params.insert("path".into(), json!(path.display().to_string()));
        if after_save.is_some() {
            self.after_save = after_save;
        }
        self.run_now(then, Value::Object(params));
        if then == "file.saveAsPath" && self.session.dirty {
            // The save failed: nothing waits on it any more.
            self.after_save = None;
        }
    }

    /// Whether the editor should take keyboard input now (no dialog or text field has it).
    fn has_keyboard(&self, ui: &Ui) -> bool {
        !self.shell_busy && self.settings.is_none() && self.board.edit.is_none() && !crate::tools::has_keyboard(self) && !ui.ctx().text_edit_focused()
    }

    /// Turn this frame's keyboard events into commands, in the order they came. `user` holds the
    /// writer's own keys (shortcut → command id; the shell keeps them).
    // core-candidate: open-source-core#39. The shell's own key handling runs a frame's key commands after the
    // frame's text, whatever was typed first, so the editor takes every event itself
    // (`ShellOptions::shortcuts` is off) until the shell keeps the order.
    pub fn handle_keys(&mut self, ui: &Ui, user: &BTreeMap<String, String>) {
        let mut keys: Option<jcj_shell::keys::KeyMap> = None;
        if !self.has_keyboard(ui) {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
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
                    let Some(shortcut) = jcj_shell::keys::shortcut(key, modifiers, self.mac) else { continue };
                    let map = keys.get_or_insert_with(|| self.key_map(user));
                    // The window sends a paste of its own for the paste key; running Paste as
                    // well would paste twice.
                    if map.bound_to(&shortcut) == Some("edit.paste") {
                        continue;
                    }
                    if !self.suggestion_key(&shortcut) && !self.leave_focus(&shortcut) {
                        // The key's own command, or for a shifted caret key the movement that
                        // extends the selection (the shell's rule for every app).
                        if let Some(cmd) = map.command_for(&shortcut) {
                            self.press(cmd);
                        }
                    }
                }
                _ => {}
            }
        }
        // Copy and cut put text on the clipboard through the commands' requests.
        self.copy_to_clipboard(ui.ctx());
        // Tab and arrows are ours, not egui's focus navigation.
        ui.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { key: egui::Key::Tab, .. })));
    }

    /// Put text a copy or cut asked for on the clipboard.
    pub fn copy_to_clipboard(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.clipboard.take() {
            ctx.copy_text(text);
        }
    }

    /// After the Paste command, ask the window for the clipboard: its text comes back as a paste
    /// event, which the editor (or the text field being typed in) takes.
    pub fn ask_paste(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.paste_asked) {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
    }

    /// Every key the window binds: the writer's own (`user`), custom elements' and macros', the
    /// commands' and the shell's own.
    pub fn key_map(&self, user: &BTreeMap<String, String>) -> jcj_shell::keys::KeyMap {
        use jcj_shell::Commands;
        let mut all = self.all_commands();
        all.extend(jcj_shell::commands::shell_commands());
        let mut map = jcj_shell::keys::KeyMap::from_commands(all.iter());
        map.set_user(user);
        map
    }

    /// Run the command a key is bound to (from [`Editor::key_map`]).
    pub fn press(&mut self, cmd: jcj_shell::Cmd) {
        if cmd.is_shell() {
            self.to_shell.push(ToShell::Run(cmd));
        } else {
            self.run_cmd(&cmd);
        }
    }

    /// Note whether the shell has the keyboard this frame (a dialog, the File page, key tips).
    pub fn set_shell_busy(&mut self, busy: bool) {
        self.shell_busy = busy;
    }

    /// Whether this frame's shell work ([`Editor::flush`] and the per-frame state) is still to
    /// do: true once per egui pass.
    pub fn first_call_this_frame(&mut self, ctx: &egui::Context) -> bool {
        let n = ctx.cumulative_pass_nr();
        if self.synced_frame == Some(n) {
            return false;
        }
        self.synced_frame = Some(n);
        true
    }

    /// Whether "save changes first?" waits to be shown by the shell; marks it shown.
    pub fn take_unsaved_question(&mut self) -> Option<(String, &'static str, &'static str)> {
        let then = self.unsaved?;
        if self.unsaved_asked {
            return None;
        }
        self.unsaved_asked = true;
        Some((script_name(&self.session), then.doing(), then.key()))
    }

    /// The message to show in the status bar when it changed since it was last shown.
    pub fn take_new_message(&mut self) -> Option<String> {
        if self.message == self.shown_message {
            return None;
        }
        self.shown_message = self.message.clone();
        Some(self.message.clone())
    }

    /// The script view in `ui` (for a split pane).
    pub fn script_pane(&mut self, ui: &mut Ui) {
        egui::ScrollArea::both().id_salt("script-pane").auto_shrink(false).show(ui, |ui| self.page_view(ui));
    }

    /// Bring the caret into view on the next frame.
    pub fn request_scroll_to_caret(&mut self) {
        self.scroll_to_caret = true;
        self.structure = None;
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
        self.session.view.mode = match view {
            View::Page => jcj_layout::ViewMode::Print,
            View::Normal | View::Draft => jcj_layout::ViewMode::Web,
        };
        self.session.view.web_width = text_width(&self.session.doc);
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
                self.session.view.mode = jcj_layout::ViewMode::Web;
                self.session.view.web_width = text_width(&self.session.doc);
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
            View::Draft => {
                let rev = self.session.rev();
                if let Some((r, s)) = &self.draft
                    && *r == rev
                {
                    return s.clone();
                }
                let doc = draft_document(&self.session.doc);
                let opts = jcj_layout::LayoutOptions { view: jcj_layout::ViewMode::Web, web_width: text_width(&doc), ..Default::default() };
                let layout = Arc::new(jcj_layout::layout(&doc, &mut self.draft_cache, &opts));
                let s = Shown { own: Some(Arc::new(doc)), layout, pag: None };
                self.draft = Some((rev, s.clone()));
                s
            }
        }
    }

    /// A page's texture at `scale` pixels per document point, drawn again when the document or
    /// scale changed.
    fn texture(&mut self, ctx: &egui::Context, shown: &Shown, index: usize, tile: usize, page: &jcj_layout::Page, scale: f32) -> TextureHandle {
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
        let opts = jcj_output::RenderSettings { dark: self.dark, ..Default::default() };
        let th = tile_height(page.h);
        let top = tile as f32 * th;
        let area = jcj_layout::Rect { x: 0.0, y: top, w: page.w, h: (page.h - top).clamp(1.0, th) };
        let r = jcj_output::render_area(shown.doc(), &shown.layout, index, area, scale, &opts).unwrap_or_default();
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
        let desk = crate::theme::get(ui.ctx()).canvas;
        let paper = if self.dark { Color32::from_gray(40) } else { Color32::WHITE };
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
                    let tex = self.texture(ui.ctx(), &pag, i, tile, page, zoom * ppp);
                    painter.image(tex.id(), tile_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                } else {
                    painter.rect_filled(tile_rect, 0.0, paper);
                }
            }
        }
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        self.draw_gutter(ui, &painter, &pag, &origins, zoom);
        self.draw_character_highlights(&painter, &pag, &origins, zoom);
        self.draw_outline_bars(&painter, &pag, &origins, zoom);
        self.draw_find_matches(&painter, &pag, &origins, zoom);
        self.draw_notes(&painter, &pag, &origins, zoom);
        self.draw_revised_pages(&painter, &pag, &origins);

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
        // Right-click: the caret goes there (unless it is in the selection); on a misspelled word
        // the menu offers its suggestions, Ignore and Add to Dictionary.
        let mut right_click_at: Option<Value> = None;
        if response.secondary_clicked()
            && let Some(p) = response.interact_pointer_pos()
            && let Some(at) = hit(p)
            && let Ok(pos) = serde_json::from_value::<jcj_doc::Pos>(at.clone())
        {
            let (a, b) = self.session.sel.ordered();
            let inside = !self.session.sel.is_collapsed() && a.path == pos.path && a.path == b.path && (a.off..=b.off).contains(&pos.off);
            if !inside {
                right_click_at = Some(at);
            }
            let found = wordsmith_script_model::spelling::check(&self.session.doc, &mut self.spell_cache);
            self.spell_menu = found.into_iter().find(|m| m.block == pos.path.last() && (m.start..=m.end).contains(&pos.off)).map(|m| {
                let s = wordsmith_script_model::spelling::suggest(&m.word, 6);
                (m, s)
            });
        }
        if let Some(at) = right_click_at {
            let _ = self.session.run("caret.set", &json!({"pos": at}));
        }
        let mut menu_action: Option<(&'static str, Value)> = None;
        let element_ids: Vec<(String, String)> = Elements::of_doc(&self.session.doc).all().iter().map(|d| (d.id.clone(), d.name.clone())).collect();
        let spell = self.spell_menu.clone();
        response.context_menu(|ui| {
            // The element of the paragraph under the caret (SPEC: a right-click submenu).
            ui.menu_button("Element", |ui| {
                for (id, name) in &element_ids {
                    if ui.button(name.as_str()).clicked() {
                        menu_action = Some(("element.set", json!({"element": id})));
                        ui.close();
                    }
                }
            });
            if ui.button("Note…").clicked() {
                menu_action = Some(("view.note", json!({})));
                ui.close();
            }
            if let Some((m, suggestions)) = &spell {
                ui.separator();
                for s in suggestions {
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
                    menu_action = Some(("review.addToDictionary", json!({"word": m.word})));
                    ui.close();
                }
            }
        });
        if let Some((id, params)) = menu_action {
            self.spell_menu = None;
            self.run(id, params);
        }
        if let Some(p) = response.interact_pointer_pos().filter(|_| !response.secondary_clicked()) {
            let shift = ui.input(|i| i.modifiers.shift);
            if response.double_clicked() {
                if let Some(pos) = hit(p) {
                    self.run("caret.set", json!({"pos": pos}));
                    self.run("select.word", json!({}));
                }
            } else if response.drag_started() || response.clicked() {
                if let Some(pos) = hit(p) {
                    self.run("caret.set", json!({"pos": pos, "extend": shift}));
                }
            } else if response.dragged()
                && let Some(pos) = hit(p)
            {
                self.run("caret.set", json!({"pos": pos, "extend": true}));
            }
            self.scroll_to_caret = false;
        }
    }

    /// The draft view names each paragraph's element in the left gutter.
    fn draw_gutter(&self, ui: &Ui, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        if self.view != View::Draft {
            return;
        }
        let elements = Elements::of_doc(&self.session.doc);
        let font = egui::FontId::proportional(10.0 * zoom.max(0.8));
        let colour = if self.dark { Color32::from_gray(150) } else { Color32::from_gray(220) };
        for (i, b) in self.session.doc.body.iter().enumerate() {
            let Some(p) = b.as_para() else { continue };
            let at = jcj_doc::Pos { story: StoryRef::Body, path: jcj_doc::Path::top(i), off: 0 };
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
            let q = bar.query();
            let fill = Color32::from_rgba_unmultiplied(255, 210, 0, 90);
            for (a, b) in wordsmith_script_model::commands::find::find_all(&self.session.doc, &q).into_iter().take(5_000) {
                let (Some(a), Some(b)) = (pag.to_output(&a), pag.to_output(&b)) else { continue };
                for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &b, 0) {
                    if let Some(min) = to_screen(page, r.x, r.y) {
                        painter.rect_filled(Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)), 0.0, fill);
                    }
                }
            }
        }
    }

    /// Outline paragraphs shown in the script: a colored bar in the left margin per nesting
    /// level, so they read apart from the script's own text (SPEC-2 8).
    fn draw_outline_bars(&self, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        let elements = Elements::of_doc(&self.session.doc);
        let colors = [Color32::from_rgb(70, 110, 200), Color32::from_rgb(60, 160, 120), Color32::from_rgb(200, 140, 40)];
        let mut level = 0usize;
        for (block, b) in self.session.doc.body.iter().enumerate() {
            let Some(p) = b.as_para() else { continue };
            let kind = elements.of_para(p).kind;
            level = match kind {
                ElementKind::Outline1 => 0,
                ElementKind::Outline2 => 1,
                ElementKind::Outline3 => 2,
                ElementKind::Note | ElementKind::Summary => level,
                _ => continue,
            };
            let at = |off| jcj_doc::Pos { story: StoryRef::Body, path: jcj_doc::Path::top(block), off };
            let (Some(a), Some(z)) = (pag.to_output(&at(0)), pag.to_output(&at(p.len()))) else { continue };
            for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &z, 0) {
                let Some(page_rect) = origins.get(page) else { continue };
                for l in 0..=level {
                    let x = page_rect.min.x + (0.35 + 0.12 * l as f32) * 72.0 * zoom;
                    let y = page_rect.min.y + r.y * zoom;
                    painter.line_segment([pos2(x, y), pos2(x, y + r.h * zoom)], Stroke::new(3.0 * zoom.max(0.5), colors[l.min(2)]));
                }
            }
        }
    }

    /// Speeches of characters whose highlight is on, tinted in their color (on screen only).
    fn draw_character_highlights(&self, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        if !self.show_highlights {
            return;
        }
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        for (first, last, color) in wordsmith_script_model::cast::highlighted_speeches(&self.session.doc) {
            let Some(c) = wordsmith_script_model::revisions::parse_color(&color) else { continue };
            let fill = Color32::from_rgba_unmultiplied(c.0, c.1, c.2, 70);
            let len = self.session.doc.body.get(last).and_then(|b| b.as_para()).map_or(0, |p| p.len());
            let at = |block, off| jcj_doc::Pos { story: StoryRef::Body, path: jcj_doc::Path::top(block), off };
            let (Some(a), Some(b)) = (pag.to_output(&at(first, 0)), pag.to_output(&at(last, len))) else { continue };
            for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &b, 0) {
                if let Some(min) = to_screen(page, r.x, r.y) {
                    painter.rect_filled(Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)), 0.0, fill);
                }
            }
        }
    }

    /// Revised pages: their side margins tinted in the color of the newest revision set on them.
    fn draw_revised_pages(&self, painter: &egui::Painter, pag: &Shown, origins: &[Rect]) {
        let Some(paginated) = &pag.pag else { return };
        let rev = wordsmith_script_model::revisions::Revisions::of_doc(&self.session.doc);
        for (page, set) in paginated.page_revisions.iter().enumerate() {
            let (Some(set), Some(r)) = (set.and_then(|i| rev.set(i)), origins.get(page)) else { continue };
            let Some(c) = wordsmith_script_model::revisions::parse_color(&set.color) else { continue };
            // White pages have no tint to show; a thin grey edge says they are revised.
            let color = if (c.0, c.1, c.2) == (255, 255, 255) { Color32::from_gray(200) } else { Color32::from_rgb(c.0, c.1, c.2) };
            let band = (r.width() * 0.035).max(4.0);
            painter.rect_filled(Rect::from_min_size(r.min, vec2(band, r.height())), 0.0, color);
            painter.rect_filled(Rect::from_min_size(pos2(r.max.x - band, r.min.y), vec2(band, r.height())), 0.0, color);
        }
    }

    /// Noted text: a light tint, with a mark in the left margin of its first line.
    fn draw_notes(&self, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        let fill = Color32::from_rgba_unmultiplied(120, 180, 255, 60);
        let mark = Color32::from_rgb(70, 130, 220);
        let mut first: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for (block, b) in self.session.doc.body.iter().enumerate() {
            let Some(p) = b.as_para() else { continue };
            for (range, props) in p.run_ranges() {
                let Some(id) = props.note() else { continue };
                let at = |off| jcj_doc::Pos { story: StoryRef::Body, path: jcj_doc::Path::top(block), off };
                let (Some(a), Some(z)) = (pag.to_output(&at(range.start)), pag.to_output(&at(range.end))) else { continue };
                for (page, r) in pag.layout.selection_rects(pag.doc(), &a, &z, 0) {
                    let Some(min) = to_screen(page, r.x, r.y) else { continue };
                    let rect = Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom));
                    painter.rect_filled(rect, 0.0, fill);
                    if first.insert(id)
                        && let Some(page_rect) = origins.get(page)
                    {
                        let x = page_rect.min.x + 0.6 * 72.0 * zoom;
                        painter.circle_filled(egui::pos2(x, rect.center().y), 4.0 * zoom, mark);
                    }
                }
            }
        }
    }

    /// Misspelled words: a dotted red line under each, on the pages in view.
    fn draw_misspellings(&mut self, ui: &Ui, painter: &egui::Painter, pag: &Shown, origins: &[Rect], zoom: f32) {
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));
        if self.spelling {
            let found = wordsmith_script_model::spelling::check(&self.session.doc, &mut self.spell_cache);
            let red = Stroke::new(1.5, Color32::from_rgb(220, 40, 40));
            for m in &found {
                let at = |off| jcj_doc::Pos { story: StoryRef::Body, path: jcj_doc::Path::top(m.block), off };
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

    /// The suggestion list, just below the caret. Clicking an entry accepts it.
    pub fn suggestion_list(&mut self, ctx: &egui::Context) {
        let (Some(sug), Some(caret)) = (self.suggestions.clone(), self.caret_screen) else { return };
        let mut chosen = None;
        egui::Area::new(egui::Id::unique("autocomplete")).order(egui::Order::Foreground).fixed_pos(caret.left_bottom() + vec2(0.0, 4.0)).show(
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
fn empty_document() -> &'static jcj_doc::Document {
    static EMPTY: std::sync::LazyLock<jcj_doc::Document> = std::sync::LazyLock::new(jcj_doc::Document::new);
    &EMPTY
}

/// The text column's width, points: what the continuous views lay the script out at, so lines
/// break as on paper.
fn text_width(doc: &jcj_doc::Document) -> f32 {
    let s = &doc.last_section;
    (s.page_w - s.margin_left - s.margin_right).max(72.0)
}

/// The draft view's document: the script with every element at the left margin, left-aligned,
/// with no space before except before Scene Headings. Paragraphs, text and casing are the
/// script's own, so positions are the same.
pub fn draft_document(doc: &jcj_doc::Document) -> jcj_doc::Document {
    let mut out = doc.clone();
    let elements = Elements::of_doc(doc);
    let sheet = Arc::make_mut(&mut out.styles);
    for st in &mut sheet.styles {
        let heading = elements.get(&st.id).is_some_and(|d| d.kind == ElementKind::SceneHeading);
        st.para.indent_left = Some(0.0);
        st.para.indent_right = Some(0.0);
        st.para.indent_first = Some(0.0);
        st.para.align = Some(jcj_doc::props::Align::Left);
        st.para.space_before = Some(if heading { wordsmith_script_model::template::LINE } else { 0.0 });
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
