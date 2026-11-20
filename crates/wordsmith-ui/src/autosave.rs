// SPDX-License-Identifier: MIT OR Apache-2.0
//! When to auto-save (SPEC: on by default, every 15 minutes, after about 10 seconds idle).
//!
//! The clock starts at the first change after a save. A save is due once that change is at least
//! the interval old and nothing has been typed for the idle time, so an auto-save never lands in
//! the middle of a sentence. Only a script with a file of its own is auto-saved; an untitled
//! script is kept by crash recovery until the writer saves it. Times are passed in, so tests run
//! on a made-up clock.

use std::time::{Duration, Instant};

/// Auto-save settings and state.
#[derive(Clone, Debug)]
pub struct AutoSave {
    pub enabled: bool,
    pub every: Duration,
    pub idle: Duration,
    /// The first change since the last save.
    dirty_since: Option<Instant>,
    /// The last change.
    last_edit: Option<Instant>,
}

impl Default for AutoSave {
    fn default() -> Self {
        AutoSave { enabled: true, every: Duration::from_secs(15 * 60), idle: Duration::from_secs(10), dirty_since: None, last_edit: None }
    }
}

impl AutoSave {
    /// The script changed at `now`.
    pub fn edited(&mut self, now: Instant) {
        self.dirty_since.get_or_insert(now);
        self.last_edit = Some(now);
    }

    /// The script was saved (by hand or automatically): the clock starts again at the next change.
    pub fn saved(&mut self) {
        self.dirty_since = None;
    }

    /// When a save becomes due, if one is waiting.
    pub fn due_at(&self, dirty: bool, has_file: bool) -> Option<Instant> {
        if !self.enabled || !dirty || !has_file {
            return None;
        }
        let since = self.dirty_since?;
        let last = self.last_edit.unwrap_or(since);
        Some((since + self.every).max(last + self.idle))
    }

    /// Whether to save now.
    pub fn due(&self, now: Instant, dirty: bool, has_file: bool) -> bool {
        self.due_at(dirty, has_file).is_some_and(|at| now >= at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);
    const SEC: Duration = Duration::from_secs(1);

    #[test]
    fn saves_after_the_interval_once_typing_pauses() {
        let t0 = Instant::now();
        let mut a = AutoSave::default();
        assert!(!a.due(t0 + 60 * MIN, true, true), "nothing changed yet");
        a.edited(t0);
        assert!(!a.due(t0 + 14 * MIN, true, true));
        // Still typing at 15 minutes: wait for 10 seconds of quiet.
        a.edited(t0 + 15 * MIN - SEC);
        assert!(!a.due(t0 + 15 * MIN, true, true));
        assert!(!a.due(t0 + 15 * MIN + 8 * SEC, true, true));
        assert!(a.due(t0 + 15 * MIN + 9 * SEC, true, true));
        assert_eq!(a.due_at(true, true), Some(t0 + 15 * MIN + 9 * SEC));
        a.saved();
        assert!(!a.due(t0 + 30 * MIN, true, true), "the clock restarts at the next change");
        a.edited(t0 + 31 * MIN);
        assert!(a.due(t0 + 46 * MIN, true, true));
    }

    #[test]
    fn never_without_a_file_changes_or_when_switched_off() {
        let t0 = Instant::now();
        let mut a = AutoSave::default();
        a.edited(t0);
        let later = t0 + 60 * MIN;
        assert!(!a.due(later, true, false), "an untitled script is kept by crash recovery");
        assert!(!a.due(later, false, true), "nothing unsaved");
        a.enabled = false;
        assert!(!a.due(later, true, true));
    }
}
