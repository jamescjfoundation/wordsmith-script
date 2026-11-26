// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script desktop app: one window, a script in each tab, on the foundation's shell.
//!
//! `wordsmith-script [script]` opens the script (a `.wss` or Fountain file; plain text, RTF and
//! PDF are imported, asking whether text is a script). `wordsmith-script --version` prints the
//! version, the credit and the acknowledgments.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jcj_shell::{Page, Shell};
use serde_json::json;
use wordsmith_script_ui::Editor;
use wordsmith_script_ui::prefs::{Prefs, PrefsFile};

/// Script's settings folder: `WORDSMITH_SCRIPT_PROFILE_DIR` when it is set (an isolated profile), else
/// the platform's folder for the app (`~/.config/wordsmith-script`, `%APPDATA%\Wordsmith Script`,
/// `~/Library/Application Support/Wordsmith Script`).
fn settings_dir() -> Option<PathBuf> {
    settings_dir_from(std::env::var_os("WORDSMITH_SCRIPT_PROFILE_DIR"))
}

/// The settings folder, given the profile variable's value.
fn settings_dir_from(profile: Option<std::ffi::OsString>) -> Option<PathBuf> {
    if let Some(dir) = profile.filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    jcj_shell::prefs::settings_dir(wordsmith_script_ui::app::ID, wordsmith_script_ui::app::NAME)
}

struct Window {
    shell: Shell,
    /// The window's part of the shell's file dialogs.
    files: jcj_shell::files::FileDialogs,
    editor: Editor,
    /// Preferences kept between runs, when there is a settings folder: the file, and the
    /// preferences as last read or shown.
    prefs: Option<(PrefsFile, Prefs)>,
    /// The personal dictionary's file.
    dictionary: Option<PathBuf>,
    /// The settings folder (statistics are kept there).
    settings: Option<PathBuf>,
    /// When the statistics were last written.
    stats_saved: Instant,
    /// An installed update asked for a restart, and closing was asked for once.
    restart_asked: bool,
}

impl Window {
    fn new(cc: &eframe::CreationContext<'_>, open: Option<String>) -> Self {
        let mut editor = Editor::new(wordsmith_script_ui::new_session());
        let mut shell = wordsmith_script_ui::shell().with_updates(wordsmith_script_ui::RELEASES);
        let settings = settings_dir();
        let mut prefs = None;
        let mut dictionary = None;
        if let Some(dir) = &settings {
            let ctx = cc.egui_ctx.clone();
            let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
            if let Err(e) = editor.session.enable_recovery(dir.join("recovery"), wake) {
                editor.message = format!("Crash recovery is off: {e}");
            }
            let (file, p, problem) = PrefsFile::open(dir);
            if let Some(problem) = problem {
                editor.message = problem;
            }
            shell.apply_prefs(&p.shell, &wordsmith_script_ui::sidebar::panels());
            p.app.apply(&mut editor);
            let folder = p.app.backups.folder.trim();
            editor.backups.dir = Some(if folder.is_empty() { dir.join("Backups") } else { PathBuf::from(folder) });
            prefs = Some((file, p));
            let words = dir.join("dictionary.txt");
            jcj_proof::load_user_dictionary(&words);
            dictionary = Some(words);
            editor.stats.data = wordsmith_script_ui::workspace::file::load(dir);
        }
        if let Some(path) = open {
            editor.run("file.openPath", json!({"path": path}));
        } else if shell.reopen_last && !shell.last_open.is_empty() {
            // The scripts open at the last close, each in its tab.
            for (i, path) in shell.last_open.clone().iter().enumerate() {
                if i > 0 {
                    editor.new_tab();
                }
                editor.run("file.openPath", json!({"path": path}));
            }
            editor.switch_tab(0);
        } else if editor.tools.show_start_on_launch {
            // The File page's New page: recent scripts, templates, first steps.
            shell.open_file_page(Page::New);
        }
        Window {
            shell,
            files: jcj_shell::files::FileDialogs::new(),
            editor,
            prefs,
            dictionary,
            settings,
            stats_saved: Instant::now(),
            restart_asked: false,
        }
    }

    /// Write the statistics when they changed (at most once a minute, unless `force`).
    fn save_stats(&mut self, force: bool) {
        let Some(dir) = &self.settings else { return };
        if !self.editor.stats.dirty || (!force && self.stats_saved.elapsed() < Duration::from_secs(60)) {
            return;
        }
        match wordsmith_script_ui::workspace::file::save(dir, &self.editor.stats.data) {
            Ok(()) => {
                self.editor.stats.dirty = false;
                self.stats_saved = Instant::now();
            }
            Err(e) => self.shell.message = format!("Writing statistics could not be saved: {e}"),
        }
    }

    /// Keep the preferences file up to date with what the window shows.
    fn save_prefs(&mut self, ctx: Option<&egui::Context>, force: bool) {
        let Some((file, shown)) = &mut self.prefs else { return };
        *shown = Prefs { shell: self.shell.prefs(), app: shown.app.of(&self.editor) };
        match file.save(shown, Instant::now(), force) {
            Ok(Some(wait)) => {
                if let Some(ctx) = ctx {
                    ctx.request_repaint_after(wait);
                }
            }
            Ok(None) => {}
            Err(e) => self.shell.message = format!("Preferences could not be saved: {e}"),
        }
    }
}

impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.editor.may_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        // An installed update waits for a restart: close as the close button would, asking about
        // unsaved scripts first (once; Cancel keeps the window open).
        let restart = self.shell.close_requested() && !self.restart_asked;
        if restart {
            self.restart_asked = true;
        }
        if (restart && self.editor.may_close()) || self.editor.close_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.save_prefs(Some(ctx), false);
        self.save_stats(false);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.editor.window_title()));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.editor);
        // Show the next file dialog the editor asked the shell for, over this window, once the
        // frame is drawn. An answer runs its command in a later frame.
        self.files.show(&mut self.shell, ui.ctx(), Some(&*frame));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.editor.session.finish_recovery();
        self.save_prefs(None, true);
        self.save_stats(true);
        if let Some(words) = &self.dictionary
            && let Err(e) = jcj_proof::save_user_dictionary(words, &[])
        {
            eprintln!("wordsmith-script: the personal dictionary could not be saved: {e}");
        }
        // Last, once everything is saved: the helper starts the installed version after this exit.
        self.shell.finish();
    }
}

fn main() -> ExitCode {
    let first = std::env::args().nth(1);
    if first.as_deref() == Some("--version") {
        println!("{}", wordsmith_script_ui::about_text());
        return ExitCode::SUCCESS;
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(wordsmith_script_ui::app::NAME)
        .with_app_id(wordsmith_script_ui::app::APP_ID)
        .with_inner_size([1000.0, 900.0]);
    if let Some(icon) = wordsmith_script_ui::brand::icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    match eframe::run_native(wordsmith_script_ui::app::NAME, options, Box::new(move |cc| Ok(Box::new(Window::new(cc, first))))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wordsmith-script: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_come_from_our_repository_under_our_name() {
        let r = wordsmith_script_ui::RELEASES;
        assert_eq!(r.repo, "jamescjfoundation/wordsmith-script");
        assert_eq!(r.id, "wordsmith-script");
        assert_eq!(r.releases_url(), "https://github.com/jamescjfoundation/wordsmith-script/releases");
        assert_eq!(r.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn the_window_and_the_program_carry_scripts_own_names() {
        // The program is `wordsmith-script`, so `wordsmith` stays free for the launcher, and the
        // window's id is the one the menu entry and the macOS bundle carry (decision 0045).
        assert_eq!(env!("CARGO_PKG_NAME"), "wordsmith-script");
        assert_eq!(env!("CARGO_BIN_NAME"), "wordsmith-script");
        assert_eq!(wordsmith_script_ui::app::APP_ID, "org.jcjfoundation.script");
        assert_eq!(wordsmith_script_ui::app::ID, "wordsmith-script");
    }

    #[test]
    fn macos_is_sent_to_the_release_page() {
        assert_eq!(wordsmith_script_ui::RELEASES.installs_in_place, !cfg!(target_os = "macos"));
    }

    #[test]
    fn an_isolated_profile_is_used_whole() {
        assert_eq!(settings_dir_from(Some("profile-folder".into())), Some(PathBuf::from("profile-folder")));
        // Otherwise the platform's folder for the app, as the shell names it.
        assert_eq!(settings_dir_from(None), jcj_shell::prefs::settings_dir("wordsmith-script", "Wordsmith Script"));
        assert_eq!(settings_dir_from(Some("".into())), settings_dir_from(None));
    }
}
