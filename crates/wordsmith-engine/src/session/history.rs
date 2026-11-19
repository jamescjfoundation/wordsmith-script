// Modified by the Wordsmith Script contributors.
use super::doc_state::{MAX_UNDO, Selection, UNDO_LIST_MAX, Undo};
use super::{Arc, Document, Session};

impl Session {
    /// Snapshot for undo before a change. Returns whether it took a step: typing that is
    /// already in an open step is not a new one.
    pub fn checkpoint(&mut self, label: &str) -> bool {
        if label == "Typing" && self.typing_open {
            return false;
        }
        self.typing_open = label == "Typing";
        self.history.push(Arc::new(Undo { label: label.to_string(), doc: self.doc.clone(), sel: self.sel.clone(), pending: self.pending.clone() }));
        if self.history.len() > MAX_UNDO {
            self.history.remove(0);
            self.undo_evicted += 1;
        }
        self.redo.clear();
        // The newest step is what a joined command merges into.
        self.join_anchor = true;
        true
    }

    /// Record `doc`/`sel` as their own undo step after the fact and close the typing group, so
    /// Undo right after an automatic change (AutoFormat, AutoCorrect) reverts only that change.
    pub fn push_undo(&mut self, label: &str, doc: Document, sel: Selection) {
        self.history.push(Arc::new(Undo { label: label.to_string(), doc, sel, pending: self.pending.clone() }));
        if self.history.len() > MAX_UNDO {
            self.history.remove(0);
            self.undo_evicted += 1;
        }
        self.typing_open = false;
        self.join_anchor = true;
    }

    /// Make the next mutating command part of the previous undo step instead of a new one, so a
    /// drag that runs a command every frame is a single Undo. Call it on every frame of the drag
    /// except the first; it is consumed by the next `run`.
    pub fn join_next_undo(&mut self) {
        self.join_next = true;
    }

    /// Whether typing is still collecting into one undo step (no other command since).
    pub fn typing_open(&self) -> bool {
        self.typing_open
    }

    /// Reopen the typing group when the last undo step is typing, so the keys typed next join
    /// it: after an input method's rewrite was joined to it (`join_next_undo` closes the group).
    pub fn continue_typing(&mut self) {
        if self.undo_label() == Some("Typing") {
            self.typing_open = true;
        }
    }

    /// Close an open typing group (caret moved, other command).
    pub fn close_typing(&mut self) {
        self.typing_open = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.history.last().map(|u| u.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|u| u.label.as_str())
    }

    /// Undo steps pushed so far, evicted ones included: only grows while steps are added.
    pub fn undo_steps_so_far(&self) -> u64 {
        self.history.len() as u64 + self.undo_evicted
    }

    /// The number of undo steps.
    pub fn undo_depth(&self) -> usize {
        self.history.len()
    }

    /// Where the undo history stands, for [`Session::merge_undo_since`].
    pub fn undo_mark(&self) -> u64 {
        (self.history.len() as u64).saturating_add(self.undo_evicted)
    }

    /// Makes the steps recorded since `mark` one undo step, the first: a drag that applied its
    /// value as it went (a slider in a pane) undoes in one go.
    pub fn merge_undo_since(&mut self, mark: u64) {
        let keep = mark.saturating_sub(self.undo_evicted).saturating_add(1);
        self.history.truncate(usize::try_from(keep).unwrap_or(usize::MAX));
    }

    pub fn undo_labels(&self) -> Vec<String> {
        self.history.iter().rev().map(|u| u.label.clone()).collect()
    }

    /// The Undo button's list (#701): the undo steps' labels (English, as commands name them),
    /// newest first, at most [`UNDO_LIST_MAX`].
    pub fn undo_history(&self) -> Vec<&str> {
        self.history.iter().rev().take(UNDO_LIST_MAX).map(|u| u.label.as_str()).collect()
    }

    /// Undo the newest `count` steps in one go, as picking an entry in the Undo list does (#701).
    /// More than the undo depth undoes everything. Returns the steps undone.
    pub fn undo_steps(&mut self, count: usize) -> usize {
        let mut done = 0;
        while done < count && self.undo() {
            done += 1;
        }
        done
    }

    /// Undoes the newest step: closes typing, swaps the document, selection and pending
    /// formatting with the redo snapshot and touches layout.
    pub fn undo(&mut self) -> bool {
        self.typing_open = false;
        let Some(u) = self.history.pop().map(Arc::unwrap_or_clone) else { return false };
        let cur = Undo {
            label: u.label.clone(),
            doc: std::mem::replace(&mut self.doc, u.doc),
            sel: std::mem::replace(&mut self.sel, u.sel),
            pending: std::mem::replace(&mut self.pending, u.pending),
        };
        self.redo.push(Arc::new(cur));
        self.touch();
        true
    }

    /// Redoes the newest undone step by swapping the snapshots back and touching layout.
    pub fn redo(&mut self) -> bool {
        self.typing_open = false;
        let Some(u) = self.redo.pop().map(Arc::unwrap_or_clone) else { return false };
        let cur = Undo {
            label: u.label.clone(),
            doc: std::mem::replace(&mut self.doc, u.doc),
            sel: std::mem::replace(&mut self.sel, u.sel),
            pending: std::mem::replace(&mut self.pending, u.pending),
        };
        self.history.push(Arc::new(cur));
        self.touch();
        true
    }

    /// Drop history (after open / new).
    pub fn reset_history(&mut self) {
        self.history.clear();
        self.redo.clear();
        self.typing_open = false;
        self.join_anchor = false;
    }
}
