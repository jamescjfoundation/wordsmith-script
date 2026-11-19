// SPDX-License-Identifier: MIT OR Apache-2.0
//! Filling pages under the screenplay rules (decision 0012):
//!
//! - space before is dropped at the top of a page;
//! - start new page starts one;
//! - keep-with-next holds a paragraph with the start of the next (a Scene Heading is never left
//!   alone at the foot of a page; a Character keeps its Parenthetical and first Dialogue lines);
//! - Action, Dialogue and General split only after a sentence, leaving at least
//!   [`Rules::min_before`] lines and carrying at least [`Rules::min_after`];
//! - a speech broken across pages gets (MORE) at the foot and the name with (CONT'D) at the
//!   head of the next page;
//! - optionally, a scene broken across pages gets (CONTINUED) and CONTINUED:.
//!
//! A paragraph taller than a whole page is split at a line break as a last resort, so pagination
//! always ends.

use wordsmith_doc::Document;

use super::measure::{Measured, measure_text};
use super::{MoresContinueds, Rules};
use crate::element::ElementKind;
use crate::template::LINE;

/// One thing on a page, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// Bytes `start..end` of body paragraph `block` (the whole paragraph, or part of a split one).
    Para { block: usize, start: usize, end: usize, lines: usize, space_before: bool },
    /// (MORE) at the foot of a page whose speech goes on.
    More,
    /// The speaker's name with (CONT'D), heading the rest of a speech; `block` is the Character.
    CharacterContd { block: usize },
    /// CONTINUED: at the head of a page whose scene began earlier; `page` is the scene's page
    /// count so far (2 on the scene's second page).
    ContinuedTop { page: usize },
    /// (CONTINUED) at the foot of a page whose scene goes on.
    ContinuedBottom,
}

/// A paragraph, or what is left of one, waiting to be placed.
#[derive(Clone, Debug)]
struct Item {
    idx: usize,
    start: usize,
    lines: Vec<(usize, usize)>,
    heights: Vec<f32>,
    /// Space before applies (not for the rest of a split paragraph).
    space_before: bool,
}

/// Most pages a script may have. Bounds hostile input.
pub const MAX_PAGES: usize = 100_000;

fn splittable(kind: ElementKind) -> bool {
    matches!(kind, ElementKind::Action | ElementKind::Dialogue | ElementKind::General | ElementKind::Custom)
}

/// Byte offsets in `text` just after each sentence end: `.`, `!` or `?` (and `...`), with any
/// closing quotes or brackets, followed by a space.
pub fn sentence_ends(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (_, c) = chars[i];
        if matches!(c, '.' | '!' | '?') {
            let mut j = i + 1;
            while j < chars.len() && matches!(chars[j].1, '.' | '!' | '?' | '"' | '\'' | '”' | '’' | ')' | ']') {
                j += 1;
            }
            if j < chars.len() && chars[j].1.is_whitespace() {
                out.push(chars[j].0);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Which speech each paragraph belongs to: the index (in `m`) of its Character, for the
/// Character itself and the Parentheticals and Dialogue that follow it.
fn speeches(m: &[Measured]) -> Vec<Option<usize>> {
    let mut out = vec![None; m.len()];
    let mut current = None;
    for (i, p) in m.iter().enumerate() {
        current = match p.rules {
            ElementKind::Character => Some(i),
            ElementKind::Dialogue | ElementKind::Parenthetical => current,
            _ => None,
        };
        out[i] = current;
    }
    out
}

/// Which scene each paragraph belongs to (the index of its Scene Heading), if any.
fn scenes(m: &[Measured]) -> Vec<Option<usize>> {
    let mut current = None;
    m.iter()
        .enumerate()
        .map(|(i, p)| {
            if p.kind == ElementKind::SceneHeading {
                current = Some(i);
            }
            current
        })
        .collect()
}

struct Planner<'a> {
    doc: &'a Document,
    m: &'a [Measured],
    marks: &'a MoresContinueds,
    rules: Rules,
    cap: f32,
    speech: Vec<Option<usize>>,
    scene: Vec<Option<usize>>,
    pages: Vec<Vec<Piece>>,
    cur: Vec<Piece>,
    /// For each piece on the current page: the item it placed whole and the height it used, so a
    /// keep-with-next paragraph can follow its partner to the next page.
    placed: Vec<Option<(Item, f32)>>,
    used: f32,
    /// The last paragraph placed (index in `m`).
    last: Option<usize>,
    /// Pages each scene has started on so far, by heading index.
    scene_pages: std::collections::HashMap<usize, usize>,
}

impl Planner<'_> {
    /// Usable height: the page body, less a line for (CONTINUED) when scene continueds are on.
    fn room(&self) -> f32 {
        self.cap - if self.marks.scene { LINE } else { 0.0 } - self.used
    }

    fn top(&self) -> bool {
        self.cur.iter().all(|p| !matches!(p, Piece::Para { .. }))
    }

    fn sb(&self, item: &Item) -> f32 {
        if item.space_before && !self.top() { self.m.get(item.idx).map_or(0.0, |p| p.space_before) } else { 0.0 }
    }

    /// The least height paragraph `j` needs to start on this page (not at its top), with whatever
    /// it must keep with.
    fn min_start(&self, j: usize, depth: usize) -> f32 {
        let Some(p) = self.m.get(j) else { return 0.0 };
        let all: f32 = p.height();
        let body = if splittable(p.rules) && p.lines.len() > self.rules.min_before + self.rules.min_after {
            let first: f32 = p.heights.iter().take(self.rules.min_before).sum();
            first + if self.continues_speech(j) || p.rules == ElementKind::Dialogue { LINE } else { 0.0 }
        } else {
            all
        };
        let keep = if p.keep_next && depth < 4 { self.min_start(j + 1, depth + 1) } else { 0.0 };
        p.space_before + body + keep
    }

    /// Paragraph `i` is in a speech that goes on after it.
    fn continues_speech(&self, i: usize) -> bool {
        matches!((self.speech.get(i), self.speech.get(i + 1)), (Some(Some(a)), Some(Some(b))) if a == b && self.m.get(i + 1).is_some_and(|p| p.rules != ElementKind::Character))
    }

    /// Place the first `end_line` lines of `item`, ending at byte `end` (the paragraph's end
    /// when it is placed whole).
    fn place(&mut self, item: &Item, end_line: usize, end: usize) {
        let Some(p) = self.m.get(item.idx) else { return };
        let end_line = end_line.min(item.lines.len());
        let h: f32 = item.heights.iter().take(end_line).sum();
        let sb = self.sb(item);
        let end = end.min(p.text.len()).max(item.start);
        self.cur.push(Piece::Para { block: p.block, start: item.start, end, lines: end_line, space_before: sb > 0.0 });
        let whole = end_line >= item.lines.len();
        self.placed.push(whole.then(|| (item.clone(), sb + h)));
        self.used += sb + h;
        self.last = Some(item.idx);
    }

    /// End the page before item `next` (index in `m`), adding the marks a broken speech or scene
    /// needs on both pages.
    fn break_page(&mut self, next: usize) {
        let in_speech = self.last.is_some_and(|l| {
            self.marks.dialogue
                && matches!((self.speech.get(l), self.speech.get(next)), (Some(Some(a)), Some(Some(b))) if a == b)
                && self.m.get(next).is_some_and(|p| p.rules != ElementKind::Character)
        });
        let in_scene = self.last.is_some_and(|l| {
            self.marks.scene
                && matches!((self.scene.get(l), self.scene.get(next)), (Some(Some(a)), Some(Some(b))) if a == b)
                && self.m.get(next).is_some_and(|p| p.kind != ElementKind::SceneHeading)
        });
        if in_speech {
            self.cur.push(Piece::More);
        }
        if in_scene {
            self.cur.push(Piece::ContinuedBottom);
        }
        self.pages.push(std::mem::take(&mut self.cur));
        self.placed.clear();
        self.used = 0.0;
        if in_scene && let Some(Some(heading)) = self.scene.get(next) {
            let n = self.scene_pages.entry(*heading).or_insert(1);
            *n += 1;
            self.cur.push(Piece::ContinuedTop { page: *n });
            self.placed.push(None);
            self.used += LINE;
        }
        if in_speech && let Some(Some(character)) = self.speech.get(next) {
            let block = self.m.get(*character).map_or(0, |p| p.block);
            self.cur.push(Piece::CharacterContd { block });
            self.placed.push(None);
            self.used += self.m.get(*character).map_or(LINE, |p| p.height());
        }
    }

    /// Where paragraph `item` can be split so its first part fits in `avail`: the number of its
    /// lines kept here and the byte offset the rest starts at. Only after a sentence, with the
    /// minimum lines on each side; `forced` (a paragraph taller than a page) allows any line.
    fn split_point(&self, item: &Item, avail: f32, forced: bool) -> Option<(usize, usize, usize)> {
        let p = self.m.get(item.idx)?;
        let more = if p.rules == ElementKind::Dialogue || self.continues_speech(item.idx) { LINE } else { 0.0 };
        let avail = avail - more;
        let mut fit = 0;
        let mut h = 0.0;
        for lh in &item.heights {
            if h + lh > avail + 0.01 {
                break;
            }
            h += lh;
            fit += 1;
        }
        if fit == 0 {
            return None;
        }
        let line_of = |b: usize| item.lines.iter().rposition(|(s, _)| *s <= b).unwrap_or(0);
        let total = item.lines.len();
        let sentence = sentence_ends(&p.text)
            .into_iter()
            .filter(|&b| b > item.start && b < p.text.len())
            .filter(|&b| {
                let l = line_of(b);
                // The boundary's line stays on this page.
                let kept = l + 1;
                kept <= fit && kept >= self.rules.min_before.min(total) && total.saturating_sub(l) >= self.rules.min_after
            })
            .max();
        if let Some(b) = sentence {
            let kept = line_of(b) + 1;
            let rest = p.text.get(b..).map_or(b, |r| b + (r.len() - r.trim_start().len()));
            // The first part ends after the sentence, without the space that followed it.
            let end = p.text.get(..b).map_or(b, |s| s.trim_end().len());
            return Some((kept, end, rest));
        }
        if forced {
            let kept = fit.min(total.saturating_sub(1)).max(1);
            let rest = item.lines.get(kept).map_or(p.text.len(), |l| l.0);
            let end = p.text.get(..rest).map_or(rest, |s| s.trim_end().len());
            return Some((kept, end, rest));
        }
        None
    }

    fn run(mut self) -> Vec<Vec<Piece>> {
        let mut queue: std::collections::VecDeque<Item> = self
            .m
            .iter()
            .enumerate()
            .map(|(idx, p)| Item { idx, start: 0, lines: p.lines.clone(), heights: p.heights.clone(), space_before: true })
            .collect();
        let mut guard = 0usize;
        while let Some(item) = queue.pop_front() {
            guard += 1;
            if self.pages.len() >= MAX_PAGES || guard > 10 * MAX_PAGES {
                break;
            }
            let Some(p) = self.m.get(item.idx) else { continue };
            if p.page_break_before && item.start == 0 && !self.top() {
                self.break_page(item.idx);
            }
            let sb = self.sb(&item);
            let whole: f32 = item.heights.iter().sum();
            let speech_reserve = if self.continues_speech(item.idx) { LINE } else { 0.0 };
            let keep = if p.keep_next { self.min_start(item.idx + 1, 0) } else { speech_reserve };
            if sb + whole + keep <= self.room() + 0.01 || (self.top() && sb + whole <= self.room() + 0.01) {
                self.place(&item, item.lines.len(), usize::MAX);
                continue;
            }
            let top = self.top();
            // Split here, if the rules allow it and nothing must stay with this paragraph.
            let can_split = splittable(p.rules) && !p.keep_next;
            let forced = top && whole > self.room();
            if can_split || forced {
                let avail = self.room() - sb;
                if let Some((kept, end, rest)) = self.split_point(&item, avail, forced) {
                    self.place(&item, kept, end);
                    let src = self.doc.story(wordsmith_doc::StoryRef::Body).and_then(|b| b.get(p.block)).and_then(|b| b.as_para());
                    let remainder = p.text.get(rest..).unwrap_or("");
                    let (lines, heights) = match src {
                        Some(sp) => measure_text(self.doc, sp, remainder),
                        None => (vec![(0, remainder.len())], vec![LINE]),
                    };
                    let lines = lines.into_iter().map(|(s, e)| (s + rest, e + rest)).collect();
                    self.break_page(item.idx);
                    queue.push_front(Item { idx: item.idx, start: rest, lines, heights, space_before: false });
                    continue;
                }
            }
            if top {
                // Nothing fits and nothing can split: place it anyway rather than loop.
                self.place(&item, item.lines.len(), usize::MAX);
                continue;
            }
            // Paragraphs kept with this one (a Character, a Scene Heading) go with it, unless that
            // would leave the page without a paragraph.
            let mut moved = vec![item];
            while let (Some(Some((prev, h))), Some(Piece::Para { .. })) = (self.placed.last(), self.cur.last()) {
                let keeps = self.m.get(prev.idx).is_some_and(|p| p.keep_next) && prev.idx + 1 == moved[0].idx;
                let paras_here = self.cur.iter().filter(|p| matches!(p, Piece::Para { .. })).count();
                if !keeps || paras_here <= 1 {
                    break;
                }
                let (prev, h) = (prev.clone(), *h);
                self.cur.pop();
                self.placed.pop();
                self.used -= h;
                self.last = self.placed.iter().rev().find_map(|p| p.as_ref().map(|(it, _)| it.idx));
                moved.insert(0, prev);
            }
            let first = moved.first().map_or(0, |i| i.idx);
            self.break_page(first);
            for it in moved.into_iter().rev() {
                queue.push_front(it);
            }
        }
        if !self.cur.is_empty() || self.pages.is_empty() {
            self.pages.push(self.cur);
        }
        self.pages
    }
}

/// Plan the pages of `doc` from its measured paragraphs.
pub fn plan(doc: &Document, m: &[Measured], marks: &MoresContinueds, rules: Rules) -> Vec<Vec<Piece>> {
    let s = &doc.last_section;
    let cap = (s.page_h - s.margin_top - s.margin_bottom).max(LINE);
    Planner {
        doc,
        m,
        marks,
        rules,
        cap,
        speech: speeches(m),
        scene: scenes(m),
        pages: Vec::new(),
        cur: Vec::new(),
        placed: Vec::new(),
        used: 0.0,
        last: None,
        scene_pages: std::collections::HashMap::new(),
    }
    .run()
}
