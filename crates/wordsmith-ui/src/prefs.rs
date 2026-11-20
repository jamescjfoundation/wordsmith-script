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
    /// "page", "normal" or "speed".
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub view: ViewPrefs,
    pub autosave: AutoSavePrefs,
    pub backups: BackupPrefs,
}

impl Prefs {
    /// Put these preferences into effect in `ed`.
    pub fn apply(&self, ed: &mut Editor) {
        ed.set_view(match self.view.view.as_str() {
            "normal" => View::Normal,
            "speed" => View::Speed,
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
    }

    /// The preferences `ed` shows now; the backups folder stays as `self` has it.
    pub fn of(&self, ed: &Editor) -> Prefs {
        Prefs {
            view: ViewPrefs {
                view: match ed.view {
                    View::Normal => "normal",
                    View::Speed => "speed",
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
