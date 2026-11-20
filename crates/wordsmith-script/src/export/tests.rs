// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF export: the pages and their text are the paginated script's.

use serde_json::json;
use wordsmith_doc::{Document, para_block};

use super::{lp_args, pdf_of};
use crate::paginate::paginate;
use crate::template::{new_script, paragraph};

/// Text of every page, through hayro's interpreter (glyph to Unicode via ToUnicode). Adapted
/// from WordCraft's `wordsmith-pdf` tests (MIT OR Apache-2.0).
pub(crate) fn extract_text(bytes: &[u8]) -> Vec<String> {
    use hayro_interpret::font::Glyph;
    use hayro_interpret::{
        BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings, Paint, PathDrawMode, SoftMask,
        interpret_page,
    };
    use hayro_syntax::Pdf;
    struct Ex(String, Option<f64>);
    impl Device<'_> for Ex {
        fn set_soft_mask(&mut self, _: Option<SoftMask<'_>>) {}
        fn set_blend_mode(&mut self, _: BlendMode) {}
        fn draw_path(&mut self, _: &kurbo::BezPath, _: kurbo::Affine, _: &Paint<'_>, _: &PathDrawMode) {}
        fn push_clip_path(&mut self, _: &ClipPath) {}
        fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
        fn draw_glyph(&mut self, g: &Glyph<'_>, t: kurbo::Affine, _: kurbo::Affine, _: &Paint<'_>, mode: &GlyphDrawMode) {
            if matches!(mode, GlyphDrawMode::Stroke(_)) {
                return;
            }
            // A new baseline starts a new line of text.
            let y = t.translation().y;
            if self.1.is_some_and(|last| (last - y).abs() > 1.0) {
                self.0.push('\n');
            }
            self.1 = Some(y);
            match g.as_unicode() {
                Some(hayro_cmap::BfString::Char(c)) => self.0.push(c),
                Some(hayro_cmap::BfString::String(s)) => self.0.push_str(&s),
                None => self.0.push('\u{FFFD}'),
            }
        }
        fn draw_image(&mut self, _: Image<'_, '_>, _: kurbo::Affine) {}
        fn pop_clip_path(&mut self) {}
        fn pop_transparency_group(&mut self) {}
    }
    let pdf = Pdf::new(bytes.to_vec()).expect("parse");
    let cache = InterpreterCache::new();
    pdf.pages()
        .iter()
        .map(|page| {
            let mut ctx =
                Context::new(kurbo::Affine::IDENTITY, kurbo::Rect::new(0.0, 0.0, 1.0, 1.0), &cache, pdf.xref(), InterpreterSettings::default());
            let mut ex = Ex(String::new(), None);
            interpret_page(page, &mut ctx, &mut ex);
            ex.0
        })
        .collect()
}

/// Each page's text as lines: glyphs grouped by where they land (baseline), left to right, top
/// to bottom, a wide gap read as a space.
pub(crate) fn page_rows(bytes: &[u8]) -> Vec<Vec<String>> {
    use hayro_interpret::font::Glyph;
    use hayro_interpret::{
        BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings, Paint, PathDrawMode, SoftMask,
        interpret_page,
    };
    struct Ex(Vec<(f64, f64, String)>);
    impl Device<'_> for Ex {
        fn set_soft_mask(&mut self, _: Option<SoftMask<'_>>) {}
        fn set_blend_mode(&mut self, _: BlendMode) {}
        fn draw_path(&mut self, _: &kurbo::BezPath, _: kurbo::Affine, _: &Paint<'_>, _: &PathDrawMode) {}
        fn push_clip_path(&mut self, _: &ClipPath) {}
        fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
        fn draw_glyph(&mut self, g: &Glyph<'_>, t: kurbo::Affine, glyph: kurbo::Affine, _: &Paint<'_>, mode: &GlyphDrawMode) {
            if matches!(mode, GlyphDrawMode::Stroke(_)) {
                return;
            }
            let s = match g.as_unicode() {
                Some(hayro_cmap::BfString::Char(c)) => c.to_string(),
                Some(hayro_cmap::BfString::String(s)) => s.to_string(),
                None => "\u{FFFD}".into(),
            };
            let at = (t * glyph).translation();
            self.0.push((at.y, at.x, s));
        }
        fn draw_image(&mut self, _: Image<'_, '_>, _: kurbo::Affine) {}
        fn pop_clip_path(&mut self) {}
        fn pop_transparency_group(&mut self) {}
    }
    let pdf = hayro_syntax::Pdf::new(bytes.to_vec()).expect("parse");
    let cache = InterpreterCache::new();
    pdf.pages()
        .iter()
        .map(|page| {
            let mut ctx =
                Context::new(kurbo::Affine::IDENTITY, kurbo::Rect::new(0.0, 0.0, 1.0, 1.0), &cache, pdf.xref(), InterpreterSettings::default());
            let mut ex = Ex(Vec::new());
            interpret_page(page, &mut ctx, &mut ex);
            let mut glyphs = ex.0;
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
    use wordsmith_engine::file_access::{AutomationRoots, confine};
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
    assert!(title.ends_with("anna@example.comFirstDraft"), "{title}");
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
    s.sel = wordsmith_engine::Selection::caret(wordsmith_doc::Pos { off, path: wordsmith_doc::Path::top(1), ..s.sel.focus.clone() });
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
        .filter(|o| matches!(o, wordsmith_doc::InlineObject::Shape { fill: Some(_), .. }))
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
    s.sel = wordsmith_engine::Selection::caret(wordsmith_doc::Pos { off: 3, path: wordsmith_doc::Path::top(10), ..s.sel.focus.clone() });
    s.run("text.insert", &json!({"text": "REVISED "})).unwrap();
    std::fs::write(path, super::pdf(&s.doc).unwrap()).unwrap();
}
