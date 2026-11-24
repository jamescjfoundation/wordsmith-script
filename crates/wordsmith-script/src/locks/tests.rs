// SPDX-License-Identifier: MIT OR Apache-2.0
use serde_json::json;
use wordsmith_doc::para_block;
use wordsmith_engine::Selection;

use super::*;
use crate::paginate::paginate;
use crate::template::{new_script, paragraph};

/// A three-page script of numbered one-line Action paragraphs.
fn session() -> Session {
    let mut doc = new_script();
    doc.body = (0..70).map(|i| para_block(paragraph("Action", &format!("Line {i}.")))).collect();
    crate::session_for(doc)
}

fn at(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(block), off }
}

fn labels(s: &Session) -> Vec<String> {
    paginate(&s.doc).labels.unwrap_or_default()
}

/// The text each page starts with.
fn firsts(s: &Session) -> Vec<String> {
    let pag = paginate(&s.doc);
    pag.pages
        .iter()
        .map(|pg| match pg.iter().find(|p| matches!(p, Piece::Para { .. } | Piece::Blank)) {
            Some(Piece::Para { block, start, .. }) => s.doc.body[*block].as_para().map_or(String::new(), |p| p.text[*start..].to_string()),
            _ => "(placeholder)".into(),
        })
        .collect()
}

#[test]
fn letters_follow_the_scheme() {
    assert_eq!(letters(1), "A");
    assert_eq!(letters(26), "Z");
    assert_eq!(letters(27), "ZA");
    assert_eq!(combined(&["10", "11"]), "10-11");
    assert_eq!(combined(&["7B", "8", "9"]), "7B-9");
}

#[test]
fn locked_pages_keep_their_numbers_and_overflow_takes_letters() {
    let mut s = session();
    let before = firsts(&s);
    assert_eq!(before.len(), 3);
    assert!(paginate(&s.doc).labels.is_none(), "not locked");
    s.run("lock.pages", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "2", "3"]);
    // Twenty more lines on page 1: they flow onto 1A; pages 2 and 3 start where they did.
    s.sel = Selection::caret(at(5, 7));
    s.run("text.insert", &json!({"text": "\nMore.".repeat(20)})).unwrap();
    assert_eq!(labels(&s), ["1", "1A", "2", "3"]);
    let after = firsts(&s);
    assert_eq!((&after[2], &after[3]), (&before[1], &before[2]));
    // Deleting most of page 1 does not pull page 2's text back.
    s.sel = Selection { anchor: at(1, 0), focus: at(40, 0) };
    s.run("text.backspace", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "2", "3"]);
    assert_eq!(firsts(&s)[1], before[1]);
}

#[test]
fn deleted_pages_leave_a_placeholder_and_typing_at_a_page_start_stays_on_it() {
    let mut s = session();
    s.run("lock.pages", &json!({})).unwrap();
    let start2 = paginate(&s.doc).pages[1].iter().find_map(|p| if let Piece::Para { block, .. } = p { Some(*block) } else { None }).unwrap();
    let start3 = paginate(&s.doc).pages[2].iter().find_map(|p| if let Piece::Para { block, .. } = p { Some(*block) } else { None }).unwrap();
    // Typing at the very start of page 2 stays on page 2.
    s.sel = Selection::caret(at(start2, 0));
    s.run("text.insert", &json!({"text": "Now "})).unwrap();
    assert!(firsts(&s)[1].starts_with("Now Line"), "{:?}", firsts(&s));
    // All of page 2's text deleted: a placeholder page keeps its number.
    s.sel = Selection { anchor: at(start2, 0), focus: at(start3, 0) };
    s.run("text.backspace", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "2", "3"]);
    assert_eq!(firsts(&s)[1], "(placeholder)");
    // Unlock all: numbered 1, 2 again, with no placeholder.
    s.run("lock.unlockAll", &json!({})).unwrap();
    assert!(paginate(&s.doc).labels.is_none());
    assert_eq!(paginate(&s.doc).pages.len(), 2);
}

#[test]
fn a_second_lock_level_and_unlocking_it() {
    let mut s = session();
    s.run("lock.pages", &json!({})).unwrap();
    s.sel = Selection::caret(at(5, 7));
    s.run("text.insert", &json!({"text": "\nMore.".repeat(20)})).unwrap();
    assert_eq!(labels(&s), ["1", "1A", "2", "3"]);
    // Lock again: 1A is locked at level 2; more text after it makes 1AA.
    s.run("lock.pages", &json!({"revisionMode": true})).unwrap();
    assert!(crate::revisions::Revisions::of_doc(&s.doc).mode, "locking can turn revision mode on");
    assert_eq!(Locks::of_doc(&s.doc).level(), 2);
    let start = paginate(&s.doc).pages[1].iter().find_map(|p| if let Piece::Para { block, .. } = p { Some(*block) } else { None }).unwrap();
    s.sel = Selection::caret(at(start + 1, 0));
    s.run("text.insert", &json!({"text": "Extra.\n".repeat(40)})).unwrap();
    // Forty two-line paragraphs fill two pages after 1A.
    assert_eq!(labels(&s), ["1", "1A", "1AA", "1AB", "2", "3"]);
    // Unlock one level: 1A is no longer locked, so the pages after 1 are 1A, 1B, 1C again.
    s.run("lock.unlockLevel", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "1A", "1B", "1C", "2", "3"]);
    assert!(s.run("lock.unlockAll", &json!({})).is_ok());
    assert!(s.run("lock.unlockLevel", &json!({})).is_err(), "nothing locked");
}

#[test]
fn locked_labels_print_in_the_header() {
    let mut s = session();
    s.run("lock.pages", &json!({})).unwrap();
    s.sel = Selection::caret(at(5, 7));
    s.run("text.insert", &json!({"text": "\nMore.".repeat(20)})).unwrap();
    let pag = paginate(&s.doc);
    assert!(pag.matches_plan);
    let rows = crate::export::tests::page_rows(&crate::export::pdf_of(&pag).unwrap());
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0][0], "Line 0.", "no number on page 1");
    assert_eq!((rows[1][0].as_str(), rows[2][0].as_str(), rows[3][0].as_str()), ("1A.", "2.", "3."));
}

#[test]
fn page_tools_move_relabel_add_and_remove_locked_pages() {
    let mut s = session();
    s.run("lock.pages", &json!({})).unwrap();
    let start2 = firsts(&s)[1].clone();
    // On page 2: its start moves down a paragraph, then back.
    let (b2, _) = lock_marks(&s.doc).values().copied().filter(|(b, _)| *b > 0).min().unwrap_or((0, 0));
    s.sel = Selection::caret(at(b2 + 1, 0));
    // Page 1 is full: moving page 2's start up a paragraph takes that line onto page 2.
    s.run("lock.adjustBreak", &json!({"by": -1})).unwrap();
    assert_eq!(firsts(&s)[1], s.doc.body[b2 - 1].as_para().map(|p| p.text.clone()).unwrap_or_default());
    s.run("lock.adjustBreak", &json!({"by": 1})).unwrap();
    assert_eq!(firsts(&s)[1], start2);
    // Moving by nothing, or by something that is not a number, is refused and changes nothing.
    for by in [json!(0), json!("down"), json!(0.5)] {
        assert!(matches!(s.run("lock.adjustBreak", &json!({ "by": by })), Err(wordsmith_engine::CmdError::Params(_))), "{by}");
    }
    assert_eq!(firsts(&s)[1], start2);
    assert_eq!(labels(&s), ["1", "2", "3"]);
    // Its number by hand.
    s.run("lock.setLabel", &json!({"label": "2X"})).unwrap();
    assert_eq!(labels(&s), ["1", "2X", "3"]);
    assert!(s.run("lock.setLabel", &json!({"label": "3"})).is_err(), "numbers stay unique");
    // A new locked page in the middle of page 1.
    s.sel = Selection::caret(at(10, 0));
    assert_eq!(s.run("lock.lockPage", &json!({})).unwrap()["label"], "1A");
    assert_eq!(labels(&s), ["1", "1A", "2X", "3"]);
    // Unlocking it lets its text flow back.
    s.run("lock.unlockPage", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "2X", "3"]);
    s.sel = Selection::caret(at(0, 0));
    assert!(s.run("lock.unlockPage", &json!({})).is_err(), "the first locked page stays");
    // Relock numbers from 1 again.
    s.run("lock.relock", &json!({})).unwrap();
    assert_eq!(labels(&s), ["1", "2", "3"]);
}
