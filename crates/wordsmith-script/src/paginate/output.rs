// SPDX-License-Identifier: MIT OR Apache-2.0
//! The output script: the source with the plan's page breaks made explicit, split paragraphs,
//! the (MORE), (CONT'D) and CONTINUED lines, automatic CONT'D on repeated speakers, and a header
//! with the page number (top right, followed by a period, none on page 1).

use std::sync::Arc;

use wordsmith_doc::props::{Align, CharProps};
use wordsmith_doc::{Document, InlineObject, Paragraph, PartKind, StoryRef, para_block};
use wordsmith_layout::{DocLayout, LayoutCache};

use super::measure::{measure, print_layout};
use super::plan::{Piece, plan};
use super::{MoresContinueds, Rules};
use crate::element::{ElementKind, Elements};
use crate::template::{self, Formatting, Placement, default_formatting, element_style};

/// Style ids of the output-only lines.
pub const MORE_STYLE: &str = "ws.More";
pub const CONTINUED_TOP_STYLE: &str = "ws.ContinuedTop";
pub const CONTINUED_BOTTOM_STYLE: &str = "ws.ContinuedBottom";
pub const PAGE_NUMBER_STYLE: &str = "ws.PageNumber";

/// Where an output paragraph came from: body block `block` of the source, starting at byte
/// `start` of it. Lines the paginator added have no source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    pub block: usize,
    pub start: usize,
}

/// A paginated script.
pub struct Paginated {
    /// The output script.
    pub doc: Document,
    /// The plan: what each page holds.
    pub pages: Vec<Vec<Piece>>,
    /// Each output body paragraph's source, by output block index.
    pub map: Vec<Option<Source>>,
    /// The output's print layout.
    pub layout: Arc<DocLayout>,
    /// The output was laid out on exactly the planned pages.
    pub matches_plan: bool,
}

impl Paginated {
    /// The output position showing source position `pos` (body story), if it is shown.
    pub fn to_output(&self, pos: &wordsmith_doc::Pos) -> Option<wordsmith_doc::Pos> {
        let block = pos.path.last();
        let (out, src) = self
            .map
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s)))
            .filter(|(_, s)| s.block == block && s.start <= pos.off)
            .max_by_key(|(_, s)| s.start)?;
        let len = self.doc.para(StoryRef::Body, &wordsmith_doc::Path::top(out)).map_or(0, |p| p.len());
        Some(wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(out), off: (pos.off - src.start).min(len) })
    }

    /// The source position an output position shows (a click on the page view).
    pub fn to_source(&self, pos: &wordsmith_doc::Pos) -> Option<wordsmith_doc::Pos> {
        let i = pos.path.last();
        let mut j = i;
        // A line the paginator added maps to the next paragraph that has a source.
        loop {
            match self.map.get(j)? {
                Some(src) => {
                    let off = if j == i { pos.off } else { 0 };
                    return Some(wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(src.block), off: src.start + off });
                }
                None => j += 1,
            }
        }
    }
}

fn mark_style(id: &str, place: Placement, align: Align) -> wordsmith_doc::Style {
    let f = Formatting { place, align, space_before_lines: 0.0, keep_with_next: false, ..default_formatting(ElementKind::General) };
    let mut st = element_style(id, id, &f, id);
    st.builtin = false;
    st
}

fn text_para(style: &str, text: &str) -> Paragraph {
    let mut p = Paragraph::with_text(text, CharProps::default());
    p.props.style = Some(style.to_string());
    p
}

/// The speaker of a Character line: its text before any extension, trimmed.
pub fn speaker(name: &str) -> &str {
    name.split('(').next().unwrap_or("").trim()
}

/// Append ` (CONT'D)` (the configured text) to a name that does not already carry it.
fn with_contd(name: &str, contd: &str) -> String {
    if name.to_uppercase().contains(&contd.to_uppercase()) { name.to_string() } else { format!("{} {contd}", name.trim_end()) }
}

/// Source block indices of Character paragraphs that get an automatic CONT'D: the same speaker
/// as the previous speech in the scene, with no other speaker between.
fn auto_contd(doc: &Document, elements: &Elements) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    let mut last: Option<String> = None;
    for (i, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        match elements.of_para(p).kind {
            ElementKind::SceneHeading => last = None,
            ElementKind::Character => {
                let who = speaker(&p.text).to_uppercase();
                if !who.is_empty() && last.as_deref() == Some(who.as_str()) {
                    out.insert(i);
                }
                last = Some(who);
            }
            _ => {}
        }
    }
    out
}

/// The output paragraph for bytes `start..end` of source paragraph `src`.
fn part_of(src: &Paragraph, start: usize, end: usize) -> Paragraph {
    let mut p = src.clone();
    let len = p.len();
    let end = end.min(len);
    if end < len {
        let _ = p.delete(end, len);
    }
    if start > 0 {
        let _ = p.delete(0, start.min(end));
    }
    p.touch();
    p
}

/// Build the output script for `pages` of `doc`.
pub fn build(doc: &Document, pages: &[Vec<Piece>], marks: &MoresContinueds) -> (Document, Vec<Option<Source>>) {
    let elements = Elements::of_doc(doc);
    let contd_blocks = if marks.character { auto_contd(doc, &elements) } else { Default::default() };
    let mut out = doc.clone();
    let sheet = Arc::make_mut(&mut out.styles);
    let character_left = sheet
        .get(elements.id_of(ElementKind::Character))
        .and_then(|s| s.para.indent_left)
        .map_or(3.7, |pt| (pt + template::MARGIN_LEFT) / template::INCH);
    sheet.upsert(mark_style(MORE_STYLE, Placement { left: character_left, right: 7.5 }, Align::Left));
    sheet.upsert(mark_style(CONTINUED_TOP_STYLE, Placement { left: 1.5, right: 7.5 }, Align::Left));
    sheet.upsert(mark_style(CONTINUED_BOTTOM_STYLE, Placement { left: 1.5, right: 7.5 }, Align::Right));
    sheet.upsert(mark_style(PAGE_NUMBER_STYLE, Placement { left: 1.5, right: 7.5 }, Align::Right));

    let mut body = Vec::new();
    let mut map = Vec::new();
    for (n, page) in pages.iter().enumerate() {
        let first = body.len();
        for piece in page {
            let (mut para, src) = match piece {
                Piece::Para { block, start, end, space_before, .. } => {
                    let Some(sp) = doc.body.get(*block).and_then(|b| b.as_para()) else { continue };
                    let mut p = part_of(sp, *start, *end);
                    if !space_before {
                        p.props.space_before = Some(0.0);
                    }
                    if *start == 0 && contd_blocks.contains(block) {
                        let name = with_contd(&p.text, &marks.contd);
                        p = Paragraph::with_text(&name, p.props_at(0).clone()).styled(p.props.style.as_deref().unwrap_or(""));
                        if !space_before {
                            p.props.space_before = Some(0.0);
                        }
                    }
                    (p, Some(Source { block: *block, start: *start }))
                }
                Piece::More => (text_para(MORE_STYLE, &marks.more), None),
                Piece::ContinuedBottom => (text_para(CONTINUED_BOTTOM_STYLE, &marks.continued_bottom), None),
                Piece::ContinuedTop { page } => {
                    let text = if marks.scene_page_count { format!("{} ({page})", marks.continued_top) } else { marks.continued_top.clone() };
                    (text_para(CONTINUED_TOP_STYLE, &text), None)
                }
                Piece::CharacterContd { block } => {
                    let name = doc.body.get(*block).and_then(|b| b.as_para()).map_or(String::new(), |p| with_contd(&p.text, &marks.contd));
                    let mut p = text_para(elements.id_of(ElementKind::Character), &name);
                    p.props.space_before = Some(0.0);
                    (p, None)
                }
            };
            // The plan decided the breaks: the layout must not move anything.
            para.props.keep_next = Some(false);
            para.props.keep_lines = Some(false);
            para.props.widow_control = Some(false);
            para.props.page_break_before = Some(body.len() == first && n > 0);
            if body.len() == first {
                para.props.space_before = Some(0.0);
            }
            para.touch();
            body.push(para_block(para));
            map.push(src);
        }
    }
    if body.is_empty() {
        body.push(para_block(Paragraph::new()));
        map.push(None);
    }
    out.body = body;
    add_page_numbers(&mut out);
    (out, map)
}

/// A header with the page number and a period, top right, and an empty first-page header, so
/// page 1 has none.
fn add_page_numbers(doc: &mut Document) {
    let mut number = text_para(PAGE_NUMBER_STYLE, ".");
    let page = InlineObject::Field { instr: "PAGE".into(), result: "1".into(), locked: false, dirty: false, code: false, form: String::new() };
    let _ = number.insert_object(0, page, &CharProps::default());
    let header = doc.add_part(PartKind::Header, vec![para_block(number)]);
    let first = doc.add_part(PartKind::Header, vec![para_block(text_para(PAGE_NUMBER_STYLE, ""))]);
    let s = &mut doc.last_section;
    s.headers.default = Some(header);
    s.headers.first = Some(first);
    s.headers.even = None;
    s.title_page = true;
    s.header = 0.5 * template::INCH;
}

/// Paginate `doc`: measure, plan, build the output and lay it out. `cache` holds paragraph
/// layouts between calls.
pub fn paginate(doc: &Document, cache: &mut LayoutCache) -> Paginated {
    let marks = MoresContinueds::of_doc(doc);
    let source = print_layout(doc, cache);
    let m = measure(doc, &source);
    let pages = plan(doc, &m, &marks, Rules::default());
    let (out, map) = build(doc, &pages, &marks);
    let layout = print_layout(&out, cache);
    let matches_plan = layout.pages.len() == pages.len();
    if !matches_plan {
        log::warn!("pagination: planned {} pages, the layout made {}", pages.len(), layout.pages.len());
    }
    Paginated { doc: out, pages, map, layout, matches_plan }
}
