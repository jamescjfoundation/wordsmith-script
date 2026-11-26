// SPDX-License-Identifier: MIT OR Apache-2.0
//! The editor as an app on the shell (`jcj_shell::App`): the script's pages in the middle, Script's
//! ribbon tabs, side panels, status bar, File pages and preferences, and its commands. The shell
//! draws and runs the window around them; what the editor asks of the shell is passed on each
//! frame ([`Editor::flush`]).

use jcj_shell::commands::ids;
use jcj_shell::dialogs::RecoveryCopy;
use jcj_shell::theme::Mode;
use jcj_shell::workspace::PanelSpec;
use jcj_shell::{About, App, Cmd, Page, PageSpec, Shell, ShellOptions, TabSpec};
use serde_json::{Value, json};

use crate::commands::{custom_element, macro_slot};
use crate::editor::{Editor, ToShell, UnsavedAnswer, has_native_file, script_name};
use crate::ribbon::widgets::Rb;

/// The app's name.
pub const NAME: &str = "Wordsmith Script";
/// The app's machine name (the settings folder on Linux, release files).
pub const ID: &str = "wordsmith-script";
/// The credit the reuse rules ask for, on the About page.
pub const CREDIT: &str = "Based on WordCraft by the ArtCraft team.";

/// The shell's options for Script: the first-run tour, the parameters that do not tell ribbon
/// controls apart for key tips, and keys handled by the editor: it takes every key press in the
/// order typed (a shortcut, then text, as typed in one frame), as Script always has, and runs the
/// shell's own commands bound to keys through the shell.
pub fn shell_options() -> ShellOptions {
    ShellOptions {
        shortcuts: false,
        tour: crate::ribbon::TOUR.iter().map(|(h, b)| ((*h).to_string(), (*b).to_string())).collect(),
        keytip_ignored_params: ["on", "snap", "id", "ids", "board"].map(String::from).to_vec(),
        ..ShellOptions::default()
    }
}

/// The shell for a Script window: its name, version, design tokens, fonts and options. The
/// desktop app adds updates (`Shell::with_updates` with [`crate::RELEASES`]).
pub fn shell() -> Shell {
    Shell::new(NAME, wordsmith_script::VERSION, crate::theme::tokens()).with_fonts(crate::fonts::faces()).with_options(shell_options())
}

impl Editor {
    /// This frame's state from the shell, once a frame: the theme, the writer's name, focus mode,
    /// the scripts open, crash recovery.
    pub fn frame_sync(&mut self, ctx: &egui::Context, shell: &mut Shell) {
        if !self.first_call_this_frame(ctx) {
            return;
        }
        self.dark = shell.mode.is_dark();
        self.extra_dark = shell.mode == Mode::ExtraDark;
        self.mac = shell.mac;
        let name = shell.user_name.trim();
        if !name.is_empty() && self.session.author != name {
            self.session.author = name.to_string();
            for s in self.tabs.iter_mut().flatten() {
                s.author = name.to_string();
            }
        }
        // Focus mode: only the page.
        shell.options.quick_row = !self.focus_mode;
        shell.options.ribbon = !self.focus_mode;
        shell.options.status_bar = !self.focus_mode;
        shell.last_open = self.open_paths();
        // Crash recovery: copies an earlier run left are offered once they are read.
        self.session.recovery_tick();
        if let Some(wait) = self.session.recovery_wait() {
            ctx.request_repaint_after(wait);
        }
        if !self.offered_recovery && self.session.recovery_ready() {
            self.offered_recovery = true;
            let copies = self.recovery_copies();
            if !copies.is_empty() {
                shell.offer_recovery(copies, self.session.is_untouched_blank());
            }
        }
        if shell.dialogs.show_recovery || !shell.dialogs.recovery.is_empty() {
            shell.dialogs.recovery = self.recovery_copies();
            shell.dialogs.can_restore = self.session.is_untouched_blank();
        }
        if crate::workspace::sprint_left(self).is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        // A question answered here rather than in the shell's window (by a command) is gone.
        if self.unsaved.is_none() {
            shell.dialogs.unsaved = None;
        }
        let busy = shell.dialogs.palette.is_some()
            || shell.chrome.file_page.is_some()
            || shell.chrome.keytips
            || shell.dialogs.unsaved.is_some()
            || shell.dialogs.show_recovery;
        self.set_shell_busy(busy);
    }

    fn recovery_copies(&self) -> Vec<RecoveryCopy> {
        self.session.recovery_documents().into_iter().map(|d| RecoveryCopy { id: d.id, name: d.name, saved_at: d.saved_at }).collect()
    }

    /// Pass on what the editor asked of the shell: pages, the palette, the theme, panels,
    /// recent scripts, "save changes first?" and the status bar's message.
    pub fn flush(&mut self, shell: &mut Shell) {
        let panels = crate::sidebar::panels();
        for ask in std::mem::take(&mut self.to_shell) {
            match ask {
                ToShell::FilePage(page) => shell.open_file_page(page),
                ToShell::Palette => {
                    if shell.dialogs.palette.is_none() {
                        shell.dialogs.palette = Some(Default::default());
                    }
                }
                ToShell::CheckForUpdates => shell.updates.check_now(),
                ToShell::Theme(mode) => shell.mode = mode,
                ToShell::ShowPanel(id) => {
                    shell.layout.show(&panels, &id);
                }
                ToShell::ToggleSide(side) => {
                    shell.layout.toggle_side(&panels, side);
                }
                ToShell::Recovery => {
                    shell.offer_recovery(self.recovery_copies(), self.session.is_untouched_blank());
                    shell.dialogs.show_recovery = true;
                }
                ToShell::Recent(path) => shell.add_recent(&path),
                ToShell::Run(cmd) => shell.run(cmd),
            }
        }
        if let Some((name, doing, tag)) = self.take_unsaved_question() {
            shell.ask_unsaved(name, doing, tag);
        }
        // A question answered here rather than in the shell's window (by a command) is gone.
        if self.unsaved.is_none() {
            shell.dialogs.unsaved = None;
        }
        if let Some(message) = self.take_new_message() {
            shell.message = message;
        }
    }

    /// Run a command the shell's controls queued: the shell's answers ("save changes first?"),
    /// macros' and custom elements' own commands, Print with the File page's choices, and every
    /// other command by id.
    pub fn run_cmd(&mut self, cmd: &Cmd) {
        #[cfg(test)]
        self.ran.push((cmd.id.clone(), cmd.params.clone()));
        let id = cmd.id.as_str();
        let empty = cmd.params.as_object().is_none_or(serde_json::Map::is_empty);
        if id == ids::UNSAVED_ANSWER {
            let answer = cmd.params.get("answer").and_then(Value::as_str).and_then(UnsavedAnswer::from_key).unwrap_or(UnsavedAnswer::Cancel);
            self.answer_unsaved(answer);
        } else if let Some(slot) = macro_slot(id) {
            self.run("macro.run", json!({"slot": slot}));
        } else if let Some(element) = custom_element(id) {
            self.run("element.set", json!({"element": element}));
        } else if id == ids::PRINT && empty {
            self.run(id, self.print.params());
        } else {
            if id == "file.exportPdfAs" && !empty {
                // The PDF's file dialog runs the export with these choices (the print page's).
                self.dialog_params = Some(cmd.params.clone());
            }
            self.run(id, cmd.params.clone());
        }
    }
}

impl App for Editor {
    fn document_name(&self) -> String {
        script_name(&self.session)
    }

    fn document_path(&self) -> Option<String> {
        self.session.path.as_ref().map(|p| p.display().to_string())
    }

    fn is_dirty(&self) -> bool {
        self.session.dirty
    }

    fn has_file(&self) -> bool {
        has_native_file(&self.session)
    }

    fn revision(&self) -> u64 {
        self.session.rev()
    }

    fn run(&mut self, cmd: &Cmd, shell: &mut Shell) -> Result<(), String> {
        self.run_cmd(cmd);
        self.flush(shell);
        Ok(())
    }

    fn autosave(&mut self) -> Result<(), String> {
        self.autosave_now()
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, shell: &mut Shell) {
        self.frame_sync(ui.ctx(), shell);
        // Read before the keys are handled: Escape that closes a list or a window is theirs.
        self.escape_for_find_bar = ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.nothing_above_find_bar(ui.ctx());
        self.handle_keys(ui, shell.keys.user());
        if !self.focus_mode {
            if self.menu_bar {
                crate::chrome::menu_bar(self, ui, shell);
            }
            crate::chrome::find_bar(self, ui);
            crate::workspace::tab_bar(self, ui);
            crate::tables::panel(self, ui);
            crate::strip::panel(self, ui);
            // With neither the bar nor the Find panel showing, nothing marks matches.
            if !self.find_bar && !shell.layout.is_open("find") {
                self.find = None;
            }
        }
        crate::workspace::panes(self, ui);
        self.suggestion_list(ui.ctx());
        crate::dialogs::show(self, ui.ctx());
        crate::workspace::track(self);
        self.copy_to_clipboard(ui.ctx());
        self.ask_paste(ui.ctx());
        self.flush(shell);
    }

    fn ribbon_tabs(&self) -> Vec<TabSpec> {
        crate::ribbon::tabs_now(self)
    }

    fn ribbon_ui(&mut self, tab: &str, ui: &mut egui::Ui, shell: &mut Shell) {
        self.frame_sync(ui.ctx(), shell);
        let data = self.ribbon_data();
        let mut rb = Rb { shell, ed: self, data };
        crate::ribbon::tabs::draw(tab, &mut rb, ui);
    }

    fn panels(&self) -> Vec<PanelSpec> {
        if self.focus_mode { Vec::new() } else { crate::sidebar::panels() }
    }

    fn panel_ui(&mut self, panel: &str, ui: &mut egui::Ui, shell: &mut Shell) {
        self.frame_sync(ui.ctx(), shell);
        crate::sidebar::panel(self, ui, shell, panel);
        self.flush(shell);
    }

    fn status_ui(&mut self, ui: &mut egui::Ui, shell: &mut Shell) {
        self.frame_sync(ui.ctx(), shell);
        crate::chrome::status_bar(self, ui, shell);
        self.flush(shell);
    }

    fn file_pages(&self) -> Vec<PageSpec> {
        vec![PageSpec::new("compare", "Compare", "compare")]
    }

    fn file_page_ui(&mut self, page: &Page, ui: &mut egui::Ui, shell: &mut Shell) {
        self.frame_sync(ui.ctx(), shell);
        crate::backstage::page_ui(self, page, ui, shell);
        self.flush(shell);
    }

    fn preferences_ui(&mut self, ui: &mut egui::Ui, shell: &mut Shell) {
        crate::dialogs::prefs_body(self, ui, shell);
    }

    fn about(&self) -> About {
        About {
            summary: "Write screenplays. Licensed under MIT OR Apache-2.0.".into(),
            credit: CREDIT.into(),
            acknowledgments: crate::acknowledgments(),
            website: Some("https://github.com/jamescjfoundation/wordsmith-script".into()),
        }
    }

    fn mark_width(&self, height: f32) -> f32 {
        crate::brand::mark_width(height)
    }

    fn paint_mark(&self, painter: &egui::Painter, rect: egui::Rect, ink: egui::Color32) {
        crate::brand::paint_mark(painter, rect.min, rect.height(), ink);
    }
}
