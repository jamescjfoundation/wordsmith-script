// Modified by the Wordsmith Script contributors.
use super::{Pos, Session, StoryRef};

impl Session {
    /// The document's revision: changes with every edit, and keys the cached layout.
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Which document this is, unique in the process (see [`Session::set_document`]).
    pub fn document_id(&self) -> u64 {
        self.document_id
    }

    pub fn touch(&mut self) {
        self.rev = self.rev.wrapping_add(1);
        self.dirty = true;
    }

    /// The story the caret is in.
    pub fn story(&self) -> StoryRef {
        self.sel.focus.story
    }

    /// The caret position.
    pub fn caret(&self) -> &Pos {
        &self.sel.focus
    }

    /// Selected plain text.
    pub fn selected_text(&self) -> String {
        let (a, b) = self.sel.ordered();
        self.doc.copy_range(&a, &b).plain_text()
    }
}
