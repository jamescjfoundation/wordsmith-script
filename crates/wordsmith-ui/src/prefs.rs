// SPDX-License-Identifier: MIT OR Apache-2.0
//! App preferences, kept between runs in `preferences.json` in the settings folder: the view
//! settings, auto-save and backups. The file is written through craft-atomic; a missing key
//! takes its default, an unreadable file is moved aside and the defaults are used, and every
//! value is bounded when it is applied, so a hand-edited or damaged file cannot ask for
//! something unreasonable. The personal dictionary is a file of its own (craft-proof's), beside
//! it. Decision 0020.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::editor::{Editor, View, ZOOM_RANGE};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewPrefs {
    /// "page", "normal" or "draft". "speed", the draft view's name before decision 0041, still
    /// reads as the draft view.
    pub view: String,
    pub dark: bool,
    pub typewriter: bool,
    pub spelling: bool,
    pub zoom: f32,
}

impl Default for ViewPrefs {
    fn default() -> Self {
        ViewPrefs { view: "page".into(), dark: false, typewriter: false, spelling: true, zoom: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoSavePrefs {
    pub enabled: bool,
    pub minutes: u32,
    pub idle_seconds: u32,
}

impl Default for AutoSavePrefs {
    fn default() -> Self {
        AutoSavePrefs { enabled: true, minutes: 15, idle_seconds: 10 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupPrefs {
    pub enabled: bool,
    /// Backups kept per script (1 to 10,000).
    pub keep: u32,
    /// The backups folder; empty for `Backups` in the settings folder.
    pub folder: String,
}

impl Default for BackupPrefs {
    fn default() -> Self {
        BackupPrefs { enabled: true, keep: 1_000, folder: String::new() }
    }
}

/// Most minutes between auto-saves, and most idle seconds waited.
pub const MAX_AUTOSAVE_MINUTES: u32 = 240;
pub const MAX_IDLE_SECONDS: u32 = 600;
/// Most backups kept per script (SPEC: up to 10,000).
pub const MAX_BACKUPS: u32 = 10_000;

/// The workspace: recent scripts, what opens at launch, the writer, keys (SPEC-2 2, 19).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspacePrefs {
    pub recent: Vec<String>,
    /// Open the scripts that were open when the app last closed.
    pub reopen_last: bool,
    /// The scripts open when the app last closed.
    pub last_open: Vec<String>,
    /// Show the start window when the app starts with no script to open.
    pub show_start: bool,
    /// The writer's name, for notes and tracked changes.
    pub writer: String,
    pub extra_dark: bool,
    /// The writer's own keys: shortcut → command id ("" turns a key off).
    pub keys: std::collections::BTreeMap<String, String>,
    /// The window's layout (v0.3): the side panels open and their widths, the ribbon shown in
    /// full or as tabs, the menu bar, and whether the first-run tour has been seen.
    pub left_panel: String,
    pub right_panel: String,
    pub left_width: f32,
    pub right_width: f32,
    pub ribbon_collapsed: bool,
    pub menu_bar: bool,
    pub tour_seen: bool,
    /// Check for a newer release when the app starts (decision 0038).
    pub check_updates: bool,
}

impl Default for WorkspacePrefs {
    fn default() -> Self {
        WorkspacePrefs {
            recent: Vec::new(),
            reopen_last: false,
            last_open: Vec::new(),
            show_start: true,
            writer: String::new(),
            extra_dark: false,
            keys: Default::default(),
            left_panel: "scenes".into(),
            right_panel: String::new(),
            left_width: 260.0,
            right_width: 290.0,
            ribbon_collapsed: false,
            menu_bar: false,
            tour_seen: false,
            check_updates: true,
        }
    }
}

/// Most keys of the writer's own kept.
pub const MAX_KEYS: usize = 500;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub view: ViewPrefs,
    pub autosave: AutoSavePrefs,
    pub backups: BackupPrefs,
    pub workspace: WorkspacePrefs,
}

impl Prefs {
    /// Put these preferences into effect in `ed`.
    pub fn apply(&self, ed: &mut Editor) {
        ed.set_view(match self.view.view.as_str() {
            "normal" => View::Normal,
            "draft" | "speed" => View::Draft,
            _ => View::Page,
        });
        ed.dark = self.view.dark;
        ed.typewriter = self.view.typewriter;
        ed.spelling = self.view.spelling;
        ed.zoom = if self.view.zoom.is_finite() { self.view.zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end()) } else { 1.0 };
        ed.autosave.enabled = self.autosave.enabled;
        ed.autosave.every = Duration::from_secs(u64::from(self.autosave.minutes.clamp(1, MAX_AUTOSAVE_MINUTES)) * 60);
        ed.autosave.idle = Duration::from_secs(u64::from(self.autosave.idle_seconds.clamp(1, MAX_IDLE_SECONDS)));
        ed.backups.enabled = self.backups.enabled;
        ed.backups.keep = self.backups.keep.clamp(1, MAX_BACKUPS) as usize;
        let w = &self.workspace;
        ed.recent = w.recent.iter().filter(|r| !r.is_empty() && r.len() < 4_096).take(crate::workspace::MAX_RECENT).cloned().collect();
        ed.extra_dark = w.extra_dark;
        ed.keys = w.keys.iter().take(MAX_KEYS).filter(|(k, v)| k.len() <= 40 && v.len() <= 80).map(|(k, v)| (k.clone(), v.clone())).collect();
        ed.tools.show_start_on_launch = w.show_start;
        ed.reopen_last = w.reopen_last;
        ed.updates.check_at_start = w.check_updates;
        ed.chrome.left = crate::ribbon::Panel::from_key(&w.left_panel);
        ed.chrome.right = crate::ribbon::Panel::from_key(&w.right_panel);
        ed.chrome.left_w = if w.left_width.is_finite() { w.left_width.clamp(200.0, 480.0) } else { 260.0 };
        ed.chrome.right_w = if w.right_width.is_finite() { w.right_width.clamp(220.0, 480.0) } else { 290.0 };
        ed.chrome.collapsed = w.ribbon_collapsed;
        ed.chrome.menu_bar = w.menu_bar;
        if !w.tour_seen {
            ed.chrome.tour = Some(0);
        }
        let writer: String = w.writer.chars().filter(|c| !c.is_control()).take(80).collect();
        if !writer.trim().is_empty() {
            ed.session.author = writer.trim().to_string();
        }
    }

    /// The preferences `ed` shows now; the backups folder stays as `self` has it.
    pub fn of(&self, ed: &Editor) -> Prefs {
        Prefs {
            view: ViewPrefs {
                view: match ed.view {
                    View::Normal => "normal",
                    View::Draft => "draft",
                    View::Page => "page",
                }
                .into(),
                dark: ed.dark,
                typewriter: ed.typewriter,
                spelling: ed.spelling,
                zoom: ed.zoom,
            },
            autosave: AutoSavePrefs {
                enabled: ed.autosave.enabled,
                minutes: (ed.autosave.every.as_secs() / 60).clamp(1, u64::from(MAX_AUTOSAVE_MINUTES)) as u32,
                idle_seconds: ed.autosave.idle.as_secs().clamp(1, u64::from(MAX_IDLE_SECONDS)) as u32,
            },
            backups: BackupPrefs {
                enabled: ed.backups.enabled,
                keep: ed.backups.keep.clamp(1, MAX_BACKUPS as usize) as u32,
                folder: self.backups.folder.clone(),
            },
            workspace: WorkspacePrefs {
                recent: ed.recent.clone(),
                reopen_last: ed.reopen_last,
                last_open: ed.open_paths(),
                show_start: ed.tools.show_start_on_launch,
                writer: ed.session.author.clone(),
                extra_dark: ed.extra_dark,
                keys: ed.keys.clone(),
                left_panel: ed.chrome.left.map_or(String::new(), |p| p.key().to_string()),
                right_panel: ed.chrome.right.map_or(String::new(), |p| p.key().to_string()),
                left_width: ed.chrome.left_w,
                right_width: ed.chrome.right_w,
                ribbon_collapsed: ed.chrome.collapsed,
                menu_bar: ed.chrome.menu_bar,
                tour_seen: ed.chrome.tour.is_none() && (self.workspace.tour_seen || ed.chrome.tour_done),
                check_updates: ed.updates.check_at_start,
            },
        }
    }
}

/// Largest preferences file read, bytes.
pub const MAX_PREFS_BYTES: u64 = 1 << 20;
/// Least time between two writes of the file while preferences change (a zoom drag).
pub const WRITE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// The preferences file and what was last written to it.
#[cfg(not(target_arch = "wasm32"))]
pub struct PrefsFile {
    pub path: std::path::PathBuf,
    written: Prefs,
    last_write: Option<std::time::Instant>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PrefsFile {
    /// The file name in the settings folder.
    pub const NAME: &'static str = "preferences.json";

    /// Read the preferences in `dir`. Never fails: a missing file gives the defaults; a damaged
    /// one is moved aside (`preferences.json.damaged`) and also gives the defaults, with a
    /// message to show.
    pub fn open(dir: &std::path::Path) -> (PrefsFile, Prefs, Option<String>) {
        let path = dir.join(Self::NAME);
        let read = || -> Result<Option<Prefs>, String> {
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
            };
            if meta.len() > MAX_PREFS_BYTES {
                return Err("the file is too large".into());
            }
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map(Some).map_err(|e| e.to_string())
        };
        let (prefs, problem) = match read() {
            Ok(p) => (p.unwrap_or_default(), None),
            Err(e) => {
                let _ = std::fs::rename(&path, dir.join(format!("{}.damaged", Self::NAME)));
                (Prefs::default(), Some(format!("Preferences could not be read ({e}); the defaults are used.")))
            }
        };
        (PrefsFile { path, written: prefs.clone(), last_write: None }, prefs, problem)
    }

    /// Write `prefs` if they changed since the last write and the last write was at least
    /// [`WRITE_INTERVAL`] ago (`force` writes now). Returns how long to wait before calling
    /// again when a change is waiting.
    pub fn save(&mut self, prefs: &Prefs, now: std::time::Instant, force: bool) -> Result<Option<std::time::Duration>, String> {
        if *prefs == self.written {
            return Ok(None);
        }
        if !force && let Some(last) = self.last_write {
            let next = last + WRITE_INTERVAL;
            if now < next {
                return Ok(Some(next - now));
            }
        }
        let text = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        self.last_write = Some(now);
        craft_atomic::atomic_write(&self.path, text.as_bytes()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.written = prefs.clone();
        Ok(None)
    }
}
