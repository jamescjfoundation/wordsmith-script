// SPDX-License-Identifier: MIT OR Apache-2.0
//! Autocomplete through commands, the way a writer meets it.

use serde_json::{Value, json};
use wordsmith_engine::{CmdError, Session};

use crate::commands::{new_session, text::body_lines};

fn typ(s: &mut Session, t: &str) {
    s.run("text.insert", &json!({"text": t})).unwrap();
}

fn run(s: &mut Session, id: &str) {
    s.run(id, &json!({})).unwrap();
}

fn offered(s: &mut Session) -> Vec<String> {
    let v = s.run("autocomplete.suggest", &json!({})).unwrap();
    v["candidates"].as_array().cloned().unwrap_or_default().into_iter().filter_map(|c| c.as_str().map(str::to_string)).collect()
}

fn list_of(s: &mut Session, v: &Value) -> String {
    v["list"].as_str().unwrap_or("").to_string() + &s.selected_text()
}

/// A scene with ANNA and BOB speaking, ending on an empty Character.
fn exchange() -> Session {
    let mut s = new_session();
    typ(&mut s, "int. kitchen - night\nRain.");
    // Each speech: Return gives Action, Tab makes it a Character, Return gives Dialogue.
    for (who, line) in [("anna", "Hi."), ("bob", "Hello."), ("Anna", "Bye.")] {
        typ(&mut s, "\n");
        run(&mut s, "text.tab");
        typ(&mut s, &format!("{who}\n{line}"));
    }
    typ(&mut s, "\n");
    s
}

#[test]
fn names_typed_are_learned_and_offered() {
    let mut s = exchange();
    let lists = s.run("autocomplete.lists", &json!({})).unwrap();
    let chars = &lists["lists"][0]["entries"];
    assert_eq!(chars, &json!(["ANNA", "BOB"]));
    // The next Character after an Action: "B" offers BOB.
    typ(&mut s, "Silence.\n");
    run(&mut s, "text.tab");
    typ(&mut s, "b");
    assert_eq!(offered(&mut s), ["BOB"]);
    // Typing on narrows, then ignores: nothing is inserted unasked.
    typ(&mut s, "ex");
    assert!(offered(&mut s).is_empty());
    assert_eq!(body_lines(&s).last().map(|l| l.1.as_str()), Some("BEX"));
}

#[test]
fn accept_replaces_only_the_typed_prefix() {
    let mut s = exchange();
    typ(&mut s, "Silence.\n");
    run(&mut s, "text.tab");
    typ(&mut s, "an");
    s.run("autocomplete.accept", &json!({"candidate": "ANNA"})).unwrap();
    assert_eq!(body_lines(&s).last().map(|l| l.1.as_str()), Some("ANNA"));
    assert_eq!(s.sel.focus.off, 4);
    assert!(matches!(s.run("autocomplete.accept", &json!({"candidate": "ZED"})), Err(CmdError::Params(_)) | Err(CmdError::Disabled(_))));
    // Undo takes the acceptance back to what was typed.
    run(&mut s, "edit.undo");
    assert_eq!(body_lines(&s).last().map(|l| l.1.as_str()), Some("AN"));
}

#[test]
fn the_caret_picks_the_list() {
    let mut s = new_session();
    typ(&mut s, "e");
    let v = s.run("autocomplete.suggest", &json!({})).unwrap();
    assert_eq!(list_of(&mut s, &v), "intros");
    assert_eq!(offered(&mut s), ["EXT"]);
    s.run("autocomplete.accept", &json!({"candidate": "EXT"})).unwrap();
    run(&mut s, "text.tab");
    typ(&mut s, "yard");
    run(&mut s, "text.tab");
    typ(&mut s, "ni");
    let v = s.run("autocomplete.suggest", &json!({})).unwrap();
    assert_eq!(v["list"], "times");
    assert_eq!(offered(&mut s), ["NIGHT"]);
    s.run("autocomplete.accept", &json!({"candidate": "NIGHT"})).unwrap();
    assert_eq!(body_lines(&s)[0].1, "EXT. YARD - NIGHT");
    // Leaving the heading learns its location; the next heading offers it.
    typ(&mut s, "\n");
    run(&mut s, "element.sceneHeading");
    typ(&mut s, "int. ya");
    let v = s.run("autocomplete.suggest", &json!({})).unwrap();
    assert_eq!(v["list"], "locations");
    assert_eq!(offered(&mut s), ["YARD"]);
    // Transitions.
    run(&mut s, "element.transition");
    let mut t = new_session();
    run(&mut t, "element.transition");
    typ(&mut t, "dis");
    assert_eq!(offered(&mut t), ["DISSOLVE TO:"]);
}

#[test]
fn tab_after_a_name_starts_an_extension_and_offers_the_list() {
    let mut s = new_session();
    run(&mut s, "element.character");
    typ(&mut s, "anna");
    run(&mut s, "text.tab");
    assert_eq!(body_lines(&s)[0].1, "ANNA (");
    typ(&mut s, "v");
    assert_eq!(offered(&mut s), ["V.O."]);
    s.run("autocomplete.accept", &json!({"candidate": "V.O."})).unwrap();
    assert_eq!(body_lines(&s)[0].1, "ANNA (V.O.)", "the bracket is closed");
    // The option can be switched off: Tab on a name then does nothing.
    let mut s = new_session();
    s.run("autocomplete.configure", &json!({"tabInsertsExtension": false})).unwrap();
    run(&mut s, "element.character");
    typ(&mut s, "anna");
    run(&mut s, "text.tab");
    assert_eq!(body_lines(&s)[0].1, "ANNA");
}

#[test]
fn the_next_speaker_is_guessed_in_a_two_person_exchange() {
    let mut s = exchange();
    // ANNA, BOB, ANNA spoke: next is BOB.
    run(&mut s, "element.character");
    assert_eq!(offered(&mut s), ["BOB"]);
    s.run("autocomplete.configure", &json!({"guessNextSpeaker": false})).unwrap();
    assert!(offered(&mut s).is_empty());
}

#[test]
fn lists_can_be_edited_switched_ordered_and_rebuilt() {
    let mut s = exchange();
    s.run("autocomplete.add", &json!({"list": "characters", "term": "Aaron"})).unwrap();
    typ(&mut s, "x\n");
    run(&mut s, "text.tab");
    typ(&mut s, "a");
    assert_eq!(offered(&mut s), ["Aaron", "ANNA"], "alphabetical");
    s.run("autocomplete.configure", &json!({"list": "characters", "order": "speeches"})).unwrap();
    assert_eq!(offered(&mut s), ["ANNA", "Aaron"], "ANNA has the most speeches");
    s.run("autocomplete.move", &json!({"list": "characters", "term": "Aaron", "to": 0})).unwrap();
    s.run("autocomplete.configure", &json!({"list": "characters", "order": "custom"})).unwrap();
    assert_eq!(offered(&mut s), ["Aaron", "ANNA"]);
    s.run("autocomplete.rename", &json!({"list": "characters", "from": "Aaron", "to": "ABE"})).unwrap();
    assert_eq!(offered(&mut s), ["ABE", "ANNA"]);
    s.run("autocomplete.configure", &json!({"list": "characters", "enabled": false})).unwrap();
    assert!(offered(&mut s).is_empty(), "a switched-off list offers nothing");
    s.run("autocomplete.configure", &json!({"list": "characters", "enabled": true})).unwrap();
    // Rebuild drops names no longer in the script and keeps those that are.
    s.run("autocomplete.rebuild", &json!({})).unwrap();
    assert_eq!(offered(&mut s), ["ANNA"]);
    s.run("autocomplete.remove", &json!({"list": "characters", "term": "anna"})).unwrap();
    assert!(offered(&mut s).is_empty());
    assert!(matches!(s.run("autocomplete.add", &json!({"list": "nope", "term": "x"})), Err(CmdError::Params(_))));
}

#[test]
fn scene_intros_share_the_scene_heading_settings() {
    let mut s = new_session();
    s.run("autocomplete.add", &json!({"list": "intros", "term": "est"})).unwrap();
    let settings = s.run("sceneHeading.settings", &json!({})).unwrap();
    assert!(settings["intros"].as_array().unwrap().contains(&json!("EST")));
    typ(&mut s, "es");
    assert_eq!(offered(&mut s), ["EST"]);
}

#[test]
fn hostile_lists_are_repaired() {
    let mut s = new_session();
    let junk = json!({"lists": {"characters": {"enabled": true, "order": "alphabetical", "entries": ["", "x".repeat(500), "ANNA", "anna", "BOB"]}}, "guessNextSpeaker": "yes"});
    s.doc.app_data.insert(crate::autocomplete::AUTOCOMPLETE_KEY.into(), std::sync::Arc::new(junk));
    let lists = s.run("autocomplete.lists", &json!({})).unwrap();
    assert_eq!(lists["lists"][0]["entries"], json!(["ANNA", "BOB"]));
    s.doc.app_data.insert(crate::autocomplete::AUTOCOMPLETE_KEY.into(), std::sync::Arc::new(json!(17)));
    assert_eq!(s.run("autocomplete.lists", &json!({})).unwrap()["lists"][1]["entries"], json!(["V.O.", "O.S.", "O.C.", "CONT'D"]));
}
