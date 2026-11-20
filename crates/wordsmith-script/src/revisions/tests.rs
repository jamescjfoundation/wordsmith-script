// SPDX-License-Identifier: MIT OR Apache-2.0
use serde_json::json;
use wordsmith_doc::para_block;
use wordsmith_engine::Selection;

use super::*;
use crate::template::{new_script, paragraph};

fn session() -> Session {
    let mut doc = new_script();
    doc.body = [("SceneHeading", "INT. A - DAY"), ("Action", "Rain hits the window."), ("Character", "ANNA"), ("Dialogue", "Hello there.")]
        .iter()
        .map(|(e, t)| para_block(paragraph(e, t)))
        .collect();
    crate::session_for(doc)
}

fn at(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(block), off }
}

/// Each run of a paragraph: (text, revision set).
fn runs(s: &Session, block: usize) -> Vec<(String, Option<u32>)> {
    let p = s.doc.body[block].as_para().unwrap();
    p.run_ranges().map(|(r, c)| (p.text[r].to_string(), c.revised)).collect()
}

#[test]
fn the_cycle_names_and_colors() {
    assert_eq!(cycle_set(1).name, "Blue");
    assert_eq!(cycle_set(9).name, "Tan");
    assert_eq!(cycle_set(10).name, "Second White");
    assert_eq!(cycle_set(11).name, "Second Blue");
    assert_eq!(cycle_set(25).name, "Third Goldenrod");
    assert!(CYCLE.iter().all(|(_, c)| parse_color(c).is_some()));
    let mut r = Revisions::default();
    assert_eq!((r.start_next(), r.current.as_str()), (Some(0), "Blue"));
    assert_eq!((r.start_next(), r.current.as_str()), (Some(1), "Pink"));
}

#[test]
fn typing_and_deleting_in_revision_mode() {
    let mut s = session();
    s.sel = Selection::caret(at(1, 4));
    s.run("text.insert", &json!({"text": "!"})).unwrap();
    assert!(runs(&s, 1).iter().all(|(_, r)| r.is_none()), "out of revision mode nothing is marked");
    s.run("revision.mode", &json!({"on": true})).unwrap();
    assert_eq!(Revisions::of_doc(&s.doc).current, "Blue", "turning it on starts Blue");
    s.sel = Selection::caret(at(1, 0));
    s.run("text.insert", &json!({"text": "Cold "})).unwrap();
    assert_eq!(runs(&s, 1)[0], ("Cold ".to_string(), Some(0)));
    // A deletion marks its paragraph.
    s.sel = Selection::caret(at(3, 5));
    s.run("text.backspace", &json!({})).unwrap();
    assert_eq!(s.doc.body[3].as_para().unwrap().mark.revised, Some(0));
    // The next revision is Pink, and typing takes it.
    s.run("revision.next", &json!({})).unwrap();
    s.sel = Selection::caret(at(3, 0));
    s.run("text.insert", &json!({"text": "Oh. "})).unwrap();
    assert_eq!(runs(&s, 3)[0], ("Oh. ".to_string(), Some(1)));
    // Out of revision mode again, typing next to revised text is not revised.
    s.run("revision.mode", &json!({})).unwrap();
    s.sel = Selection::caret(at(3, 4));
    s.run("text.insert", &json!({"text": "x"})).unwrap();
    assert_eq!(runs(&s, 3)[1].1, None);
}

#[test]
fn mark_clear_and_roll_back() {
    let mut s = session();
    s.sel = Selection { anchor: at(1, 0), focus: at(1, 4) };
    s.run("revision.markRevised", &json!({})).unwrap();
    assert_eq!(runs(&s, 1)[0], ("Rain".to_string(), Some(0)));
    s.run("revision.next", &json!({})).unwrap();
    s.sel = Selection::caret(at(3, 0));
    s.run("revision.markRevised", &json!({})).unwrap();
    assert_eq!(runs(&s, 3), [("Hello there.".to_string(), Some(1))]);
    // Roll back to Blue: Pink's marks go, Blue's stay.
    s.run("revision.rollback", &json!({"to": "Blue"})).unwrap();
    assert_eq!(runs(&s, 3)[0].1, None);
    assert_eq!(runs(&s, 1)[0].1, Some(0));
    assert_eq!(Revisions::of_doc(&s.doc).sets.len(), 1);
    // Clear the paragraph's marks, then all.
    s.sel = Selection::caret(at(1, 2));
    s.run("revision.clearRevised", &json!({})).unwrap();
    assert!(runs(&s, 1).iter().all(|(_, r)| r.is_none()));
    s.run("revision.rollback", &json!({"to": ""})).unwrap();
    assert!(Revisions::of_doc(&s.doc).sets.is_empty());
    assert!(s.run("revision.rollback", &json!({"to": "Mauve"})).is_err());
}

#[test]
fn settings_are_bounded_and_configurable() {
    let mut s = session();
    s.run("revision.next", &json!({})).unwrap();
    s.run(
        "revision.configure",
        &json!({"mark": "#", "markAt": 7.5, "set": {"index": 0, "name": "Blue Draft", "color": "#3366ff", "underline": true}}),
    )
    .unwrap();
    let r = Revisions::of_doc(&s.doc);
    assert_eq!((r.mark.as_str(), r.mark_at, r.current.as_str()), ("#", 7.5, "Blue Draft"));
    assert!(r.sets[0].underline);
    assert!(s.run("revision.configure", &json!({"markAt": 40})).is_err());
    assert!(s.run("revision.configure", &json!({"set": {"index": 0, "color": "blue"}})).is_err());
    let mut doc = new_script();
    doc.app_data
        .insert(REVISIONS_KEY.into(), Arc::new(json!({"sets": [{"name": "X", "color": "nope"}], "current": "Y", "mark": "  ", "markAt": 99})));
    let r = Revisions::of_doc(&doc);
    assert_eq!((r.sets[0].color.as_str(), r.current.as_str(), r.mark.as_str(), r.mark_at), ("#FFFFFF", "", "*", 8.0));
}
