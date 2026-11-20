// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script desktop app: one window per script.
//!
//! `wordsmith [script]` opens the script (a `.wss` or Fountain file; plain text, RTF and PDF are
//! imported, asking whether text is a script).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use wordsmith_ui::Editor;
use wordsmith_ui::prefs::{Prefs, PrefsFile};

mod updates;

struct App {
    editor: Editor,
    /// Whether the recovery prompt has been offered for copies left by an earlier run.
    offered_recovery: bool,
    /// Preferences kept between runs, when there is a settings folder: the file, and the
    /// preferences as last read or shown.
    prefs: Option<(PrefsFile, Prefs)>,
    /// The personal dictionary's file.
    dictionary: Option<PathBuf>,
    /// The settings folder (statistics are kept there).
    settings: Option<PathBuf>,
    /// When the statistics were last written.
    stats_saved: Instant,
    /// Update checks and the installed update waiting for the exit (decision 0038).
    updater: updates::Updater,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, open: Option<String>) -> Self {
        wordsmith_ui::fonts::install(&cc.egui_ctx);
        let mut editor = Editor::new(wordsmith_ui::new_session());
        let ctx = cc.egui_ctx.clone();
        editor.dialogs = craft_dialogs::Dialogs::new(Arc::new(move || ctx.request_repaint()));
        let mut prefs = None;
        let mut dictionary = None;
        if let Some(dir) = wordsmith_control_key::settings_dir() {
            let ctx = cc.egui_ctx.clone();
            let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
            if let Err(e) = editor.session.enable_recovery(dir.join("recovery"), wake) {
                editor.message = format!("Crash recovery is off: {e}");
            }
            let (file, p, problem) = PrefsFile::open(&dir);
            if let Some(problem) = problem {
                editor.message = problem;
            }
            p.apply(&mut editor);
            let folder = p.backups.folder.trim();
            editor.backups.dir = Some(if folder.is_empty() { dir.join("Backups") } else { PathBuf::from(folder) });
            prefs = Some((file, p));
            let words = dir.join("dictionary.txt");
            craft_proof::load_user_dictionary(&words);
            dictionary = Some(words);
        }
        let settings = wordsmith_control_key::settings_dir();
        if let Some(dir) = &settings {
            editor.stats.data = wordsmith_ui::workspace::file::load(dir);
        }
        let last_open = prefs.as_ref().map(|(_, p)| p.workspace.last_open.clone()).unwrap_or_default();
        if let Some(path) = open {
            editor.run("file.openPath", json!({"path": path}));
        } else if editor.reopen_last && !last_open.is_empty() {
            // The scripts open at the last close, each in its tab.
            for (i, path) in last_open.iter().enumerate() {
                if i > 0 {
                    editor.new_tab();
                }
                editor.run("file.openPath", json!({"path": path}));
            }
            editor.switch_tab(0);
        } else if editor.tools.show_start_on_launch {
            // The File page's New page: recent scripts, templates, first steps.
            editor.chrome.backstage = Some(wordsmith_ui::backstage::Page::New);
        }
        let updater = updates::Updater::new();
        updater.read_outcome(&mut editor.updates);
        App { editor, offered_recovery: false, prefs, dictionary, settings, stats_saved: Instant::now(), updater }
    }

    /// Write the statistics when they changed (at most once a minute, unless `force`).
    fn save_stats(&mut self, force: bool) {
        let Some(dir) = &self.settings else { return };
        if !self.editor.stats.dirty || (!force && self.stats_saved.elapsed() < std::time::Duration::from_secs(60)) {
            return;
        }
        match wordsmith_ui::workspace::file::save(dir, &self.editor.stats.data) {
            Ok(()) => {
                self.editor.stats.dirty = false;
                self.stats_saved = Instant::now();
            }
            Err(e) => self.editor.message = format!("Writing statistics could not be saved: {e}"),
        }
    }

    /// Keep the preferences file up to date with what the editor shows.
    fn save_prefs(&mut self, ctx: Option<&egui::Context>, force: bool) {
        let Some((file, shown)) = &mut self.prefs else { return };
        *shown = shown.of(&self.editor);
        match file.save(shown, Instant::now(), force) {
            Ok(Some(wait)) => {
                if let Some(ctx) = ctx {
                    ctx.request_repaint_after(wait);
                }
            }
            Ok(None) => {}
            Err(e) => self.editor.message = format!("Preferences could not be saved: {e}"),
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let s = &mut self.editor.session;
        s.recovery_tick();
        if !self.offered_recovery && s.recovery_ready() {
            self.offered_recovery = true;
            if !s.recovery_documents().is_empty() {
                self.editor.recovery = Some(wordsmith_ui::dialogs::RecoveryPrompt);
            }
        }
        if let Some(wait) = self.editor.session.recovery_wait() {
            ctx.request_repaint_after(wait);
        }
        if let Some(wait) = self.editor.tick(Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        // Closing with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.editor.may_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.updater.tick(&mut self.editor.updates, ctx) {
            // Restart Now: close as the close button would, asking about unsaved scripts first.
            if self.editor.may_close() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.editor.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.save_prefs(Some(ctx), false);
        self.save_stats(false);
        if wordsmith_ui::workspace::sprint_left(&self.editor).is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.editor.window_title()));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.editor.ui(ui);
        self.editor.native_dialogs(Some(&*frame));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.editor.session.finish_recovery();
        self.save_prefs(None, true);
        self.save_stats(true);
        if let Some(words) = &self.dictionary
            && let Err(e) = craft_proof::save_user_dictionary(words, &[])
        {
            eprintln!("wordsmith: the personal dictionary could not be saved: {e}");
        }
        // Last, once everything is saved: the helper starts the installed version after this exit.
        self.updater.finish();
    }
}

fn main() -> ExitCode {
    // Windows: the update helper is a copy of this program started with `--finish-update`.
    #[cfg(windows)]
    if let Some(result) = craft_update::self_update::finish(&updates::APP, &std::env::args_os().skip(1).collect::<Vec<_>>()) {
        return if result.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }
    let first = std::env::args().nth(1);
    if first.as_deref() == Some("--version") {
        println!("{}", wordsmith_ui::about_text());
        return ExitCode::SUCCESS;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Wordsmith Script").with_inner_size([1000.0, 900.0]),
        ..Default::default()
    };
    match eframe::run_native("Wordsmith Script", options, Box::new(move |cc| Ok(Box::new(App::new(cc, first))))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wordsmith: {e}");
            ExitCode::FAILURE
        }
    }
}
