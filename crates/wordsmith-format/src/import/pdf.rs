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
    let mut first_page: Vec<Segment> = Vec::new();
    let mut count = 0;
    for page in file.pages().iter().take(MAX_PAGES) {
        let mut ctx = Context::new(Affine::IDENTITY, Rect::new(0.0, 0.0, 1.0, 1.0), &cache, file.xref(), InterpreterSettings::default());
        let mut glyphs = Glyphs(Vec::new());
        craft_pdf::interpret_page(page, &mut ctx, &mut glyphs).map_err(|e| ImportError::NotPdf(e.to_string()))?;
        if pages.is_empty() {
            first_page = segments(glyphs.0.clone());
        }
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
    // A first page with no scene and no speech, before more pages, is a title page: its fields
    // are read back and its lines are not part of the script.
    let title = if pages.len() > 1 { title_page_of(&first_page) } else { None };
    let skip = usize::from(title.is_some());
    let mut lines: Vec<Line> = Vec::with_capacity(count);
    for rows in pages.into_iter().skip(skip) {
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
    let mut doc = classify(lines, true, true);
    if let Some(t) = title {
        if !t.title.is_empty() {
            doc.core.title = t.title.split('\n').collect::<Vec<_>>().join(" ");
        }
        t.store(&mut doc);
    }
    Ok(doc)
}

/// A run of text on one baseline: (baseline y in points, left and right edges in inches, text).
/// A gap of six characters or more splits a baseline into separate segments (the two sides of a
/// title page's foot).
type Segment = (f64, f32, f32, String);

fn segments(mut glyphs: Vec<(f64, f64, String)>) -> Vec<Segment> {
    glyphs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<Segment> = Vec::new();
    let mut row: Vec<(f64, f64, String)> = Vec::new();
    let flush = |row: &mut Vec<(f64, f64, String)>, out: &mut Vec<Segment>| {
        let mut steps: Vec<f64> = row.windows(2).map(|w| w[1].1 - w[0].1).filter(|d| *d > 0.5).collect();
        steps.sort_by(f64::total_cmp);
        let advance = steps.get(steps.len() / 4).copied().unwrap_or(7.2).clamp(2.0, 30.0);
        let mut start = 0;
        for i in 0..row.len() {
            let split = row.get(i + 1).is_none_or(|n| n.1 - row[i].1 > advance * 6.0);
            if split {
                let piece = &row[start..=i];
                let text: String = {
                    let mut t = String::new();
                    let mut last: Option<f64> = None;
                    for (_, x, s) in piece {
                        if last.is_some_and(|lx| x - lx > advance * 1.6) {
                            t.push(' ');
                        }
                        t.push_str(s);
                        last = Some(*x);
                    }
                    t.split_whitespace().collect::<Vec<_>>().join(" ")
                };
                if let (Some(a), Some(b)) = (piece.first(), piece.last())
                    && !text.is_empty()
                {
                    out.push((a.0, (a.1 / 72.0) as f32, ((b.1 + advance) / 72.0) as f32, text));
                }
                start = i + 1;
            }
        }
        row.clear();
    };
    for g in glyphs {
        if row.last().is_some_and(|l| (l.0 - g.0).abs() >= 1.0) {
            flush(&mut row, &mut out);
        }
        row.push(g);
    }
    flush(&mut row, &mut out);
    out
}

/// The title page on a first page, if it is one: no scene heading, no cue with speech under it,
/// and not too many lines. Centered blocks are, in order, the title, then a credit ("… by"),
/// the authors, the source ("Based on …") and notes; at the foot, the left side is the contact
/// and the right side the draft date.
fn title_page_of(segs: &[Segment]) -> Option<wordsmith_script::title_page::TitlePage> {
    use crate::fountain::{is_upper, scene_heading};
    if segs.is_empty() || segs.len() > 40 || segs.iter().any(|s| scene_heading(&s.3).is_some()) {
        return None;
    }
    // Speech: a capitals line in the cue column with a line in the dialogue column under it.
    let speech = segs.windows(2).any(|w| (3.4..4.6).contains(&w[0].1) && is_upper(&w[0].3) && (2.0..2.8).contains(&w[1].1));
    if speech {
        return None;
    }
    let centered = |s: &Segment| ((s.1 + s.2) / 2.0 - 4.25).abs() < 0.5 && s.1 > 1.6;
    let mut t = wordsmith_script::title_page::TitlePage::default();
    // Centered lines in blocks separated by blank lines.
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut last_y: Option<f64> = None;
    for s in segs.iter().filter(|s| centered(s)) {
        match (blocks.last_mut(), last_y) {
            (Some(b), Some(y)) if y - s.0 < 18.0 => b.push(&s.3),
            _ => blocks.push(vec![&s.3]),
        }
        last_y = Some(s.0);
    }
    let mut it = blocks.into_iter().peekable();
    if let Some(b) = it.next() {
        t.set("title", &b.join("\n"));
    }
    let mut notes: Vec<String> = Vec::new();
    while let Some(b) = it.next() {
        let text = b.join("\n");
        let lower = text.to_lowercase();
        if t.credit.is_empty() && t.authors.is_empty() && (lower.ends_with(" by") || lower == "by") {
            t.set("credit", &text);
            if let Some(a) = it.next() {
                t.set("authors", &a.join("\n"));
            }
        } else if t.source.is_empty() && (lower.starts_with("based on") || lower.starts_with("adapted from") || lower.starts_with("from ")) {
            t.set("source", &text);
        } else if t.authors.is_empty() && t.credit.is_empty() {
            t.set("authors", &text);
        } else {
            notes.push(text);
        }
    }
    t.set("notes", &notes.join("\n\n"));
    let left: Vec<&str> = segs.iter().filter(|s| !centered(s) && s.1 < 4.0).map(|s| s.3.as_str()).collect();
    let right: Vec<&str> = segs.iter().filter(|s| !centered(s) && s.1 >= 4.0).map(|s| s.3.as_str()).collect();
    t.set("contact", &left.join("\n"));
    t.set("draftDate", &right.join("\n"));
    Some(t)
}
