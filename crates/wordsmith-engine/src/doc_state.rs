// Modified by the Wordsmith Script contributors.
use serde::{Deserialize, Serialize};
use wordsmith_doc::props::CharProps;
use wordsmith_doc::{Document, Pos};

/// A selection: anchor (where it started) and focus (where the caret is).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Selection {
    pub anchor: Pos,
    pub focus: Pos,
}

impl Selection {
    pub fn caret(p: Pos) -> Self {
        Selection { anchor: p.clone(), focus: p }
    }
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }
    /// (start, end) in document order.
    pub fn ordered(&self) -> (Pos, Pos) {
        if self.anchor <= self.focus { (self.anchor.clone(), self.focus.clone()) } else { (self.focus.clone(), self.anchor.clone()) }
    }
}

#[derive(Clone)]
pub(super) struct Undo {
    pub(super) label: String,
    pub(super) doc: Document,
    pub(super) sel: Selection,
    /// Formatting waiting at the caret (Bold with nothing selected), so Undo takes it back too.
    pub(super) pending: Option<CharProps>,
}

/// Most pending front-end requests a session keeps.
pub const MAX_UI_REQUESTS: usize = 64;

/// Maximum undo depth.
pub(super) const MAX_UNDO: usize = 500;

/// How many steps the Undo button's list shows (#701); `edit.undoTo` reaches no further.
pub const UNDO_LIST_MAX: usize = 100;
