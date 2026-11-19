// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typing a script the way a writer does, through commands only.

use serde_json::{Value, json};
use wordsmith_engine::{CmdError, Session};

use super::{new_session, registry, text::body_lines};

fn run(s: &mut Session, id: &str) {
    s.run(id, &json!({})).unwrap_or_else(|e| panic!("{id}: {e}"));
}

fn typ(s: &mut Session, text: &str) {
    s.run("text.insert", &json!({"text": text})).unwrap_or_else(|e| panic!("typing {text:?}: {e}"));
}

fn lines(s: &Session) -> Vec<(String, String)> {
    body_lines(s)
}

fn l(element: &str, text: &str) -> (String, String) {
    (element.to_string(), text.to_string())
}

fn caret_off(s: &Session) -> usize {
    s.sel.focus.off
}

#[test]
fn a_scene_typed_from_a_blank_page() {
    let mut s = new_session();
    typ(&mut s, "int");
    run(&mut s, "text.tab");
    typ(&mut s, "kitchen");
    run(&mut s, "text.tab");
    typ(&mut s, "night");
    run(&mut s, "text.newParagraph");
    typ(&mut s, "Rain hammers the window.");
    run(&mut s, "text.newParagraph");
    run(&mut s, "text.tab");
    typ(&mut s, "anna");
    run(&mut s, "text.newParagraph");
    run(&mut s, "text.tab");
    typ(&mut s, "quietly");
    run(&mut s, "text.newParagraph");
    typ(&mut s, "Did you hear that?");
    run(&mut s, "text.newParagraph");
    typ(&mut s, "Silence.");
    run(&mut s, "text.newParagraph");
    run(&mut s, "text.tab");
    run(&mut s, "text.tab");
    typ(&mut s, "cut to:");
    run(&mut s, "text.newParagraph");
    assert_eq!(
        lines(&s),
        vec![
            l("SceneHeading", "INT. KITCHEN - NIGHT"),
            l("Action", "Rain hammers the window."),
            l("Character", "ANNA"),
            l("Parenthetical", "(quietly)"),
            l("Dialogue", "Did you hear that?"),
            l("Action", "Silence."),
            l("Transition", "CUT TO:"),
            l("SceneHeading", ""),
        ]
    );
}

#[test]
fn return_creates_the_next_element_for_every_core_element() {
    for (from, want) in [
        ("element.general", "General"),
        ("element.sceneHeading", "Action"),
        ("element.action", "Action"),
        ("element.character", "Dialogue"),
        ("element.dialogue", "Action"),
        ("element.parenthetical", "Dialogue"),
        ("element.transition", "SceneHeading"),
        ("element.shot", "Action"),
        ("element.castList", "Action"),
        ("element.newAct", "SceneHeading"),
        ("element.endOfAct", "NewAct"),
        ("element.summary", "Summary"),
    ] {
        let mut s = new_session();
        run(&mut s, from);
        typ(&mut s, "x");
        s.run("caret.docEnd", &json!({})).unwrap();
        run(&mut s, "text.newParagraph");
        assert_eq!(lines(&s).last().map(|x| x.0.clone()).as_deref(), Some(want), "Return after {from}");
    }
}

#[test]
fn tab_follows_the_table() {
    for (from, want) in [
        ("element.sceneHeading", "Action"),
        ("element.action", "Character"),
        ("element.character", "Transition"),
        ("element.dialogue", "Parenthetical"),
        ("element.parenthetical", "Dialogue"),
        ("element.transition", "SceneHeading"),
        ("element.outline1", "Outline2"),
        ("element.outline2", "Outline3"),
        ("element.outline3", "Note"),
        ("element.note", "Note"),
        ("element.general", "General"),
        ("element.shot", "Shot"),
    ] {
        let mut s = new_session();
        run(&mut s, from);
        run(&mut s, "text.tab");
        assert_eq!(lines(&s)[0].0, want, "Tab from {from}");
    }
}

#[test]
fn character_tab_switches_only_on_a_blank_line() {
    let mut s = new_session();
    run(&mut s, "element.character");
    typ(&mut s, "bob");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s), vec![l("Character", "BOB")]);
}

#[test]
fn scene_heading_separators() {
    let mut s = new_session();
    typ(&mut s, "ext.");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s)[0].1, "EXT. ");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s)[0].1, "EXT. ", "Tab again after the intro adds nothing");
    typ(&mut s, "yard");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s)[0].1, "EXT. YARD - ");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s)[0].1, "EXT. YARD - ", "the time separator is added once");
    typ(&mut s, "day");
    assert_eq!(lines(&s), vec![l("SceneHeading", "EXT. YARD - DAY")]);
    // Not at the end: nothing. A blank heading: Action.
    run(&mut s, "caret.lineStart");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s), vec![l("SceneHeading", "EXT. YARD - DAY")]);
    let mut s = new_session();
    run(&mut s, "text.tab");
    assert_eq!(lines(&s), vec![l("Action", "")]);
    // A heading that does not start with an intro gets no separator.
    let mut s = new_session();
    typ(&mut s, "somewhere");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s)[0].1, "SOMEWHERE");
}

#[test]
fn uppercase_elements_store_capitals() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "straße and café");
    run(&mut s, "element.sceneHeading");
    assert_eq!(lines(&s), vec![l("SceneHeading", "STRASSE AND CAFÉ")]);
    // Dialogue keeps what was typed.
    run(&mut s, "element.dialogue");
    typ(&mut s, " ok");
    assert_eq!(lines(&s)[0].1, "STRASSE AND CAFÉ ok");
}

#[test]
fn parenthetical_brackets_come_and_go_on_an_empty_line() {
    let mut s = new_session();
    run(&mut s, "element.parenthetical");
    assert_eq!(lines(&s)[0], l("Parenthetical", "()"));
    assert_eq!(caret_off(&s), 1, "the caret is between the brackets");
    run(&mut s, "element.dialogue");
    assert_eq!(lines(&s)[0], l("Dialogue", ""));
}

#[test]
fn return_in_the_middle_splits_and_keeps_the_element() {
    let mut s = new_session();
    run(&mut s, "element.dialogue");
    typ(&mut s, "One. Two.");
    for _ in 0..4 {
        run(&mut s, "caret.left");
    }
    run(&mut s, "text.newParagraph");
    assert_eq!(lines(&s), vec![l("Dialogue", "One. "), l("Dialogue", "Two.")]);
}

#[test]
fn shortcuts_reach_the_core_elements() {
    let r = registry();
    let ids = [
        "element.general",
        "element.sceneHeading",
        "element.action",
        "element.character",
        "element.parenthetical",
        "element.dialogue",
        "element.transition",
        "element.shot",
        "element.castList",
        "element.newAct",
    ];
    for (n, id) in ids.iter().enumerate() {
        assert_eq!(r.by_shortcut(&format!("Mod+{n}")).map(|c| c.id), Some(*id));
    }
    assert_eq!(r.by_shortcut("Enter").map(|c| c.id), Some("text.newParagraph"));
    assert_eq!(r.by_shortcut("Tab").map(|c| c.id), Some("text.tab"));
}

#[test]
fn change_element_applies_to_every_selected_paragraph() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "one\ntwo\nthree");
    run(&mut s, "select.all");
    run(&mut s, "element.shot");
    assert_eq!(lines(&s), vec![l("Shot", "ONE"), l("Shot", "TWO"), l("Shot", "THREE")]);
    assert!(matches!(s.run("element.set", &json!({"element": "Nope"})), Err(CmdError::Params(_))));
    s.run("element.set", &json!({"element": "Action"})).unwrap();
    assert_eq!(lines(&s)[0].0, "Action");
}

#[test]
fn undo_takes_back_each_step_including_element_changes() {
    let mut s = new_session();
    typ(&mut s, "int");
    typ(&mut s, ". house");
    run(&mut s, "text.newParagraph");
    run(&mut s, "text.tab");
    assert_eq!(lines(&s), vec![l("SceneHeading", "INT. HOUSE"), l("Character", "")]);
    run(&mut s, "edit.undo");
    assert_eq!(lines(&s), vec![l("SceneHeading", "INT. HOUSE"), l("Action", "")], "undo Tab");
    run(&mut s, "edit.undo");
    assert_eq!(lines(&s), vec![l("SceneHeading", "INT. HOUSE")], "undo Return");
    run(&mut s, "edit.undo");
    assert_eq!(lines(&s), vec![l("SceneHeading", "")], "typing is one step");
    run(&mut s, "edit.redo");
    run(&mut s, "edit.redo");
    run(&mut s, "edit.redo");
    assert_eq!(lines(&s), vec![l("SceneHeading", "INT. HOUSE"), l("Character", "")]);
}

#[test]
fn backspace_and_delete_join_paragraphs() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "ab\ncd");
    run(&mut s, "caret.lineStart");
    run(&mut s, "text.backspace");
    assert_eq!(lines(&s), vec![l("Action", "abcd")]);
    assert_eq!(caret_off(&s), 2);
    run(&mut s, "text.backspace");
    assert_eq!(lines(&s), vec![l("Action", "acd")]);
    run(&mut s, "text.delete");
    assert_eq!(lines(&s), vec![l("Action", "ad")]);
    run(&mut s, "caret.docStart");
    run(&mut s, "text.backspace");
    assert_eq!(lines(&s), vec![l("Action", "ad")], "Backspace at the start does nothing");
    run(&mut s, "caret.docEnd");
    run(&mut s, "text.delete");
    assert_eq!(lines(&s), vec![l("Action", "ad")]);
}

#[test]
fn caret_moves_and_selections() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "Hello brave world.\nSecond line.");
    run(&mut s, "caret.docStart");
    run(&mut s, "caret.wordRight");
    assert_eq!(caret_off(&s), 6);
    run(&mut s, "select.wordRight");
    assert_eq!(s.selected_text(), "brave ");
    typ(&mut s, "bold ");
    assert_eq!(lines(&s)[0].1, "Hello bold world.");
    run(&mut s, "caret.lineEnd");
    run(&mut s, "caret.right");
    assert_eq!((s.sel.focus.path.last(), caret_off(&s)), (1, 0), "Right at a paragraph's end goes to the next");
    run(&mut s, "caret.left");
    assert_eq!(s.sel.focus.path.last(), 0);
    run(&mut s, "caret.down");
    assert_eq!(s.sel.focus.path.last(), 1, "Down reaches the next line");
    run(&mut s, "caret.up");
    assert_eq!(s.sel.focus.path.last(), 0);
    run(&mut s, "caret.wordLeft");
    run(&mut s, "select.all");
    assert_eq!(s.selected_text(), "Hello bold world.\nSecond line.");
    run(&mut s, "caret.left");
    assert!(s.sel.is_collapsed());
    assert_eq!((s.sel.focus.path.last(), caret_off(&s)), (0, 0), "Left collapses a selection to its start");
}

#[test]
fn up_and_down_keep_their_column() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "0123456789\n01\n0123456789");
    run(&mut s, "caret.docStart");
    for _ in 0..8 {
        run(&mut s, "caret.right");
    }
    run(&mut s, "caret.down");
    assert_eq!(caret_off(&s), 2, "the short line clamps the column");
    run(&mut s, "caret.down");
    assert_eq!(caret_off(&s), 8, "the column comes back on a long line");
}

#[test]
fn hostile_text_is_cleaned_or_refused() {
    let mut s = new_session();
    run(&mut s, "element.action");
    typ(&mut s, "a\u{0}b\u{7}c\td\r\ne");
    assert_eq!(lines(&s), vec![l("Action", "abc d"), l("Action", "e")]);
    let huge = "x".repeat(super::text::MAX_INSERT + 1);
    assert!(matches!(s.run("text.insert", &json!({"text": huge})), Err(CmdError::Params(_))));
    let many = "\n".repeat(super::text::MAX_INSERT_PARAGRAPHS + 1);
    assert!(matches!(s.run("text.insert", &json!({"text": many})), Err(CmdError::Params(_))));
    assert!(matches!(s.run("text.insert", &json!({})), Err(CmdError::Params(_))));
    assert_eq!(lines(&s), vec![l("Action", "abc d"), l("Action", "e")], "refused input leaves no trace");
}

#[test]
fn a_custom_return_target_is_followed() {
    // The SPEC's example: Dialogue's next element changed from Action to Character.
    let mut s = new_session();
    let styles = std::sync::Arc::make_mut(&mut s.doc.styles);
    if let Some(d) = styles.get_mut("Dialogue") {
        d.next = Some("Character".into());
    }
    run(&mut s, "element.dialogue");
    typ(&mut s, "Hi.");
    run(&mut s, "text.newParagraph");
    assert_eq!(lines(&s).last().map(|x| x.0.as_str()), Some("Character"));
}

#[test]
fn inspect_reports_paragraphs_scenes_and_the_current_element() {
    let mut s = new_session();
    typ(&mut s, "int. a - day\nAction here.");
    let v: Value = s.run("document.inspect", &json!({})).unwrap();
    assert_eq!(v["paragraphs"][0]["element"], "SceneHeading");
    assert_eq!(v["paragraphs"][1]["text"], "Action here.");
    assert_eq!(v["scenes"][0]["heading"], "INT. A - DAY");
    assert_eq!(v["element"], "Action");
    run(&mut s, "file.new");
    assert_eq!(lines(&s), vec![l("SceneHeading", "")]);
    assert!(!s.can_undo());
}
