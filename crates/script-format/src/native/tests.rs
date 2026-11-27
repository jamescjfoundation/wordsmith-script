// SPDX-License-Identifier: MIT OR Apache-2.0

use jcj_doc::props::{CharProps, Underline};
use jcj_doc::{Document, Paragraph, para_block};
use serde_json::json;
use wordsmith_script_model::keys::ScriptPara;
use wordsmith_script_model::keys::ScriptRun;
use wordsmith_script_model::template::{new_script, paragraph};
use wordsmith_script_model::{Elements, commands::text::body_lines};

use super::{ReadError, VERSION, read, write};

fn lines(doc: &Document) -> Vec<(String, String)> {
    body_lines(&wordsmith_script_model::session_for(doc.clone()))
}

/// A script with something of everything the format keeps.
fn rich() -> Document {
    let mut s = wordsmith_script_model::new_session();
    s.run("text.insert", &json!({"text": "int. kitchen - night\nRain *hits* the_window \\ @home.\n"})).unwrap();
    s.run("text.tab", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "anna\n"})).unwrap();
    s.run("element.parenthetical", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "beat"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "Line one"})).unwrap();
    s.run("element.configure", &json!({"element": "Dialogue", "spaceBefore": 1.0, "bold": true})).unwrap();
    s.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+Y"})).unwrap();
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
    let text = write(&doc).unwrap();
    let back = read(&text).unwrap();
    assert_eq!(lines(&back), lines(&doc));
    assert_eq!(content(&back), content(&doc));
    assert_eq!(back.ext, doc.ext, "every extension data entry");
    assert_eq!(back.styles, doc.styles, "element styles");
    assert_eq!(back.core.title, "The Test");
    assert_eq!(back.last_section.page_w, doc.last_section.page_w);
    // Saving what was read gives the same bytes.
    assert_eq!(write(&back).unwrap(), text);
}

#[test]
fn the_file_reads_as_text() {
    let text = write(&rich()).unwrap();
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
    let text = include_str!("../../../../tests/fixtures/v1-short.wss");
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
    let mut s = wordsmith_script_model::new_session();
    s.run("text.insert", &json!({"text": "INT. A {B} - DAY"})).unwrap();
    s.sel = jcj_engine::Selection { anchor: jcj_doc::Pos { off: 5, ..s.sel.focus.clone() }, focus: s.sel.focus.clone() };
    s.run("note.add", &json!({"text": "Is {B} right?"})).unwrap();
    let text = write(&s.doc).unwrap();
    assert!(text.contains("INT. {n1}A \\{B} - DAY{/n}"), "{text}");
    let back = read(&text).unwrap();
    assert_eq!(back.body, s.doc.body);
    assert_eq!(wordsmith_script_model::notes::Notes::of_doc(&back), wordsmith_script_model::notes::Notes::of_doc(&s.doc));
    assert_eq!(write(&back).unwrap(), text, "stable");
    // A version 1 file with braces reads them as text.
    let v1 =
        text.replacen(&format!("Wordsmith Script {VERSION}"), "Wordsmith Script 1", 1).replace("{n1}", "").replace("{/n}", "").replace("\\{", "{");
    let old = read(&v1).unwrap();
    assert_eq!(old.body[0].as_para().map(|p| p.text.as_str()), Some("INT. A {B} - DAY"));
    assert!(old.body[0].as_para().unwrap().run_ranges().all(|(_, c)| c.note().is_none()));
}

#[test]
fn revisions_round_trip_and_version_2_files_read() {
    let mut s = wordsmith_script_model::new_session();
    s.run("element.action", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "Rain hits the window."})).unwrap();
    s.run("revision.mode", &json!({"on": true})).unwrap();
    s.run("text.insert", &json!({"text": " More."})).unwrap();
    s.run("text.backspace", &json!({})).unwrap();
    let text = write(&s.doc).unwrap();
    assert!(text.contains("@Action revised=0\nRain hits the window.{r0} More{/r}\n"), "{text}");
    let back = read(&text).unwrap();
    assert_eq!(back.body, s.doc.body);
    assert_eq!(write(&back).unwrap(), text);
    // A version 2 file (no revision marks) reads as before.
    let v2 = "Wordsmith Script 2\n=== settings ===\n{}\n=== script ===\n@Action\nA {n1}note{/n} and \\{braces}\n";
    let old = read(v2).unwrap();
    assert_eq!(old.body[0].as_para().map(|p| p.text.as_str()), Some("A note and {braces}"));
}

#[test]
fn locked_pages_and_scene_numbers_round_trip() {
    let mut doc = new_script();
    let mut body = vec![para_block(paragraph("SceneHeading", "INT. A - DAY"))];
    body.extend((0..70).map(|i| para_block(paragraph("Action", &format!("Line {i}.")))));
    doc.body = body;
    let mut s = wordsmith_script_model::session_for(doc);
    s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
    s.run("lock.pages", &json!({})).unwrap();
    s.run("scene.omit", &json!({"scene": 0})).unwrap();
    let text = write(&s.doc).unwrap();
    assert!(text.contains("@SceneHeading #1 omitted=1\n"), "{}", &text[text.find("=== script").unwrap_or(0)..][..200.min(text.len())]);
    let back = read(&text).unwrap();
    assert_eq!(back.body, s.doc.body);
    assert_eq!(write(&back).unwrap(), text);
    let pages = |d: &Document| wordsmith_script_model::paginate::paginate(d).labels;
    assert_eq!(pages(&back), pages(&s.doc));
}

#[test]
fn the_story_board_and_sent_beats_round_trip() {
    let mut s = wordsmith_script_model::new_session();
    let id = s.run("beat.add", &json!({"title": "Anna arrives", "body": "Late."})).unwrap()["id"].clone();
    s.run("beat.toScript", &json!({"id": id})).unwrap();
    let text = write(&s.doc).unwrap();
    assert!(text.contains("@Outline1 beat=1\nAnna arrives\n"), "{text}");
    let back = read(&text).unwrap();
    assert_eq!(back.body, s.doc.body);
    assert_eq!(wordsmith_script_model::storyboard::Storyboard::of_doc(&back), wordsmith_script_model::storyboard::Storyboard::of_doc(&s.doc));
}

#[test]
fn more_styles_bookmarks_and_dual_dialogue_round_trip() {
    use jcj_doc::props::{Highlight, Rgb, TextColor, VertAlign};
    let mut doc = new_script();
    let mut p = Paragraph::with_text("a", CharProps::default()).styled("Action");
    let styles = [
        CharProps { underline: Some(Underline::Double), ..CharProps::default() },
        CharProps { underline: Some(Underline::Words), strike: Some(true), ..CharProps::default() },
        CharProps { underline: Some(Underline::Dotted), vert_align: Some(VertAlign::Superscript), ..CharProps::default() },
        CharProps { vert_align: Some(VertAlign::Subscript), small_caps: Some(true), caps: Some(true), ..CharProps::default() },
        CharProps { highlight: Some(Highlight::Yellow), bold: Some(true), ..CharProps::default() },
        CharProps { shading: Some(Rgb(0x12, 0xAB, 0xEF)), color: Some(TextColor::Rgb(Rgb(1, 2, 3))), ..CharProps::default() },
    ];
    for (i, c) in styles.iter().enumerate() {
        let _ = p.insert_text(p.len(), &format!("{i}{{"), c);
    }
    p.props.set_bookmark(Some(3));
    let mut dual = paragraph("Character", "BEN");
    dual.props.set_dual(Some(true));
    doc.body = vec![para_block(p), para_block(dual)];
    let text = write(&doc).unwrap();
    assert!(text.lines().next().is_some_and(|l| l.ends_with(&format!(" {VERSION}"))));
    let back = read(&text).unwrap();
    let bp = back.body[0].as_para().unwrap();
    for (i, c) in styles.iter().enumerate() {
        let off = 1 + i * 2;
        let got = bp.props_at(off + 1);
        assert_eq!(
            (got.underline, got.strike, got.vert_align, got.caps, got.small_caps, got.highlight, got.shading, got.color, got.bold),
            (c.underline, c.strike, c.vert_align, c.caps, c.small_caps, c.highlight, c.shading, c.color, c.bold),
            "style {i} in {text}"
        );
    }
    assert_eq!(bp.text, "a0{1{2{3{4{5{");
    assert_eq!(bp.props.bookmark(), Some(3));
    assert_eq!(back.body[1].as_para().unwrap().props.dual(), Some(true));
    // A version 3 file reads as before.
    let old = text.replacen(&format!(" {VERSION}\n"), " 3\n", 1);
    assert!(read(&old).is_ok());
}

#[test]
fn tracked_changes_round_trip() {
    let mut s = wordsmith_script_model::new_session();
    s.run("text.insert", &json!({"text": "int. hall\nRain falls."})).unwrap();
    s.author = "Ana".into();
    s.run("track.toggle", &json!({"on": true})).unwrap();
    s.run("text.insert", &json!({"text": " Hard."})).unwrap();
    s.sel = jcj_engine::Selection { anchor: wordsmith_script_model::data::body_pos(1, 0), focus: wordsmith_script_model::data::body_pos(1, 4) };
    s.run("text.backspace", &json!({})).unwrap();
    let back = read(&write(&s.doc).unwrap()).unwrap();
    assert_eq!(back.revisions, s.doc.revisions);
    let changes = |d: &Document| wordsmith_script_model::tracking::changes(d);
    assert_eq!(changes(&back), changes(&s.doc));
    assert_eq!(changes(&back).len(), 2);
}

/// Read `text`, save it again and read that: nothing may be lost, and saving again is stable.
fn resaves_without_loss(name: &str, text: &str) -> Document {
    let first = read(text).unwrap_or_else(|e| panic!("{name}: {e}"));
    let saved = write(&first).unwrap();
    assert!(saved.starts_with(&format!("Wordsmith Script {VERSION}\n")), "{name}: saved in the newest version");
    let again = read(&saved).unwrap();
    assert_eq!(again.body, first.body, "{name}: paragraphs");
    assert_eq!(again.ext, first.ext, "{name}: extension data");
    assert_eq!(again.revisions, first.revisions, "{name}: tracked-change records");
    assert_eq!(again.core.title, first.core.title, "{name}: title");
    assert_eq!(again.styles, first.styles, "{name}: styles");
    assert_eq!(write(&again).unwrap(), saved, "{name}: stable bytes");
    first
}

#[test]
fn a_stored_file_of_every_version_reads_and_saves_again_without_loss() {
    let v1 = resaves_without_loss("v1", include_str!("../../../../tests/fixtures/v1-short.wss"));
    assert_eq!(v1.core.title, "Fixture");

    // Version 2: a note anchored in the text, an escaped brace, emphasis, an empty line and a page break.
    let v2 = resaves_without_loss("v2", include_str!("../../../../tests/fixtures/v2-notes.wss"));
    let heading = v2.body[0].as_para().unwrap();
    assert_eq!(heading.text, "INT. HALL {B} - NIGHT");
    assert_eq!(heading.props_at(6).note(), Some(1));
    assert_eq!(wordsmith_script_model::notes::Notes::of_doc(&v2).notes.get(&1).map(|n| n.text.clone()), Some("Is {B} right?".into()));
    assert_eq!(v2.body[1].as_para().unwrap().text, "Rain hits the window.\n\nA second line.");
    assert_eq!(v2.body[1].as_para().unwrap().props_at(6).bold, Some(true));
    assert_eq!(v2.body[2].as_para().unwrap().props.page_break_before, Some(true));

    // Version 3: a locked page start, revised text with a deletion mark, stored scene numbers.
    let v3 = resaves_without_loss("v3", include_str!("../../../../tests/fixtures/v3-revisions.wss"));
    let first = v3.body[0].as_para().unwrap();
    assert_eq!(first.props.scene_number().as_deref(), Some("1"));
    assert_eq!(first.props_at(1).page_lock(), Some(1));
    let action = v3.body[1].as_para().unwrap();
    assert_eq!((action.mark.revised(), action.props_at(7).revised()), (Some(0), Some(0)));
    assert_eq!(wordsmith_script_model::scene_numbers::numbers(&v3), vec!["1".to_string(), "1A".to_string()]);

    // Version 4: more styles, tracked changes and their records, a bookmark and dual dialogue.
    let v4 = resaves_without_loss("v4", include_str!("../../../../tests/fixtures/v4-styles.wss"));
    assert_eq!(v4.revisions.len(), 2);
    assert_eq!(wordsmith_script_model::tracking::changes(&v4).len(), 2);
    let styled = v4.body[2].as_para().unwrap();
    assert_eq!(styled.props.bookmark(), Some(1));
    assert_eq!(styled.props_at(1).strike, Some(true));
    assert_eq!(styled.props_at(8).underline, Some(Underline::Double));
    let marked = styled.text.find("marked").unwrap();
    assert!(styled.props_at(marked + 1).highlight.is_some(), "{:?}", styled.props_at(marked + 1));
    let raised = styled.text.find("raised").unwrap();
    assert_eq!(styled.props_at(raised + 1).vert_align, Some(jcj_doc::props::VertAlign::Superscript));
    assert_eq!(v4.body[5].as_para().unwrap().props.dual(), Some(true));

    // Version 5: the settings' `ext` (a shared feature's key and one of Script's own) and
    // `trackChanges`; a stored scene number, a scene id, a note's anchor, escapes, an empty line
    // inside a paragraph and a page break.
    let v5 = resaves_without_loss("v5", include_str!("../../../../tests/fixtures/v5-ext.wss"));
    assert_eq!(v5.core.title, "Harbour Lights");
    assert!(v5.settings.track_changes, "trackChanges");
    assert_eq!(wordsmith_script_model::notes::Notes::of_doc(&v5).notes.get(&1).map(|n| n.text.clone()), Some("Is the pier lit?".into()));
    let heading = v5.body[0].as_para().unwrap();
    assert_eq!(heading.props.scene_number().as_deref(), Some("1"));
    assert_eq!(heading.props.scene_id(), Some(1));
    let action = v5.body[1].as_para().unwrap();
    assert_eq!(action.text, "Rain hammers the pier. A *star* and a {brace} show.");
    let pier = action.text.find("pier").unwrap();
    assert_eq!(action.props_at(pier + 1).note(), Some(1));
    assert_eq!(v5.body[4].as_para().unwrap().text, "We go now.\n\nBefore the tide.");
    assert_eq!(v5.body[5].as_para().unwrap().props.page_break_before, Some(true));
}

/// Every format version has its fixture, `v<N>-<what>.wss`, declaring that version. Wordsmith
/// Manuscript's reader is tested against the same files (`docs/format.md`, "Other readers"), so a
/// new version without one fails here first.
#[test]
fn every_format_version_has_a_fixture() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("wss") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        let declared: u32 = text.lines().next().and_then(|l| l.strip_prefix("Wordsmith Script ")).and_then(|v| v.parse().ok()).unwrap_or(0);
        assert!(name.starts_with(&format!("v{declared}-")), "{name} declares format {declared}");
        versions.push(declared);
    }
    versions.sort_unstable();
    assert_eq!(versions, (1..=VERSION).collect::<Vec<_>>(), "one fixture for each format 1 to {VERSION}");
}

#[test]
fn attributes_of_a_newer_version_are_kept_and_written_back() {
    let text = format!(
        "Wordsmith Script {VERSION}\n=== settings ===\n{{}}\n=== script ===\n@SceneHeading #4 shot=12 page-break colour=#FF0000\nINT. A - DAY\n\n@Action future\nRain.\n\n"
    );
    let doc = read(&text).unwrap();
    let p = doc.body[0].as_para().unwrap();
    assert_eq!(p.props.extra_attrs(), vec!["shot=12".to_string(), "colour=#FF0000".to_string()]);
    assert_eq!(p.props.scene_number().as_deref(), Some("4"));
    assert_eq!(p.props.page_break_before, Some(true));
    let saved = write(&doc).unwrap();
    assert!(saved.contains("@SceneHeading page-break #4 shot=12 colour=#FF0000\nINT. A - DAY\n"), "{saved}");
    assert!(saved.contains("@Action future\nRain.\n"), "{saved}");
    assert_eq!(read(&saved).unwrap().body, doc.body);
    // Editing the paragraph keeps them; splitting it keeps them on the first part only.
    let mut s = wordsmith_script_model::session_for(doc);
    s.sel = jcj_engine::Selection::caret(wordsmith_script_model::data::body_pos(0, 4));
    s.run("text.insert", &json!({"text": "X"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    assert_eq!(s.doc.body[0].as_para().unwrap().props.extra_attrs().len(), 2);
    assert!(s.doc.body[1].as_para().unwrap().props.extra_attrs().is_empty());
    // A hostile line keeps at most so many, and never a known attribute.
    let many: Vec<String> = (0..100).map(|i| format!("x{i}=1")).collect();
    let text = format!("Wordsmith Script {VERSION}\n=== settings ===\n{{}}\n=== script ===\n@Action {} {}\nA.\n", many.join(" "), "y".repeat(1000));
    let p = read(&text).unwrap().body[0].as_para().unwrap().props.extra_attrs().clone();
    assert_eq!(p.len(), super::MAX_EXTRA_ATTRS);
    assert!(p.iter().all(|a| a.len() <= super::MAX_EXTRA_ATTR_LEN));
}

#[test]
fn too_many_tracked_change_records_are_refused_not_cut() {
    let mut doc = new_script();
    doc.revisions = vec![jcj_doc::Revision::default(); super::MAX_REVISIONS + 1];
    assert_eq!(write(&doc), Err(super::WriteError::TooManyRevisions(super::MAX_REVISIONS + 1)));
    doc.revisions.pop();
    let text = write(&doc).unwrap();
    assert_eq!(read(&text).unwrap().revisions.len(), super::MAX_REVISIONS);
    // A file with more is damaged.
    let more = text.replacen("\"revisions\": [\n", "\"revisions\": [\n    {},\n", 1);
    assert!(matches!(read(&more), Err(ReadError::Damaged(_))));
}

#[test]
fn every_scene_number_the_app_makes_survives_saving_and_opening() {
    for first in ["1", "99", "123456789012", "12-B.7"] {
        let mut doc = new_script();
        doc.body =
            (0..3).flat_map(|i| [para_block(paragraph("SceneHeading", &format!("INT. {i}"))), para_block(paragraph("Action", "A."))]).collect();
        let mut s = wordsmith_script_model::session_for(doc);
        s.run("scene.setNumber", &json!({"scene": 0, "number": first})).unwrap();
        for i in 0..80 {
            s.doc.body.insert(2, para_block(paragraph("SceneHeading", &format!("INT. ADDED {i}"))));
        }
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        let shown = wordsmith_script_model::scene_numbers::numbers(&s.doc);
        let back = read(&write(&s.doc).unwrap()).unwrap();
        assert_eq!(wordsmith_script_model::scene_numbers::numbers(&back), shown, "{first}");
        assert_eq!(back.body, s.doc.body, "{first}: every stored number is read back");
    }
    // A number stored some other way that the file cannot keep is not written, and so does not
    // come back different.
    let mut doc = new_script();
    let mut p = paragraph("SceneHeading", "INT. A");
    p.props.set_scene_number(Some("a number far too long".into()));
    doc.body = vec![para_block(p)];
    let text = write(&doc).unwrap();
    assert!(text.contains("@SceneHeading\n"), "{text}");
    let mut p = paragraph("SceneHeading", "INT. A");
    p.props.set_scene_number(Some(" 12a ".into()));
    doc.body = vec![para_block(p)];
    assert!(write(&doc).unwrap().contains("@SceneHeading #12A\n"));
}
