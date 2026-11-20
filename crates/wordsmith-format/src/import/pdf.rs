// SPDX-License-Identifier: MIT OR Apache-2.0
//! Text-based PDF import: each page's glyphs are read with craft-pdf's guarded interpreter,
//! grouped into lines by baseline, and placed by their left edge. A scanned script has no
//! glyphs and is refused with a clear message.

use craft_pdf::interpret::font::Glyph;
use craft_pdf::interpret::hayro_cmap::BfString;
use craft_pdf::interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings, Paint, PathDrawMode, SoftMask,
};
use kurbo::{Affine, BezPath, Rect};
use wordsmith_doc::Document;

use super::{ImportError, Line, MAX_IMPORT_BYTES, MAX_LINES, classify};

/// Most pages read.
pub const MAX_PAGES: usize = 5_000;
/// Most glyphs kept from one page.
const MAX_GLYPHS: usize = 200_000;

/// The glyphs of a page: (baseline y, x, text, size).
struct Glyphs(Vec<(f64, f64, String)>);

impl Device<'_> for Glyphs {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'_>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'_>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
    fn draw_glyph(&mut self, g: &Glyph<'_>, t: Affine, glyph: Affine, _: &Paint<'_>, mode: &GlyphDrawMode) {
        if matches!(mode, GlyphDrawMode::Invisible) || self.0.len() >= MAX_GLYPHS {
            return;
        }
        let s = match g.as_unicode() {
            Some(BfString::Char(c)) => c.to_string(),
            Some(BfString::String(s)) => s.to_string(),
            None => return,
        };
        // Where the glyph lands on the page: its own placement, then the page transform.
        let at = (t * glyph).translation();
        if at.x.is_finite() && at.y.is_finite() {
            self.0.push((at.y, at.x, s));
        }
    }
    fn draw_image(&mut self, _: Image<'_, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// A page's lines, top to bottom: (baseline y in points from the bottom, left x in points,
/// text).
fn page_lines(mut glyphs: Vec<(f64, f64, String)>) -> Vec<(f64, f64, String)> {
    // PDF y grows upward: the top line has the largest y.
    glyphs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
    let mut rows: Vec<(f64, Vec<(f64, String)>)> = Vec::new();
    for (y, x, s) in glyphs {
        match rows.last_mut() {
            Some((ly, row)) if (*ly - y).abs() < 1.0 => row.push((x, s)),
            _ => rows.push((y, vec![(x, s)])),
        }
    }
    rows.into_iter()
        .filter_map(|(y, mut row)| {
            row.sort_by(|a, b| a.0.total_cmp(&b.0));
            // The usual distance from one glyph to the next is a character's width; a wider gap
            // is a space the PDF does not draw.
            let mut steps: Vec<f64> = row.windows(2).map(|w| w[1].0 - w[0].0).filter(|d| *d > 0.5).collect();
            steps.sort_by(f64::total_cmp);
            let advance = steps.get(steps.len() / 4).copied().unwrap_or(7.2).clamp(2.0, 30.0);
            let left = row.first().map(|g| g.0)?;
            let mut text = String::new();
            let mut last_x: Option<f64> = None;
            for (x, s) in row {
                if last_x.is_some_and(|lx| x - lx > advance * 1.6) && !text.ends_with(' ') {
                    text.push(' ');
                }
                text.push_str(&s);
                last_x = Some(x);
            }
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then_some((y, left, text))
        })
        .collect()
}

/// Import a text-based PDF as a script, placing each line by its left edge.
pub fn pdf(bytes: &[u8]) -> Result<Document, ImportError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge);
    }
    let file = craft_pdf::open_slice(bytes, None).map_err(|e| ImportError::NotPdf(e.to_string()))?;
    let cache = InterpreterCache::new();
    let mut pages: Vec<Vec<(f64, f64, String)>> = Vec::new();
    let mut count = 0;
    for page in file.pages().iter().take(MAX_PAGES) {
        let mut ctx = Context::new(Affine::IDENTITY, Rect::new(0.0, 0.0, 1.0, 1.0), &cache, file.xref(), InterpreterSettings::default());
        let mut glyphs = Glyphs(Vec::new());
        craft_pdf::interpret_page(page, &mut ctx, &mut glyphs).map_err(|e| ImportError::NotPdf(e.to_string()))?;
        let rows = page_lines(glyphs.0);
        count += rows.len();
        pages.push(rows);
        if count >= MAX_LINES {
            break;
        }
    }
    // The script's line pitch: a low distance between baselines across the whole script (one
    // page may have only single-line paragraphs, a blank line apart).
    let mut pitches: Vec<f64> = pages.iter().flat_map(|rows| rows.windows(2).map(|w| w[0].0 - w[1].0)).filter(|d| *d > 2.0).collect();
    pitches.sort_by(f64::total_cmp);
    let pitch = pitches.get(pitches.len() / 10).copied().unwrap_or(12.0).clamp(4.0, 40.0);
    let mut lines: Vec<Line> = Vec::with_capacity(count);
    for rows in pages {
        let mut prev_y: Option<f64> = None;
        for (y, x, text) in rows {
            // The first line on a page starts a paragraph; after that, a gap of more than a line
            // and a half is a blank line.
            let gap_before = prev_y.is_none_or(|py| py - y > pitch * 1.5);
            lines.push(Line { left: Some((x / 72.0) as f32), text, gap_before });
            prev_y = Some(y);
        }
    }
    if lines.is_empty() {
        return Err(ImportError::NoText);
    }
    Ok(classify(lines, true, true))
}
