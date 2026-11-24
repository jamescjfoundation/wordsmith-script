// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dual dialogue (SPEC-2 3.2, [STD]): two speeches set side by side. The second speech's
//! Character paragraph carries `ParaProps::dual`; the speech before it is the first.
//!
//! For pagination the script is copied with the first speech narrowed to the left half of the
//! text column and the second speech left out of the pages; the second speech is then drawn as
//! a text box in the right half, beside the first speech's name. When the second speech is the
//! longer one, the paragraph after the pair starts lower by the difference. The second speech is
//! edited in the normal and draft views (decision 0035).

use crate::keys::ScriptPara;
use std::collections::HashSet;

use jcj_doc::props::Align;
use jcj_doc::{Document, InlineObject, Paragraph, PartKind, para_block};

use crate::element::{ElementKind, Elements};
use crate::template::{self, INCH, MARGIN_LEFT};

/// Where the two halves start, inches from the page's left edge, and how much narrower an
/// element's own column becomes in them.
const LEFT_START: f32 = 1.5;
const RIGHT_START: f32 = 4.6;
const SCALE: f32 = 0.48;

/// A pair: the first speech's blocks and the second's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub first: std::ops::Range<usize>,
    pub second: std::ops::Range<usize>,
}

/// The end of the speech that starts with the Character paragraph at `start`.
fn speech_end(kinds: &[ElementKind], start: usize) -> usize {
    let mut i = start + 1;
    while matches!(kinds.get(i), Some(ElementKind::Dialogue | ElementKind::Parenthetical)) {
        i += 1;
    }
    i
}

/// The dual-dialogue pairs of `doc`: a Character marked `dual` straight after another speech.
pub fn pairs(doc: &Document) -> Vec<Pair> {
    let elements = Elements::of_doc(doc);
    let kinds: Vec<ElementKind> = doc.body.iter().map(|b| b.as_para().map_or(ElementKind::General, |p| elements.of_para(p).kind)).collect();
    let mut out = Vec::new();
    for (i, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        if p.props.dual() != Some(true) || kinds.get(i) != Some(&ElementKind::Character) {
            continue;
        }
        // The speech before: back over Dialogue and Parentheticals to its Character.
        let mut c = i;
        while c > 0 && matches!(kinds.get(c - 1), Some(ElementKind::Dialogue | ElementKind::Parenthetical)) {
            c -= 1;
        }
        if c == i || c == 0 || kinds.get(c - 1) != Some(&ElementKind::Character) {
            continue;
        }
        let first = c - 1..i;
        // A first speech that is itself a second speech is not paired again.
        if out.last().is_some_and(|p: &Pair| p.second.contains(&first.start)) {
            continue;
        }
        out.push(Pair { first, second: i..speech_end(&kinds, i) });
    }
    out
}

/// Why the paragraph at `block` cannot start a second speech, if it cannot.
pub fn why_not(doc: &Document, block: usize) -> Option<&'static str> {
    let elements = Elements::of_doc(doc);
    let kind = |i: usize| doc.body.get(i).and_then(|b| b.as_para()).map(|p| elements.of_para(p).kind);
    let mut c = block;
    while matches!(kind(c), Some(ElementKind::Dialogue | ElementKind::Parenthetical)) && c > 0 {
        c -= 1;
    }
    if kind(c) != Some(ElementKind::Character) {
        return Some("Put the caret in the second of two speeches: dual dialogue sets a speech beside the one just before it.");
    }
    if c == 0 || !matches!(kind(c - 1), Some(ElementKind::Dialogue | ElementKind::Parenthetical)) {
        return Some(
            "There is no speech straight before this one to set it beside. Dual dialogue needs two speeches in a row, each a name with its dialogue.",
        );
    }
    None
}

/// The Character paragraph of the speech at `block` (the second speech of a pair).
pub fn speech_start(doc: &Document, block: usize) -> usize {
    let elements = Elements::of_doc(doc);
    let mut c = block;
    while c > 0
        && doc
            .body
            .get(c)
            .and_then(|b| b.as_para())
            .is_some_and(|p| matches!(elements.of_para(p).kind, ElementKind::Dialogue | ElementKind::Parenthetical))
    {
        c -= 1;
    }
    c
}

/// A copy of `p` moved into a half of the text column starting at `start` inches.
fn narrowed(doc: &Document, p: &Paragraph, start: f32) -> Paragraph {
    let rp = doc.styles.resolve_para(&p.props);
    let left_in = (rp.indent_left + MARGIN_LEFT) / INCH;
    let right_in = (doc.last_section.page_w - doc.last_section.margin_right - rp.indent_right) / INCH;
    let new_left = start + (left_in - LEFT_START).max(0.0) * SCALE;
    let new_right = (start + (right_in - LEFT_START).max(0.5) * SCALE).max(new_left + 0.5);
    let mut q = p.clone();
    q.props.indent_left = Some(template::indent_left_pt(new_left));
    q.props.indent_right = Some(template::indent_right_pt(new_right));
    q.props.set_dual(None);
    // Transitions and centred names keep their alignment; the rest read from the left.
    if rp.align == Align::Right {
        q.props.align = Some(Align::Right);
    }
    q.touch();
    q
}

/// The script as paginated with its dual dialogue: the copy, the blocks left out of the pages,
/// and for each pair the first speech's Character block with the second speech's paragraphs.
pub struct Prepared {
    pub doc: Document,
    pub dropped: HashSet<usize>,
    pub boxes: Vec<(usize, Vec<Paragraph>)>,
}

/// `doc` made ready to paginate, if it has dual dialogue.
pub fn prepare(doc: &Document) -> Option<Prepared> {
    let pairs = pairs(doc);
    if pairs.is_empty() {
        return None;
    }
    let mut out = doc.clone();
    let mut dropped = HashSet::new();
    let mut boxes = Vec::new();
    for pair in pairs {
        let mut left_h = 0.0;
        for i in pair.first.clone() {
            let Some(p) = doc.body.get(i).and_then(|b| b.as_para()) else { continue };
            let mut q = narrowed(doc, p, LEFT_START);
            // The first speech stays together, beside the second.
            q.props.keep_next = Some(i + 1 < pair.first.end);
            q.props.keep_lines = Some(true);
            left_h += lines_height(doc, &q);
            if let Some(b) = out.body.get_mut(i) {
                *b = para_block(q);
            }
        }
        let mut right = Vec::new();
        let mut right_h = 0.0;
        for i in pair.second.clone() {
            let Some(p) = doc.body.get(i).and_then(|b| b.as_para()) else { continue };
            let mut q = narrowed(doc, p, RIGHT_START);
            q.props.space_before = Some(0.0);
            q.props.page_break_before = Some(false);
            right_h += lines_height(doc, &q);
            right.push(q);
            dropped.insert(i);
        }
        // A longer second speech pushes what follows down.
        if right_h > left_h
            && let Some(p) = out.body.get(pair.second.end).and_then(|b| b.as_para()).cloned()
        {
            let mut q = p;
            let before = doc.styles.resolve_para(&q.props).space_before;
            q.props.space_before = Some(before + (right_h - left_h));
            q.touch();
            if let Some(b) = out.body.get_mut(pair.second.end) {
                *b = para_block(q);
            }
        }
        boxes.push((pair.first.start, right));
    }
    Some(Prepared { doc: out, dropped, boxes })
}

fn lines_height(doc: &Document, p: &Paragraph) -> f32 {
    let (_, heights) = super::measure::measure_text(doc, p, &p.text);
    heights.iter().sum()
}

/// The second speech as a text box beside the first speech's name, `y` below the top of the
/// name's paragraph (its space before).
pub fn speech_box(out: &mut Document, paras: &[Paragraph], y: f32) -> InlineObject {
    let blocks = paras.iter().map(|p| para_block(p.clone())).collect();
    let story = out.add_part(PartKind::TextBox, blocks);
    if let Some(part) = out.parts.get_mut(&story) {
        part.body.inset = Some([0; 4]);
    }
    let lines: f32 = paras.iter().map(|p| lines_height(out, p)).sum();
    let float = jcj_doc::para::Float {
        wrap: jcj_doc::para::Wrap::InFrontOfText,
        h_rel: jcj_doc::para::Anchor::Page,
        v_rel: jcj_doc::para::Anchor::Paragraph,
        // Over the text column, so the paragraphs' indents (from the margin) hold.
        x: out.last_section.margin_left,
        y,
        ..Default::default()
    };
    let s = &out.last_section;
    InlineObject::Shape {
        kind: jcj_doc::para::ShapeKind::Rectangle,
        w: (s.page_w - s.margin_left - s.margin_right).max(72.0),
        h: lines.max(template::LINE),
        fill: None,
        stroke: None,
        stroke_width: 0.0,
        float,
        story: Some(story),
        freeform: None,
        effects: Default::default(),
        extra: Default::default(),
    }
}
