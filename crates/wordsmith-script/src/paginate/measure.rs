// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the paginator needs from the layout: each body paragraph's lines (byte ranges and
//! heights) and the resolved spacing and keep settings the layout used.

use std::sync::Arc;

use wordsmith_doc::{Document, Path, StoryRef, para_block};
use wordsmith_layout::{DocLayout, LayoutCache, LayoutOptions, ParaLayout, Placed, ViewMode};

use crate::element::{ElementKind, Elements};

/// One body paragraph, measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// Index of the paragraph's block in the body.
    pub block: usize,
    /// The element whose page-break rules apply (its own, or the one it paginates as).
    pub rules: ElementKind,
    /// The element itself.
    pub kind: ElementKind,
    pub text: String,
    /// Each line's byte range in `text`.
    pub lines: Vec<(usize, usize)>,
    /// Each line's height, points.
    pub heights: Vec<f32>,
    pub space_before: f32,
    pub keep_next: bool,
    pub page_break_before: bool,
}

impl Measured {
    pub fn height(&self) -> f32 {
        self.heights.iter().sum()
    }
}

/// Byte ranges of a laid-out paragraph's lines.
pub fn line_ranges(p: &ParaLayout) -> Vec<(usize, usize)> {
    p.lines
        .iter()
        .map(|l| {
            let start = p.clusters.get(l.c0).map_or(p.text_len, |c| c.start);
            let end = if l.c1 > l.c0 { p.clusters.get(l.c1 - 1).map_or(start, |c| c.end) } else { start };
            (start, end.max(start))
        })
        .collect()
}

/// The layout of body block `i`, wherever its first piece was placed.
fn para_layout(layout: &DocLayout, i: usize) -> Option<Arc<ParaLayout>> {
    let pieces = layout.index.get(&(StoryRef::Body, Path::top(i)))?;
    let (page, item) = pieces.first()?;
    match layout.pages.get(*page)?.items.get(*item)? {
        Placed::Lines { para, .. } => Some(para.clone()),
        _ => None,
    }
}

/// Measure every body paragraph of `doc` from `layout` (a print-view layout of `doc`).
pub fn measure(doc: &Document, layout: &DocLayout) -> Vec<Measured> {
    let elements = Elements::of_doc(doc);
    let Some(blocks) = doc.story(StoryRef::Body) else { return Vec::new() };
    let mut out = Vec::with_capacity(blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        let Some(p) = block.as_para() else { continue };
        let def = elements.of_para(p);
        let rules = def.paginate_as.as_deref().and_then(|id| elements.get(id)).map_or(def.kind, |d| d.kind);
        let (lines, heights, rp) = match para_layout(layout, i) {
            Some(pl) => (line_ranges(&pl), pl.lines.iter().map(|l| l.height).collect::<Vec<f32>>(), Some(pl.rp.clone())),
            None => (vec![(0, p.text.len())], vec![crate::template::LINE], None),
        };
        let (lines, heights) = if lines.is_empty() { (vec![(0, 0)], vec![crate::template::LINE]) } else { (lines, heights) };
        out.push(Measured {
            block: i,
            rules,
            kind: def.kind,
            text: p.text.clone(),
            lines,
            heights,
            space_before: rp.as_ref().map_or(0.0, |r| r.space_before),
            keep_next: rp.as_ref().is_some_and(|r| r.keep_next),
            page_break_before: rp.as_ref().is_some_and(|r| r.page_break_before),
        });
    }
    out
}

/// Lines of each paragraph, kept between paginations: an edit lays out only the paragraphs it
/// changed. Keyed by the paragraph's revision and a fingerprint of the styles and page width.
#[derive(Default)]
pub struct MeasureCache {
    lines: std::collections::HashMap<(u64, u64), (Vec<(usize, usize)>, Vec<f32>)>,
    /// The styles the entries were made with, and their fingerprint.
    env: Option<(Arc<wordsmith_doc::StyleSheet>, u64)>,
}

/// Most paragraphs kept; past this the cache starts over. Bounds memory on huge scripts.
const MAX_CACHED: usize = 200_000;

impl MeasureCache {
    fn env(&mut self, doc: &Document) -> u64 {
        use std::hash::{Hash, Hasher};
        if let Some((styles, key)) = &self.env
            && Arc::ptr_eq(styles, &doc.styles)
        {
            return *key;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&*doc.styles).unwrap_or_default().hash(&mut h);
        text_width(doc).to_bits().hash(&mut h);
        let key = h.finish();
        self.env = Some((doc.styles.clone(), key));
        key
    }
}

/// The body text column's width, points.
fn text_width(doc: &Document) -> f32 {
    let s = &doc.last_section;
    (s.page_w - s.margin_left - s.margin_right).max(1.0)
}

/// [`measure`] without laying out the whole script: each paragraph is laid out on its own (it
/// does not depend on where it lands), through `cache`.
pub fn measure_cached(doc: &Document, cache: &mut MeasureCache) -> Vec<Measured> {
    let env_key = cache.env(doc);
    if cache.lines.len() > MAX_CACHED {
        cache.lines.clear();
    }
    let elements = Elements::of_doc(doc);
    let fields = wordsmith_layout::FieldCtx::default();
    let width = text_width(doc);
    let Some(blocks) = doc.story(StoryRef::Body) else { return Vec::new() };
    let mut out = Vec::with_capacity(blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        let Some(p) = block.as_para() else { continue };
        let def = elements.of_para(p);
        let rules = def.paginate_as.as_deref().and_then(|id| elements.get(id)).map_or(def.kind, |d| d.kind);
        let (lines, heights) = cache
            .lines
            .entry((p.rev, env_key))
            .or_insert_with(|| {
                crate::work::count(|w| w.paragraphs_measured += 1);
                let env = wordsmith_layout::para::ParaEnv {
                    doc,
                    width,
                    label: None,
                    fields: &fields,
                    show_hidden: false,
                    field_codes: false,
                    hide_deleted: Default::default(),
                    table: None,
                    proofing: None,
                    typing_word: None,
                    numeral: Default::default(),
                    exclusions: &[],
                    read_line_spacing: None,
                    grid: Default::default(),
                    fit_objects: false,
                    fitted: &[],
                };
                let pl = wordsmith_layout::para::layout_para(p, &env);
                (line_ranges(&pl), pl.lines.iter().map(|l| l.height).collect())
            })
            .clone();
        let (lines, heights) = if lines.is_empty() { (vec![(0, 0)], vec![crate::template::LINE]) } else { (lines, heights) };
        let rp = doc.styles.resolve_para(&p.props);
        out.push(Measured {
            block: i,
            rules,
            kind: def.kind,
            text: p.text.clone(),
            lines,
            heights,
            space_before: rp.space_before,
            keep_next: rp.keep_next,
            page_break_before: rp.page_break_before,
        });
    }
    out
}

/// A print-view layout of `doc`.
pub fn print_layout(doc: &Document, cache: &mut LayoutCache) -> Arc<DocLayout> {
    Arc::new(wordsmith_layout::layout(doc, cache, &LayoutOptions { view: ViewMode::Print, ..Default::default() }))
}

/// The lines `text` would take as paragraph `src` (its style and formatting), for the remainder
/// of a split paragraph. Returns byte ranges in `text` and heights.
pub fn measure_text(doc: &Document, src: &wordsmith_doc::Paragraph, text: &str) -> (Vec<(usize, usize)>, Vec<f32>) {
    let mut probe = Document::new();
    probe.styles = doc.styles.clone();
    probe.last_section = doc.last_section.clone();
    let mut p = src.clone();
    let len = p.len();
    let ok = p.delete(0, len).and_then(|()| p.insert_text(0, text, &src.props_at(0).clone()));
    if ok.is_err() {
        return (vec![(0, text.len())], vec![crate::template::LINE]);
    }
    p.props.page_break_before = Some(false);
    probe.body = vec![para_block(p)];
    let layout = print_layout(&probe, &mut LayoutCache::new());
    match para_layout(&layout, 0) {
        Some(pl) if !pl.lines.is_empty() => (line_ranges(&pl), pl.lines.iter().map(|l| l.height).collect()),
        _ => (vec![(0, text.len())], vec![crate::template::LINE]),
    }
}
