// SPDX-License-Identifier: MIT OR Apache-2.0
//! Imports into a script: plain text, RTF and text-based PDF. Decision 0019 records the rules.
//!
//! Every source becomes a list of [`Line`]s: the text, whether a blank line (or a gap as tall)
//! comes before it, and, when the source says where the text sits, its left edge in inches from
//! the page's left edge. Lines with positions are classified by column, using the template's
//! element positions (so a script this app printed or exported comes back as the same
//! elements); lines without are classified by convention (scene-heading intros, a name in
//! capitals with speech under it, parentheticals, transitions ending in `TO:`). Text the writer
//! says is not a script becomes one Action paragraph per paragraph of text.
//!
//! Marks that pagination adds are taken out: `(MORE)`, `(CONTINUED)`, `CONTINUED:` and a cue's
//! `(CONT'D)`; a speech split across a page by `(MORE)` is joined again.

mod pdf;
#[cfg(test)]
mod tests;

use std::fmt;

use jcj_doc::{Document, para_block};
use wordsmith_script::template::{new_script, paragraph};
use wordsmith_script::{ElementKind, Elements};

use crate::fountain::{is_upper, scene_heading};

pub use pdf::pdf;

/// Largest file imported, bytes.
pub const MAX_IMPORT_BYTES: usize = 32 << 20;
/// Most lines read from one source.
pub const MAX_LINES: usize = 1_000_000;

/// Why an import failed.
#[derive(Clone, Debug, PartialEq)]
pub enum ImportError {
    TooLarge,
    NotRtf,
    NotPdf(String),
    /// A PDF with no text in it, as a scanned script is.
    NoText,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::TooLarge => write!(f, "the file is larger than {} MB", MAX_IMPORT_BYTES >> 20),
            ImportError::NotRtf => write!(f, "this is not an RTF file"),
            ImportError::NotPdf(why) => write!(f, "the PDF could not be read: {why}"),
            ImportError::NoText => write!(f, "the PDF has no text to import (a scanned script needs text recognition first)"),
        }
    }
}

impl std::error::Error for ImportError {}

/// One line of a source.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    /// Inches from the page's left edge, when the source says where the text sits.
    pub left: Option<f32>,
    pub text: String,
    /// A blank line, or a gap as tall, comes before it.
    pub gap_before: bool,
}

/// The column a positioned line sits in, by the template's element positions (left edges at
/// 1.5, 2.5, 3.1 and 3.7 inches; transitions end at the right margin).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    Full,
    Dialogue,
    Parenthetical,
    Cue,
    Right,
}

fn column(left: f32) -> Column {
    match left {
        l if l < 2.0 => Column::Full,
        l if l < 2.8 => Column::Dialogue,
        l if l < 3.4 => Column::Parenthetical,
        l if l < 4.6 => Column::Cue,
        _ => Column::Right,
    }
}

/// The name in a cue, without its extension.
fn name_of(cue: &str) -> &str {
    cue.split('(').next().unwrap_or("").trim()
}

fn is_cue(t: &str) -> bool {
    let name = name_of(t);
    !name.is_empty() && is_upper(name) && name.chars().count() <= 50 && scene_heading(t).is_none()
}

/// A heading with no intro, as a laid-out script shows one: a paragraph in capitals in the
/// action column, without closing punctuation ("BOARDING HOUSE STEPS", but not "BOOM!" or "FADE IN:").
fn bare_heading(t: &str) -> bool {
    is_upper(t) && !t.ends_with(['.', '!', '?', ':', ';', ',', '"', '\'', '\u{201d}', '-'])
}

/// A cue with an extension in brackets ("ANNA (V.O.)"), which is never a heading.
fn is_cue_with_extension(t: &str) -> bool {
    t.ends_with(')') && t.contains('(') && is_cue(t)
}

fn is_transition(t: &str) -> bool {
    is_upper(t) && t.ends_with("TO:")
}

/// A cue without the `(CONT'D)` pagination adds (any case, straight or curly apostrophe).
fn without_contd(cue: &str) -> String {
    let mut out = cue.trim().to_string();
    for mark in ["(CONT'D)", "(CONT\u{2019}D)", "(CONTD)", "(CONT.)", "(CONTINUING)"] {
        if out.len() >= mark.len()
            && let Some(tail) = out.get(out.len() - mark.len()..)
            && tail.eq_ignore_ascii_case(mark)
        {
            out.truncate(out.len() - mark.len());
            out = out.trim_end().to_string();
        }
    }
    out
}

fn is_page_number(t: &str) -> bool {
    let core = t.trim_end_matches('.');
    !core.is_empty() && core.len() <= 6 && core.chars().next().is_some_and(|c| c.is_ascii_digit()) && core.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Takes out the marks pagination adds and page numbers, and joins a speech split by `(MORE)`.
fn drop_marks(lines: Vec<Line>, page_numbers: bool) -> Vec<Line> {
    let shift = margin_shift(&lines);
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    let mut after_more = false;
    let mut join_next = false;
    for mut l in lines {
        let t = l.text.trim();
        let upper = t.to_uppercase();
        if upper == "(MORE)" {
            after_more = true;
            continue;
        }
        if upper == "(CONTINUED)" || upper.starts_with("CONTINUED:") && upper.len() <= 20 {
            continue;
        }
        // A page number: any in a PDF; in text, one at the right where the page's number goes.
        if is_page_number(t) && (page_numbers || l.left.is_some_and(|x| x - shift >= 6.0)) {
            continue;
        }
        if after_more && is_cue(t) {
            // The speaker's name again at the head of the next page: the speech goes on.
            after_more = false;
            join_next = true;
            continue;
        }
        after_more = false;
        if join_next {
            l.gap_before = false;
            join_next = false;
        }
        out.push(l);
    }
    out
}

/// How far the source's action column is from the template's: the leftmost text, when a scene
/// heading there shows it is the action column; otherwise none (positions are taken as they
/// are, so a fragment of dialogue stays dialogue).
fn margin_shift(lines: &[Line]) -> f32 {
    let min = lines.iter().filter(|l| !l.text.trim().is_empty()).filter_map(|l| l.left).fold(f32::INFINITY, f32::min);
    let heading_there = lines.iter().any(|l| l.left.is_some_and(|x| (x - min).abs() < 0.05) && scene_heading(l.text.trim()).is_some());
    if min.is_finite() && heading_there { min - 1.5 } else { 0.0 }
}

/// Paragraphs from lines with positions, by column.
fn by_position(lines: &[Line]) -> Vec<(ElementKind, String)> {
    let shift = margin_shift(lines);
    let mut out: Vec<(ElementKind, String, Column)> = Vec::new();
    let lines: Vec<&Line> = lines.iter().filter(|l| !l.text.trim().is_empty()).collect();
    for l in lines {
        let t = l.text.trim();
        let col = column(l.left.unwrap_or(1.5) - shift);
        if !l.gap_before
            && let Some((kind, text, c)) = out.last_mut()
            && *c == col
            && !matches!(kind, ElementKind::Character | ElementKind::Transition)
        {
            // A wrapped line of the same paragraph.
            join(text, t);
            continue;
        }
        let prev = out.last().map(|p| p.0);
        let kind = match col {
            Column::Full if (l.gap_before || prev.is_none()) && scene_heading(t).is_some() => ElementKind::SceneHeading,
            Column::Full if is_transition(t) => ElementKind::Transition,
            Column::Full => ElementKind::Action,
            Column::Dialogue => ElementKind::Dialogue,
            Column::Parenthetical if t.starts_with('(') => ElementKind::Parenthetical,
            Column::Parenthetical => ElementKind::Dialogue,
            Column::Cue if is_cue(t) => ElementKind::Character,
            Column::Cue => ElementKind::Action,
            Column::Right if is_upper(t) => ElementKind::Transition,
            Column::Right => ElementKind::Action,
        };
        let text = if kind == ElementKind::Character { without_contd(t) } else { t.to_string() };
        out.push((kind, text, col));
    }
    out.into_iter()
        .map(|(k, t, c)| {
            // A heading with no intro reads as Action until its whole paragraph is known.
            let heading = k == ElementKind::Action && c == Column::Full && t.chars().count() <= 120 && bare_heading(&t);
            (if heading { ElementKind::SceneHeading } else { k }, t)
        })
        .collect()
}

/// Paragraphs from lines without positions, by convention. Blank lines separate paragraphs. A
/// line in capitals on its own without closing punctuation is a heading with no intro, as on a
/// laid-out page.
fn by_convention(lines: &[Line]) -> Vec<(ElementKind, String)> {
    let mut out = Vec::new();
    for block in blocks(lines) {
        let Some(first) = block.first() else { continue };
        if scene_heading(first).is_some() && is_upper(first) {
            // A long heading wraps: the lines in capitals after it are still the heading.
            let k = 1 + block.iter().skip(1).take_while(|l| is_upper(l)).count();
            let (heading, rest) = block.split_at(k.min(block.len()));
            out.push((ElementKind::SceneHeading, joined(heading)));
            if !rest.is_empty() {
                out.push((ElementKind::Action, joined(rest)));
            }
        } else if block.len() == 1 && is_transition(first) {
            out.push((ElementKind::Transition, first.to_string()));
        } else if block.len() == 1 && bare_heading(first) && !is_cue_with_extension(first) {
            out.push((ElementKind::SceneHeading, first.to_string()));
        } else if block.len() > 1 && is_cue(first) {
            out.push((ElementKind::Character, without_contd(first)));
            let mut in_paren = false;
            for line in block.iter().skip(1) {
                let continues = matches!(out.last(), Some((k, _)) if *k == ElementKind::Dialogue && !line.starts_with('(') || *k == ElementKind::Parenthetical && in_paren);
                if continues && let Some((_, text)) = out.last_mut() {
                    join(text, line);
                } else if line.starts_with('(') {
                    out.push((ElementKind::Parenthetical, line.to_string()));
                } else {
                    out.push((ElementKind::Dialogue, line.to_string()));
                }
                if line.starts_with('(') {
                    in_paren = true;
                }
                if line.ends_with(')') {
                    in_paren = false;
                }
            }
        } else {
            out.push((ElementKind::Action, joined(&block)));
        }
    }
    out
}

/// Join a wrapped line to the text before it: with a space, except after a dash attached to a
/// word ("MEET--" then "HELPS"), where the line was broken without one.
fn join(text: &mut String, line: &str) {
    let mut tail = text.chars().rev();
    let attached_dash = matches!(tail.next(), Some('-' | '\u{2014}' | '\u{2013}')) && tail.next().is_some_and(|c| !c.is_whitespace());
    if !attached_dash && !text.is_empty() {
        text.push(' ');
    }
    text.push_str(line);
}

/// Text of a block's lines joined as wrapped lines.
fn joined(block: &[&str]) -> String {
    let mut out = String::new();
    for line in block {
        join(&mut out, line);
    }
    out
}

/// Runs of non-blank lines, trimmed.
fn blocks(lines: &[Line]) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = Vec::new();
    for l in lines {
        let t = l.text.trim();
        if t.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(b) if !l.gap_before => b.push(t),
            _ => out.push(vec![t]),
        }
    }
    out
}

/// A script holding `paras`, in each element's casing.
fn script_of(paras: Vec<(ElementKind, String)>) -> Document {
    let mut doc = new_script();
    let elements = Elements::of_doc(&doc);
    let body: Vec<_> = paras
        .into_iter()
        .take(crate::native::MAX_PARAGRAPHS)
        .map(|(kind, text)| match elements.of_kind(kind) {
            Some(def) => para_block(paragraph(&def.id, &def.casing.apply(&text))),
            None => para_block(paragraph(elements.id_of(kind), &text)),
        })
        .collect();
    if !body.is_empty() {
        doc.body = body;
    }
    doc
}

/// Classify `lines` (as a script, or not) into a document. `laid_out`: the lines come from
/// printed pages (a PDF), so their positions always count and lone numbers are page numbers.
fn classify(lines: Vec<Line>, as_script: bool, laid_out: bool) -> Document {
    if !as_script {
        return script_of(blocks(&lines).into_iter().map(|b| (ElementKind::Action, joined(&b))).collect());
    }
    let lines = drop_marks(lines, laid_out);
    if laid_out || positioned(&lines) { script_of(by_position(&lines)) } else { script_of(by_convention(&lines)) }
}

/// Whether the lines' positions say anything: enough of them sit right of the leftmost text.
fn positioned(lines: &[Line]) -> bool {
    let lefts: Vec<f32> = lines.iter().filter(|l| !l.text.trim().is_empty()).filter_map(|l| l.left).collect();
    let Some(min) = lefts.iter().copied().reduce(f32::min) else { return false };
    let indented = lefts.iter().filter(|l| **l - min >= 0.75).count();
    indented >= 3 || (indented > 0 && indented * 20 >= lefts.len())
}

/// Lines of plain text: tabs expanded to every 8 columns, positions at 10 characters per inch.
fn text_lines(src: &str) -> Vec<Line> {
    let src = src.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace(['\r', '\u{c}'], "\n");
    let mut out = Vec::new();
    let mut gap = true;
    for raw in src.split('\n').take(MAX_LINES) {
        let mut expanded = String::with_capacity(raw.len());
        for c in raw.chars() {
            if c == '\t' {
                let n = 8 - expanded.chars().count() % 8;
                expanded.extend(std::iter::repeat_n(' ', n));
            } else if !c.is_control() {
                expanded.push(c);
            }
        }
        let trimmed = expanded.trim();
        if trimmed.is_empty() {
            gap = true;
            continue;
        }
        let col = expanded.chars().take_while(|c| *c == ' ').count();
        out.push(Line { left: Some(1.5 + col as f32 / 10.0), text: trimmed.to_string(), gap_before: gap });
        gap = false;
    }
    out
}

/// Import plain text. `as_script`: the writer's answer to "is this a script?" (see
/// [`looks_like_script`] for a default).
pub fn text(src: &str, as_script: bool) -> Result<Document, ImportError> {
    if src.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge);
    }
    Ok(classify(text_lines(src), as_script, false))
}

/// A guess at the answer to "is this a script?": scene headings, transitions, or names in
/// capitals with speech under them.
pub fn looks_like_script(src: &str) -> bool {
    let lines: Vec<Line> = text_lines(src.get(..src.len().min(1 << 20)).unwrap_or(src)).into_iter().map(|l| Line { left: None, ..l }).collect();
    let headings = lines.iter().filter(|l| l.gap_before && is_upper(&l.text) && scene_heading(&l.text).is_some()).count();
    let found = by_convention(&lines);
    let cues = found.iter().filter(|(k, _)| *k == ElementKind::Character).count();
    let transitions = found.iter().filter(|(k, _)| *k == ElementKind::Transition).count();
    headings >= 1 || cues >= 3 || transitions >= 2
}

/// An RTF paragraph as the import reads it: its text and, in twips, its left and first-line
/// indents and space before.
struct RtfPara {
    text: String,
    left_indent: i32,
    first_line_indent: i32,
    space_before: i32,
}

fn rtf_text(p: &RtfPara) -> String {
    p.text.clone()
}

fn twips(pt: f32) -> i32 {
    (pt * 20.0).round().clamp(-1_000_000.0, 1_000_000.0) as i32
}

/// Every paragraph of an RTF file (tables' cells too), through the core's interchange.
fn rtf_paras(bytes: &[u8]) -> Result<Vec<RtfPara>, ImportError> {
    let doc = jcj_interchange::import(jcj_interchange::Format::Rtf, bytes).map_err(|_| ImportError::NotRtf)?;
    let mut out = Vec::new();
    for path in doc.para_paths(jcj_doc::StoryRef::Body) {
        let Some(p) = doc.para(jcj_doc::StoryRef::Body, &path) else { continue };
        let r = doc.styles.resolve_para(&p.props);
        let text: String = p.text.chars().filter(|c| *c != jcj_doc::para::OBJ).collect();
        out.push(RtfPara { text, left_indent: twips(r.indent_left), first_line_indent: twips(r.indent_first), space_before: twips(r.space_before) });
    }
    Ok(out)
}

/// The text of an RTF file, a paragraph per line (for guessing whether it is a script).
pub fn rtf_plain_text(bytes: &[u8]) -> Result<String, ImportError> {
    Ok(rtf_paras(bytes)?.into_iter().map(|p| p.text).collect::<Vec<_>>().join("\n\n"))
}

/// Import RTF. Paragraph indents, when the file has them, place the text (an RTF this app
/// exported comes back as the same elements); otherwise the text is read as plain text.
pub fn rtf(bytes: &[u8], as_script: bool) -> Result<Document, ImportError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge);
    }
    let paras = rtf_paras(bytes)?;
    let indented = paras.iter().filter(|p| p.left_indent + p.first_line_indent >= 720 && !rtf_text(p).trim().is_empty()).count();
    if indented < 3 {
        // Space before of half a line or more is a blank line, as below.
        let plain: Vec<String> = paras.iter().map(|p| if p.space_before >= 120 { format!("\n{}", rtf_text(p)) } else { rtf_text(p) }).collect();
        return text(&plain.join("\n").replace('\u{c}', "\n"), as_script);
    }
    let mut lines = Vec::new();
    let mut gap = true;
    for p in paras.iter().take(MAX_LINES) {
        let left = 1.5 + (p.left_indent + p.first_line_indent).max(0) as f32 / 1440.0;
        let body = rtf_text(p);
        if body.trim().is_empty() {
            gap = true;
            continue;
        }
        // Space before of half a line or more counts as a blank line. Line breaks inside a
        // paragraph continue it.
        gap |= p.space_before >= 120;
        for piece in body.split(['\n', '\u{c}']) {
            if piece.trim().is_empty() {
                continue;
            }
            lines.push(Line { left: Some(left), text: piece.replace('\t', " ").trim().to_string(), gap_before: gap });
            gap = false;
        }
    }
    Ok(classify(lines, as_script, false))
}
