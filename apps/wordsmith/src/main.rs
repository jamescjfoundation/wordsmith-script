// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script desktop app: one window per script.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;
use std::sync::Arc;

use wordsmith_ui::Editor;

struct App {
    editor: Editor,
    /// Whether the recovery prompt has been offered for copies left by an earlier run.
    offered_recovery: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        wordsmith_ui::fonts::install(&cc.egui_ctx);
        let mut editor = Editor::new(wordsmith_ui::new_session());
        if let Some(dir) = wordsmith_control_key::settings_dir() {
            let ctx = cc.egui_ctx.clone();
            let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
            if let Err(e) = editor.session.enable_recovery(dir.join("recovery"), wake) {
                editor.message = format!("Crash recovery is off: {e}");
            }
        }
        App { editor, offered_recovery: false }
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
        let title = if self.editor.session.dirty { "Wordsmith Script — Untitled •" } else { "Wordsmith Script — Untitled" };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.into()));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.editor.ui(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.editor.session.finish_recovery();
    }
}

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("{}", wordsmith_ui::about_text());
        return ExitCode::SUCCESS;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Wordsmith Script").with_inner_size([1000.0, 900.0]),
        ..Default::default()
    };
    match eframe::run_native("Wordsmith Script", options, Box::new(|cc| Ok(Box::new(App::new(cc))))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wordsmith: {e}");
            ExitCode::FAILURE
        }
    }
}
