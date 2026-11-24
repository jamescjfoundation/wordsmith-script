// SPDX-License-Identifier: MIT OR Apache-2.0
//! Checks that need the real layout: the script font's pitch and line spacing.

use std::sync::Arc;

use wordsmith_doc::{Path, Pos, StoryRef, para_block};
use wordsmith_layout::{LayoutCache, LayoutOptions, ViewMode};

use crate::template::{INCH, new_script};

fn layout_of(lines: &[(&str, &str)]) -> (wordsmith_doc::Document, Arc<wordsmith_layout::DocLayout>) {
    let mut doc = new_script();
    doc.body = lines.iter().map(|(style, text)| para_block(crate::template::paragraph(style, text))).collect();
    let opts = LayoutOptions { view: ViewMode::Print, ..Default::default() };
    let l = Arc::new(wordsmith_layout::layout(&doc, &mut LayoutCache::new(), &opts));
    (doc, l)
}

fn pos(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: Path::top(block), off }
}

#[test]
fn identity_is_plain() {
    assert_eq!(crate::PRODUCT_NAME, "Wordsmith Script");
    assert!(!crate::VERSION.is_empty());
}

#[test]
fn courier_prime_sets_ten_characters_per_inch_at_12pt() {
    let (_, l) = layout_of(&[("Action", "0123456789MMMMMMMMMMiiiiiiiiii")]);
    let x = |off| l.caret(&pos(0, off)).map(|c| c.x).unwrap_or(f32::NAN);
    // Every character, wide or narrow, is a tenth of an inch. Courier Prime's advance is
    // 1228/2048 em, so 10 characters at 12 pt measure 71.95 pt: within 0.07% of an inch, and a
    // full 60-character line is 0.28 pt short (docs/decisions/0009-element-model.md).
    let per_char = x(1) - x(0);
    for (a, b) in [(0, 10), (10, 20), (20, 30)] {
        let w = x(b) - x(a);
        assert!((w - INCH).abs() < 0.1, "10 characters from {a} measure {w} pt, not 72");
        assert!((w - 10.0 * per_char).abs() < 0.01, "characters {a}..{b} are not all the same width");
    }
    assert!((x(30) - x(0) - 3.0 * INCH).abs() < 0.2);
    // Action starts at 1.5 in from the page edge.
    assert!((x(0) - 1.5 * INCH).abs() < 0.01, "Action starts at {} pt", x(0));
}

#[test]
fn lines_are_six_per_inch_at_12pt() {
    // One Action paragraph long enough to wrap onto several lines (60 characters a line).
    let text = "word ".repeat(80);
    let (_, l) = layout_of(&[("Action", text.trim_end())]);
    let top = |off| l.caret(&pos(0, off)).map(|c| c.top).unwrap_or(f32::NAN);
    let first = top(0);
    let second = top(60);
    let fifth = top(240);
    assert!((second - first - 12.0).abs() < 0.01, "line pitch {} pt", second - first);
    assert!((fifth - first - 48.0).abs() < 0.01, "four lines measure {} pt", fifth - first);
    // The first line sits below the top margin by Action's one blank line; dropping space
    // before at the top of a page is the paginator's rule (`paginate::plan`), not this layout's.
    assert!((first - (INCH + 12.0)).abs() < 1.0, "first line top {} pt", first);
}

#[test]
fn elements_sit_where_the_template_puts_them() {
    let (_, l) = layout_of(&[("SceneHeading", "INT. ROOM - DAY"), ("Character", "ANNA"), ("Dialogue", "Hi."), ("Parenthetical", "(beat)")]);
    let left = |b| l.caret(&pos(b, 0)).map(|c| c.x / INCH).unwrap_or(f32::NAN);
    assert!((left(0) - 1.5).abs() < 0.01);
    assert!((left(1) - 3.7).abs() < 0.01);
    assert!((left(2) - 2.5).abs() < 0.01);
    assert!((left(3) - 3.1).abs() < 0.01);
}

/// Commands that add to their parameters take any JSON without panicking.
#[test]
fn parameters_that_are_not_objects_are_refused_not_a_panic() {
    let mut s = crate::new_session();
    for id in ["file.exportRevisedPdf", "headerFooter.set", "scene.list", "scene.setHeading", "revision.configure"] {
        for params in [serde_json::json!(5), serde_json::json!("x"), serde_json::json!([1]), serde_json::Value::Null] {
            let _ = s.run(id, &params);
        }
    }
}
