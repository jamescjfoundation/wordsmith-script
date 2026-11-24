// SPDX-License-Identifier: MIT OR Apache-2.0
//! Things the output script shows beside its text: revision marks, scene numbers and page
//! colors (decisions 0024 and 0025).
//!
//! A margin item is a small text box anchored to its paragraph and placed in the margin, with one
//! line for each of the paragraph's lines (a mark, or nothing), so the page view, the PDF and
//! printing show the same thing from the one output script. A paragraph-anchored box is measured
//! from the top of the paragraph's space before, so it is moved down by that space.

use std::sync::Arc;

use jcj_doc::para::{Anchor, Float, ShapeKind, Wrap};
use jcj_doc::props::{Align, CharProps, LineSpacing, ParaProps, Rgb};
use jcj_doc::{Document, InlineObject, Paragraph, PartKind, Style, StyleKind, para_block};

use crate::template;

/// Style of the lines in margin boxes.
pub const MARGIN_STYLE: &str = "ws.Margin";

/// Make sure the margin style exists in `out`.
pub fn add_style(out: &mut Document) {
    let sheet = Arc::make_mut(&mut out.styles);
    if sheet.get(MARGIN_STYLE).is_some() {
        return;
    }
    sheet.upsert(Style {
        id: MARGIN_STYLE.into(),
        name: "Margin".into(),
        kind: StyleKind::Paragraph,
        para: ParaProps {
            align: Some(Align::Left),
            indent_left: Some(0.0),
            indent_right: Some(0.0),
            indent_first: Some(0.0),
            space_before: Some(0.0),
            space_after: Some(0.0),
            line_spacing: Some(LineSpacing::Exactly(template::LINE)),
            ..ParaProps::default()
        },
        chr: CharProps { font: Some(template::SCRIPT_FONT.into()), size: Some(template::SCRIPT_SIZE), ..CharProps::default() },
        builtin: true,
        ..Style::default()
    });
}

/// A text box `w` points wide at `x` points from the page's left edge, `y` points below the top
/// of its paragraph's space before, holding `lines` (an empty line is kept as a blank line).
pub fn margin_box(out: &mut Document, x: f32, y: f32, w: f32, lines: &[String], align: Align) -> InlineObject {
    margin_box_in(out, x, y, w, lines, align, None)
}

/// [`margin_box`] in `font` (the margin style's when `None`).
pub fn margin_box_in(out: &mut Document, x: f32, y: f32, w: f32, lines: &[String], align: Align, font: Option<&str>) -> InlineObject {
    add_style(out);
    let blocks = lines
        .iter()
        .map(|l| {
            // A blank line still takes its line.
            let text = if l.is_empty() { "\u{a0}" } else { l.as_str() };
            let mut p = Paragraph::with_text(text, CharProps { font: font.map(str::to_string), ..CharProps::default() });
            p.props.style = Some(MARGIN_STYLE.into());
            p.props.align = Some(align);
            para_block(p)
        })
        .collect();
    let story = out.add_part(PartKind::TextBox, blocks);
    if let Some(part) = out.parts.get_mut(&story) {
        part.body.inset = Some([0; 4]);
    }
    let float = Float { wrap: Wrap::InFrontOfText, h_rel: Anchor::Page, v_rel: Anchor::Paragraph, x, y, ..Float::default() };
    InlineObject::Shape {
        kind: ShapeKind::Rectangle,
        w,
        h: lines.len().max(1) as f32 * template::LINE,
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

/// A rectangle the size of the page, behind the text, in `color`.
pub fn page_fill(out: &Document, color: Rgb) -> InlineObject {
    let s = &out.last_section;
    let float = Float { wrap: Wrap::BehindText, h_rel: Anchor::Page, v_rel: Anchor::Page, x: 0.0, y: 0.0, ..Float::default() };
    InlineObject::Shape {
        kind: ShapeKind::Rectangle,
        w: s.page_w,
        h: s.page_h,
        fill: Some(jcj_doc::fill::Fill::Solid { color, transparency: 0.0 }),
        stroke: None,
        stroke_width: 0.0,
        float,
        story: None,
        freeform: None,
        effects: Default::default(),
        extra: Default::default(),
    }
}
