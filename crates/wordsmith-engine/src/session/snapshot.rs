// Modified by the Wordsmith Script contributors.
use super::doc_state::Selection;
use super::{Document, Session, StoryRef, next_document_id};

impl Session {
    /// Replace the document (open/new). The new document has its own id, an empty undo history
    /// and no unsaved changes.
    pub fn set_document(&mut self, doc: Document) {
        self.document_id = next_document_id();
        self.doc = doc;
        self.doc.ensure_nonempty();
        self.file_name = None;
        self.sel = Selection::caret(self.doc.start_of(StoryRef::Body));
        self.pending = None;
        // A copy the replaced document kept stays in the recovery list.
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.recovery_slot = Default::default();
        }
        self.reset_history();
        self.rev = self.rev.wrapping_add(1);
        self.dirty = false;
    }

    /// Make the selection valid for the current document.
    pub fn clamp_selection(&mut self) {
        self.sel.anchor = self.doc.clamp(&self.sel.anchor);
        self.sel.focus = self.doc.clamp(&self.sel.focus);
    }

    /// Why command `id` can't run now, for greying it out: its own `enabled` check. `None` when
    /// it can run (or doesn't exist).
    pub fn why_disabled(&self, id: &str) -> Option<&'static str> {
        let spec = self.registry.get(id)?;
        (spec.enabled)(self)
    }

    /// Put the revision back to `rev` after a command that changed nothing or failed. What the
    /// command cached at the revision it touched to is dropped: that number comes round again
    /// with the next edit, for a different document.
    pub(super) fn restore_rev(&mut self, rev: u64) {
        if self.rev == rev {
            return;
        }
        self.rev = rev;
        for slot in &mut self.layout {
            if slot.as_ref().is_some_and(|l| l.0 != rev) {
                *slot = None;
            }
        }
    }
}
