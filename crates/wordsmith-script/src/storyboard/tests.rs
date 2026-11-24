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
    assert_eq!(b.links, [Link { from: 1, to: 2, ..Link::default() }]);
    assert_eq!(b.grid, 5.0);
}

#[test]
fn boards_hold_their_own_beats() {
    let mut s = crate::commands::new_session();
    let second = s.run("storyboard.addBoard", &json!({"name": "Act Two"})).unwrap()["id"].as_u64().unwrap();
    let a = s.run("beat.add", &json!({"title": "One"})).unwrap()["id"].clone();
    let b = s.run("beat.add", &json!({"title": "Two", "board": second})).unwrap()["id"].clone();
    s.run("beat.link", &json!({"from": a, "to": b, "style": "both", "color": "#123456"})).unwrap();
    let board = Storyboard::of_doc(&s.doc);
    assert_eq!(board.boards.len(), 2);
    assert_eq!(board.beats.iter().map(|x| x.board).collect::<Vec<_>>(), [0, second as u32]);
    assert_eq!((board.links[0].style.as_str(), board.links[0].color.as_str()), ("both", "#123456"));
    s.run("beat.styleLink", &json!({"from": a, "to": b, "style": "line"})).unwrap();
    assert!(s.run("beat.styleLink", &json!({"from": a, "to": b, "style": "zigzag"})).is_err());
    s.run("storyboard.renameBoard", &json!({"id": second, "name": "Middle"})).unwrap();
    s.run("storyboard.deleteBoard", &json!({"id": second})).unwrap();
    let board = Storyboard::of_doc(&s.doc);
    assert_eq!((board.beats.len(), board.links.len(), board.boards.len()), (1, 0, 1));
    assert!(s.run("storyboard.deleteBoard", &json!({"id": 0})).is_err(), "the last board stays");
    assert!(s.run("beat.add", &json!({"board": 9})).is_err());
}

#[test]
fn beats_line_up_in_a_column_row_cascade_or_on_the_grid() {
    let mut s = crate::commands::new_session();
    s.run("storyboard.configure", &json!({"snap": false})).unwrap();
    let ids: Vec<serde_json::Value> =
        (0..3).map(|i| s.run("beat.add", &json!({"x": 100.0 + i as f32 * 333.0, "y": 50.0 + i as f32 * 7.0})).unwrap()["id"].clone()).collect();
    s.run("beats.arrange", &json!({"ids": ids, "how": "column"})).unwrap();
    let b = Storyboard::of_doc(&s.doc);
    assert!(b.beats.iter().all(|x| x.x == 100.0));
    assert_eq!(b.beats[1].y, 50.0 + BEAT_H + 20.0);
    s.run("beats.arrange", &json!({"ids": ids, "how": "row"})).unwrap();
    let b = Storyboard::of_doc(&s.doc);
    assert!(b.beats.iter().all(|x| x.y == 50.0));
    s.run("beats.arrange", &json!({"ids": ids, "how": "cascade"})).unwrap();
    let b = Storyboard::of_doc(&s.doc);
    assert_eq!((b.beats[2].x - b.beats[0].x, b.beats[2].y - b.beats[0].y), (60.0, 60.0));
    s.run("beat.update", &json!({"id": ids[0], "x": 13.0})).unwrap();
    s.run("beats.arrange", &json!({"ids": [ids[0].clone()], "how": "grid"})).unwrap();
    assert_eq!(Storyboard::of_doc(&s.doc).beats[0].x, 20.0);
    assert!(s.run("beats.arrange", &json!({"ids": [], "how": "row"})).is_err());
}

#[test]
fn old_boards_read_with_one_board_and_forward_links() {
    let mut doc = Document::new();
    doc.app_data
        .insert(STORYBOARD_KEY.into(), std::sync::Arc::new(json!({"beats": [{"id": 1}, {"id": 2, "board": 7}], "links": [{"from": 1, "to": 2}]})));
    let b = Storyboard::of_doc(&doc);
    assert_eq!(b.boards, vec![BoardInfo::default()]);
    assert_eq!(b.beats[1].board, 0, "a beat on a missing board moves to the first");
    assert_eq!(b.links[0].style, "forward");
}
