// SPDX-License-Identifier: MIT OR Apache-2.0
//! Counts of the costly work done on this thread: paginations, paragraphs laid out anew for
//! them, and element registries read from a script's settings. A test types into a long script
//! and checks each keystroke does no more than it should, which no machine's speed can change
//! (docs/performance.md).

use std::cell::Cell;

/// Work done on this thread since it started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// Paginations run.
    pub paginations: u64,
    /// Paragraphs laid out for a pagination (not served by its cache).
    pub paragraphs_measured: u64,
    /// Element registries parsed from a script's settings.
    pub element_parses: u64,
}

impl Work {
    /// The work done since `before`.
    pub fn since(self, before: Work) -> Work {
        Work {
            paginations: self.paginations.saturating_sub(before.paginations),
            paragraphs_measured: self.paragraphs_measured.saturating_sub(before.paragraphs_measured),
            element_parses: self.element_parses.saturating_sub(before.element_parses),
        }
    }
}

thread_local! {
    static WORK: Cell<Work> = const { Cell::new(Work { paginations: 0, paragraphs_measured: 0, element_parses: 0 }) };
}

/// The work done on this thread so far.
pub fn so_far() -> Work {
    WORK.with(Cell::get)
}

/// Count some work.
pub(crate) fn count(f: impl FnOnce(&mut Work)) {
    WORK.with(|w| {
        let mut now = w.get();
        f(&mut now);
        w.set(now);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_add_up_on_this_thread_only() {
        let before = so_far();
        let doc = crate::template::new_script();
        let _ = crate::paginate::paginate(&doc);
        let w = so_far().since(before);
        assert_eq!(w.paginations, 1);
        assert!(w.paragraphs_measured >= 1);
        let other = std::thread::spawn(so_far).join().unwrap_or_default();
        assert_eq!(other, Work::default(), "another thread starts from nothing");
    }

    #[test]
    fn the_element_registry_is_read_once_until_it_changes() {
        let mut doc = crate::template::new_script();
        let first = crate::Elements::of_doc(&doc);
        let before = so_far();
        for _ in 0..10 {
            assert_eq!(crate::Elements::of_doc(&doc), first);
        }
        assert_eq!(so_far().since(before).element_parses, 0);
        // A changed registry is read again.
        let mut changed = first.clone();
        if let Some(d) = changed.get_mut("Action") {
            d.name = "Description".into();
        }
        changed.store(&mut doc);
        assert_eq!(crate::Elements::of_doc(&doc).get("Action").map(|d| d.name.clone()), Some("Description".into()));
        assert_eq!(so_far().since(before).element_parses, 1);
    }
}
