// SPDX-License-Identifier: MIT OR Apache-2.0
//! Notes (decision 0023) are the core's (`jcj_features::notes`): anchors are run marks
//! (`core.note`), the notes themselves document data (`core.notes`, `core.note_types`).

pub use jcj_features::notes::*;

use jcj_doc::{Document, Pos};

use crate::keys::ScriptRun;

/// The note the character after `pos` is anchored to.
pub fn note_at_char(doc: &Document, pos: &Pos) -> Option<u32> {
    let p = doc.para_at(pos)?;
    let n = p.text.get(pos.off..).and_then(|t| t.chars().next()).map(char::len_utf8)?;
    p.props_at(pos.off + n).note()
}
