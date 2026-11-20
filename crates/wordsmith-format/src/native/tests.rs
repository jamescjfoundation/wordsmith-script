// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_json::json;
use wordsmith_doc::props::{CharProps, Underline};
use wordsmith_doc::{Document, Paragraph, para_block};
use wordsmith_script::template::{new_script, paragraph};
use wordsmith_script::{Elements, commands::text::body_lines};

use super::{ReadError, VERSION, read, write};

fn lines(doc: &Document) -> Vec<(String, String)> {
    body_lines(&wordsmith_script::session_for(doc.clone()))
}

/// A script with something of everything the format keeps.
fn rich() -> Document {
    let mut s = wordsmith_script::new_session();
    s.run("text.insert", &json!({"text": "int. kitchen - night\nRain *hits* the_window \\ @home.\n"})).unwrap();
    s.run("text.tab", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "anna\n"})).unwrap();
    s.run("element.parenthetical", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "beat"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "Line one"})).unwrap();
    s.run("element.configure", &json!({"element": "Dialogue", "spaceBefore": 1.0, "bold": true})).unwrap();
    s.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+L"})).unwrap();
    s.run("autocomplete.add", &json!({"list": "locations", "term": "ROOFTOP"})).unwrap();
    s.run("moresContinueds.configure", &json!({"more": "(MORE TO COME)"})).unwrap();
    let mut doc = s.doc.clone();
    doc.core.title = "The Test".into();
    // Emphasis, a manual line break with an empty line, and lines that look like markers.
    let mut p = Paragraph::with_text("plain ", CharProps::default()).styled("Action");
    let _ = p.insert_text(p.len(), "bold", &CharProps { bold: Some(true), ..CharProps::default() });
    let _ = p.insert_text(p.len(), " both", &CharProps { bold: Some(true), italic: Some(true), ..CharProps::default() });
    let _ = p.insert_text(p.len(), " under", &CharProps { underline: Some(Underline::Single), ..CharProps::default() });
    let _ = p.insert_text(p.len(), "\n\n@not an element\n=== script ===\n\\", &CharProps::default());
    doc.body.push(para_block(p));
    let mut pb = paragraph("NewAct", "ACT TWO");
    pb.props.page_break_before = Some(true);
    doc.body.push(para_block(pb));
    doc.body.push(para_block(paragraph("Action", "")));
    doc
}

/// Text and emphasis of each paragraph, for comparing.
fn content(doc: &Document) -> Vec<(String, Vec<(String, bool, bool, bool)>, Option<bool>)> {
    let e = Elements::of_doc(doc);
    doc.body
        .iter()
        .filter_map(|b| b.as_para())
        .map(|p| {
            let runs = p
                .run_ranges()
                .filter(|(r, _)| !r.is_empty())
                .map(|(r, c)| {
                    (
                        p.text.get(r).unwrap_or("").to_string(),
                        c.bold == Some(true),
                        c.italic == Some(true),
                        c.underline.is_some_and(|u| u != Underline::None),
                    )
                })
                .collect();
            (e.of_para(p).id.clone(), runs, p.props.page_break_before.filter(|b| *b))
        })
        .collect()
}

#[test]
fn everything_round_trips_exactly() {
    let doc = rich();
    let text = write(&doc);
    let back = read(&text).unwrap();
    assert_eq!(lines(&back), lines(&doc));
    assert_eq!(content(&back), content(&doc));
    assert_eq!(back.app_data, doc.app_data, "every app_data entry");
    assert_eq!(back.styles, doc.styles, "element styles");
    assert_eq!(back.core.title, "The Test");
    assert_eq!(back.last_section.page_w, doc.last_section.page_w);
    // Saving what was read gives the same bytes.
    assert_eq!(write(&back), text);
}

#[test]
fn the_file_reads_as_text() {
    let text = write(&rich());
    assert!(text.starts_with(&format!("Wordsmith Script {VERSION}\n=== settings ===\n{{")));
    assert!(
        text.contains("\n=== script ===\n@SceneHeading\nINT. KITCHEN - NIGHT\n\n@Action\nRain \\*hits\\* the\\_window \\\\ @home.\n\n"),
        "{text}"
    );
    assert!(text.contains("@Action\nplain **bold* both***_ under_\n\\\n\\@not an element\n\\=== script ===\n\\\\\n\n"), "{text}");
    assert!(text.contains("@NewAct page-break\nACT TWO\n"));
    assert!(text.contains("@Action\n\n"), "an empty paragraph");
}

#[test]
fn a_stored_version_1_file_still_reads() {
    let text = include_str!("../../tests/fixtures/v1-short.wss");
    let doc = read(text).unwrap();
    assert_eq!(
        lines(&doc),
        vec![
            ("SceneHeading".into(), "EXT. YARD - DAY".into()),
            ("Action".into(), "Wind.".into()),
            ("Character".into(), "BOB".into()),
            ("Dialogue".into(), "Hi.".into())
        ]
    );
    assert_eq!(doc.core.title, "Fixture");
}

#[test]
fn bad_files_are_refused_cleanly() {
    assert_eq!(read("hello"), Err(ReadError::NotNative));
    assert_eq!(read("Wordsmith Script 99\n=== settings ===\n{}\n=== script ===\n"), Err(ReadError::NewerVersion(99)));
    assert!(matches!(read("Wordsmith Script 1\n"), Err(ReadError::Damaged(_))));
    assert!(matches!(read("Wordsmith Script 1\n=== settings ===\n{oops\n=== script ===\n"), Err(ReadError::Damaged(_))));
    assert!(matches!(read("Wordsmith Script 1\n=== settings ===\n{}\n=== script ===\nno element line\n"), Err(ReadError::Damaged(_))));
    assert!(matches!(read(&"x".repeat(super::MAX_BYTES + 1)), Err(ReadError::TooLarge)));
    // A nested-JSON bomb is refused, not overflowed.
    let bomb = format!("Wordsmith Script 1\n=== settings ===\n{}\n=== script ===\n", "[".repeat(100_000));
    assert!(matches!(read(&bomb), Err(ReadError::Damaged(_))));
}

#[test]
fn missing_settings_are_filled_and_unknown_elements_read_as_general() {
    let doc = read("Wordsmith Script 1\n=== settings ===\n{}\n=== script ===\n@Mystery\nWho?\n\n@Dialogue\nHi.\n").unwrap();
    assert_eq!(lines(&doc), vec![("General".into(), "Who?".into()), ("Dialogue".into(), "Hi.".into())]);
    assert!(doc.styles.get("Dialogue").is_some(), "styles come from the template");
    let empty = read("Wordsmith Script 1\n=== settings ===\n{}\n=== script ===").unwrap();
    assert_eq!(empty.body.len(), 1);
    let _ = new_script();
}

#[test]
fn notes_and_their_anchors_round_trip_and_version_1_braces_stay_braces() {
    let mut s = wordsmith_script::new_session();
    s.run("text.insert", &json!({"text": "INT. A {B} - DAY"})).unwrap();
    s.sel = wordsmith_engine::Selection { anchor: wordsmith_doc::Pos { off: 5, ..s.sel.focus.clone() }, focus: s.sel.focus.clone() };
    s.run("note.add", &json!({"text": "Is {B} right?"})).unwrap();
    let text = write(&s.doc);
    assert!(text.contains("INT. {n1}A \\{B} - DAY{/n}"), "{text}");
    let back = read(&text).unwrap();
    assert_eq!(back.body, s.doc.body);
    assert_eq!(wordsmith_script::notes::Notes::of_doc(&back), wordsmith_script::notes::Notes::of_doc(&s.doc));
    assert_eq!(write(&back), text, "stable");
    // A version 1 file with braces reads them as text.
    let v1 = text.replacen("Wordsmith Script 2", "Wordsmith Script 1", 1).replace("{n1}", "").replace("{/n}", "").replace("\\{", "{");
    let old = read(&v1).unwrap();
    assert_eq!(old.body[0].as_para().map(|p| p.text.as_str()), Some("INT. A {B} - DAY"));
    assert!(old.body[0].as_para().unwrap().run_ranges().all(|(_, c)| c.note.is_none()));
}
