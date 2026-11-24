// SPDX-License-Identifier: MIT OR Apache-2.0
use wordsmith_doc::para_block;
use wordsmith_script::template::{new_script, paragraph};

use super::*;

fn lines(doc: &Document) -> Vec<(String, String)> {
    let e = Elements::of_doc(doc);
    doc.body.iter().filter_map(|b| b.as_para()).map(|p| (e.of_para(p).id.clone(), p.text.clone())).collect()
}

fn l(e: &str, t: &str) -> (String, String) {
    (e.to_string(), t.to_string())
}

fn script(paras: &[(&str, &str)]) -> Document {
    let mut doc = new_script();
    doc.body = paras.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
    doc
}

#[test]
fn flat_text_is_classified_by_convention() {
    let src = "INT. KITCHEN - NIGHT\n\nRain hits the window,\nhard.\n\nANNA (V.O.)\n(quietly,\nalmost to herself)\nDid you hear that?\nI did.\n\nBOB (CONT'D)\nNo.\n\nCUT TO:\n\nJUST CAPITALS\n\nBOOM!\n";
    let doc = text(src, true).unwrap();
    assert_eq!(
        lines(&doc),
        [
            l("SceneHeading", "INT. KITCHEN - NIGHT"),
            l("Action", "Rain hits the window, hard."),
            l("Character", "ANNA (V.O.)"),
            l("Parenthetical", "(quietly, almost to herself)"),
            l("Dialogue", "Did you hear that? I did."),
            l("Character", "BOB"),
            l("Dialogue", "No."),
            l("Transition", "CUT TO:"),
            l("SceneHeading", "JUST CAPITALS"),
            l("Action", "BOOM!"),
        ]
    );
}

#[test]
fn text_that_is_not_a_script_is_action() {
    let doc = text("INT. A heading?\n\nANNA\nspeaks.\n\nSecond\nparagraph.", false).unwrap();
    assert_eq!(lines(&doc), [l("Action", "INT. A heading?"), l("Action", "ANNA speaks."), l("Action", "Second paragraph.")]);
}

#[test]
fn indented_text_is_classified_by_column() {
    // As a typewriter or another program lays a script out: 10 characters per inch.
    let src = "\
INT. KITCHEN - NIGHT

Rain hits the window. It goes
on and on.

                      ANNA
                (quietly)
          Did you hear that? It sounded
          like a door.

                      BOB
          No.
                                             CUT TO:

EXT. YARD - DAY
";
    let doc = text(src, true).unwrap();
    assert_eq!(
        lines(&doc),
        [
            l("SceneHeading", "INT. KITCHEN - NIGHT"),
            l("Action", "Rain hits the window. It goes on and on."),
            l("Character", "ANNA"),
            l("Parenthetical", "(quietly)"),
            l("Dialogue", "Did you hear that? It sounded like a door."),
            l("Character", "BOB"),
            l("Dialogue", "No."),
            l("Transition", "CUT TO:"),
            l("SceneHeading", "EXT. YARD - DAY"),
        ]
    );
}

#[test]
fn pagination_marks_are_taken_out_and_split_speeches_joined() {
    let src = "                      ANNA
          The first half of what
          she says
                      (MORE)
\u{c}
                                                        2.
                      ANNA (CONT'D)
          and the second half.

                                                (CONTINUED)
";
    let doc = text(src, true).unwrap();
    assert_eq!(lines(&doc), [l("Character", "ANNA"), l("Dialogue", "The first half of what she says and the second half.")]);
    // In text, only a number at the right is the page's.
    let doc = text("                      ANNA\n          Hi.\n\n                      BOB\n          2.\n", true).unwrap();
    assert_eq!(lines(&doc).last(), Some(&l("Dialogue", "2.")));
}

#[test]
fn guessing_whether_text_is_a_script() {
    assert!(looks_like_script("INT. HOUSE - DAY\n\nSomething happens."));
    assert!(looks_like_script("ANNA\nHi.\n\nBOB\nHello.\n\nANNA\nBye."));
    assert!(!looks_like_script("Dear Sam,\n\nThanks for the notes. See you Tuesday.\n\nBest,\nAnna"));
}

#[test]
fn rtf_with_indents_is_placed_and_plain_rtf_is_read_as_text() {
    use craft_rtf::{Block, CharFmt, Para, Run};
    let para = |text: &str, left_in: f32, space_lines: i32| {
        Block::Para(Para {
            runs: vec![Run { text: text.into(), fmt: CharFmt::default() }],
            left_indent: (left_in * 1440.0) as i32,
            space_before: space_lines * 240,
            ..Para::default()
        })
    };
    let placed = craft_rtf::Document {
        blocks: vec![
            para("INT. KITCHEN - NIGHT", 0.0, 1),
            para("Rain.", 0.0, 1),
            para("ANNA", 2.2, 1),
            para("(quietly)", 1.6, 0),
            para("Did you hear that?", 1.0, 0),
            para("BOB", 2.2, 1),
            para("No.", 1.0, 0),
            para("CUT TO:", 4.5, 1),
        ],
        pictures_left_out: 0,
    };
    let doc = rtf(craft_rtf::write(&placed, "Courier Prime").as_bytes(), true).unwrap();
    assert_eq!(
        lines(&doc),
        [
            l("SceneHeading", "INT. KITCHEN - NIGHT"),
            l("Action", "Rain."),
            l("Character", "ANNA"),
            l("Parenthetical", "(quietly)"),
            l("Dialogue", "Did you hear that?"),
            l("Character", "BOB"),
            l("Dialogue", "No."),
            l("Transition", "CUT TO:"),
        ]
    );
    let plain = craft_rtf::Document {
        blocks: vec![para("EXT. YARD - DAY", 0.0, 0), para("", 0.0, 0), para("ANNA", 0.0, 0), para("Hello.", 0.0, 0)],
        pictures_left_out: 0,
    };
    let doc = rtf(craft_rtf::write(&plain, "Courier Prime").as_bytes(), true).unwrap();
    assert_eq!(lines(&doc), [l("SceneHeading", "EXT. YARD - DAY"), l("Character", "ANNA"), l("Dialogue", "Hello.")]);
    assert_eq!(rtf(b"not rtf", true).unwrap_err(), ImportError::NotRtf);
}

#[test]
fn a_pdf_this_app_printed_comes_back_as_the_same_elements() {
    let mut paras: Vec<(&str, &str)> = vec![
        ("SceneHeading", "INT. KITCHEN - NIGHT"),
        ("Action", "Rain hits the window. It goes on for a long time, long enough that the line has to wrap at the margin."),
        ("Character", "ANNA (V.O.)"),
        ("Parenthetical", "(quietly)"),
        ("Dialogue", "Did you hear that? It sounded like the door, or the wind, or something worse than either."),
        ("Character", "BOB"),
        ("Dialogue", "No."),
        ("Transition", "CUT TO:"),
        ("SceneHeading", "EXT. YARD - DAY"),
    ];
    // Enough to run over several pages, with a long speech that pagination splits with (MORE).
    let long = "Words that go on and on. ".repeat(60);
    let long = long.trim_end();
    for _ in 0..12 {
        paras.push(("Action", "Something happens in the yard."));
        paras.push(("Character", "ANNA"));
        paras.push(("Dialogue", long));
    }
    let doc = script(&paras);
    let bytes = wordsmith_script::export::pdf(&doc).unwrap();
    let back = pdf(&bytes).unwrap();
    assert_eq!(lines(&back), lines(&doc));
}

#[test]
fn bad_pdfs_are_refused_cleanly() {
    assert!(matches!(pdf(b"not a pdf"), Err(ImportError::NotPdf(_))));
    assert!(pdf(b"%PDF-1.7\n%%EOF").is_err());
    assert_eq!(pdf(&vec![0; MAX_IMPORT_BYTES + 1]).unwrap_err(), ImportError::TooLarge);
}

#[test]
fn hostile_text_never_panics() {
    for nasty in ["", "\n\n", "\t\t\t", "(", "(MORE)", "\u{c}\u{c}", "ANNA\n(", "                    ", "É\u{301}\n\u{feff}", "INT.\n\n>"] {
        let _ = text(nasty, true);
        let _ = text(nasty, false);
        let _ = looks_like_script(nasty);
    }
    assert_eq!(text(&"x".repeat(MAX_IMPORT_BYTES + 1), true).unwrap_err(), ImportError::TooLarge);
}

#[test]
fn revision_marks_in_a_pdf_are_not_read_as_text() {
    let doc = script(&[("SceneHeading", "INT. KITCHEN - NIGHT"), ("Action", "Rain hits the window."), ("Character", "ANNA"), ("Dialogue", "Hello.")]);
    let mut s = wordsmith_script::session_for(doc);
    s.run("revision.mode", &serde_json::json!({"on": true})).unwrap();
    for block in [1, 3] {
        let len = s.doc.body[block].as_para().map_or(0, |p| p.len());
        s.sel = wordsmith_engine::Selection::caret(wordsmith_doc::Pos { off: len, path: wordsmith_doc::Path::top(block), ..s.sel.focus.clone() });
        s.run("text.insert", &serde_json::json!({"text": " Yes."})).unwrap();
    }
    let bytes = wordsmith_script::export::pdf(&s.doc).unwrap();
    assert_eq!(lines(&pdf(&bytes).unwrap()), lines(&s.doc));
}
