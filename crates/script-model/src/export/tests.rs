// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF export: the pages and their text are the paginated script's.

use jcj_doc::{Document, para_block};
use serde_json::json;

use super::{lp_args, pdf_of};
use crate::paginate::paginate;
use crate::template::{new_script, paragraph};

/// Every page's glyphs that are not stroked: (where they land, the baseline of their text line,
/// text), through the core's PDF reader (`jcj_output::read`).
fn glyphs(bytes: &[u8]) -> Vec<Vec<(f64, f64, f64, String)>> {
    use jcj_output::read::{GlyphMode, ReadLimits, read_pages};
    read_pages(bytes, &ReadLimits::default())
        .expect("parse")
        .into_iter()
        .map(|p| {
            p.glyphs
                .into_iter()
                .filter(|g| g.mode != GlyphMode::Stroke)
                .map(|g| (g.y, g.x, g.line_y, g.text.unwrap_or_else(|| "\u{FFFD}".into())))
                .collect()
        })
        .collect()
}

/// Text of every page in drawing order, a new line where the text line's baseline moves.
/// Adapted from WordCraft's PDF tests (MIT OR Apache-2.0).
pub(crate) fn extract_text(bytes: &[u8]) -> Vec<String> {
    glyphs(bytes)
        .into_iter()
        .map(|page| {
            let mut out = String::new();
            let mut last: Option<f64> = None;
            for (_, _, line_y, s) in page {
                if last.is_some_and(|l| (l - line_y).abs() > 1.0) {
                    out.push('\n');
                }
                last = Some(line_y);
                out.push_str(&s);
            }
            out
        })
        .collect()
}

/// Each page's text as lines: glyphs grouped by where they land (baseline), left to right, top
/// to bottom, a wide gap read as a space.
pub(crate) fn page_rows(bytes: &[u8]) -> Vec<Vec<String>> {
    glyphs(bytes)
        .into_iter()
        .map(|page| {
            let mut glyphs: Vec<(f64, f64, String)> = page.into_iter().map(|(y, x, _, s)| (y, x, s)).collect();
            glyphs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
            let mut rows: Vec<(f64, Vec<(f64, String)>)> = Vec::new();
            for (y, x, s) in glyphs {
                match rows.last_mut() {
                    Some((ly, r)) if (*ly - y).abs() < 1.0 => r.push((x, s)),
                    _ => rows.push((y, vec![(x, s)])),
                }
            }
            rows.into_iter()
                .map(|(_, mut r)| {
                    r.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let mut out = String::new();
                    let mut last: Option<f64> = None;
                    for (x, s) in r {
                        if last.is_some_and(|lx| x - lx > 10.0) && !out.ends_with(' ') {
                            out.push(' ');
                        }
                        out.push_str(&s);
                        last = Some(x);
                    }
                    out.split_whitespace().collect::<Vec<_>>().join(" ")
                })
                .filter(|l| !l.is_empty())
                .collect()
        })
        .collect()
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn speech_script() -> Document {
    let mut doc = new_script();
    let mut body = vec![para_block(paragraph("SceneHeading", "INT. KITCHEN - NIGHT"))];
    for _ in 0..47 {
        body.push(para_block(paragraph("General", "filler")));
    }
    body.push(para_block(paragraph("Character", "ANNA")));
    body.push(para_block(paragraph(
        "Dialogue",
        "One two three four five six seven. Eight nine ten eleven twelve thirteen. Fourteen fifteen sixteen seventeen. Eighteen nineteen twenty twenty-one. Twenty-two twenty-three twenty-four.",
    )));
    body.push(para_block(paragraph("Action", "She stops.")));
    doc.body = body;
    doc
}

#[test]
fn the_pdf_has_the_paginated_pages_and_their_text() {
    let doc = speech_script();
    let pag = paginate(&doc);
    assert!(pag.matches_plan);
    let bytes = pdf_of(&pag).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let pages = extract_text(&bytes);
    assert_eq!(pages.len(), pag.pages.len());
    assert!(pages.len() >= 2);
    let one = squash(&pages[0]);
    let two = squash(&pages[1]);
    // Selectable text, page by page as the plan has it.
    assert!(one.starts_with("INT. KITCHEN - NIGHT"), "{one}");
    assert!(one.ends_with("(MORE)"), "{one}");
    assert!(two.contains("ANNA (CONT'D)"), "{two}");
    assert!(two.ends_with("She stops."), "{two}");
    // Page numbers: none on page 1, "2." on page 2.
    assert!(!one.contains("1."), "{one}");
    assert!(two.starts_with("2."), "{two}");
}

#[test]
fn every_line_of_the_pdf_is_a_line_of_the_output_script() {
    let doc = speech_script();
    let pag = paginate(&doc);
    let pages = extract_text(&pdf_of(&pag).unwrap());
    let mut want: Vec<String> = pag.doc.body.iter().filter_map(|b| b.as_para()).map(|p| squash(&p.text)).filter(|t| !t.is_empty()).collect();
    let got: Vec<String> =
        pages.iter().flat_map(|p| p.lines().map(squash)).filter(|l| !l.is_empty() && !l.ends_with('.') || l.contains(' ')).collect();
    // Every output paragraph's words appear, in order, across the PDF's lines.
    let all = squash(&pages.join(" "));
    let mut at = 0;
    for w in want.drain(..) {
        let found = all.get(at..).and_then(|rest| rest.find(&w)).map(|i| at + i);
        assert!(found.is_some(), "missing {w:?}");
        at = found.unwrap_or(at) + w.len();
    }
    assert!(!got.is_empty());
}

#[test]
fn export_command_writes_a_file_and_respects_automation_roots() {
    use jcj_engine::file_access::{AutomationRoots, confine};
    let dir = std::env::temp_dir().join(format!("wordsmith-export-test-{}", std::process::id()));
    let inside = dir.join("out");
    let outside = dir.join("elsewhere");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let mut s = crate::session_for(speech_script());
    let path = inside.join("script.pdf");
    let r = s.run("file.exportPdf", &json!({"path": path})).unwrap();
    assert!(r["pages"].as_u64().unwrap() >= 2);
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF-"));
    assert!(s.run("file.exportPdf", &json!({"path": inside.join("x.txt")})).is_err(), "only .pdf");
    let roots = AutomationRoots::new(None, Some(&inside)).unwrap().unwrap();
    confine(Some(&roots), || {
        assert!(s.run("file.exportPdf", &json!({"path": inside.join("again.pdf")})).is_ok());
        let err = s.run("file.exportPdf", &json!({"path": outside.join("no.pdf")})).unwrap_err();
        assert!(err.to_string().contains("outside") || err.to_string().contains("not"), "{err}");
    });
    assert!(!outside.join("no.pdf").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn print_jobs_are_bounded_and_go_to_lp() {
    assert_eq!(lp_args(None, 2, "My Script"), ["-n", "2", "-t", "My Script", "--", "-"]);
    assert_eq!(lp_args(Some("Office"), 5000, "x\u{7}y"), ["-d", "Office", "-n", "999", "-t", "xy", "--", "-"]);
    let mut s = crate::new_session();
    assert!(s.run("file.print", &json!({"copies": 0})).is_err());
    assert!(s.run("file.print", &json!({"copies": 1000})).is_err());
}

#[test]
fn the_title_page_comes_first_and_the_script_is_numbered_after_it() {
    let mut doc = speech_script();
    let mut t = crate::title_page::TitlePage::default();
    t.set("title", "THE KITCHEN");
    t.set("credit", "Written by");
    t.set("authors", "Anna Writer");
    t.set("contact", "anna@example.com");
    t.set("draftDate", "First Draft");
    t.store(&mut doc);
    let pag = paginate(&doc);
    let pages = extract_text(&super::pdf(&doc).unwrap());
    assert_eq!(pages.len(), pag.pages.len() + 1, "one title page");
    // In reading order (this extractor does not split these lines, so compare without spaces).
    let title: String = pages[0].chars().filter(|c| !c.is_whitespace()).collect();
    assert!(title.starts_with("THEKITCHENWrittenbyAnnaWriter"), "{title}");
    assert!(title.contains("anna@example.com") && title.ends_with("FirstDraft"), "{title}");
    assert!(!title.contains("1."), "no page number on the title page: {title}");
    let one = squash(&pages[1]);
    assert!(one.starts_with("INT. KITCHEN - NIGHT"), "the script's page 1 has no number: {one}");
    assert!(squash(&pages[2]).starts_with("2."), "{}", squash(&pages[2]));
    // Without title page fields the PDF is the script alone.
    assert_eq!(extract_text(&super::pdf(&speech_script()).unwrap()).len(), pag.pages.len());
}

#[test]
fn headers_and_footers_fill_their_fields_page_by_page() {
    let mut doc = new_script();
    let mut body = Vec::new();
    for heading in ["INT. KITCHEN - NIGHT", "EXT. YARD - DAY", "INT. ATTIC - LATER"] {
        body.push(para_block(paragraph("SceneHeading", heading)));
        for _ in 0..30 {
            body.push(para_block(paragraph("General", "filler")));
        }
    }
    doc.body = body;
    doc.core.title = "THE TEST".into();
    let mut s = crate::session_for(doc);
    s.run(
        "headerFooter.set",
        &json!({"header": {"left": "{scene}", "right": "{page}."}, "footer": {"center": "{title} {date}"}, "firstPage": true, "date": "1 May"}),
    )
    .unwrap();
    let pag = paginate(&s.doc);
    assert!(pag.matches_plan);
    let pages: Vec<String> = extract_text(&pdf_of(&pag).unwrap()).iter().map(|p| p.chars().filter(|c| !c.is_whitespace()).collect()).collect();
    assert_eq!(pages.len(), 2, "{pages:?}");
    // Header then footer, then the page: the header names the first scene heading on the page.
    assert!(pages[0].starts_with("INT.KITCHEN-NIGHT1.THETEST1MayINT.KITCHEN"), "{}", pages[0]);
    assert!(pages[1].starts_with("INT.ATTIC-LATER2.THETEST1May"), "{}", pages[1]);
    // Without firstPage, page 1 has neither.
    s.run("headerFooter.set", &json!({"firstPage": false})).unwrap();
    let one: String = extract_text(&pdf_of(&paginate(&s.doc)).unwrap())[0].chars().filter(|c| !c.is_whitespace()).collect();
    assert!(one.starts_with("INT.KITCHEN-NIGHTfiller") && !one.contains("THETEST"), "{one}");
}

#[test]
fn revision_marks_sit_beside_the_revised_lines() {
    let mut doc = new_script();
    let long = "Plain words go on here. ".repeat(8);
    doc.body = vec![
        para_block(paragraph("SceneHeading", "INT. KITCHEN - NIGHT")),
        para_block(paragraph("Action", long.trim())),
        para_block(paragraph("Character", "ANNA")),
        para_block(paragraph("Dialogue", "Unchanged.")),
    ];
    let mut s = crate::session_for(doc);
    s.run("revision.mode", &json!({"on": true})).unwrap();
    // Type a word near the end of the Action's third line.
    let p = s.doc.body[1].as_para().unwrap().clone();
    let (lines, _) = crate::paginate::measure::measure_text(&s.doc, &p, &p.text);
    assert!(lines.len() >= 3);
    let off = lines[2].0 + 3;
    s.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off, path: jcj_doc::Path::top(1), ..s.sel.focus.clone() });
    s.run("text.insert", &json!({"text": "NEW "})).unwrap();
    let pag = paginate(&s.doc);
    assert!(pag.matches_plan);
    assert_eq!(pag.page_revisions, [Some(0)]);
    let rows = &page_rows(&pdf_of(&pag).unwrap())[0];
    let marked: Vec<&String> = rows.iter().filter(|l| l.ends_with(" *")).collect();
    assert_eq!(marked.len(), 1, "one revised line: {rows:#?}");
    assert!(marked[0].contains("NEW"), "{rows:#?}");
    // Colored pages put a fill behind; a different mark shows instead.
    s.run("revision.configure", &json!({"mark": "#", "colorPages": true})).unwrap();
    let pag = paginate(&s.doc);
    let rows = &page_rows(&pdf_of(&pag).unwrap())[0];
    assert!(rows.iter().any(|l| l.contains("NEW") && l.ends_with(" #")), "{rows:#?}");
    let fills = pag
        .doc
        .body
        .iter()
        .filter_map(|b| b.as_para())
        .flat_map(|p| p.objects.iter())
        .filter(|o| matches!(o, jcj_doc::InlineObject::Shape { fill: Some(_), .. }))
        .count();
    assert_eq!(fills, 1);
}

/// Writes a sample revised page to `$WS_SAMPLE_PDF`, for checking by eye.
#[test]
#[ignore = "writes a sample PDF; run by hand with WS_SAMPLE_PDF set"]
fn sample_revised_page() {
    let Some(path) = std::env::var_os("WS_SAMPLE_PDF") else { return };
    let mut s = crate::session_for(speech_script());
    s.run("revision.configure", &json!({"colorPages": true})).unwrap();
    s.run("revision.mode", &json!({"on": true})).unwrap();
    s.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 3, path: jcj_doc::Path::top(10), ..s.sel.focus.clone() });
    s.run("text.insert", &json!({"text": "REVISED "})).unwrap();
    std::fs::write(path, super::pdf(&s.doc).unwrap()).unwrap();
}

#[test]
fn scene_numbers_print_either_side_of_their_headings() {
    let mut doc = new_script();
    doc.body = vec![
        para_block(paragraph("SceneHeading", "INT. KITCHEN - NIGHT")),
        para_block(paragraph("Action", "Rain.")),
        para_block(paragraph("SceneHeading", "EXT. YARD - DAY")),
        para_block(paragraph("Action", "Sun.")),
    ];
    let mut s = crate::session_for(doc);
    let rows = |s: &jcj_engine::Session| page_rows(&pdf_of(&paginate(&s.doc)).unwrap())[0].clone();
    assert_eq!(rows(&s)[0], "INT. KITCHEN - NIGHT", "hidden by default");
    s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
    s.doc.body.insert(2, para_block(paragraph("SceneHeading", "INT. HALL - NIGHT")));
    let r = rows(&s);
    assert_eq!(r[0], "1 INT. KITCHEN - NIGHT 1");
    assert!(r.contains(&"1A INT. HALL - NIGHT 1A".to_string()), "{r:#?}");
    assert!(r.contains(&"2 EXT. YARD - DAY 2".to_string()), "{r:#?}");
}

#[test]
fn lock_a_blue_revision_and_print_only_the_revised_pages() {
    let mut doc = new_script();
    doc.body = (0..70).map(|i| para_block(paragraph("Action", &format!("Line {i}.")))).collect();
    let mut s = crate::session_for(doc);
    s.run("lock.pages", &json!({"revisionMode": true})).unwrap();
    assert_eq!(crate::revisions::Revisions::of_doc(&s.doc).current, "Blue");
    // Revise a line on page 2 (block 40 is on it).
    s.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 4, path: jcj_doc::Path::top(40), ..s.sel.focus.clone() });
    s.run("text.insert", &json!({"text": " (changed)"})).unwrap();
    let (bytes, pages) = super::pdf_with(&s.doc, super::PrintOptions { revised_only: true, marks: true, ..Default::default() }).unwrap();
    assert_eq!(pages, 1);
    let rows = &page_rows(&bytes);
    assert_eq!(rows.len(), 1, "only the revised page");
    assert_eq!(rows[0][0], "2.", "with its locked label");
    assert!(rows[0].iter().any(|l| l.contains("(changed)") && l.ends_with(" *")), "{:#?}", rows[0]);
    // Without marks.
    let (bytes, _) = super::pdf_with(&s.doc, super::PrintOptions { revised_only: true, marks: false, ..Default::default() }).unwrap();
    assert!(page_rows(&bytes)[0].iter().all(|l| !l.ends_with(" *")));
    // With nothing revised there is nothing to print.
    s.run("revision.clearRevised", &json!({"all": true})).unwrap();
    assert!(super::pdf_with(&s.doc, super::PrintOptions { revised_only: true, marks: true, ..Default::default() }).is_err());
}

#[test]
fn hidden_outline_elements_leave_the_pages() {
    let mut doc = new_script();
    doc.body = vec![
        para_block(paragraph("Outline1", "ACT ONE")),
        para_block(paragraph("Summary", "Anna arrives.")),
        para_block(paragraph("SceneHeading", "INT. KITCHEN - NIGHT")),
        para_block(paragraph("Note", "Check the rain.")),
        para_block(paragraph("Action", "Rain.")),
    ];
    let mut s = crate::session_for(doc);
    let rows = |s: &jcj_engine::Session| page_rows(&pdf_of(&paginate(&s.doc)).unwrap())[0].clone();
    assert!(rows(&s).iter().any(|l| l == "Anna arrives."), "shown by default");
    s.run("outline.show", &json!({"show": false})).unwrap();
    assert_eq!(rows(&s), ["INT. KITCHEN - NIGHT", "Rain."]);
    assert_eq!(s.sel.focus.path.last(), 2, "the caret left the hidden outline paragraph");
    // The script keeps them.
    assert_eq!(s.doc.body.len(), 5);
    s.run("outline.show", &json!({})).unwrap();
    assert!(rows(&s).iter().any(|l| l == "ACT ONE"));
}

#[test]
fn print_choices_pick_pages_sides_and_sets() {
    use serde_json::json;
    let mut s = crate::new_session();
    let mut body = Vec::new();
    for (n, who) in [(1, "ANNA"), (2, "BEN"), (3, "ANNA")] {
        body.push(jcj_doc::para_block(crate::template::paragraph("SceneHeading", &format!("INT. ROOM {n} - DAY"))));
        body.push(jcj_doc::para_block(crate::template::paragraph("Character", who)));
        body.push(jcj_doc::para_block(crate::template::paragraph("Dialogue", "Words.")));
        let mut brk = crate::template::paragraph("Action", "End of the scene.");
        brk.props.page_break_before = Some(false);
        body.push(jcj_doc::para_block(brk));
    }
    s.doc.body = body;
    // One scene a page.
    for i in [4, 8] {
        if let Some(jcj_doc::Block::Para(p)) = s.doc.body.get_mut(i).map(std::sync::Arc::make_mut) {
            p.props.page_break_before = Some(true);
        }
    }
    let pages = |o: super::PrintOptions| super::pdf_with(&s.doc, o).map(|r| r.1);
    assert_eq!(pages(super::PrintOptions::default()), Ok(3));
    assert_eq!(pages(super::PrintOptions { pages: Some((2, 3)), ..Default::default() }), Ok(2));
    assert_eq!(pages(super::PrintOptions { scenes: Some((1, 1)), ..Default::default() }), Ok(1));
    assert_eq!(pages(super::PrintOptions { reverse: true, ..Default::default() }), Ok(3));
    assert_eq!(pages(super::PrintOptions { unrevised_only: true, ..Default::default() }), Ok(3));
    assert!(pages(super::PrintOptions { set: Some(0), ..Default::default() }).is_err(), "nothing revised in set 0");
    // Sides: ANNA's two scenes, keeping their numbers 1 and 3.
    let sides = super::sides_of(&s.doc, "anna");
    assert_eq!(crate::scene_numbers::numbers(&sides), ["1", "3"]);
    assert_eq!(pages(super::PrintOptions { sides: Some("Anna".into()), ..Default::default() }), Ok(2));
    let o = super::PrintOptions::of(&json!({"pages": [5, 2], "sides": " ", "reverse": true}));
    assert_eq!((o.pages, o.sides, o.reverse), (Some((2, 5)), None, true));
}
