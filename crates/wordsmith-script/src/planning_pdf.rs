// SPDX-License-Identifier: MIT OR Apache-2.0
//! The story board and the index cards as PDF pages (SPEC-2 9.1 and 9.2). Each is built as a
//! document of shapes placed on its pages (text boxes for beats and cards, freeform lines for
//! links) and printed with the same layout and PDF writer as the script.

use std::sync::Arc;

use jcj_doc::fill::Fill;
use jcj_doc::freeform::{FreePath, Freeform};
use jcj_doc::para::{Anchor, Float, ShapeKind, Wrap};
use jcj_doc::props::{Align, CharProps, LineSpacing, Rgb};
use jcj_doc::{Document, InlineObject, Paragraph, PartKind, para_block};
use serde_json::Value;

use crate::storyboard::Storyboard;
use crate::template::{INCH, SCRIPT_FONT};

/// One line of text in a box: its text, bold, and size in points.
struct Line {
    text: String,
    bold: bool,
    size: f32,
}

fn rgb(hex: &str) -> Option<Rgb> {
    crate::revisions::parse_color(hex)
}

/// A box at (`x`, `y`) on its page holding `lines`.
fn text_box(out: &mut Document, at: (f32, f32, f32, f32), fill: Option<Rgb>, stroke: Option<Rgb>, lines: &[Line]) -> InlineObject {
    let (x, y, w, h) = at;
    let blocks = lines
        .iter()
        .map(|l| {
            let c = CharProps { font: Some(SCRIPT_FONT.into()), size: Some(l.size), bold: l.bold.then_some(true), ..CharProps::default() };
            let mut p = Paragraph::with_text(if l.text.is_empty() { "\u{a0}" } else { &l.text }, c);
            p.props.align = Some(Align::Left);
            p.props.space_after = Some(l.size * 0.25);
            p.props.line_spacing = Some(LineSpacing::Exactly(l.size * 1.15));
            para_block(p)
        })
        .collect();
    let story = out.add_part(PartKind::TextBox, blocks);
    if let Some(part) = out.parts.get_mut(&story) {
        part.body.inset = Some([45_720; 4]);
    }
    let float = Float { wrap: Wrap::InFrontOfText, h_rel: Anchor::Page, v_rel: Anchor::Page, x, y, ..Float::default() };
    InlineObject::Shape {
        kind: ShapeKind::Rectangle,
        w,
        h,
        fill: fill.map(|color| Fill::Solid { color, transparency: 0.0 }),
        stroke,
        stroke_width: if stroke.is_some() { 0.75 } else { 0.0 },
        float,
        story: Some(story),
        freeform: None,
        effects: Default::default(),
        extra: Default::default(),
    }
}

/// Lines (open paths) or filled shapes (closed paths) drawn across a whole page.
fn page_paths(page: (f32, f32), paths: Vec<FreePath>, color: Rgb, filled: bool) -> InlineObject {
    let (w, h) = page;
    let float = Float { wrap: Wrap::InFrontOfText, h_rel: Anchor::Page, v_rel: Anchor::Page, x: 0.0, y: 0.0, ..Float::default() };
    InlineObject::Shape {
        kind: ShapeKind::Freeform,
        w,
        h,
        fill: filled.then_some(Fill::Solid { color, transparency: 0.0 }),
        stroke: Some(color),
        stroke_width: 1.0,
        float,
        story: None,
        freeform: Some(Arc::new(Freeform { w, h, paths, alpha: 1.0, ink: None, order: None })),
        effects: Default::default(),
        extra: Default::default(),
    }
}

/// A page-sized document of `pages` pages, each holding its objects.
fn paged(page: (f32, f32), margin: f32, pages: Vec<(String, Vec<InlineObject>)>, out: &mut Document) {
    let s = &mut out.last_section;
    s.page_w = page.0;
    s.page_h = page.1;
    s.margin_top = margin;
    s.margin_bottom = margin;
    s.margin_left = margin;
    s.margin_right = margin;
    out.body = pages
        .into_iter()
        .enumerate()
        .map(|(i, (heading, objects))| {
            let c = CharProps { font: Some(SCRIPT_FONT.into()), size: Some(10.0), bold: Some(true), ..CharProps::default() };
            let mut p = Paragraph::with_text(if heading.is_empty() { "\u{a0}" } else { &heading }, c.clone());
            p.props.page_break_before = Some(i > 0);
            for obj in objects {
                let at = p.len();
                let _ = p.insert_object(at, obj, &c);
            }
            para_block(p)
        })
        .collect();
    out.ensure_nonempty();
}

/// Where a line from the centre of `a` towards `b`'s centre leaves `a` (x, y, w, h).
fn edge(a: (f32, f32, f32, f32), to: (f32, f32)) -> (f32, f32) {
    let c = (a.0 + a.2 / 2.0, a.1 + a.3 / 2.0);
    let d = (to.0 - c.0, to.1 - c.1);
    if d.0.abs() < 1e-3 && d.1.abs() < 1e-3 {
        return c;
    }
    let tx = if d.0.abs() > 1e-3 { (a.2 / 2.0) / d.0.abs() } else { f32::INFINITY };
    let ty = if d.1.abs() > 1e-3 { (a.3 / 2.0) / d.1.abs() } else { f32::INFINITY };
    let t = tx.min(ty);
    (c.0 + d.0 * t, c.1 + d.1 * t)
}

/// An arrowhead at `tip` pointing from `from`.
fn arrowhead(from: (f32, f32), tip: (f32, f32), size: f32) -> FreePath {
    let (dx, dy) = (tip.0 - from.0, tip.1 - from.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / len, dy / len);
    let (sx, sy) = (-uy, ux);
    let back = (tip.0 - ux * size, tip.1 - uy * size);
    FreePath {
        pts: vec![[tip.0, tip.1], [back.0 + sx * size * 0.5, back.1 + sy * size * 0.5], [back.0 - sx * size * 0.5, back.1 - sy * size * 0.5]],
        closed: true,
    }
}

/// Board `board` of `doc`'s story board as a one-page landscape document, scaled to fit.
pub fn storyboard_document(doc: &Document, board: u32) -> Document {
    let sb = Storyboard::of_doc(doc);
    let name = sb.boards.iter().find(|b| b.id == board).map_or_else(|| "Story Board".to_string(), |b| b.name.clone());
    let beats: Vec<&crate::storyboard::Beat> = sb.beats.iter().filter(|b| b.board == board).collect();
    let page = (11.0 * INCH, 8.5 * INCH);
    let margin = 0.5 * INCH;
    let top = margin + 24.0;
    let mut out = crate::template::new_script();
    let mut objects = Vec::new();
    if !beats.is_empty() {
        let x0 = beats.iter().map(|b| b.x).fold(f32::INFINITY, f32::min);
        let y0 = beats.iter().map(|b| b.y).fold(f32::INFINITY, f32::min);
        let x1 = beats.iter().map(|b| b.x + b.w).fold(0.0, f32::max);
        let y1 = beats.iter().map(|b| b.y + b.h).fold(0.0, f32::max);
        let scale = ((page.0 - 2.0 * margin) / (x1 - x0).max(1.0)).min((page.1 - top - margin) / (y1 - y0).max(1.0)).min(1.0);
        let place = |b: &crate::storyboard::Beat| (margin + (b.x - x0) * scale, top + (b.y - y0) * scale, b.w * scale, b.h * scale);
        let ink = Rgb(80, 80, 80);
        for l in &sb.links {
            let (Some(a), Some(b)) = (beats.iter().find(|x| x.id == l.from), beats.iter().find(|x| x.id == l.to)) else { continue };
            let (ra, rb) = (place(a), place(b));
            let (p, q) = (edge(ra, (rb.0 + rb.2 / 2.0, rb.1 + rb.3 / 2.0)), edge(rb, (ra.0 + ra.2 / 2.0, ra.1 + ra.3 / 2.0)));
            let color = rgb(&l.color).unwrap_or(ink);
            objects.push(page_paths(page, vec![FreePath { pts: vec![[p.0, p.1], [q.0, q.1]], closed: false }], color, false));
            let mut heads = Vec::new();
            if matches!(l.style.as_str(), "forward" | "both") {
                heads.push(arrowhead(p, q, 7.0));
            }
            if matches!(l.style.as_str(), "back" | "both") {
                heads.push(arrowhead(q, p, 7.0));
            }
            if !heads.is_empty() {
                objects.push(page_paths(page, heads, color, true));
            }
        }
        let size = (10.0 * scale).clamp(5.0, 10.0);
        for b in &beats {
            let mut lines = vec![Line { text: if b.title.is_empty() { "Untitled beat".into() } else { b.title.clone() }, bold: true, size }];
            lines.extend(b.body.lines().map(|l| Line { text: l.to_string(), bold: false, size: size * 0.9 }));
            objects.push(text_box(&mut out, place(b), rgb(&b.color), Some(Rgb(120, 120, 120)), &lines));
        }
    }
    paged(page, margin, vec![(name, objects)], &mut out);
    out
}

/// How index cards print.
#[derive(Clone, Debug, PartialEq)]
pub struct CardOptions {
    /// One 3 × 5 in card per page, or a grid on Letter pages.
    pub card_stock: bool,
    pub columns: usize,
    pub rows: usize,
    pub frames: bool,
    pub grey: bool,
    /// Only these scenes (indices); empty for all.
    pub scenes: Vec<usize>,
}

impl Default for CardOptions {
    fn default() -> Self {
        CardOptions { card_stock: false, columns: 2, rows: 3, frames: true, grey: false, scenes: Vec::new() }
    }
}

impl CardOptions {
    pub fn from_params(v: &Value) -> Self {
        let d = CardOptions::default();
        let n = |k: &str, dflt: usize| v.get(k).and_then(Value::as_u64).map_or(dflt, |x| (x as usize).clamp(1, 6));
        CardOptions {
            card_stock: v.get("stock").and_then(Value::as_str) == Some("card"),
            columns: n("columns", d.columns),
            rows: n("rows", d.rows),
            frames: v.get("frames").and_then(Value::as_bool).unwrap_or(d.frames),
            grey: v.get("grey").and_then(Value::as_bool).unwrap_or(d.grey),
            scenes: v
                .get("scenes")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as usize).collect())
                .unwrap_or_default(),
        }
    }
}

/// The scenes as index cards.
pub fn cards_document(doc: &Document, o: &CardOptions) -> Document {
    let elements = crate::element::Elements::of_doc(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let all = crate::scenes::scenes(doc);
    let picked: Vec<&crate::scenes::Scene> = all.iter().filter(|s| o.scenes.is_empty() || o.scenes.contains(&s.index)).collect();
    let mut out = crate::template::new_script();
    let (page, margin, per_page) =
        if o.card_stock { ((5.0 * INCH, 3.0 * INCH), 0.2 * INCH, 1) } else { ((8.5 * INCH, 11.0 * INCH), 0.5 * INCH, o.columns * o.rows) };
    let (cols, rows) = if o.card_stock { (1, 1) } else { (o.columns, o.rows) };
    let gap = if o.card_stock { 0.0 } else { 0.2 * INCH };
    let cw = (page.0 - 2.0 * margin - gap * (cols as f32 - 1.0)) / cols as f32;
    let ch = (page.1 - 2.0 * margin - gap * (rows as f32 - 1.0)) / rows as f32;
    let mut pages = Vec::new();
    for chunk in picked.chunks(per_page.max(1)) {
        let mut objects = Vec::new();
        for (k, sc) in chunk.iter().enumerate() {
            let (c, r) = (k % cols, k / cols);
            let at = (margin + c as f32 * (cw + gap), margin + r as f32 * (ch + gap), cw, ch);
            let info = crate::commands::scene::scene_json(doc, &elements, sc);
            let data = crate::scene_data::of_scene(doc, sc);
            let number = numbers.get(sc.index).cloned().unwrap_or_else(|| (sc.index + 1).to_string());
            let mut lines = vec![Line { text: format!("{number}  {}", sc.heading), bold: true, size: 10.0 }];
            if !data.title.is_empty() {
                lines.push(Line { text: data.title.clone(), bold: true, size: 9.0 });
            }
            let field = |k: &str| info.get(k).and_then(Value::as_str).filter(|t| !t.is_empty());
            let summary = field("summary").or_else(|| field("firstAction")).unwrap_or("");
            lines.push(Line { text: summary.to_string(), bold: false, size: 9.0 });
            let fill = if o.grey { Some(Rgb(235, 235, 235)) } else { None };
            let stroke = o.frames.then_some(Rgb(90, 90, 90));
            objects.push(text_box(&mut out, at, fill, stroke, &lines));
        }
        pages.push((String::new(), objects));
    }
    if pages.is_empty() {
        pages.push(("No scenes".into(), Vec::new()));
    }
    paged(page, margin, pages, &mut out);
    out
}

/// A built document as PDF bytes.
pub fn pdf(doc: &Document) -> Result<Vec<u8>, String> {
    let layout = crate::paginate::measure::print_layout(doc, &mut jcj_layout::LayoutCache::new());
    jcj_output::export_pdf(doc, &layout, &jcj_output::PdfSettings::default()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use serde_json::json;

    #[test]
    fn a_board_prints_on_one_page_with_its_beats_and_links() {
        let mut s = new_session();
        let a = s.run("beat.add", &json!({"title": "Arrival", "body": "1. Rain\n2. Door"})).unwrap()["id"].clone();
        let b = s.run("beat.add", &json!({"title": "Fight", "x": 3000, "y": 2000})).unwrap()["id"].clone();
        s.run("beat.link", &json!({"from": a, "to": b, "style": "both"})).unwrap();
        let d = storyboard_document(&s.doc, 0);
        let p = d.body[0].as_para().cloned().unwrap_or_default();
        // Two beats, one line and one set of arrowheads.
        assert_eq!(p.objects.len(), 4);
        let bytes = pdf(&d).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        // Everything fits the page.
        for o in &p.objects {
            if let InlineObject::Shape { w, h, float, .. } = o {
                assert!(float.x + w <= d.last_section.page_w + 0.5 && float.y + h <= d.last_section.page_h + 0.5);
            }
        }
    }

    #[test]
    fn cards_fill_grids_or_card_stock() {
        let mut s = new_session();
        s.doc.body = (0..7).map(|i| para_block(crate::template::paragraph("SceneHeading", &format!("INT. ROOM {i} - DAY")))).collect();
        let grid = cards_document(&s.doc, &CardOptions::default());
        assert_eq!(grid.body.len(), 2, "six cards a page");
        let stock = cards_document(&s.doc, &CardOptions { card_stock: true, scenes: vec![1, 3], ..CardOptions::default() });
        assert_eq!(stock.body.len(), 2);
        assert_eq!((stock.last_section.page_w, stock.last_section.page_h), (5.0 * INCH, 3.0 * INCH));
        assert!(pdf(&stock).unwrap().starts_with(b"%PDF"));
        let o = CardOptions::from_params(&json!({"stock": "grid", "columns": 99, "rows": 0}));
        assert_eq!((o.columns, o.rows), (6, 1));
    }
}
