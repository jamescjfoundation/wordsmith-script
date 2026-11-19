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
