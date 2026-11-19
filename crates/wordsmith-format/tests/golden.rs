// SPDX-License-Identifier: MIT OR Apache-2.0
//! Golden tests: three public-domain photoplays (docs/golden.md), transcribed into Fountain,
//! paginated and exported to PDF. Each page's PDF text must match the reviewed golden file.
//! After a deliberate change, regenerate with `UPDATE_GOLDEN=1 cargo test -p wordsmith-format
//! --test golden`, read the diff, and commit it with the change.

use std::path::{Path, PathBuf};

use wordsmith_format::fountain;
use wordsmith_script::export::pdf_of;
use wordsmith_script::paginate::paginate;

const SCRIPTS: [&str; 3] = ["everybodys-girl", "timid-teddy", "salt-of-vengeance"];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Each page's text, a line per baseline. Adapted from WordCraft's `wordsmith-pdf` tests
/// (MIT OR Apache-2.0).
fn pdf_pages(bytes: &[u8]) -> Vec<Vec<String>> {
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
            // Where the glyph lands on the page: its own placement, then the page transform.
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
            // Group glyphs into lines by baseline, then order each line left to right.
            let mut glyphs = ex.0;
            // PDF y grows upward: the top line has the largest y.
            glyphs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
            let mut lines: Vec<(f64, Vec<(f64, String)>)> = Vec::new();
            for (y, x, s) in glyphs {
                match lines.last_mut() {
                    Some((ly, l)) if (*ly - y).abs() < 1.0 => l.push((x, s)),
                    _ => lines.push((y, vec![(x, s)])),
                }
            }
            lines
                .into_iter()
                .map(|(_, mut l)| {
                    l.sort_by(|a, b| a.0.total_cmp(&b.0));
                    // A gap wider than a character is a space the PDF does not draw.
                    let mut out = String::new();
                    let mut last_x: Option<f64> = None;
                    for (x, s) in l {
                        if last_x.is_some_and(|lx| x - lx > 10.0) && !out.ends_with(' ') {
                            out.push(' ');
                        }
                        out.push_str(&s);
                        last_x = Some(x);
                    }
                    out.split_whitespace().collect::<Vec<_>>().join(" ")
                })
                .filter(|l| !l.is_empty())
                .collect()
        })
        .collect()
}

fn render(name: &str) -> (String, usize) {
    let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
    let script = fountain::read(&text).expect("reads");
    let pag = paginate(&script.doc);
    assert!(pag.matches_plan, "{name}: the layout disagrees with the pagination plan");
    let pages = pdf_pages(&pdf_of(&pag).expect("pdf"));
    assert_eq!(pages.len(), pag.pages.len(), "{name}: PDF pages");
    let mut out = String::new();
    for (n, lines) in pages.iter().enumerate() {
        out.push_str(&format!("=== page {} ===\n", n + 1));
        for l in lines {
            out.push_str(l);
            out.push('\n');
        }
    }
    (out, pages.len())
}

#[test]
fn golden_scripts_paginate_and_print_as_reviewed() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut failures = Vec::new();
    for name in SCRIPTS {
        let (got, pages) = render(name);
        let path = dir().join(format!("{name}.golden.txt"));
        if update {
            std::fs::write(&path, &got).expect("write golden");
            eprintln!("{name}: {pages} pages written to {}", path.display());
            continue;
        }
        let want = std::fs::read_to_string(&path).unwrap_or_default();
        if got != want {
            let first = got.lines().zip(want.lines()).position(|(a, b)| a != b).unwrap_or(0);
            failures.push(format!(
                "{name}: differs from {} at line {}\n  got:  {:?}\n  want: {:?}",
                path.display(),
                first + 1,
                got.lines().nth(first),
                want.lines().nth(first)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn golden_scripts_have_their_scenes() {
    for (name, scenes) in [("everybodys-girl", 156), ("timid-teddy", 52), ("salt-of-vengeance", 40)] {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let script = fountain::read(&text).expect("reads");
        assert_eq!(wordsmith_script::scenes(&script.doc).len(), scenes, "{name}");
        assert_eq!(script.scene_numbers.len(), scenes, "{name}: scene numbers");
    }
}

#[test]
fn golden_scripts_survive_the_native_format() {
    use wordsmith_format::native;
    for name in SCRIPTS {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let doc = fountain::read(&text).expect("reads").doc;
        let saved = native::write(&doc);
        let back = native::read(&saved).expect("native reads");
        let lines = |d: &wordsmith_doc::Document| wordsmith_script::commands::text::body_lines(&wordsmith_script::session_for(d.clone()));
        assert_eq!(lines(&back), lines(&doc), "{name}");
        assert_eq!(native::write(&back), saved, "{name}: stable bytes");
    }
}
