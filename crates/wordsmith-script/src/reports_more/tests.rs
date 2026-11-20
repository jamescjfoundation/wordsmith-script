// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_json::json;
use wordsmith_doc::para_block;
use wordsmith_engine::Session;

use super::*;
use crate::commands::new_session;
use crate::template::paragraph;

fn session() -> Session {
    let mut s = new_session();
    let long = "word ".repeat(120);
    s.doc.body = [
        ("SceneHeading", "INT. HALL - DAY"),
        ("Character", "ANNA"),
        ("Dialogue", "Hi there, damn it."),
        ("Character", "BEN"),
        ("Dialogue", long.trim()),
        ("SceneHeading", "EXT. YARD - NIGHT"),
        ("Action", "Wind."),
        ("Character", "ANNA"),
        ("Dialogue", "Cold."),
        ("SceneHeading", "INT. HALL - NIGHT"),
        ("Action", "Dark."),
        ("SceneHeading", "ROOFTOP"),
    ]
    .iter()
    .map(|(e, t)| para_block(paragraph(e, t)))
    .collect();
    s
}

#[test]
fn headings_split_into_parts() {
    let s = session();
    assert_eq!(heading_parts(&s.doc, "INT. HALL - DAY"), ("INT".into(), "HALL".into(), "DAY".into()));
    assert_eq!(heading_parts(&s.doc, "I/E. CAR - MOVING - LATER"), ("I/E".into(), "CAR - MOVING".into(), "LATER".into()));
    assert_eq!(heading_parts(&s.doc, "ROOFTOP"), (String::new(), "ROOFTOP".into(), String::new()));
}

#[test]
fn scene_and_location_reports() {
    let mut s = session();
    let r = s.run("report.run", &json!({"kind": "scenes", "sort": "longest"})).unwrap();
    assert_eq!(r["sections"][0]["rows"][0][0], "1", "the long speech makes scene 1 the longest");
    assert!(r["lines"].as_array().is_some_and(|l| l.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("Last page")))));
    let r = location_report(&s.doc, "most");
    assert_eq!(r.sections[0].rows[0][0], "HALL");
    assert_eq!(r.sections[0].rows[0][1], "2");
    assert_eq!(r.sections[0].rows[0][3], "DAY, NIGHT");
    assert!(r.lines.iter().any(|l| l.contains("ROOFTOP") && l.contains("intro") && l.contains("time of day")));
    let prod = scene_report(&s.doc, "production");
    let order: Vec<&str> = prod.sections[0].rows.iter().map(|r| r[0].as_str()).collect();
    assert_eq!(order, ["1", "3", "4", "2"]);
}

#[test]
fn cast_and_character_reports() {
    let mut s = session();
    s.run("scene.addCharacter", &json!({"scene": 2, "name": "BEN"})).unwrap();
    let r = cast_report(&s.doc, "script");
    let anna = &r.sections[0].rows[0];
    assert_eq!(anna, &["ANNA", "2", "5", "2", "0", "2"].map(String::from).to_vec());
    let ben = &r.sections[0].rows[1];
    assert_eq!((ben[4].as_str(), ben[5].as_str()), ("1", "2"));
    assert!(r.lines[0].starts_with("Most dialogue: BEN"));
    let c = character_report(&s.doc, "ben", true);
    assert_eq!(c.sections[0].rows.len(), 2);
    assert_eq!(c.sections[1].rows[0], vec!["ANNA".to_string(), "1".to_string()]);
    assert_eq!(c.sections[2].rows.len(), 1, "one monologue");
    assert_eq!(c.sections[3].rows.len(), 1);
}

#[test]
fn filtered_notes_and_statistics() {
    let mut s = session();
    let f = s.run("report.run", &json!({"kind": "filtered", "elements": ["SceneHeading"]})).unwrap();
    assert_eq!(f["sections"][0]["rows"].as_array().map(Vec::len), Some(4));
    assert_eq!(filtered_script(&s.doc, &["Action".into()]).body.len(), 2);
    s.sel = wordsmith_engine::Selection::caret(crate::data::body_pos(6, 0));
    s.run("note.add", &json!({"text": "Louder", "type": "Story", "now": 1_700_000_000u64})).unwrap();
    let n = notes_report(&s.doc, "script", &[]);
    assert_eq!(n.sections[0].rows[0][2], "Louder");
    assert_eq!(n.sections[0].rows[0][6], "2023-11-14");
    assert!(notes_report(&s.doc, "script", &["Research".into()]).sections[0].rows.is_empty());
    let st = statistics_report(&s.doc, &[]);
    let lang = st.sections.iter().find(|x| x.heading == "Flagged language").cloned().unwrap_or_default();
    assert_eq!(lang.rows, vec![vec!["damn".to_string(), "1".into(), "1".into(), "ANNA".into()]]);
    let scenes = st.sections.iter().find(|x| x.heading == "Scenes").cloned().unwrap_or_default();
    assert_eq!(scenes.rows[0][3], "3", "scene 3 is also in the hall");
    s.run("report.setFlaggedWords", &json!({"words": ["Wind"]})).unwrap();
    assert_eq!(flagged_words(&s.doc), vec!["wind".to_string()]);
}

#[test]
fn text_and_csv_and_bad_kinds() {
    let mut s = session();
    let t = s.run("report.text", &json!({"kind": "cast"})).unwrap();
    assert!(t.as_str().is_some_and(|t| t.starts_with("Cast Report\n===========\n") && t.contains("ANNA")));
    let c = s.run("report.csv", &json!({"kind": "locations"})).unwrap();
    assert!(c.as_str().is_some_and(|c| c.starts_with("Location,Scenes,Intro,Time of day,Scene numbers\n") && c.contains("\"DAY, NIGHT\"")));
    assert!(s.run("report.run", &json!({"kind": "tags"})).is_err());
    assert_eq!(date(0), "");
    assert_eq!(date(951_782_400), "2000-02-29");
}
