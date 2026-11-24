// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comparing two versions of a script (SPEC-2 11.6), on the core's comparison
//! (`jcj_features::compare`). Script groups by scene: the text before the first Scene Heading,
//! then each scene. Scenes are matched first (same heading, else the most similar text below the
//! heading), then paragraphs within matched scenes; scenes whose order changed are found as
//! moved. The result is the newer script marked up with tracked changes: added text as insertions
//! and removed paragraphs put back as deletions, by "Comparison", and a note on each moved
//! scene's heading saying where it was.

use std::ops::Range;

use jcj_doc::Document;
use jcj_features::compare::Grouping;
use serde_json::{Value, json};

use crate::scenes::scenes;

/// What changed between two versions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub scenes_added: usize,
    pub scenes_removed: usize,
    pub scenes_moved: usize,
    pub paragraphs_added: usize,
    pub paragraphs_removed: usize,
}

impl Summary {
    pub fn to_json(&self) -> Value {
        json!({
            "scenesAdded": self.scenes_added,
            "scenesRemoved": self.scenes_removed,
            "scenesMoved": self.scenes_moved,
            "paragraphsAdded": self.paragraphs_added,
            "paragraphsRemoved": self.paragraphs_removed,
        })
    }
}

/// The text before the first heading, if any, then each scene.
fn spans(doc: &Document) -> Vec<Range<usize>> {
    let all = scenes(doc);
    let mut out = Vec::new();
    let first = all.first().map_or(doc.body.len(), |s| s.start);
    if first > 0 {
        out.push(0..first);
    }
    out.extend(all.iter().map(|s| s.start..s.end));
    out
}

/// Script's grouping: by scene.
struct ByScene {
    /// The old version has text before its first scene (its group 0 is not a scene).
    old_prologue: bool,
}

impl Grouping for ByScene {
    fn groups(&self, doc: &Document) -> Vec<Range<usize>> {
        spans(doc)
    }

    fn moved_note(&self, old: usize) -> Option<String> {
        Some(format!("Moved: this was scene {} in the other version.", old + 1 - usize::from(self.old_prologue)))
    }
}

/// `new` marked up against `old`, and what changed.
pub fn compare(old: &Document, new: &Document) -> (Document, Summary) {
    let first = scenes(old).first().map_or(old.body.len(), |s| s.start);
    let (marked, sum) = jcj_features::compare::compare(old, new, &ByScene { old_prologue: first > 0 });
    let summary = Summary {
        scenes_added: sum.groups_added,
        scenes_removed: sum.groups_removed,
        scenes_moved: sum.groups_moved,
        paragraphs_added: sum.paragraphs_added,
        paragraphs_removed: sum.paragraphs_removed,
    };
    (marked, summary)
}

#[cfg(test)]
mod tests {
    use jcj_doc::para_block;

    use super::*;
    use crate::template::{new_script, paragraph};

    fn script(lines: &[(&str, &str)]) -> Document {
        let mut d = new_script();
        d.body = lines.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
        d
    }

    #[test]
    fn added_removed_changed_and_moved() {
        let old = script(&[
            ("SceneHeading", "INT. HALL - DAY"),
            ("Action", "Rain falls."),
            ("Action", "Anna waits."),
            ("SceneHeading", "EXT. YARD - NIGHT"),
            ("Action", "Wind."),
            ("SceneHeading", "INT. ATTIC - DAY"),
            ("Action", "Dust."),
        ]);
        let new = script(&[
            ("SceneHeading", "EXT. YARD - NIGHT"),
            ("Action", "Wind."),
            ("SceneHeading", "INT. HALL - DAY"),
            ("Action", "Rain falls."),
            ("Action", "Anna paces."),
            ("SceneHeading", "INT. CELLAR - DAY"),
            ("Action", "Damp."),
        ]);
        let (marked, sum) = compare(&old, &new);
        assert_eq!((sum.scenes_added, sum.scenes_removed, sum.scenes_moved), (1, 1, 1));
        assert_eq!((sum.paragraphs_added, sum.paragraphs_removed), (3, 3));
        let changes = crate::tracking::changes(&marked);
        let text = |c: &crate::tracking::Change| marked.body[c.path.last()].as_para().map(|p| p.text[c.start..c.end].to_string()).unwrap_or_default();
        assert!(changes.iter().any(|c| !c.inserted && text(c) == "Anna waits."));
        assert!(changes.iter().any(|c| c.inserted && text(c) == "Anna paces."));
        assert!(changes.iter().any(|c| !c.inserted && text(c) == "INT. ATTIC - DAY"));
        assert_eq!(crate::notes::Notes::of_doc(&marked).notes.len(), 1, "one moved scene noted");
        let (same, s2) = compare(&new, &new);
        assert_eq!(s2, Summary::default());
        assert!(crate::tracking::changes(&same).is_empty());
    }
}
