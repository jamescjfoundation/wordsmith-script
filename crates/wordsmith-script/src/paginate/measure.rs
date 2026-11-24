// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the paginator needs from the layout: each body paragraph's lines (byte ranges and
//! heights) and the resolved spacing and keep settings the layout used.

use std::sync::Arc;

use jcj_doc::{Document, StoryRef};
use jcj_layout::{DocLayout, LayoutCache, LayoutOptions, ViewMode};

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

pub use jcj_layout::measure::{line_ranges, measure_text, text_width};

/// The layout of body block `i`, wherever its first piece was placed.
fn para_layout(layout: &DocLayout, i: usize) -> Option<Arc<jcj_layout::ParaLayout>> {
    jcj_layout::measure::body_paragraph_layout(layout, i)
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
    env: Option<(Arc<jcj_doc::StyleSheet>, u64)>,
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

/// [`measure`] without laying out the whole script: each paragraph is laid out on its own (it
/// does not depend on where it lands), through `cache`.
pub fn measure_cached(doc: &Document, cache: &mut MeasureCache) -> Vec<Measured> {
    crate::fonts::ensure();
    let env_key = cache.env(doc);
    if cache.lines.len() > MAX_CACHED {
        cache.lines.clear();
    }
    let elements = Elements::of_doc(doc);
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
                let pl = jcj_layout::measure::layout_alone(doc, p, width);
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
    Arc::new(jcj_layout::layout(doc, cache, &LayoutOptions { view: ViewMode::Print, ..Default::default() }))
}
