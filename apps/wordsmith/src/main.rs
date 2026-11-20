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

struct App {
    editor: Editor,
    /// Whether the recovery prompt has been offered for copies left by an earlier run.
    offered_recovery: bool,
    /// Preferences kept between runs, when there is a settings folder: the file, and the
    /// preferences as last read or shown.
    prefs: Option<(PrefsFile, Prefs)>,
    /// The personal dictionary's file.
    dictionary: Option<PathBuf>,
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
        if let Some(path) = open {
            editor.run("file.openPath", json!({"path": path}));
        }
        App { editor, offered_recovery: false, prefs, dictionary }
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
        if self.editor.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.save_prefs(Some(ctx), false);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.editor.window_title()));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.editor.ui(ui);
        self.editor.native_dialogs(Some(&*frame));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.editor.session.finish_recovery();
        self.save_prefs(None, true);
        if let Some(words) = &self.dictionary
            && let Err(e) = craft_proof::save_user_dictionary(words, &[])
        {
            eprintln!("wordsmith: the personal dictionary could not be saved: {e}");
        }
    }
}

fn main() -> ExitCode {
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
