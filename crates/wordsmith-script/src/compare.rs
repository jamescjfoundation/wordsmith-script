// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comparing two versions of a script (SPEC-2 11.6). Scenes are matched first (same heading,
//! else the most similar text), then paragraphs within matched scenes; scenes whose order
//! changed are found as moved. The result is the newer script marked up with tracked changes:
//! added text as insertions and removed paragraphs put back as deletions, by "Comparison", and a
//! note on each moved scene's heading saying where it was.

use std::collections::HashSet;

use serde_json::{Value, json};
use wordsmith_doc::{Document, Paragraph, Revision, RevisionKind, para_block};

use crate::element::Elements;
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

/// A scene's blocks as (start, end); the text before the first heading is a scene of its own.
fn spans(doc: &Document) -> Vec<(usize, usize)> {
    let all = scenes(doc);
    let mut out = Vec::new();
    let first = all.first().map_or(doc.body.len(), |s| s.start);
    if first > 0 {
        out.push((0, first));
    }
    out.extend(all.iter().map(|s| (s.start, s.end)));
    out
}

/// A paragraph as compared: its element and its words.
fn key(elements: &Elements, p: &Paragraph) -> String {
    format!("{}|{}", elements.of_para(p).id, p.text.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn words(doc: &Document, span: (usize, usize)) -> HashSet<String> {
    (span.0..span.1)
        .filter_map(|i| doc.body.get(i).and_then(|b| b.as_para()))
        .flat_map(|p| p.text.split_whitespace().map(str::to_lowercase).collect::<Vec<_>>())
        .collect()
}

fn heading(doc: &Document, span: (usize, usize)) -> String {
    doc.body.get(span.0).and_then(|b| b.as_para()).map(|p| p.text.trim().to_uppercase()).unwrap_or_default()
}

/// The longest common subsequence of `a` and `b`, as index pairs.
#[allow(clippy::indexing_slicing, reason = "i < n and j < m by the loops, and the table is (n + 1) by (m + 1)")]
fn lcs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    // Long scripts: a quadratic table over every paragraph would be too big; fall back to a
    // greedy forward match beyond this.
    if n.saturating_mul(m) > 4_000_000 {
        let mut out = Vec::new();
        let mut j0 = 0;
        for (i, x) in a.iter().enumerate() {
            if let Some(j) = (j0..m).find(|&j| b.get(j) == Some(x)) {
                out.push((i, j));
                j0 = j + 1;
            }
        }
        return out;
    }
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i] == b[j] { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Match new scene `j` with old scene `i`.
fn pair(matched: &mut [Option<usize>], taken: &mut [bool], j: usize, i: usize) {
    if let Some(m) = matched.get_mut(j) {
        *m = Some(i);
    }
    if let Some(t) = taken.get_mut(i) {
        *t = true;
    }
}

/// Compare `old` with `new`: the marked-up copy of `new` and what changed.
pub fn compare(old: &Document, new: &Document) -> (Document, Summary) {
    let (eo, en) = (Elements::of_doc(old), Elements::of_doc(new));
    let (so, sn) = (spans(old), spans(new));
    // Match scenes: same heading first, then the most similar words (at least half shared).
    let mut matched: Vec<Option<usize>> = vec![None; sn.len()];
    let mut taken = vec![false; so.len()];
    for (j, s) in sn.iter().enumerate() {
        let h = heading(new, *s);
        if let Some(i) = so.iter().enumerate().position(|(i, o)| taken.get(i) == Some(&false) && heading(old, *o) == h) {
            pair(&mut matched, &mut taken, j, i);
        }
    }
    for (j, s) in sn.iter().enumerate() {
        if matched.get(j).is_some_and(Option::is_some) {
            continue;
        }
        // The scene's text below its heading: headings alone share too many words.
        let wn = words(new, (s.0 + 1, s.1));
        if wn.is_empty() {
            continue;
        }
        let best = so
            .iter()
            .enumerate()
            .filter(|(i, _)| taken.get(*i) == Some(&false))
            .map(|(i, o)| {
                let wo = words(old, (o.0 + 1, o.1));
                let shared = wn.intersection(&wo).count() as f32;
                (i, shared / (wn.len().max(wo.len()).max(1) as f32))
            })
            .filter(|(_, sim)| *sim >= 0.5)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = best {
            pair(&mut matched, &mut taken, j, i);
        }
    }
    // Moved: matched scenes outside the longest run kept in order.
    let order: Vec<usize> = matched.iter().flatten().copied().collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    let kept: HashSet<usize> = lcs(&order, &sorted).into_iter().filter_map(|(k, _)| order.get(k).copied()).collect();
    let mut out = new.clone();
    let mut summary = Summary::default();
    let ins = u32::try_from(out.revisions.len()).unwrap_or(0);
    out.revisions.push(Revision { kind: RevisionKind::Insert, author: "Comparison".into(), date: String::new(), move_name: None });
    let del = ins + 1;
    out.revisions.push(Revision { kind: RevisionKind::Delete, author: "Comparison".into(), date: String::new(), move_name: None });
    let mark = |p: &mut Paragraph, which: u32, inserted: bool| {
        let len = p.len();
        let _ = p.format(0, len, &|c| {
            if inserted {
                c.ins = Some(which);
            } else {
                c.del = Some(which);
            }
        });
        p.touch();
    };
    // Build the new body scene by scene, putting removed paragraphs back as deletions.
    let mut body = Vec::new();
    let mut notes = crate::notes::Notes::of_doc(&out);
    let mut moved_notes: Vec<(usize, String)> = Vec::new();
    let mut removed_scenes: Vec<usize> = (0..so.len()).filter(|i| taken.get(*i) == Some(&false)).collect();
    // The old scene `r`'s blocks.
    let old_span = |r: usize| so.get(r).map_or(0..0, |&(a, b)| a..b);
    for (j, s) in sn.iter().enumerate() {
        let new_paras: Vec<Paragraph> = (s.0..s.1).filter_map(|i| new.body.get(i).and_then(|b| b.as_para()).cloned()).collect();
        // Removed scenes that came before this one's match go in first.
        let matched_j = matched.get(j).copied().flatten();
        if let Some(i) = matched_j {
            let before: Vec<usize> = removed_scenes.iter().copied().filter(|r| *r < i).collect();
            for r in before {
                removed_scenes.retain(|x| *x != r);
                summary.scenes_removed += 1;
                for k in old_span(r) {
                    if let Some(mut p) = old.body.get(k).and_then(|b| b.as_para()).cloned() {
                        mark(&mut p, del, false);
                        summary.paragraphs_removed += 1;
                        body.push(para_block(p));
                    }
                }
            }
        }
        match matched_j {
            None => {
                summary.scenes_added += 1;
                for mut p in new_paras {
                    mark(&mut p, ins, true);
                    summary.paragraphs_added += 1;
                    body.push(para_block(p));
                }
            }
            Some(i) => {
                if !kept.contains(&i) {
                    summary.scenes_moved += 1;
                    // Scene numbers count headings only, not the text before the first.
                    let prologue = usize::from(scenes(old).first().is_some_and(|f| f.start > 0));
                    moved_notes.push((body.len(), format!("Moved: this was scene {} in the other version.", i + 1 - prologue)));
                }
                let old_paras: Vec<Paragraph> = old_span(i).filter_map(|k| old.body.get(k).and_then(|b| b.as_para()).cloned()).collect();
                let ko: Vec<String> = old_paras.iter().map(|p| key(&eo, p)).collect();
                let kn: Vec<String> = new_paras.iter().map(|p| key(&en, p)).collect();
                let same = lcs(&ko, &kn);
                let (mut oi, mut ni) = (0, 0);
                for (a, b) in same.iter().copied().chain(std::iter::once((ko.len(), kn.len()))) {
                    for p in old_paras.get(oi..a).unwrap_or_default() {
                        let mut p = p.clone();
                        mark(&mut p, del, false);
                        summary.paragraphs_removed += 1;
                        body.push(para_block(p));
                    }
                    for p in new_paras.get(ni..b).unwrap_or_default() {
                        let mut p = p.clone();
                        mark(&mut p, ins, true);
                        summary.paragraphs_added += 1;
                        body.push(para_block(p));
                    }
                    if let Some(p) = new_paras.get(b) {
                        body.push(para_block(p.clone()));
                    }
                    oi = a + 1;
                    ni = b + 1;
                }
            }
        }
    }
    for r in removed_scenes {
        summary.scenes_removed += 1;
        for k in old_span(r) {
            if let Some(mut p) = old.body.get(k).and_then(|b| b.as_para()).cloned() {
                mark(&mut p, del, false);
                summary.paragraphs_removed += 1;
                body.push(para_block(p));
            }
        }
    }
    out.body = body;
    out.ensure_nonempty();
    // A note on each moved scene's heading.
    for (block, text) in moved_notes {
        let id = notes.notes.keys().next_back().map_or(1, |k| k + 1);
        if let Some(wordsmith_doc::Block::Para(p)) = out.body.get_mut(block).map(std::sync::Arc::make_mut) {
            let len = p.len();
            if len == 0 {
                continue;
            }
            let _ = p.format(0, len, &|c| c.note = Some(id));
            notes.notes.insert(id, crate::notes::Note { text, author: "Comparison".into(), kind: "Production".into(), ..Default::default() });
        }
    }
    notes.store(&mut out);
    (out, summary)
}

#[cfg(test)]
mod tests {
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
        let text = |c: &crate::tracking::Change| marked.body[c.block].as_para().map(|p| p.text[c.start..c.end].to_string()).unwrap_or_default();
        assert!(changes.iter().any(|c| !c.inserted && text(c) == "Anna waits."));
        assert!(changes.iter().any(|c| c.inserted && text(c) == "Anna paces."));
        assert!(changes.iter().any(|c| !c.inserted && text(c) == "INT. ATTIC - DAY"));
        assert_eq!(crate::notes::Notes::of_doc(&marked).notes.len(), 1, "one moved scene noted");
        let (same, s2) = compare(&new, &new);
        assert_eq!(s2, Summary::default());
        assert!(crate::tracking::changes(&same).is_empty());
    }
}
