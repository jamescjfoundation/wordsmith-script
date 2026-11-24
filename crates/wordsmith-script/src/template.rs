// SPDX-License-Identifier: MIT OR Apache-2.0
//! The screenplay template: page geometry and one paragraph style per element.
//!
//! Values come from the public format standards in `docs/SPEC.md` ("Page geometry defaults"):
//! US Letter, 12 pt Courier-style type at 10 characters per inch and 6 lines per inch, margins
//! top 1 in, bottom 1 in, left 1.5 in, right 1 in. Element positions are measured, as the spec
//! does, in inches from the left edge of the page. Elements the table does not cover use the
//! defaults in `docs/decisions/0009-element-model.md`.

use std::sync::Arc;

use wordsmith_doc::props::{Align, CharProps, LineSpacing, ParaProps};
use wordsmith_doc::{Document, Paragraph, Style, StyleKind, StyleSheet};

use crate::element::{ElementKind, Elements};

/// Points per inch.
pub const INCH: f32 = 72.0;
/// One line at 6 lines per inch, in points.
pub const LINE: f32 = 12.0;
/// The script font.
pub const SCRIPT_FONT: &str = "Courier Prime";
/// The script font size, points.
pub const SCRIPT_SIZE: f32 = 12.0;

/// US Letter, points.
pub const PAGE_W: f32 = 8.5 * INCH;
pub const PAGE_H: f32 = 11.0 * INCH;
pub const MARGIN_TOP: f32 = INCH;
pub const MARGIN_BOTTOM: f32 = INCH;
pub const MARGIN_LEFT: f32 = 1.5 * INCH;
pub const MARGIN_RIGHT: f32 = INCH;

/// Where an element's text sits, in inches from the left edge of the page, as the spec measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub left: f32,
    pub right: f32,
}

/// The default formatting of a built-in element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Formatting {
    pub place: Placement,
    pub align: Align,
    /// Blank lines before the paragraph.
    pub space_before_lines: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub start_new_page: bool,
    /// Keep with the next paragraph (a Character with its Dialogue, a heading with what follows).
    pub keep_with_next: bool,
    pub next: ElementKind,
}

/// The default formatting of `kind`.
pub fn default_formatting(kind: ElementKind) -> Formatting {
    use ElementKind as K;
    // The spec's text column: 1.5 in to 7.5 in.
    let full = Placement { left: 1.5, right: 7.5 };
    let f = |place, space_before_lines, next| Formatting {
        place,
        align: Align::Left,
        space_before_lines,
        bold: false,
        italic: false,
        underline: false,
        start_new_page: false,
        keep_with_next: false,
        next,
    };
    match kind {
        K::General => f(full, 0.0, K::General),
        K::SceneHeading => Formatting { keep_with_next: true, ..f(full, 1.0, K::Action) },
        K::Action => f(full, 1.0, K::Action),
        K::Character => Formatting { keep_with_next: true, ..f(Placement { left: 3.7, right: 7.5 }, 1.0, K::Dialogue) },
        K::Dialogue => f(Placement { left: 2.5, right: 6.0 }, 0.0, K::Action),
        K::Parenthetical => Formatting { keep_with_next: true, ..f(Placement { left: 3.1, right: 5.6 }, 0.0, K::Dialogue) },
        K::Transition => Formatting { align: Align::Right, ..f(Placement { left: 5.5, right: 7.5 }, 1.0, K::SceneHeading) },
        K::Shot => Formatting { keep_with_next: true, ..f(full, 1.0, K::Action) },
        K::CastList => f(full, 0.0, K::Action),
        K::NewAct => {
            Formatting { align: Align::Center, underline: true, start_new_page: true, keep_with_next: true, ..f(full, 0.0, K::SceneHeading) }
        }
        K::EndOfAct => Formatting { align: Align::Center, underline: true, ..f(full, 2.0, K::NewAct) },
        // SPEC-2 3.1: 1.25 in, two blank lines before, bold and underlined capitals.
        K::Sequence => {
            Formatting { bold: true, underline: true, keep_with_next: true, ..f(Placement { left: 1.25, right: 7.5 }, 2.0, K::SceneHeading) }
        }
        K::Outline1 => Formatting { bold: true, keep_with_next: true, ..f(full, 1.0, K::Outline1) },
        K::Outline2 => Formatting { bold: true, keep_with_next: true, ..f(Placement { left: 1.75, right: 7.5 }, 1.0, K::Outline2) },
        K::Outline3 => f(Placement { left: 2.0, right: 7.5 }, 1.0, K::Outline3),
        K::Note => Formatting { italic: true, ..f(Placement { left: 2.0, right: 7.5 }, 1.0, K::Note) },
        K::Summary => f(full, 1.0, K::Summary),
        K::Custom => f(full, 1.0, K::Action),
    }
}

/// Inches from the page's left edge to a style's left indent, in points.
pub fn indent_left_pt(left_in: f32) -> f32 {
    left_in * INCH - MARGIN_LEFT
}

/// Inches from the page's left edge to a style's right indent, in points.
pub fn indent_right_pt(right_in: f32) -> f32 {
    (PAGE_W - MARGIN_RIGHT) - right_in * INCH
}

/// A paragraph style for element `id`, named `name`, with formatting `f` and Return target
/// `next` (an element id).
pub fn element_style(id: &str, name: &str, f: &Formatting, next: &str) -> Style {
    Style {
        id: id.to_string(),
        name: name.to_string(),
        kind: StyleKind::Paragraph,
        next: Some(next.to_string()),
        para: ParaProps {
            align: Some(f.align),
            indent_left: Some(indent_left_pt(f.place.left)),
            indent_right: Some(indent_right_pt(f.place.right)),
            indent_first: Some(0.0),
            space_before: Some(f.space_before_lines * LINE),
            space_after: Some(0.0),
            line_spacing: Some(LineSpacing::Exactly(LINE)),
            keep_next: Some(f.keep_with_next),
            keep_lines: Some(false),
            page_break_before: Some(f.start_new_page),
            widow_control: Some(true),
            ..ParaProps::default()
        },
        chr: CharProps {
            font: Some(SCRIPT_FONT.to_string()),
            size: Some(SCRIPT_SIZE),
            bold: Some(f.bold),
            italic: Some(f.italic),
            underline: f.underline.then_some(wordsmith_doc::props::Underline::Single),
            ..CharProps::default()
        },
        builtin: true,
        quick: true,
        ..Style::default()
    }
}

/// The screenplay style sheet: one paragraph style per element, Courier Prime 12 pt throughout.
pub fn style_sheet(elements: &Elements) -> StyleSheet {
    let mut sheet = StyleSheet::empty();
    sheet.default_chr = CharProps { font: Some(SCRIPT_FONT.to_string()), size: Some(SCRIPT_SIZE), ..CharProps::default() };
    sheet.default_para = ParaProps { line_spacing: Some(LineSpacing::Exactly(LINE)), space_after: Some(0.0), ..ParaProps::default() };
    for def in elements.all() {
        let kind = if def.kind == ElementKind::Custom { ElementKind::Action } else { def.kind };
        let f = default_formatting(kind);
        let next = elements.id_of(f.next).to_string();
        sheet.styles.push(element_style(&def.id, &def.name, &f, &next));
    }
    sheet
}

/// A paragraph of element `element` holding `text` (as given; casing is the caller's).
pub fn paragraph(element: &str, text: &str) -> Paragraph {
    let mut p = Paragraph::with_text(text, CharProps::default());
    p.props.style = Some(element.to_string());
    p
}

/// A new, empty screenplay: US Letter with the spec's margins, the element registry, the
/// screenplay styles, scene-heading settings and one empty Scene Heading.
pub fn new_script() -> Document {
    let mut doc = Document::new();
    let elements = Elements::defaults();
    doc.styles = Arc::new(style_sheet(&elements));
    elements.store(&mut doc);
    crate::scene_heading::SceneHeadingSettings::default().store(&mut doc);
    let s = &mut doc.last_section;
    s.page_w = PAGE_W;
    s.page_h = PAGE_H;
    s.margin_top = MARGIN_TOP;
    s.margin_bottom = MARGIN_BOTTOM;
    s.margin_left = MARGIN_LEFT;
    s.margin_right = MARGIN_RIGHT;
    doc.body = vec![wordsmith_doc::para_block(paragraph(elements.id_of(ElementKind::SceneHeading), ""))];
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style<'a>(sheet: &'a StyleSheet, id: &str) -> &'a Style {
        sheet.get(id).unwrap_or_else(|| panic!("style {id}"))
    }

    #[test]
    fn geometry_matches_the_spec_defaults() {
        let sheet = style_sheet(&Elements::defaults());
        let left_in = |id: &str| (style(&sheet, id).para.indent_left.unwrap_or(0.0) + MARGIN_LEFT) / INCH;
        let right_in = |id: &str| (PAGE_W - MARGIN_RIGHT - style(&sheet, id).para.indent_right.unwrap_or(0.0)) / INCH;
        for id in ["SceneHeading", "Action", "Shot"] {
            assert!((left_in(id) - 1.5).abs() < 1e-4 && (right_in(id) - 7.5).abs() < 1e-4, "{id}");
        }
        assert!((3.7 - 1e-4..=3.8 + 1e-4).contains(&left_in("Character")));
        assert!((left_in("Dialogue") - 2.5).abs() < 1e-4);
        assert!((right_in("Dialogue") - left_in("Dialogue") - 3.5).abs() < 1e-4, "dialogue is about 3.5 in wide");
        assert!((3.0 - 1e-4..=3.1 + 1e-4).contains(&left_in("Parenthetical")));
        assert!((right_in("Parenthetical") - left_in("Parenthetical") - 2.5).abs() < 1e-4);
        assert_eq!(style(&sheet, "Transition").para.align, Some(Align::Right));
        assert!((right_in("Transition") - 7.5).abs() < 1e-4);
        assert_eq!(style(&sheet, "Dialogue").para.space_before, Some(0.0));
        assert_eq!(style(&sheet, "Parenthetical").para.space_before, Some(0.0));
        assert!((1.0 * LINE..=2.0 * LINE).contains(&style(&sheet, "SceneHeading").para.space_before.unwrap_or(0.0)));
    }

    #[test]
    fn return_targets_match_the_spec() {
        let sheet = style_sheet(&Elements::defaults());
        let next = |id: &str| style(&sheet, id).next.clone().unwrap_or_default();
        for (id, want) in [
            ("General", "General"),
            ("SceneHeading", "Action"),
            ("Action", "Action"),
            ("Character", "Dialogue"),
            ("Dialogue", "Action"),
            ("Parenthetical", "Dialogue"),
            ("Transition", "SceneHeading"),
            ("Shot", "Action"),
            ("CastList", "Action"),
            ("NewAct", "SceneHeading"),
            ("EndOfAct", "NewAct"),
            ("Summary", "Summary"),
        ] {
            assert_eq!(next(id), want, "Return after {id}");
        }
    }

    #[test]
    fn every_element_uses_the_script_font() {
        let sheet = style_sheet(&Elements::defaults());
        for s in &sheet.styles {
            assert_eq!(s.chr.font.as_deref(), Some(SCRIPT_FONT), "{}", s.id);
            assert_eq!(s.chr.size, Some(SCRIPT_SIZE), "{}", s.id);
            assert_eq!(s.para.line_spacing, Some(LineSpacing::Exactly(LINE)), "{}", s.id);
        }
    }

    #[test]
    fn a_new_script_is_letter_with_spec_margins_and_starts_on_a_scene_heading() {
        let doc = new_script();
        let s = &doc.last_section;
        assert_eq!((s.page_w, s.page_h), (612.0, 792.0));
        assert_eq!((s.margin_top, s.margin_bottom, s.margin_left, s.margin_right), (72.0, 72.0, 108.0, 72.0));
        let first = doc.body.first().and_then(|b| b.as_para()).unwrap();
        assert_eq!(first.props.style.as_deref(), Some("SceneHeading"));
        assert!(doc.app_data.contains_key(crate::element::ELEMENTS_KEY));
    }
}
