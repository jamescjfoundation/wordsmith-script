// SPDX-License-Identifier: MIT OR Apache-2.0
//! Change tracking (SPEC-2 11.5) is the core's (`jcj_features::tracking`): the model's tracked
//! changes and its switch (`settings.track_changes`), listing, stepping, accepting and rejecting.
//!
//! Script types with its own rules (`commands::text`), so it records its own tracked insertions
//! and deletions: each with who made the change and the day. Deleting your own tracked insertion
//! simply removes it. Paragraph breaks are not tracked.

pub use jcj_features::tracking::*;

use jcj_doc::{Pos, Revision, RevisionKind};
use jcj_engine::{CmdError, Session};

/// Most tracked-change records a script keeps.
pub const MAX_RECORDS: usize = 100_000;

/// The writer's name, or a stand-in.
fn author(s: &Session) -> String {
    if s.author.trim().is_empty() { "Writer".into() } else { s.author.trim().to_string() }
}

/// The record for a change of `kind` by the session's writer today, made if needed.
pub fn record(s: &mut Session, kind: RevisionKind) -> Option<u32> {
    let who = author(s);
    let date = crate::reports_more::date(crate::clock::now());
    if let Some(i) = s.doc.revisions.iter().position(|r| r.kind == kind && r.author == who && r.date == date && r.move_name.is_none()) {
        return u32::try_from(i).ok();
    }
    if s.doc.revisions.len() >= MAX_RECORDS {
        return None;
    }
    s.doc.revisions.push(Revision { kind, author: who, date, move_name: None });
    u32::try_from(s.doc.revisions.len() - 1).ok()
}

/// Delete `a..b` as a tracked change: the writer's own tracked insertions go; everything else
/// stays, marked deleted. Returns where the caret goes: `a` (backwards) or the end of what was
/// passed over (forwards).
pub fn delete(s: &mut Session, a: &Pos, b: &Pos, forwards: bool) -> Result<Pos, CmdError> {
    let del = record(s, RevisionKind::Delete);
    let me = author(s);
    let mine: Vec<u32> = s
        .doc
        .revisions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.author == me && r.kind == RevisionKind::Insert)
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .collect();
    let mut end = b.clone();
    // Last paragraph first, and in each the last run first, so earlier offsets stay right.
    for path in s.doc.para_paths_in(a, b).into_iter().rev() {
        let Ok(para) = s.doc.para_mut(a.story, &path) else { continue };
        let from = if path == a.path { a.off } else { 0 };
        let to = if path == b.path { b.off } else { para.len() };
        let runs: Vec<(usize, usize, Option<u32>, Option<u32>)> =
            para.run_ranges().filter(|(r, _)| r.end > from && r.start < to).map(|(r, c)| (r.start.max(from), r.end.min(to), c.ins, c.del)).collect();
        for (start, stop, ins, gone) in runs.into_iter().rev() {
            if gone.is_some() {
                continue;
            }
            if ins.is_some_and(|i| mine.contains(&i)) {
                let para = s.doc.para_mut(a.story, &path)?;
                para.delete(start, stop)?;
                if path == b.path {
                    end.off -= stop - start;
                }
            } else {
                let (pa, pb) = (Pos { story: a.story, path: path.clone(), off: start }, Pos { story: a.story, path: path.clone(), off: stop });
                s.doc.format_range(&pa, &pb, &|c| c.del = del)?;
            }
        }
        if let Ok(p) = s.doc.para_mut(a.story, &path) {
            p.touch();
        }
    }
    Ok(if forwards { end } else { a.clone() })
}

#[cfg(test)]
mod tests {
    use jcj_engine::Selection;
    use serde_json::json;

    use super::*;
    use crate::commands::new_session;
    use crate::commands::text::body_lines;
    use crate::data::body_pos;

    fn session() -> Session {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. hall\nRain falls hard."})).unwrap();
        s.author = "Ana".into();
        crate::clock::fix(Some(1_700_000_000));
        s.run("track.toggle", &json!({"on": true})).unwrap();
        s
    }

    #[test]
    fn typing_and_deleting_are_tracked_with_who_and_when() {
        let mut s = session();
        s.sel = Selection::caret(body_pos(1, 4));
        s.run("text.insert", &json!({"text": " still"})).unwrap();
        s.sel = Selection { anchor: body_pos(1, 16), focus: body_pos(1, 21) };
        s.run("text.backspace", &json!({})).unwrap();
        // Nothing is gone yet: the deletion is marked.
        assert_eq!(body_lines(&s)[1].1, "Rain still falls hard.");
        let list = s.run("track.list", &json!({})).unwrap();
        let ch = list["changes"].as_array().cloned().unwrap_or_default();
        assert_eq!(ch.len(), 2);
        assert_eq!(
            (ch[0]["kind"].as_str(), ch[0]["text"].as_str(), ch[0]["author"].as_str(), ch[0]["date"].as_str()),
            (Some("inserted"), Some(" still"), Some("Ana"), Some("2023-11-14"))
        );
        assert_eq!((ch[1]["kind"].as_str(), ch[1]["text"].as_str()), (Some("deleted"), Some(" hard")));
        // Deleting your own insertion removes it.
        s.sel = Selection::caret(body_pos(1, 10));
        for _ in 0..6 {
            s.run("text.backspace", &json!({})).unwrap();
        }
        assert_eq!(body_lines(&s)[1].1, "Rain falls hard.");
        assert_eq!(changes(&s.doc).len(), 1);
    }

    #[test]
    fn accept_reject_and_step() {
        let mut s = session();
        s.sel = Selection::caret(body_pos(1, 0));
        s.run("text.insert", &json!({"text": "Cold "})).unwrap();
        s.sel = Selection { anchor: body_pos(1, 15), focus: body_pos(1, 20) };
        s.run("text.delete", &json!({})).unwrap();
        s.sel = Selection::caret(body_pos(0, 0));
        s.run("track.next", &json!({})).unwrap();
        assert_eq!(s.selected_text(), "Cold ");
        s.run("track.accept", &json!({})).unwrap();
        s.run("track.next", &json!({})).unwrap();
        assert_eq!(s.selected_text(), " hard");
        s.run("track.reject", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[1].1, "Cold Rain falls hard.");
        assert!(changes(&s.doc).is_empty());
        // Accept all removes deletions for good.
        s.sel = Selection { anchor: body_pos(1, 0), focus: body_pos(1, 5) };
        s.run("text.backspace", &json!({})).unwrap();
        s.run("track.acceptAll", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[1].1, "Rain falls hard.");
        assert!(s.run("track.accept", &json!({})).is_err());
        s.run("track.toggle", &json!({})).unwrap();
        assert!(!is_on(&s.doc));
    }
}
