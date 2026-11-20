// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bookmarks and labels (SPEC-2 10): named places in the script to go to. A label is a
//! bookmark that is also offered by name in Go To's label list; both are kept the same way.
//!
//! A bookmark's name lives in `app_data` under [`BOOKMARKS_KEY`]; its place is the paragraph
//! carrying its id (`ParaProps::bookmark`), so it moves with the paragraph through every edit.
//! A paragraph holds at most one. Splitting a paragraph leaves the bookmark on the first part.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::data::{body_pos, clean_line};

/// `app_data` key of the bookmarks' names.
pub const BOOKMARKS_KEY: &str = "wordsmith.bookmarks";
pub const MAX_BOOKMARKS: usize = 10_000;
pub const MAX_NAME_CHARS: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Bookmark {
    pub name: String,
    /// Listed as a label too.
    pub label: bool,
}

/// A script's bookmarks, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bookmarks {
    pub items: BTreeMap<u32, Bookmark>,
}

impl Bookmarks {
    pub fn of_doc(doc: &Document) -> Self {
        let map: BTreeMap<String, Bookmark> = crate::data::load(doc, BOOKMARKS_KEY);
        let items = map
            .into_iter()
            .take(MAX_BOOKMARKS)
            .filter_map(|(k, mut b)| {
                b.name = clean_line(&b.name, MAX_NAME_CHARS);
                k.parse().ok().map(|id| (id, b))
            })
            .collect();
        Bookmarks { items }
    }

    pub fn store(&self, doc: &mut Document) {
        let map: BTreeMap<String, &Bookmark> = self.items.iter().map(|(k, v)| (k.to_string(), v)).collect();
        crate::data::store(doc, BOOKMARKS_KEY, &map);
    }
}

/// Every bookmark that is in the script, as (body block, id), in script order.
pub fn placed(doc: &Document) -> Vec<(usize, u32)> {
    let known = Bookmarks::of_doc(doc);
    doc.body
        .iter()
        .enumerate()
        .filter_map(|(i, b)| b.as_para().and_then(|p| p.props.bookmark).filter(|id| known.items.contains_key(id)).map(|id| (i, id)))
        .collect()
}

fn list(s: &mut Session, _: &Value) -> CmdResult {
    let names = Bookmarks::of_doc(&s.doc);
    let pag = crate::paginate::paginate(&s.doc);
    let page_of =
        |block: usize| pag.pages.iter().position(|pg| pg.iter().any(|pc| matches!(pc, crate::paginate::Piece::Para { block: b, .. } if *b == block)));
    let items: Vec<Value> = placed(&s.doc)
        .into_iter()
        .filter_map(|(block, id)| {
            let b = names.items.get(&id)?;
            let page = page_of(block).map(|i| pag.labels.as_ref().and_then(|l| l.get(i).cloned()).unwrap_or_else(|| (i + 1).to_string()));
            let text = s.doc.body.get(block).and_then(|b| b.as_para()).map(|p| p.text.chars().take(80).collect::<String>()).unwrap_or_default();
            Some(json!({"id": id, "name": b.name, "label": b.label, "block": block, "page": page, "text": text}))
        })
        .collect();
    Ok(json!({"bookmarks": items}))
}

/// `bookmark.add {name?, label?}`: bookmark the caret's paragraph (renaming its bookmark if it
/// has one).
fn add(s: &mut Session, v: &Value) -> CmdResult {
    let block = s.sel.focus.path.last();
    let para_text = s
        .doc
        .body
        .get(block)
        .and_then(|b| b.as_para())
        .map(|p| p.text.clone())
        .ok_or_else(|| CmdError::Failed("the caret is not in a paragraph".into()))?;
    let mut marks = Bookmarks::of_doc(&s.doc);
    let given = p::str(v, "name").map(|n| clean_line(n, MAX_NAME_CHARS)).filter(|n| !n.is_empty());
    let name = given.unwrap_or_else(|| {
        let t = clean_line(&para_text, 40);
        if t.is_empty() { format!("Bookmark {}", marks.items.len() + 1) } else { t }
    });
    let label = p::bool(v, "label").unwrap_or(false);
    let existing = s.doc.body.get(block).and_then(|b| b.as_para()).and_then(|p| p.props.bookmark).filter(|id| marks.items.contains_key(id));
    let id = match existing {
        Some(id) => id,
        None => {
            if marks.items.len() >= MAX_BOOKMARKS {
                return Err(CmdError::Failed(format!("a script keeps at most {MAX_BOOKMARKS} bookmarks")));
            }
            marks.items.keys().next_back().map_or(1, |k| k.saturating_add(1))
        }
    };
    marks.items.insert(id, Bookmark { name: name.clone(), label });
    marks.store(&mut s.doc);
    let para = s.doc.para_mut(wordsmith_doc::StoryRef::Body, &wordsmith_doc::Path::top(block))?;
    para.props.bookmark = Some(id);
    para.touch();
    Ok(json!({"id": id, "name": name, "label": label}))
}

fn id_param(s: &Session, v: &Value) -> Result<u32, CmdError> {
    if let Some(id) = v.get("id").and_then(Value::as_u64) {
        return u32::try_from(id).map_err(|_| CmdError::Params("`id` is out of range".into()));
    }
    if let Some(name) = p::str(v, "name") {
        let marks = Bookmarks::of_doc(&s.doc);
        return marks
            .items
            .iter()
            .find(|(_, b)| b.name.eq_ignore_ascii_case(name.trim()))
            .map(|(k, _)| *k)
            .ok_or_else(|| CmdError::Failed(format!("no bookmark `{name}`")));
    }
    s.doc
        .body
        .get(s.sel.focus.path.last())
        .and_then(|b| b.as_para())
        .and_then(|p| p.props.bookmark)
        .ok_or_else(|| CmdError::Params("give `id` or `name`, or put the caret on a bookmarked paragraph".into()))
}

fn goto_block(s: &mut Session, block: usize) -> CmdResult {
    s.sel = Selection::caret(body_pos(block, 0));
    s.clamp_selection();
    sel_result(s)
}

/// `bookmark.goto {id | name}`.
fn goto(s: &mut Session, v: &Value) -> CmdResult {
    let id = id_param(s, v)?;
    let block = placed(&s.doc)
        .into_iter()
        .find(|(_, b)| *b == id)
        .map(|(i, _)| i)
        .ok_or_else(|| CmdError::Failed("that bookmark is no longer in the script".into()))?;
    goto_block(s, block)
}

/// `bookmark.delete {id | name}`, or the caret paragraph's bookmark.
fn delete(s: &mut Session, v: &Value) -> CmdResult {
    let id = id_param(s, v)?;
    let mut marks = Bookmarks::of_doc(&s.doc);
    marks.items.remove(&id);
    marks.store(&mut s.doc);
    for (i, b) in s.doc.body.clone().iter().enumerate() {
        if b.as_para().and_then(|p| p.props.bookmark) == Some(id) {
            let para = s.doc.para_mut(wordsmith_doc::StoryRef::Body, &wordsmith_doc::Path::top(i))?;
            para.props.bookmark = None;
            para.touch();
        }
    }
    Ok(Value::Null)
}

/// The next (or previous) bookmark from the caret, wrapping round.
fn step(s: &mut Session, forward: bool) -> CmdResult {
    let here = s.sel.focus.path.last();
    let all = placed(&s.doc);
    let pick = if forward {
        all.iter().find(|(b, _)| *b > here).or_else(|| all.first())
    } else {
        all.iter().rev().find(|(b, _)| *b < here).or_else(|| all.last())
    };
    let Some((block, _)) = pick.copied() else { return Err(CmdError::Failed("the script has no bookmarks".into())) };
    goto_block(s, block)
}

fn has_bookmarks(s: &Session) -> Option<&'static str> {
    if placed(&s.doc).is_empty() { Some("the script has no bookmarks") } else { None }
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Go";
    vec![
        CommandSpec::new("bookmark.add", "Add Bookmark", LOC, add).key("Mod+M").params(r#"{"name"?: string, "label"?: bool}"#),
        CommandSpec::new("bookmark.list", "Bookmarks", LOC, list).pure(),
        CommandSpec::new("bookmark.goto", "Go to Bookmark", LOC, goto).params(r#"{"id"?: number, "name"?: string}"#),
        CommandSpec::new("bookmark.delete", "Delete Bookmark", LOC, delete).params(r#"{"id"?: number, "name"?: string}"#),
        CommandSpec::new("bookmark.next", "Next Bookmark", LOC, |s, _| step(s, true)).when(has_bookmarks),
        CommandSpec::new("bookmark.previous", "Previous Bookmark", LOC, |s, _| step(s, false)).when(has_bookmarks),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;

    fn type_lines(s: &mut Session, text: &str) {
        s.run("text.insert", &json!({"text": text})).unwrap();
    }

    #[test]
    fn bookmarks_follow_their_paragraph_and_wrap() {
        let mut s = new_session();
        type_lines(&mut s, "INT. HALL - DAY\nRain.\nSun.");
        s.sel = Selection::caret(body_pos(1, 0));
        let b = s.run("bookmark.add", &json!({"name": "rain"})).unwrap();
        assert_eq!(b["name"], "rain");
        // A paragraph inserted before it moves it down.
        s.sel = Selection::caret(body_pos(0, 0));
        s.run("text.insert", &json!({"text": "FADE IN:\n"})).unwrap();
        let listed = s.run("bookmark.list", &json!({})).unwrap();
        assert_eq!(listed["bookmarks"][0]["block"], 2);
        assert_eq!(listed["bookmarks"][0]["page"], "1");
        s.sel = Selection::caret(body_pos(3, 0));
        s.run("bookmark.next", &json!({})).unwrap();
        assert_eq!(s.sel.focus.path.last(), 2, "wraps round");
        s.run("bookmark.delete", &json!({"name": "RAIN"})).unwrap();
        assert!(placed(&s.doc).is_empty());
        assert!(s.run("bookmark.next", &json!({})).is_err());
    }

    #[test]
    fn splitting_keeps_the_bookmark_on_the_first_part() {
        let mut s = new_session();
        type_lines(&mut s, "INT. HALL - DAY\nRain falls.");
        s.run("bookmark.add", &json!({"label": true})).unwrap();
        s.sel = Selection::caret(body_pos(1, 4));
        s.run("text.newParagraph", &json!({})).unwrap();
        assert_eq!(placed(&s.doc), vec![(1, 1)]);
        let listed = s.run("bookmark.list", &json!({})).unwrap();
        assert_eq!(listed["bookmarks"][0]["label"], true);
        assert_eq!(listed["bookmarks"][0]["name"], "Rain falls.");
    }

    #[test]
    fn hostile_names_are_bounded() {
        let mut doc = Document::new();
        crate::data::store(&mut doc, BOOKMARKS_KEY, &json!({"1": {"name": "x".repeat(10_000)}, "nope": {}, "2": 7}));
        let b = Bookmarks::of_doc(&doc);
        assert!(b.items.is_empty() || b.items.values().all(|m| m.name.chars().count() <= MAX_NAME_CHARS));
    }
}
