// SPDX-License-Identifier: MIT OR Apache-2.0
//! Preferences, kept between runs in `preferences.json` in the settings folder (decision 0020).
//!
//! The file is the shell's (`jcj_shell::prefs`): the shell's own preferences (theme, auto-save,
//! recent scripts, keys, the panels' layout, the tour, updates) under `"shell"`, and Script's
//! (the view, backups, the File page at start, the menu bar) under `"app"`. A file written by
//! Script 0.1 (`view`, `autosave`, `backups`, `workspace`) is read into the new shape, so nothing
//! is lost on the first start of 0.2. Values are bounded when they are applied. The personal
//! dictionary is a file of its own beside it.

use std::collections::BTreeMap;

use jcj_shell::prefs::{AutoSavePrefs, ShellPrefs};
use jcj_shell::theme::Mode;
use jcj_shell::workspace::{Layout, Side};
use serde::{Deserialize, Serialize};

use crate::editor::{Editor, View, ZOOM_RANGE};

/// The view as it was left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewPrefs {
    /// "page", "normal" or "draft". "speed", the draft view's name before decision 0041, still
    /// reads as the draft view.
    pub view: String,
    pub typewriter: bool,
    pub spelling: bool,
    pub zoom: f32,
}

impl Default for ViewPrefs {
    fn default() -> Self {
        ViewPrefs { view: "page".into(), typewriter: false, spelling: true, zoom: 1.0 }
    }
}

/// Backup copies made on every save.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupPrefs {
    pub enabled: bool,
    /// Backups kept per script (1 to [`MAX_BACKUPS`]).
    pub keep: u32,
    /// The backups folder; empty for `Backups` in the settings folder.
    pub folder: String,
}

impl Default for BackupPrefs {
    fn default() -> Self {
        BackupPrefs { enabled: true, keep: 1_000, folder: String::new() }
    }
}

/// Most backups kept per script (SPEC: up to 10,000).
pub const MAX_BACKUPS: u32 = 10_000;

/// Script's own preferences (the shell keeps the rest).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScriptPrefs {
    pub view: ViewPrefs,
    pub backups: BackupPrefs,
    /// Show the File page's New page when the app starts with no script to open.
    pub show_start: bool,
    /// The menu bar under the quick-access row.
    pub menu_bar: bool,
}

impl Default for ScriptPrefs {
    fn default() -> Self {
        ScriptPrefs { view: ViewPrefs::default(), backups: BackupPrefs::default(), show_start: true, menu_bar: false }
    }
}

impl ScriptPrefs {
    /// Put these preferences into effect in `ed`.
    pub fn apply(&self, ed: &mut Editor) {
        ed.set_view(match self.view.view.as_str() {
            "normal" => View::Normal,
            "draft" | "speed" => View::Draft,
            _ => View::Page,
        });
        ed.typewriter = self.view.typewriter;
        ed.spelling = self.view.spelling;
        ed.zoom = if self.view.zoom.is_finite() { self.view.zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end()) } else { 1.0 };
        ed.backups.enabled = self.backups.enabled;
        ed.backups.keep = self.backups.keep.clamp(1, MAX_BACKUPS) as usize;
        ed.tools.show_start_on_launch = self.show_start;
        ed.menu_bar = self.menu_bar;
    }

    /// The preferences `ed` shows now; the backups folder stays as `self` has it.
    pub fn of(&self, ed: &Editor) -> ScriptPrefs {
        ScriptPrefs {
            view: ViewPrefs {
                view: match ed.view {
                    View::Normal => "normal",
                    View::Draft => "draft",
                    View::Page => "page",
                }
                .into(),
                typewriter: ed.typewriter,
                spelling: ed.spelling,
                zoom: ed.zoom,
            },
            backups: BackupPrefs {
                enabled: ed.backups.enabled,
                keep: ed.backups.keep.clamp(1, MAX_BACKUPS as usize) as u32,
                folder: self.backups.folder.clone(),
            },
            show_start: ed.tools.show_start_on_launch,
            menu_bar: ed.menu_bar,
        }
    }
}

/// The preferences file's contents: the shell's and Script's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "Read")]
pub struct Prefs {
    pub shell: ShellPrefs,
    pub app: ScriptPrefs,
}

impl Default for Prefs {
    /// A first start: the Scenes panel open, as in 0.1.
    fn default() -> Self {
        let shell = ShellPrefs { layout: Layout::with_open(&crate::sidebar::panels(), &["scenes"]), ..ShellPrefs::default() };
        Prefs { shell, app: ScriptPrefs::default() }
    }
}

/// The preferences file: the shell's, for [`Prefs`].
pub type PrefsFile = jcj_shell::prefs::PrefsFile<Prefs>;

/// A preferences file as read: the current shape, or 0.1's.
#[derive(Deserialize)]
#[serde(default)]
#[derive(Default)]
struct Read {
    shell: Option<ShellPrefs>,
    app: Option<ScriptPrefs>,
    view: Option<OldView>,
    autosave: Option<AutoSavePrefs>,
    backups: Option<BackupPrefs>,
    workspace: Option<OldWorkspace>,
}

/// 0.1's view preferences.
#[derive(Default, Deserialize)]
#[serde(default)]
struct OldView {
    view: Option<String>,
    dark: bool,
    typewriter: bool,
    spelling: Option<bool>,
    zoom: Option<f32>,
}

/// 0.1's workspace preferences.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct OldWorkspace {
    recent: Vec<String>,
    reopen_last: bool,
    last_open: Vec<String>,
    show_start: Option<bool>,
    writer: String,
    extra_dark: bool,
    keys: BTreeMap<String, String>,
    left_panel: Option<String>,
    right_panel: String,
    left_width: Option<f32>,
    right_width: Option<f32>,
    ribbon_collapsed: bool,
    menu_bar: bool,
    tour_seen: bool,
    check_updates: Option<bool>,
}

impl From<Read> for Prefs {
    fn from(r: Read) -> Prefs {
        let old = r.view.is_some() || r.workspace.is_some() || r.autosave.is_some() || r.backups.is_some();
        if r.shell.is_some() || r.app.is_some() || !old {
            let d = Prefs::default();
            return Prefs { shell: r.shell.unwrap_or(d.shell), app: r.app.unwrap_or(d.app) };
        }
        // A file from 0.1.
        let v = r.view.unwrap_or_default();
        let w = r.workspace.unwrap_or_default();
        let theme = match (v.dark, w.extra_dark) {
            (true, true) => Mode::ExtraDark,
            (true, false) => Mode::Dark,
            _ => Mode::Light,
        };
        let panels = crate::sidebar::panels();
        let left = w.left_panel.unwrap_or_else(|| "scenes".into());
        let open: Vec<&str> = [left.as_str(), w.right_panel.as_str()].into_iter().filter(|p| !p.is_empty()).collect();
        let mut layout = Layout::with_open(&panels, &open);
        layout.set_size(Side::Left, w.left_width.unwrap_or(260.0));
        layout.set_size(Side::Right, w.right_width.unwrap_or(290.0));
        let shell = ShellPrefs {
            theme,
            autosave: r.autosave.unwrap_or_default(),
            recent: w.recent,
            reopen_last: w.reopen_last,
            last_open: w.last_open,
            user_name: w.writer,
            keys: w.keys,
            layout,
            ribbon_collapsed: w.ribbon_collapsed,
            tour_seen: w.tour_seen,
            check_updates: w.check_updates.unwrap_or(true),
        };
        let app = ScriptPrefs {
            view: ViewPrefs {
                view: v.view.unwrap_or_else(|| "page".into()),
                typewriter: v.typewriter,
                spelling: v.spelling.unwrap_or(true),
                zoom: v.zoom.unwrap_or(1.0),
            },
            backups: r.backups.unwrap_or_default(),
            show_start: w.show_start.unwrap_or(true),
            menu_bar: w.menu_bar,
        };
        Prefs { shell, app }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_from_0_1_reads_into_the_new_shape() {
        let old = r#"{
            "view": {"view": "speed", "dark": true, "typewriter": true, "spelling": false, "zoom": 1.5},
            "autosave": {"enabled": false, "minutes": 5, "idle_seconds": 20},
            "backups": {"enabled": true, "keep": 12, "folder": "/backups"},
            "workspace": {"recent": ["/s/a.wss"], "writer": "Ann", "extraDark": true, "keys": {"Mod+K": "edit.find"},
                          "leftPanel": "notes", "rightPanel": "scene", "leftWidth": 300, "ribbonCollapsed": true,
                          "menuBar": true, "tourSeen": true, "checkUpdates": false, "showStart": false}
        }"#;
        let p: Prefs = serde_json::from_str(old).unwrap();
        assert_eq!(p.shell.theme, Mode::ExtraDark);
        assert_eq!((p.shell.autosave.enabled, p.shell.autosave.minutes, p.shell.autosave.idle_seconds), (false, 5, 20));
        assert_eq!(p.shell.recent, ["/s/a.wss"]);
        assert_eq!(p.shell.user_name, "Ann");
        assert_eq!(p.shell.keys.get("Mod+K").map(String::as_str), Some("edit.find"));
        assert_eq!((p.shell.layout.open_on(Side::Left), p.shell.layout.open_on(Side::Right)), (Some("notes"), Some("scene")));
        assert_eq!(p.shell.layout.size.left, 300.0);
        assert!(p.shell.ribbon_collapsed && p.shell.tour_seen && !p.shell.check_updates);
        assert_eq!(p.app.view, ViewPrefs { view: "speed".into(), typewriter: true, spelling: false, zoom: 1.5 });
        assert_eq!((p.app.backups.keep, p.app.backups.folder.as_str()), (12, "/backups"));
        assert!(!p.app.show_start && p.app.menu_bar);
        // Written back, it is the new shape, and reads as itself.
        let text = serde_json::to_string(&p).unwrap();
        assert!(text.contains("\"shell\"") && text.contains("\"app\"") && !text.contains("\"workspace\""));
        assert_eq!(serde_json::from_str::<Prefs>(&text).unwrap(), p);
    }

    #[test]
    fn a_first_start_opens_the_scenes_and_an_empty_file_takes_the_defaults() {
        assert_eq!(Prefs::default().shell.layout.open_on(Side::Left), Some("scenes"));
        assert_eq!(serde_json::from_str::<Prefs>("{}").unwrap(), Prefs::default());
    }
}
