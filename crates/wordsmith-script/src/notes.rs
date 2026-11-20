// SPDX-License-Identifier: MIT OR Apache-2.0
//! Notes attached to script text (SPEC: "Notes attached to script text, edited in a popover,
//! listed in the navigator"), decision 0023.
//!
//! A note's text lives in `app_data` under [`NOTES_KEY`] (undoable, saved with the script). Its
//! anchor is a mark on the characters it is attached to (`CharProps::note`), so it moves with
//! them through typing, deleting, splitting and joining, and the script's text itself is never
//! changed. Typing at the edge of a noted range does not extend it. A note whose text was all
//! deleted is still listed, as detached, until it is deleted.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::scenes::{scene_at, scenes};

/// `app_data` key of the notes' texts.
pub const NOTES_KEY: &str = "wordsmith.notes";
/// Most notes a script keeps, and the longest note.
pub const MAX_NOTES: usize = 10_000;
pub const MAX_NOTE_CHARS: usize = 10_000;
/// Characters of anchored text a listing quotes.
pub const MAX_QUOTE_CHARS: usize = 120;

/// One note.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Note {
    pub text: String,
    pub author: String,
}

fn clean(text: &str) -> String {
    text.replace("\r\n", "\n").chars().filter(|c| *c == '\n' || !c.is_control()).take(MAX_NOTE_CHARS).collect::<String>().trim().to_string()
}

/// The notes of a script, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    pub notes: BTreeMap<u32, Note>,
}

impl Notes {
    /// The notes `doc` holds; bad entries are skipped and texts cut to their limit.
    pub fn of_doc(doc: &Document) -> Self {
        let mut notes = BTreeMap::new();
        if let Some(Value::Object(map)) = doc.app_data.get(NOTES_KEY).map(|v| (**v).clone()) {
            for (k, v) in map.into_iter().take(MAX_NOTES) {
                if let (Ok(id), Ok(mut n)) = (k.parse::<u32>(), serde_json::from_value::<Note>(v)) {
                    n.text = clean(&n.text);
                    n.author = n.author.chars().filter(|c| !c.is_control()).take(100).collect();
                    notes.insert(id, n);
                }
            }
        }
        Notes { notes }
    }

    pub fn store(&self, doc: &mut Document) {
        let map: serde_json::Map<String, Value> =
            self.notes.iter().map(|(k, v)| (k.to_string(), serde_json::to_value(v).unwrap_or_default())).collect();
        doc.app_data.insert(NOTES_KEY.to_string(), Arc::new(Value::Object(map)));
    }

    fn next_id(&self) -> u32 {
        self.notes.keys().next_back().map_or(1, |k| k.saturating_add(1))
    }
}

/// Where a note is anchored: its first and last body positions and the text it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub quote: String,
}

/// Every note's anchor in the body, in script order of their starts.
pub fn anchors(doc: &Document) -> BTreeMap<u32, Anchor> {
    let mut out: BTreeMap<u32, Anchor> = BTreeMap::new();
    for (block, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        for (range, props) in p.run_ranges() {
            let Some(id) = props.note else { continue };
            let piece = p.text.get(range.clone()).unwrap_or("");
            let a = out.entry(id).or_insert_with(|| Anchor { start: (block, range.start), end: (block, range.end), quote: String::new() });
            // A note running into the next paragraph quotes it after a space.
            let next_paragraph = a.end.0 != block;
            a.end = (block, range.end);
            if a.quote.chars().count() < MAX_QUOTE_CHARS {
                if next_paragraph && !a.quote.is_empty() {
                    a.quote.push(' ');
                }
                a.quote.extend(piece.chars().filter(|c| *c != wordsmith_doc::para::OBJ).take(MAX_QUOTE_CHARS));
            }
        }
    }
    out
}

/// The note at a body position, if the character there (or just before it) carries one.
pub fn note_at(doc: &Document, pos: &Pos) -> Option<u32> {
    let p = doc.para_at(pos)?;
    let after = p.text.get(pos.off..).and_then(|t| t.chars().next()).map(char::len_utf8);
    let here = after.and_then(|n| p.props_at(pos.off + n).note);
    here.or_else(|| (pos.off > 0).then(|| p.props_at(pos.off).note).flatten())
}

/// The note on the character just after a position, if any.
pub fn note_at_char(doc: &Document, pos: &Pos) -> Option<u32> {
    let p = doc.para_at(pos)?;
    let n = p.text.get(pos.off..).and_then(|t| t.chars().next()).map(char::len_utf8)?;
    p.props_at(pos.off + n).note
}

fn pos(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(block), off }
}

/// `note.add {text}`: attach a note to the selection (or, with a caret, to its paragraph).
fn add(s: &mut Session, v: &Value) -> CmdResult {
    let text = clean(p::req_str(v, "text")?);
    let mut notes = Notes::of_doc(&s.doc);
    if notes.notes.len() >= MAX_NOTES {
        return Err(CmdError::Failed(format!("the script already has {MAX_NOTES} notes")));
    }
    let (mut a, mut b) = s.sel.ordered();
    if a == b {
        let len = s.doc.para_at(&a).map_or(0, |p| p.len());
        if len == 0 {
            return Err(CmdError::Failed("there is no text here to attach a note to".into()));
        }
        a.off = 0;
        b.off = len;
    }
    if a.story != StoryRef::Body {
        return Err(CmdError::Failed("notes attach to the script's text".into()));
    }
    let id = notes.next_id();
    s.doc.format_range(&a, &b, &|c| c.note = Some(id))?;
    notes.notes.insert(id, Note { text, author: s.author.clone() });
    notes.store(&mut s.doc);
    Ok(json!({"id": id}))
}

fn note_param(s: &Session, v: &Value) -> Result<(u32, Notes), CmdError> {
    let id =
        v.get("id").and_then(Value::as_u64).and_then(|i| u32::try_from(i).ok()).ok_or_else(|| CmdError::Params("`id` must be a note id".into()))?;
    let notes = Notes::of_doc(&s.doc);
    if !notes.notes.contains_key(&id) {
        return Err(CmdError::Params(format!("there is no note {id}")));
    }
    Ok((id, notes))
}

/// `note.edit {id, text}`: change a note's text.
fn edit(s: &mut Session, v: &Value) -> CmdResult {
    let (id, mut notes) = note_param(s, v)?;
    let text = clean(p::req_str(v, "text")?);
    if let Some(n) = notes.notes.get_mut(&id) {
        n.text = text;
    }
    notes.store(&mut s.doc);
    Ok(json!({"id": id}))
}

/// `note.delete {id}`: delete a note and its anchor.
fn delete(s: &mut Session, v: &Value) -> CmdResult {
    let (id, mut notes) = note_param(s, v)?;
    if let Some(a) = anchors(&s.doc).get(&id) {
        s.doc.format_range(&pos(a.start.0, a.start.1), &pos(a.end.0, a.end.1), &|c| {
            if c.note == Some(id) {
                c.note = None;
            }
        })?;
    }
    notes.notes.remove(&id);
    notes.store(&mut s.doc);
    sel_result(s)
}

/// The notes as a listing: id, text, author, the text they are attached to, the scene, and
/// whether they are still attached.
pub fn list_json(doc: &Document) -> Value {
    let all = scenes(doc);
    let at = anchors(doc);
    let mut rows: Vec<(Option<(usize, usize)>, Value)> = Notes::of_doc(doc)
        .notes
        .into_iter()
        .map(|(id, n)| {
            let a = at.get(&id);
            let scene = a.and_then(|a| scene_at(&all, a.start.0)).map(|sc| sc.index);
            (
                a.map(|a| a.start),
                json!({
                    "id": id,
                    "text": n.text,
                    "author": n.author,
                    "quote": a.map(|a| a.quote.clone()).unwrap_or_default(),
                    "block": a.map(|a| a.start.0),
                    "scene": scene,
                    "attached": a.is_some(),
                }),
            )
        })
        .collect();
    // In script order; detached notes last.
    rows.sort_by_key(|(start, _)| start.unwrap_or((usize::MAX, 0)));
    json!(rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>())
}

fn list(s: &mut Session, _: &Value) -> CmdResult {
    Ok(list_json(&s.doc))
}

/// `note.at`: the note at the caret, if any.
fn at(s: &mut Session, _: &Value) -> CmdResult {
    let Some(id) = note_at(&s.doc, &s.sel.focus) else { return Ok(Value::Null) };
    let n = Notes::of_doc(&s.doc).notes.get(&id).cloned().unwrap_or_default();
    Ok(json!({"id": id, "text": n.text, "author": n.author}))
}

/// `note.goto {id}`: select the text a note is attached to.
fn goto(s: &mut Session, v: &Value) -> CmdResult {
    let (id, _) = note_param(s, v)?;
    let Some(a) = anchors(&s.doc).get(&id).cloned() else { return Err(CmdError::Failed(format!("note {id} is no longer attached to any text"))) };
    s.sel = Selection { anchor: pos(a.start.0, a.start.1), focus: pos(a.end.0, a.end.1) };
    sel_result(s)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("note.add", "Add Note", "Script", add).params(r#"{"text": string}"#),
        CommandSpec::new("note.edit", "Edit Note", "Script", edit).params(r#"{"id": number, "text": string}"#),
        CommandSpec::new("note.delete", "Delete Note", "Script", delete).params(r#"{"id": number}"#),
        CommandSpec::new("note.list", "List Notes", "Script", list).pure(),
        CommandSpec::new("note.at", "Note at Caret", "Script", at).pure(),
        CommandSpec::new("note.goto", "Go to Note", "Script", goto).params(r#"{"id": number}"#).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::text::body_lines;
    use crate::template::{new_script, paragraph};
    use wordsmith_doc::para_block;

    fn session() -> Session {
        let mut doc = new_script();
        doc.body = [("SceneHeading", "INT. A - DAY"), ("Action", "Rain hits the window."), ("Character", "ANNA"), ("Dialogue", "Hello there.")]
            .iter()
            .map(|(e, t)| para_block(paragraph(e, t)))
            .collect();
        crate::session_for(doc)
    }

    fn select(s: &mut Session, block: usize, a: usize, b: usize) {
        s.sel = Selection { anchor: pos(block, a), focus: pos(block, b) };
    }

    #[test]
    fn notes_attach_list_edit_and_delete() {
        let mut s = session();
        let before = body_lines(&s);
        select(&mut s, 1, 0, 4);
        let id = s.run("note.add", &json!({"text": "Make it snow?"})).unwrap()["id"].as_u64().unwrap();
        assert_eq!(body_lines(&s), before, "the script's text is not changed");
        let list = s.run("note.list", &json!({})).unwrap();
        assert_eq!(list[0]["quote"], "Rain");
        assert_eq!(list[0]["scene"], 0);
        assert_eq!(list[0]["attached"], true);
        s.sel = Selection::caret(pos(1, 2));
        assert_eq!(s.run("note.at", &json!({})).unwrap()["text"], "Make it snow?");
        s.run("note.edit", &json!({"id": id, "text": "Snow."})).unwrap();
        // A caret note covers its paragraph.
        s.sel = Selection::caret(pos(3, 0));
        s.run("note.add", &json!({"text": "Too polite."})).unwrap();
        let list = s.run("note.list", &json!({})).unwrap();
        assert_eq!((list[0]["text"].as_str(), list[1]["quote"].as_str()), (Some("Snow."), Some("Hello there.")));
        s.run("note.delete", &json!({"id": id})).unwrap();
        assert_eq!(s.run("note.list", &json!({})).unwrap().as_array().map(Vec::len), Some(1));
        assert!(s.doc.body[1].as_para().unwrap().run_ranges().all(|(_, c)| c.note.is_none()), "its anchor is gone");
        assert!(s.run("note.edit", &json!({"id": 99, "text": "x"})).is_err());
    }

    #[test]
    fn anchors_move_with_their_text() {
        let mut s = session();
        select(&mut s, 1, 5, 9);
        let id = s.run("note.add", &json!({"text": "hits?"})).unwrap()["id"].as_u64().unwrap() as u32;
        // Typing before the noted word moves the anchor; typing right after it does not extend it.
        s.sel = Selection::caret(pos(1, 0));
        s.run("text.insert", &json!({"text": "Cold "})).unwrap();
        assert_eq!(anchors(&s.doc)[&id].quote, "hits");
        s.sel = Selection::caret(pos(1, 14));
        s.run("text.insert", &json!({"text": "!"})).unwrap();
        assert_eq!(anchors(&s.doc)[&id].quote, "hits", "typing at the end of a note does not extend it");
        // Typing inside it does.
        s.sel = Selection::caret(pos(1, 12));
        s.run("text.insert", &json!({"text": "-"})).unwrap();
        assert_eq!(anchors(&s.doc)[&id].quote, "hi-ts");
        // A split keeps it on both halves; deleting all its text detaches it.
        s.sel = Selection::caret(pos(1, 12));
        s.run("text.newParagraph", &json!({})).unwrap();
        assert_eq!(anchors(&s.doc)[&id].quote, "hi -ts");
        s.run("edit.undo", &json!({})).unwrap();
        select(&mut s, 1, 10, 15);
        s.run("text.backspace", &json!({})).unwrap();
        let list = s.run("note.list", &json!({})).unwrap();
        assert_eq!(list[0]["attached"], false);
        assert!(s.run("note.goto", &json!({"id": id})).is_err());
    }

    #[test]
    fn hostile_note_data_reads_safely() {
        let mut doc = new_script();
        doc.app_data.insert(NOTES_KEY.into(), Arc::new(json!({"x": {"text": "bad id"}, "3": {"text": 5}, "4": {"text": "ok\u{0}"}})));
        let n = Notes::of_doc(&doc);
        assert_eq!(n.notes.len(), 1);
        assert_eq!(n.notes[&4].text, "ok");
        assert_eq!(n.next_id(), 5);
    }
}
