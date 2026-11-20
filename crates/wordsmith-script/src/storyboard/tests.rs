// SPDX-License-Identifier: MIT OR Apache-2.0
use serde_json::json;

use super::*;
use crate::commands::text::body_lines;

fn board(s: &Session) -> Storyboard {
    Storyboard::of_doc(&s.doc)
}

#[test]
fn beats_are_added_changed_moved_and_snapped() {
    let mut s = crate::new_session();
    let a = s.run("beat.add", &json!({"title": "Anna arrives", "body": "She is late."})).unwrap()["id"].as_u64().unwrap() as u32;
    let b = s.run("beat.add", &json!({"title": "The fight", "x": 433, "y": 207})).unwrap()["id"].as_u64().unwrap() as u32;
    assert_ne!(a, b);
    let bb = board(&s).beat(b).cloned().unwrap();
    assert_eq!((bb.x, bb.y), (440.0, 200.0), "snapped to the 20-point grid");
    s.run("storyboard.configure", &json!({"snap": false})).unwrap();
    s.run("beat.update", &json!({"id": b, "x": 433, "y": 207, "color": "#b8d4f5", "title": "The big fight"})).unwrap();
    let bb = board(&s).beat(b).cloned().unwrap();
    assert_eq!((bb.x, bb.y, bb.color.as_str(), bb.title.as_str()), (433.0, 207.0, "#B8D4F5", "The big fight"));
    assert!(s.run("beat.update", &json!({"id": b, "color": "red"})).is_err());
    assert!(s.run("beat.update", &json!({"id": 99, "title": "x"})).is_err());
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(board(&s).beat(b).map(|x| x.title.as_str()), Some("The fight"), "undo");
}

#[test]
fn links_stay_with_their_beats_and_go_with_them() {
    let mut s = crate::new_session();
    let ids: Vec<u32> = (0..3).map(|_| s.run("beat.add", &json!({})).unwrap()["id"].as_u64().unwrap() as u32).collect();
    s.run("beat.link", &json!({"from": ids[0], "to": ids[1]})).unwrap();
    s.run("beat.link", &json!({"from": ids[0], "to": ids[1]})).unwrap();
    s.run("beat.link", &json!({"from": ids[1], "to": ids[2]})).unwrap();
    assert_eq!(board(&s).links.len(), 2, "a link once");
    assert!(s.run("beat.link", &json!({"from": ids[0], "to": ids[0]})).is_err());
    // Moving a beat keeps its links (they join beats, not places).
    s.run("beat.update", &json!({"id": ids[1], "x": 1000, "y": 1000})).unwrap();
    assert_eq!(board(&s).links.len(), 2);
    s.run("beat.delete", &json!({"id": ids[1]})).unwrap();
    assert!(board(&s).links.is_empty(), "a deleted beat's links go too");
    s.run("beat.unlink", &json!({"from": ids[0], "to": ids[2]})).unwrap();
}

#[test]
fn a_beat_goes_to_the_script_as_outline_elements() {
    let mut s = crate::new_session();
    s.run("text.insert", &json!({"text": "INT. KITCHEN - NIGHT"})).unwrap();
    let id = s.run("beat.add", &json!({"title": "Anna arrives", "body": "She is late and wet."})).unwrap()["id"].as_u64().unwrap() as u32;
    s.run("beat.toScript", &json!({"id": id})).unwrap();
    let lines = body_lines(&s);
    assert_eq!(lines[0], ("Outline1".to_string(), "Anna arrives".to_string()));
    assert_eq!(lines[1], ("Summary".to_string(), "She is late and wet.".to_string()));
    assert_eq!(lines[2].1, "INT. KITCHEN - NIGHT");
    assert_eq!(sent_paragraph(&s.doc, id), Some(0));
    assert_eq!(s.sel.focus.path.last(), 2, "the caret stays with the script text");
}

#[test]
fn hostile_story_boards_read_safely() {
    let mut doc = crate::template::new_script();
    doc.app_data.insert(
        STORYBOARD_KEY.into(),
        Arc::new(json!({
            "beats": [{"id": 1, "x": "NaN", "color": "nope", "lane": 9}, {"id": 1}, {"id": 2, "x": 1e30, "w": -5}],
            "links": [{"from": 1, "to": 1}, {"from": 1, "to": 3}, {"from": 1, "to": 2}],
            "grid": 0
        })),
    );
    let b = Storyboard::of_doc(&doc);
    assert_eq!(b.beats.len(), 2);
    assert_eq!(b.beats[0].color, PALETTE[0]);
    assert_eq!(b.beats[0].lane, None);
    assert_eq!((b.beats[1].x, b.beats[1].w), (CANVAS, 60.0));
    assert_eq!(b.links, [Link { from: 1, to: 2 }]);
    assert_eq!(b.grid, 5.0);
}
