// SPDX-License-Identifier: MIT OR Apache-2.0
//! Change tracking (SPEC-2 11.5). While it is on, typed text is marked as inserted and deleted
//! text is kept, marked as deleted, each with who made the change and when (the document's
//! tracked-change records, `Document::revisions`). Changes can be stepped through, accepted and
//! rejected, one at a time or all at once. Deleting your own tracked insertion simply removes
//! it. Paragraph breaks are not tracked. The setting is kept in `app_data` under
//! [`TRACKING_KEY`]; writers' colors come from the order they first made a change.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Path, Pos, Revision, RevisionKind, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result};

/// `app_data` key of the tracking switch.
pub const TRACKING_KEY: &str = "wordsmith.tracking";
/// Most tracked-change records a script keeps.
pub const MAX_RECORDS: usize = 100_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tracking {
    pub on: bool,
}

pub fn is_on(doc: &Document) -> bool {
    crate::data::load::<Tracking>(doc, TRACKING_KEY).on
}

/// The writer's name, or a stand-in.
fn author(s: &Session) -> String {
    if s.author.trim().is_empty() { "Writer".into() } else { s.author.trim().to_string() }
}

/// The record for a change of `kind` by the session's writer today, made if needed.
pub fn record(s: &mut Session, kind: RevisionKind) -> Option<u32> {
    let who = author(s);
    let date = crate::reports_more::date(s.now);
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

/// One tracked change in the script: where, what kind, by whom and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub block: usize,
    pub start: usize,
    pub end: usize,
    pub inserted: bool,
    pub record: u32,
}

/// Every tracked change in the body, in order (runs next to each other with the same record
/// are one change).
pub fn changes(doc: &Document) -> Vec<Change> {
    let mut out: Vec<Change> = Vec::new();
    for (block, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        for (r, c) in p.run_ranges() {
            let (inserted, record) = match (c.ins, c.del) {
                (_, Some(d)) => (false, d),
                (Some(i), None) => (true, i),
                _ => continue,
            };
            if let Some(last) = out.last_mut()
                && last.block == block
                && last.end == r.start
                && last.inserted == inserted
                && last.record == record
            {
                last.end = r.end;
                continue;
            }
            out.push(Change { block, start: r.start, end: r.end, inserted, record });
        }
    }
    out
}

fn pos(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: Path::top(block), off }
}

/// Accept (`keep` true) or reject one change.
fn settle(doc: &mut Document, c: &Change, accept: bool) -> Result<(), CmdError> {
    let remove = c.inserted != accept;
    if remove {
        let para = doc.para_mut(StoryRef::Body, &Path::top(c.block))?;
        para.delete(c.start, c.end)?;
        para.touch();
    } else {
        doc.format_range(&pos(c.block, c.start), &pos(c.block, c.end), &|p| {
            p.ins = None;
            p.del = None;
        })?;
    }
    Ok(())
}

fn toggle(s: &mut Session, v: &Value) -> CmdResult {
    let mut t: Tracking = crate::data::load(&s.doc, TRACKING_KEY);
    t.on = v.get("on").and_then(Value::as_bool).unwrap_or(!t.on);
    crate::data::store(&mut s.doc, TRACKING_KEY, &t);
    Ok(json!({"on": t.on}))
}

fn list(s: &mut Session, _: &Value) -> CmdResult {
    let items: Vec<Value> = changes(&s.doc)
        .iter()
        .map(|c| {
            let r = s.doc.revisions.get(c.record as usize);
            let text = s.doc.body.get(c.block).and_then(|b| b.as_para()).and_then(|p| p.text.get(c.start..c.end)).unwrap_or("").to_string();
            json!({"block": c.block, "start": c.start, "end": c.end, "kind": if c.inserted { "inserted" } else { "deleted" }, "author": r.map(|r| r.author.clone()), "date": r.map(|r| r.date.clone()), "text": text})
        })
        .collect();
    Ok(json!({"on": is_on(&s.doc), "changes": items}))
}

/// The changes the selection touches, or the one at the caret.
fn picked(s: &Session) -> Vec<Change> {
    let (a, b) = s.sel.ordered();
    let at = |c: &Change| (c.block, c.start) <= (b.path.last(), b.off) && (c.block, c.end) >= (a.path.last(), a.off);
    changes(&s.doc).into_iter().filter(at).collect()
}

fn settle_cmd(s: &mut Session, v: &Value, accept: bool) -> CmdResult {
    let all = v.get("all").and_then(Value::as_bool) == Some(true);
    let mut list = if all { changes(&s.doc) } else { picked(s) };
    if list.is_empty() {
        return Err(CmdError::Failed("there is no tracked change here".into()));
    }
    // Last first, so earlier offsets stay right.
    list.sort_by_key(|c| std::cmp::Reverse((c.block, c.start)));
    let n = list.len();
    for c in &list {
        settle(&mut s.doc, c, accept)?;
    }
    s.clamp_selection();
    Ok(json!({"settled": n}))
}

fn step(s: &mut Session, forward: bool) -> CmdResult {
    let here = (s.sel.focus.path.last(), s.sel.focus.off);
    let all = changes(&s.doc);
    let pick = if forward {
        all.iter().find(|c| (c.block, c.start) > here).or_else(|| all.first())
    } else {
        all.iter().rev().find(|c| (c.block, c.start) < here).or_else(|| all.last())
    };
    let Some(c) = pick.cloned() else { return Err(CmdError::Failed("there are no tracked changes".into())) };
    s.sel = Selection { anchor: pos(c.block, c.start), focus: pos(c.block, c.end) };
    sel_result(s)
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Production › Changes";
    vec![
        CommandSpec::new("track.toggle", "Track Changes", LOC, toggle).params(r#"{"on"?: bool}"#).key("Mod+Shift+C"),
        CommandSpec::new("track.list", "Tracked Changes", LOC, list).pure(),
        CommandSpec::new("track.accept", "Accept Change", LOC, |s, v| settle_cmd(s, v, true)).params(r#"{"all"?: bool}"#),
        CommandSpec::new("track.reject", "Reject Change", LOC, |s, v| settle_cmd(s, v, false)).params(r#"{"all"?: bool}"#),
        CommandSpec::new("track.acceptAll", "Accept All Changes", LOC, |s, _| settle_cmd(s, &json!({"all": true}), true)),
        CommandSpec::new("track.rejectAll", "Reject All Changes", LOC, |s, _| settle_cmd(s, &json!({"all": true}), false)),
        CommandSpec::new("track.next", "Next Change", LOC, |s, _| step(s, true)).pure(),
        CommandSpec::new("track.previous", "Previous Change", LOC, |s, _| step(s, false)).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::commands::text::body_lines;
    use crate::data::body_pos;

    fn session() -> Session {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. hall\nRain falls hard."})).unwrap();
        s.author = "Ana".into();
        s.now = 1_700_000_000;
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
