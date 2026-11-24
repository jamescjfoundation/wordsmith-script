// SPDX-License-Identifier: MIT OR Apache-2.0
//! Text-based PDF import: each page's glyphs are read with craft-pdf's guarded interpreter,
//! grouped into lines by baseline, and placed by their left edge. A scanned script has no
//! glyphs and is refused with a clear message.

use jcj_doc::Document;
use jcj_output::read::{GlyphMode, ReadLimits, read_pages};

use super::{ImportError, Line, MAX_IMPORT_BYTES, MAX_LINES, classify};

/// Most pages read.
pub const MAX_PAGES: usize = 5_000;
/// Most glyphs kept from one page.
const MAX_GLYPHS: usize = 200_000;

/// A page's visible glyphs that have text: (baseline y, x, text), in drawing order.
fn page_glyphs(page: &jcj_output::read::PdfPage) -> Vec<(f64, f64, String)> {
    page.glyphs
        .iter()
        .filter(|g| g.mode != GlyphMode::Invisible && g.x.is_finite() && g.y.is_finite())
        .filter_map(|g| g.text.clone().map(|t| (g.y, g.x, t)))
        .collect()
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
            let mut steps: Vec<f64> = row.iter().zip(row.iter().skip(1)).map(|(a, b)| b.0 - a.0).filter(|d| *d > 0.5).collect();
            steps.sort_by(f64::total_cmp);
            let advance = steps.get(steps.len() / 4).copied().unwrap_or(7.2).clamp(2.0, 30.0);
            // Margin marks (revision marks, scene numbers) are not script text: a short run of
            // text left of the action column or right of the right margin, set apart by a gap.
            let row = without_margin_marks(row, advance);
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

/// Most characters a margin mark has.
const MARGIN_MARK_CHARS: usize = 6;

/// `row` (sorted left to right) without margin marks: text ending left of 1.3 in, or starting
/// right of 7.52 in, where script text never goes, when there are at most six characters of it.
fn without_margin_marks(row: Vec<(f64, String)>, advance: f64) -> Vec<(f64, String)> {
    let left = |g: &(f64, String)| g.0 + advance <= 1.3 * 72.0;
    let right = |g: &(f64, String)| g.0 >= 7.52 * 72.0;
    let count = |f: &dyn Fn(&(f64, String)) -> bool| row.iter().filter(|g| f(g)).map(|g| g.1.chars().count()).sum::<usize>();
    let (drop_left, drop_right) = (count(&left) <= MARGIN_MARK_CHARS, count(&right) <= MARGIN_MARK_CHARS);
    row.into_iter().filter(|g| !(drop_left && left(g)) && !(drop_right && right(g))).collect()
}

/// Import a text-based PDF as a script, placing each line by its left edge.
pub fn pdf(bytes: &[u8]) -> Result<Document, ImportError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge);
    }
    // The core's PDF reader (`jcj_output::read`, craft-libs' guarded interpreter underneath).
    let limits = ReadLimits { max_input_bytes: MAX_IMPORT_BYTES, max_pages: MAX_PAGES, max_glyphs_per_page: MAX_GLYPHS };
    let read = read_pages(bytes, &limits).map_err(|e| ImportError::NotPdf(e.to_string()))?;
    let mut pages: Vec<Vec<(f64, f64, String)>> = Vec::new();
    let mut first_page: Vec<Segment> = Vec::new();
    let mut count = 0;
    for page in &read {
        let glyphs = page_glyphs(page);
        if pages.is_empty() {
            first_page = segments(glyphs.clone());
        }
        let rows = page_lines(glyphs);
        count += rows.len();
        pages.push(rows);
        if count >= MAX_LINES {
            break;
        }
    }
    // The script's line pitch: a low distance between baselines across the whole script (one
    // page may have only single-line paragraphs, a blank line apart).
    let mut pitches: Vec<f64> =
        pages.iter().flat_map(|rows| rows.iter().zip(rows.iter().skip(1)).map(|(a, b)| a.0 - b.0)).filter(|d| *d > 2.0).collect();
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
        let mut steps: Vec<f64> = row.iter().zip(row.iter().skip(1)).map(|(a, b)| b.1 - a.1).filter(|d| *d > 0.5).collect();
        steps.sort_by(f64::total_cmp);
        let advance = steps.get(steps.len() / 4).copied().unwrap_or(7.2).clamp(2.0, 30.0);
        let mut start = 0;
        for (i, cur) in row.iter().enumerate() {
            let split = row.get(i + 1).is_none_or(|n| n.1 - cur.1 > advance * 6.0);
            if split {
                let piece = row.get(start..=i).unwrap_or_default();
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
    let speech = segs.iter().zip(segs.iter().skip(1)).any(|(a, b)| (3.4..4.6).contains(&a.1) && is_upper(&a.3) && (2.0..2.8).contains(&b.1));
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
