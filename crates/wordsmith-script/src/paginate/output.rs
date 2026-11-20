// SPDX-License-Identifier: MIT OR Apache-2.0
//! The output script: the source with the plan's page breaks made explicit, split paragraphs,
//! the (MORE), (CONT'D) and CONTINUED lines, automatic CONT'D on repeated speakers, and the
//! header and footer ([`crate::header_footer`]; by default the page number top right, followed by
//! a period, none on page 1).

use std::sync::Arc;

use wordsmith_doc::props::{Align, CharProps};
use wordsmith_doc::{Document, Paragraph, StoryRef, para_block};
use wordsmith_layout::{DocLayout, LayoutCache};

use super::measure::{MeasureCache, measure_cached, print_layout};
use super::plan::{Piece, plan};
use super::{MoresContinueds, Rules};
use crate::element::{ElementKind, Elements};
use crate::template::{self, Formatting, Placement, default_formatting, element_style};

/// Style ids of the output-only lines.
pub const MORE_STYLE: &str = "ws.More";
pub const CONTINUED_TOP_STYLE: &str = "ws.ContinuedTop";
pub const CONTINUED_BOTTOM_STYLE: &str = "ws.ContinuedBottom";

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
    /// Each page's newest revision set, if it has revised text.
    pub page_revisions: Vec<Option<u32>>,
    /// Each page's label, when the pages are locked (decision 0026).
    pub labels: Option<Vec<String>>,
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

/// A revision for an output paragraph that is the same every time the same source paragraph is
/// shown the same way, so an unchanged page lays out from the cache. The high bit keeps these
/// apart from the counter `Paragraph::touch` uses.
fn stable_rev(parts: &[u64]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish() | (1 << 63)
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
    p
}

/// Which of the output paragraph's lines are revised (for the margin marks), and the newest set
/// on it: its revised characters, and, on its first line, a deletion in the paragraph.
fn revised_lines(doc: &Document, sp: &Paragraph, start: usize, end: usize, para: &Paragraph, only: Option<u32>) -> (Vec<bool>, Option<u32>) {
    // With the current set alone showing, other sets' marks are left out.
    let counts = |x: Option<u32>| x.filter(|i| only.is_none_or(|o| o == *i));
    let deleted = if start == 0 { counts(sp.mark.revised) } else { None };
    let newest = para.run_ranges().filter_map(|(_, c)| counts(c.revised)).chain(deleted).max();
    if newest.is_none() {
        return (Vec::new(), None);
    }
    let (lines, _) = super::measure::measure_text(doc, sp, sp.text.get(start..end).unwrap_or(""));
    let mut flags: Vec<bool> =
        lines.iter().map(|&(a, b)| para.run_ranges().any(|(r, c)| counts(c.revised).is_some() && r.start < b.max(a + 1) && r.end > a)).collect();
    if deleted.is_some()
        && let Some(f) = flags.first_mut()
    {
        *f = true;
    }
    (flags, newest)
}

/// The revision sets' text styles on revised runs.
fn set_styles(para: &mut Paragraph, rev: &crate::revisions::Revisions) {
    let ranges: Vec<(std::ops::Range<usize>, u32)> = para.run_ranges().filter_map(|(r, c)| c.revised.map(|i| (r, i))).collect();
    for (r, i) in ranges {
        let Some(set) = rev.set(i) else { continue };
        if set.bold || set.underline || set.strike {
            let (b, u, k) = (set.bold, set.underline, set.strike);
            let _ = para.format(r.start, r.end, &|c| {
                if b {
                    c.bold = Some(true);
                }
                if u {
                    c.underline = Some(wordsmith_doc::props::Underline::Single);
                }
                if k {
                    c.strike = Some(true);
                }
            });
        }
    }
}

/// The output script, each output paragraph's source, and each page's newest revision set.
pub struct Built {
    pub doc: Document,
    pub map: Vec<Option<Source>>,
    pub page_revisions: Vec<Option<u32>>,
}

/// Build the output script for `pages` of `doc`.
pub fn build(doc: &Document, pages: &[Vec<Piece>], marks: &MoresContinueds, labels: Option<&[String]>) -> Built {
    let elements = Elements::of_doc(doc);
    let rev = crate::revisions::Revisions::of_doc(doc);
    let rev_hash = stable_rev(&[serde_json::to_string(&rev)
        .map(|t| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut h);
            h.finish()
        })
        .unwrap_or(0)]);
    let mut page_revisions = Vec::with_capacity(pages.len());
    // Scene numbers beside the headings, when shown (decision 0025).
    let numbering = crate::scene_numbers::SceneNumbers::of_doc(doc);
    let scene_numbers: std::collections::HashMap<usize, String> = if numbering.show {
        crate::scenes::scenes(doc).iter().map(|sc| sc.start).zip(crate::scene_numbers::numbers(doc)).collect()
    } else {
        Default::default()
    };
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

    let mut body = Vec::new();
    let mut map = Vec::new();
    for (n, page) in pages.iter().enumerate() {
        let first = body.len();
        let mut page_newest: Option<u32> = None;
        for piece in page {
            // What makes this output paragraph what it is, for its stable revision.
            let mut identity: Vec<u64> = Vec::with_capacity(8);
            let (mut para, src) = match piece {
                Piece::Para { block, start, end, space_before, .. } => {
                    let Some(sp) = doc.body.get(*block).and_then(|b| b.as_para()) else { continue };
                    identity.extend([
                        1,
                        sp.rev,
                        *start as u64,
                        *end as u64,
                        u64::from(*space_before),
                        u64::from(*start == 0 && contd_blocks.contains(block)),
                    ]);
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
                Piece::More => {
                    identity.push(2);
                    (text_para(MORE_STYLE, &marks.more), None)
                }
                Piece::Blank => {
                    identity.push(7);
                    (text_para(elements.id_of(ElementKind::General), ""), None)
                }
                Piece::ContinuedBottom => {
                    identity.push(3);
                    (text_para(CONTINUED_BOTTOM_STYLE, &marks.continued_bottom), None)
                }
                Piece::ContinuedTop { page } => {
                    identity.extend([4, *page as u64]);
                    let text = if marks.scene_page_count { format!("{} ({page})", marks.continued_top) } else { marks.continued_top.clone() };
                    (text_para(CONTINUED_TOP_STYLE, &text), None)
                }
                Piece::CharacterContd { block } => {
                    identity.extend([5, doc.body.get(*block).and_then(|b| b.as_para()).map_or(0, |p| p.rev)]);
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
            identity.extend([u64::from(body.len() == first), u64::from(n > 0)]);
            // The scene number either side of its heading's first line.
            if let Piece::Para { block, start: 0, .. } = piece
                && let Some(number) = scene_numbers.get(block)
            {
                let y = out.styles.resolve_para(&para.props).space_before.max(0.0);
                let lines = [number.clone()];
                let numbering = crate::scene_numbers::SceneNumbers::of_doc(doc);
                let font = Some(numbering.font.as_str()).filter(|f| !f.is_empty());
                let mut objs = Vec::new();
                if numbering.sides != "right" {
                    objs.push(super::margin::margin_box_in(&mut out, 0.7 * template::INCH, y, 0.55 * template::INCH, &lines, Align::Right, font));
                }
                if numbering.sides != "left" {
                    objs.push(super::margin::margin_box_in(&mut out, 7.55 * template::INCH, y, 0.45 * template::INCH, &lines, Align::Left, font));
                }
                for obj in objs {
                    let at = para.len();
                    let _ = para.insert_object(at, obj, &CharProps::default());
                }
                identity.push(stable_rev(&[number.bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)))]));
            }
            // Revision marks beside the revised lines, and the sets' text styles.
            if let Piece::Para { block, start, end, .. } = piece
                && let Some(sp) = doc.body.get(*block).and_then(|b| b.as_para())
            {
                let only = if rev.active_only { Some(rev.current_index().unwrap_or(u32::MAX)) } else { None };
                let (flags, newest) = revised_lines(doc, sp, *start, *end, &para, only);
                if newest.is_some() {
                    page_newest = page_newest.max(newest);
                    set_styles(&mut para, &rev);
                }
                if newest.is_some() && rev.show_marks {
                    let y = out.styles.resolve_para(&para.props).space_before.max(0.0);
                    // The newest set's own mark, else the script's.
                    let mark =
                        newest.and_then(|i| rev.set(i)).map(|set| set.mark.clone()).filter(|m| !m.is_empty()).unwrap_or_else(|| rev.mark.clone());
                    let lines: Vec<String> = flags.iter().map(|f| if *f { mark.clone() } else { String::new() }).collect();
                    let mark = super::margin::margin_box(&mut out, rev.mark_at * template::INCH, y, 0.5 * template::INCH, &lines, Align::Left);
                    let at = para.len();
                    let _ = para.insert_object(at, mark, &CharProps::default());
                    identity.push(rev_hash);
                }
            }
            if matches!(piece, Piece::More | Piece::ContinuedBottom | Piece::ContinuedTop { .. }) {
                // The mark texts are settings; they are part of what the paragraph shows.
                identity.push(marks_hash(marks));
            }
            para.rev = stable_rev(&identity);
            body.push(para_block(para));
            map.push(src);
        }
        // With color pages on, a revised page is colored in its newest set's color.
        if rev.color_pages
            && let Some(color) = page_newest.and_then(|i| rev.set(i)).and_then(|set| crate::revisions::parse_color(&set.color))
            && let Some(wordsmith_doc::Block::Para(p)) = body.get_mut(first).map(Arc::make_mut)
        {
            let fill = super::margin::page_fill(&out, color);
            let at = p.len();
            let _ = p.insert_object(at, fill, &CharProps::default());
            p.rev = stable_rev(&[p.rev, rev_hash, 6]);
        }
        page_revisions.push(page_newest);
    }
    if body.is_empty() {
        body.push(para_block(Paragraph::new()));
        map.push(None);
    }
    out.body = body;
    crate::header_footer::apply(&mut out, labels);
    Built { doc: out, map, page_revisions }
}

fn marks_hash(marks: &MoresContinueds) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (&marks.more, &marks.contd, &marks.continued_top, &marks.continued_bottom).hash(&mut h);
    h.finish()
}

/// What pagination keeps between calls: paragraph lines of the script, and the output's layout
/// cache.
#[derive(Default)]
pub struct Paginator {
    pub measure: MeasureCache,
    pub layout: LayoutCache,
}

impl Paginator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Paginate `doc`: measure, plan, build the output and lay it out.
    pub fn paginate(&mut self, doc: &Document) -> Paginated {
        paginate_with(doc, &mut self.measure, &mut self.layout)
    }
}

/// Paginate `doc` once, with fresh caches.
pub fn paginate(doc: &Document) -> Paginated {
    Paginator::new().paginate(doc)
}

fn paginate_with(doc: &Document, measure: &mut MeasureCache, cache: &mut LayoutCache) -> Paginated {
    let marks = MoresContinueds::of_doc(doc);
    // Dual dialogue: the first speech narrowed, the second left out and drawn beside it.
    let dual = super::dual::prepare(doc);
    let src = dual.as_ref().map_or(doc, |d| &d.doc);
    let mut m = measure_cached(src, measure);
    // Hidden outline elements are left out of the pages (decision 0028).
    if !crate::outline::shows_outline(doc) {
        m.retain(|p| !p.kind.is_outline());
    }
    if let Some(d) = &dual {
        m.retain(|p| !d.dropped.contains(&p.block));
    }
    let pages = plan(src, &m, &marks, Rules::default());
    let (pages, labels) = crate::locks::label_pages(src, pages);
    let Built { doc: mut out, map, page_revisions } = build(src, &pages, &marks, labels.as_deref());
    if let Some(d) = dual {
        for (block, paras) in d.boxes {
            let Some(o) = map.iter().position(|m| m.is_some_and(|m| m.block == block && m.start == 0)) else { continue };
            let y = out.body.get(o).and_then(|b| b.as_para()).map_or(0.0, |p| out.styles.resolve_para(&p.props).space_before.max(0.0));
            let obj = super::dual::speech_box(&mut out, &paras, y);
            if let Some(wordsmith_doc::Block::Para(p)) = out.body.get_mut(o).map(std::sync::Arc::make_mut) {
                // At the end, so the text's offsets (and the caret's mapping) stay as they are.
                let at = p.len();
                let _ = p.insert_object(at, obj, &CharProps::default());
            }
        }
    }
    crate::watermark::apply(&mut out, &crate::watermark::Watermark::of_doc(doc));
    let layout = print_layout(&out, cache);
    let matches_plan = layout.pages.len() == pages.len();
    if !matches_plan {
        log::warn!("pagination: planned {} pages, the layout made {}", pages.len(), layout.pages.len());
    }
    Paginated { doc: out, pages, map, layout, matches_plan, page_revisions, labels }
}
