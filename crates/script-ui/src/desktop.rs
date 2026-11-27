// SPDX-License-Identifier: MIT OR Apache-2.0
//! What only Script does at start-up, while it runs and at the end. The start-up itself is the
//! core's (`jcj_shell::desktop`, core decision 0014): the window with its title, app id and icon,
//! the settings folder, the preferences file, the update check, the close that starts an
//! installed update and its helper, the file dialogs and the steps of every frame. `apps/script`
//! gives it Script's data and builds the editor with [`build`]; the steps here are the ones it
//! calls back.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use jcj_shell::desktop::{DesktopApp, Start};
use jcj_shell::{Page, Shell};
use serde_json::json;

use crate::editor::Editor;
use crate::prefs::{Prefs, ScriptPrefs};

/// The personal dictionary's file in the settings folder.
pub const DICTIONARY: &str = "dictionary.txt";

/// Build the editor once the window exists: Script's own shell (its fonts and options), crash
/// recovery in `recovery` in the settings folder, the personal dictionary and the writer's
/// statistics, both kept beside the preferences.
pub fn build(start: &mut Start<'_>) -> Editor {
    start.shell = crate::shell();
    let mut editor = Editor::new(crate::new_session());
    if let Some(dir) = start.settings {
        let ctx = start.ctx.clone();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || ctx.request_repaint());
        if let Err(e) = editor.session.enable_recovery(dir.join("recovery"), wake) {
            editor.message = format!("Crash recovery is off: {e}");
        }
        jcj_proof::load_user_dictionary(&dir.join(DICTIONARY));
        let (stats, problem) = jcj_shell::writing::stats::Statistics::open(dir, crate::workspace::fresh_stats());
        editor.stats.data = stats;
        if let Some(problem) = problem {
            editor.message = problem;
        }
        editor.settings_dir = Some(dir.to_path_buf());
    }
    editor
}

impl Editor {
    /// Write the statistics into the settings folder when they changed (at most once a minute,
    /// unless `force`); a failure is said in the status bar.
    fn save_stats_in(&mut self, dir: &Path, force: bool, shell: &mut Shell) {
        if let Err(e) = crate::workspace::save_stats(self, dir, force) {
            shell.message = format!("Writing statistics could not be saved: {e}");
        }
    }
}

impl DesktopApp for Editor {
    type Prefs = Prefs;

    /// Script's preferences, and the backups folder they name (empty: `Backups` in the settings
    /// folder).
    fn apply_prefs(&mut self, prefs: &ScriptPrefs, _shell: &mut Shell) {
        prefs.apply(self);
        if let Some(dir) = &self.settings_dir {
            let folder = prefs.backups.folder.trim();
            self.backups.dir = Some(if folder.is_empty() { dir.join("Backups") } else { PathBuf::from(folder) });
        }
    }

    fn prefs(&self, last: &ScriptPrefs) -> ScriptPrefs {
        last.of(self)
    }

    /// The script named on the command line; else the scripts open at the last close, each in its
    /// tab, when the writer asked for that; else the File page's New page (recent scripts,
    /// templates, first steps), unless turned off.
    fn open(&mut self, files: &[PathBuf], shell: &mut Shell) {
        if let Some(path) = files.first() {
            // Commands take a path as text, so a name that is not Unicode is not found and the
            // window says so; it never stops the program.
            self.run("file.openPath", json!({"path": path.to_string_lossy()}));
        } else if shell.reopen_last && !shell.last_open.is_empty() {
            for (i, path) in shell.last_open.clone().iter().enumerate() {
                if i > 0 {
                    self.new_tab();
                }
                self.run("file.openPath", json!({"path": path}));
            }
            self.switch_tab(0);
        } else if self.tools.show_start_on_launch {
            shell.open_file_page(Page::New);
        }
    }

    fn report(&mut self, problem: String, _shell: &mut Shell) {
        self.message = problem;
    }

    /// Unsaved scripts: the editor asks "save changes first?" and the window stays open.
    fn may_close(&mut self, _shell: &mut Shell) -> bool {
        Editor::may_close(self)
    }

    fn close_confirmed(&self) -> bool {
        self.close_confirmed
    }

    /// The statistics are written while they change, at most once a minute.
    fn tick(&mut self, _ctx: &egui::Context, shell: &mut Shell) {
        if let Some(dir) = self.settings_dir.clone() {
            self.save_stats_in(&dir, false, shell);
        }
    }

    /// The script's name, a dot for unsaved changes, and the app's name.
    fn window_title(&self) -> Option<String> {
        Some(Editor::window_title(self))
    }

    /// Crash recovery's last copies are put away, and the statistics and the personal dictionary
    /// written. The core writes the preferences after this, and starts an installed update last.
    fn exit(&mut self, shell: &mut Shell) {
        self.session.finish_recovery();
        if let Some(dir) = self.settings_dir.clone() {
            self.save_stats_in(&dir, true, shell);
            if let Err(e) = jcj_proof::save_user_dictionary(&dir.join(DICTIONARY), &[]) {
                eprintln!("{}: the personal dictionary could not be saved: {e}", crate::app::ID);
            }
        }
    }
}
