// SPDX-License-Identifier: MIT OR Apache-2.0
//! Page locking (decision 0026) is the core's (`jcj_features::locks`): lock marks (`core.page_lock`),
//! labels (12, 12A), levels and placeholders, and `LockPass`, which Script's pagination runs in.

pub use jcj_features::locks::*;

use jcj_doc::{Document, Pos, StoryRef};

use crate::keys::ScriptRun;

/// The lock ids marked between `a` and `b` (body only).
pub fn marks_in(doc: &Document, a: &Pos, b: &Pos) -> Vec<u32> {
    if a.story != StoryRef::Body || a == b {
        return Vec::new();
    }
    let mut out = Vec::new();
    for block in a.path.last()..=b.path.last() {
        let Some(p) = doc.body.get(block).and_then(|x| x.as_para()) else { continue };
        let from = if block == a.path.last() { a.off } else { 0 };
        let to = if block == b.path.last() { b.off } else { p.len() };
        for (range, c) in p.run_ranges() {
            if let Some(id) = c.page_lock()
                && range.start >= from
                && range.start < to
            {
                out.push(id);
            }
        }
    }
    out
}
