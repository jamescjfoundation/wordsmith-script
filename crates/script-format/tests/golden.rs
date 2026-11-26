// SPDX-License-Identifier: MIT OR Apache-2.0
//! Golden tests: three public-domain photoplays (docs/golden.md), transcribed into Fountain,
//! paginated and exported to PDF. Each page's PDF text must match the reviewed golden file.
//! After a deliberate change, regenerate with `UPDATE_GOLDEN=1 cargo test -p wordsmith-script-format
//! --test golden`, read the diff, and commit it with the change.

use std::path::{Path, PathBuf};

use wordsmith_script_format::fountain;
use wordsmith_script_model::export::pdf_of;
use wordsmith_script_model::paginate::paginate;

const SCRIPTS: [&str; 3] = ["everybodys-girl", "timid-teddy", "salt-of-vengeance"];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

/// Each page's text, a line per baseline, through the core's PDF reader (`jcj_output::read`).
/// The grouping is the one 0.1.x used (adapted from WordCraft's `wordsmith-pdf` tests, MIT OR
/// Apache-2.0): glyphs by where they land, top to bottom, left to right, a gap wider than a
/// character read as a space; stroked text is left out.
fn pdf_pages(bytes: &[u8]) -> Vec<Vec<String>> {
    use jcj_output::read::{GlyphMode, ReadLimits, read_pages};
    let pages = read_pages(bytes, &ReadLimits::default()).expect("parse");
    pages
        .iter()
        .map(|page| {
            let mut glyphs: Vec<(f64, f64, String)> = page
                .glyphs
                .iter()
                .filter(|g| g.mode != GlyphMode::Stroke)
                .map(|g| (g.y, g.x, g.text.clone().unwrap_or_else(|| "\u{FFFD}".into())))
                .collect();
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

type Flat = Vec<(String, String)>;

/// Paragraphs (element, text with line breaks as spaces), for comparing imports.
fn flat(d: &jcj_doc::Document) -> Flat {
    wordsmith_script_model::commands::text::body_lines(&wordsmith_script_model::session_for(d.clone()))
        .into_iter()
        .map(|(e, t)| (e, t.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect()
}

/// An import against the script it came from. Two things a printed or laid-out page cannot show
/// (decision 0019): a heading with no intro that ends in punctuation looks exactly like Action
/// in capitals, and a paragraph split across a page looks like two. Everything else must match.
fn compare(name: &str, want: &Flat, got: &Flat) {
    let (mut ambiguous, mut splits) = (0, 0);
    let (mut i, mut j) = (0, 0);
    while i < want.len() && j < got.len() {
        let (w, g) = (&want[i], &got[j]);
        if w == g {
        } else if w.0 == "SceneHeading" && g.0 == "Action" && w.1 == g.1 && w.1.ends_with(['.', '!', '?', '"', '-', ':', ';', ',']) {
            ambiguous += 1;
        } else if let Some(next) = got.get(j + 1)
            && w.0 == g.0
            && next.0 == g.0
            && w.1 == format!("{} {}", g.1, next.1)
        {
            splits += 1;
            j += 1;
        } else {
            panic!("{name}: paragraph {i}: {w:?} came back as {g:?}");
        }
        i += 1;
        j += 1;
    }
    assert_eq!((i, j), (want.len(), got.len()), "{name}: every paragraph accounted for");
    eprintln!("{name}: {} paragraphs; {ambiguous} punctuated headings read as Action; {splits} split by a page", want.len());
    assert!(ambiguous + splits <= want.len() / 50, "{name}: too many ambiguities");
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
        assert_eq!(wordsmith_script_model::scenes(&script.doc).len(), scenes, "{name}");
        assert_eq!(script.scene_numbers.len(), scenes, "{name}: scene numbers");
    }
}

#[test]
fn golden_scripts_survive_the_native_format() {
    use wordsmith_script_format::native;
    for name in SCRIPTS {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let doc = fountain::read(&text).expect("reads").doc;
        let saved = native::write(&doc).expect("native writes");
        let back = native::read(&saved).expect("native reads");
        let lines = |d: &jcj_doc::Document| wordsmith_script_model::commands::text::body_lines(&wordsmith_script_model::session_for(d.clone()));
        assert_eq!(lines(&back), lines(&doc), "{name}");
        assert_eq!(native::write(&back).expect("native writes"), saved, "{name}: stable bytes");
    }
}

#[test]
fn golden_scripts_survive_a_fountain_round_trip() {
    for name in SCRIPTS {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let doc = fountain::read(&text).expect("reads").doc;
        let written = fountain::write(&doc);
        let back = fountain::read(&written).expect("our Fountain reads");
        let lines = |d: &jcj_doc::Document| wordsmith_script_model::commands::text::body_lines(&wordsmith_script_model::session_for(d.clone()));
        assert_eq!(lines(&back.doc), lines(&doc), "{name}");
        assert_eq!(back.doc.core.title, doc.core.title, "{name}: title");
        assert_eq!(fountain::write(&back.doc), written, "{name}: stable text");
    }
}

#[test]
fn golden_scripts_come_back_from_their_pdfs() {
    // Line breaks inside a paragraph are not visible in a PDF: compare with them as spaces.
    for name in SCRIPTS {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let doc = fountain::read(&text).expect("reads").doc;
        let bytes = wordsmith_script_model::export::pdf(&doc).expect("pdf");
        let back = wordsmith_script_format::import::pdf(&bytes).expect("imports");
        // The title page comes back as the title page, not as script.
        use wordsmith_script_model::title_page::TitlePage;
        let (want_page, got_page) = (TitlePage::of_doc(&doc), TitlePage::of_doc(&back));
        assert!(!want_page.is_empty(), "{name}: the golden script has a title page");
        // A page cannot tell a credit that names the authors ("By …") from the authors, and a
        // long line wraps: compare the credit and authors together, as words.
        let words = |t: &TitlePage| format!("{} {}", t.credit, t.authors).split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(got_page.title, want_page.title, "{name}");
        assert_eq!(words(&got_page), words(&want_page), "{name}");
        let (want, got) = (flat(&doc), flat(&back));
        if let Some(d) = std::env::var_os("DUMP_IMPORT") {
            let dump = |v: &Vec<(String, String)>| v.iter().map(|(e, t)| format!("{e}\t{t}\n")).collect::<String>();
            let d = std::path::PathBuf::from(d);
            let _ = std::fs::write(d.join(format!("{name}.want")), dump(&want));
            let _ = std::fs::write(d.join(format!("{name}.got")), dump(&got));
        }
        compare(&format!("{name} (PDF)"), &want, &got);
    }
}

#[test]
fn golden_scripts_come_back_from_text_and_rtf_exports() {
    use wordsmith_script_format::{export, import};
    for name in SCRIPTS {
        let text = std::fs::read_to_string(dir().join(format!("{name}.fountain"))).expect("golden script");
        let doc = fountain::read(&text).expect("reads").doc;
        let from_text = import::text(&export::text(&doc), true).expect("text imports");
        compare(&format!("{name} (plain text)"), &flat(&doc), &flat(&from_text));
        let from_rtf = import::rtf(export::rtf(&doc).as_bytes(), true).expect("rtf imports");
        compare(&format!("{name} (RTF)"), &flat(&doc), &flat(&from_rtf));
    }
}
