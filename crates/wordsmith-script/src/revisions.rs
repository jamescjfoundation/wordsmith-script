// SPDX-License-Identifier: MIT OR Apache-2.0
//! Revision mode and revision sets (SPEC "Production"), decision 0024.
//!
//! Settings live in `app_data` under [`REVISIONS_KEY`]: the sets (name, color, text styles), the
//! current set's name, whether revision mode is on, the mark and where it sits, and whether PDF
//! pages are colored. Revised text carries a mark (`CharProps::revised`, the set's index), so it
//! moves with the text; a deletion marks its paragraph (the paragraph mark's `revised`). The
//! original draft is White and carries no marks; the sets follow the industry cycle from Blue.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

/// `app_data` key of the revision settings.
pub const REVISIONS_KEY: &str = crate::header_footer::REVISIONS_KEY;
/// Most sets a script keeps.
pub const MAX_SETS: usize = 100;

/// The industry color cycle: names and the colors the page view and PDF use for them.
pub const CYCLE: [(&str, &str); 10] = [
    ("White", "#FFFFFF"),
    ("Blue", "#A9C8F2"),
    ("Pink", "#F6B8C8"),
    ("Yellow", "#FBF09A"),
    ("Green", "#B9E3B0"),
    ("Goldenrod", "#E9C46A"),
    ("Buff", "#F1DFB3"),
    ("Salmon", "#F6A98E"),
    ("Cherry", "#E3707F"),
    ("Tan", "#D5B892"),
];

const ROUNDS: [&str; 10] = ["", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth"];

/// One revision set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RevisionSet {
    pub name: String,
    /// `#RRGGBB`.
    pub color: String,
    pub bold: bool,
    pub underline: bool,
    pub strike: bool,
}

impl Default for RevisionSet {
    fn default() -> Self {
        cycle_set(1)
    }
}

/// The set at position `n` of the cycle (0 is White, 1 Blue, … 10 Second White).
pub fn cycle_set(n: usize) -> RevisionSet {
    let (base, color) = CYCLE[n % CYCLE.len()];
    let round = n / CYCLE.len();
    let name = match ROUNDS.get(round) {
        Some(&"") => base.to_string(),
        Some(r) => format!("{r} {base}"),
        None => format!("{base} {}", round + 1),
    };
    RevisionSet { name, color: color.into(), bold: false, underline: false, strike: false }
}

/// The revision settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Revisions {
    /// The sets made so far, oldest first; marks refer to them by index.
    pub sets: Vec<RevisionSet>,
    /// The current set's name (empty: the original draft).
    pub current: String,
    /// Revision mode: typing and deleting are marked in the current set.
    pub mode: bool,
    /// The margin mark.
    pub mark: String,
    /// Where the mark sits, inches from the page's left edge.
    pub mark_at: f32,
    /// Color whole pages in PDF output with the newest set on them.
    pub color_pages: bool,
    /// Show the margin marks (off: print without marks).
    pub show_marks: bool,
}

impl Default for Revisions {
    fn default() -> Self {
        Revisions { sets: Vec::new(), current: String::new(), mode: false, mark: "*".into(), mark_at: 8.0, color_pages: false, show_marks: true }
    }
}

/// `#RRGGBB` as a color, if it is one.
pub fn parse_color(s: &str) -> Option<wordsmith_doc::props::Rgb> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(wordsmith_doc::props::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

impl Revisions {
    /// The settings `doc` holds, or the defaults; bad values are brought within limits.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(REVISIONS_KEY) else { return Self::default() };
        let mut r: Revisions = serde_json::from_value((**v).clone()).unwrap_or_default();
        r.sets.truncate(MAX_SETS);
        for s in &mut r.sets {
            s.name = s.name.chars().filter(|c| !c.is_control()).take(40).collect();
            if parse_color(&s.color).is_none() {
                s.color = "#FFFFFF".into();
            }
        }
        r.mark = r.mark.chars().find(|c| !c.is_control() && !c.is_whitespace()).map_or("*".into(), |c| c.to_string());
        if !r.mark_at.is_finite() {
            r.mark_at = 8.0;
        }
        r.mark_at = r.mark_at.clamp(0.1, 8.0);
        if r.current_index().is_none() {
            r.current = String::new();
        }
        r
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(REVISIONS_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }

    /// The current set's index, if there is one.
    pub fn current_index(&self) -> Option<u32> {
        self.sets.iter().position(|s| s.name == self.current).and_then(|i| u32::try_from(i).ok())
    }

    /// Add the next set in the cycle and make it current.
    pub fn start_next(&mut self) -> Option<u32> {
        if self.sets.len() >= MAX_SETS {
            return None;
        }
        let set = cycle_set(self.sets.len() + 1);
        self.current = set.name.clone();
        self.sets.push(set);
        u32::try_from(self.sets.len() - 1).ok()
    }

    /// The set marks with index `i` belong to.
    pub fn set(&self, i: u32) -> Option<&RevisionSet> {
        self.sets.get(i as usize)
    }
}

/// The set new text takes now: the current one, in revision mode.
pub fn typing_set(doc: &Document) -> Option<u32> {
    let r = Revisions::of_doc(doc);
    if r.mode { r.current_index() } else { None }
}

/// In revision mode, mark the paragraph at `at` as changed (text was deleted there, or its
/// element changed).
pub fn touch(doc: &mut Document, at: &Pos) {
    let Some(set) = typing_set(doc) else { return };
    if at.story != StoryRef::Body {
        return;
    }
    if let Ok(p) = doc.para_mut(at.story, &at.path) {
        p.mark.revised = Some(set);
        p.touch();
    }
}

fn body_end(doc: &Document) -> Pos {
    let last = doc.body.len().saturating_sub(1);
    Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(last), off: doc.body.get(last).and_then(|b| b.as_para()).map_or(0, |p| p.len()) }
}

fn body_start() -> Pos {
    Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(0), off: 0 }
}

/// Clear marks in `a..b` for which `drop(set)` holds, paragraph marks included.
fn clear_marks(doc: &mut Document, a: &Pos, b: &Pos, drop: &dyn Fn(u32) -> bool) -> Result<(), CmdError> {
    doc.format_range(a, b, &|c| {
        if c.revised.is_some_and(drop) {
            c.revised = None;
        }
    })?;
    let (first, last) = (a.path.last(), b.path.last());
    for i in first..=last {
        if let Some(wordsmith_doc::Block::Para(p)) = doc.body.get_mut(i).map(Arc::make_mut)
            && p.mark.revised.is_some_and(drop)
        {
            p.mark.revised = None;
            p.touch();
        }
    }
    Ok(())
}

/// `revision.settings`: the settings.
fn settings(s: &mut Session, _: &Value) -> CmdResult {
    Ok(serde_json::to_value(Revisions::of_doc(&s.doc)).unwrap_or_default())
}

/// `revision.mode {on?}`: switch revision mode (on, off, or the other way). Turning it on with no
/// set starts the first (Blue).
fn mode(s: &mut Session, v: &Value) -> CmdResult {
    let mut r = Revisions::of_doc(&s.doc);
    r.mode = v.get("on").and_then(Value::as_bool).unwrap_or(!r.mode);
    if r.mode && r.current_index().is_none() {
        r.start_next();
    }
    r.store(&mut s.doc);
    Ok(json!({"mode": r.mode, "current": r.current}))
}

/// `revision.next`: start the next set in the cycle, make it current, and turn revision mode on.
fn next(s: &mut Session, _: &Value) -> CmdResult {
    let mut r = Revisions::of_doc(&s.doc);
    if r.start_next().is_none() {
        return Err(CmdError::Failed(format!("a script keeps at most {MAX_SETS} revision sets")));
    }
    r.mode = true;
    r.store(&mut s.doc);
    Ok(json!({"current": r.current}))
}

/// `revision.setCurrent {name}`: make an existing set current.
fn set_current(s: &mut Session, v: &Value) -> CmdResult {
    let name = p::req_str(v, "name")?;
    let mut r = Revisions::of_doc(&s.doc);
    if !r.sets.iter().any(|x| x.name == name) {
        return Err(CmdError::Params(format!("there is no revision set `{name}`")));
    }
    r.current = name.to_string();
    r.store(&mut s.doc);
    Ok(json!({"current": r.current}))
}

/// `revision.configure {mark?, markAt?, colorPages?, set?: {index, name?, color?, bold?, underline?, strike?}}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut r = Revisions::of_doc(&s.doc);
    if let Some(m) = v.get("mark").and_then(Value::as_str) {
        r.mark = m
            .chars()
            .find(|c| !c.is_whitespace() && !c.is_control())
            .ok_or_else(|| CmdError::Params("`mark` must be a character".into()))?
            .to_string();
    }
    if let Some(x) = v.get("markAt").and_then(Value::as_f64) {
        if !(0.1..=8.0).contains(&x) {
            return Err(CmdError::Params("`markAt` is inches from the page's left edge, 0.1 to 8".into()));
        }
        r.mark_at = x as f32;
    }
    if let Some(b) = v.get("colorPages").and_then(Value::as_bool) {
        r.color_pages = b;
    }
    if let Some(b) = v.get("showMarks").and_then(Value::as_bool) {
        r.show_marks = b;
    }
    if let Some(set) = v.get("set") {
        let i = set
            .get("index")
            .and_then(Value::as_u64)
            .map(|i| i as usize)
            .filter(|i| *i < r.sets.len())
            .ok_or_else(|| CmdError::Params("`set.index` must name a set".into()))?;
        let was_current = r.sets[i].name == r.current;
        let target = &mut r.sets[i];
        if let Some(n) = set.get("name").and_then(Value::as_str) {
            let n: String = n.chars().filter(|c| !c.is_control()).take(40).collect::<String>().trim().to_string();
            if n.is_empty() {
                return Err(CmdError::Params("a set needs a name".into()));
            }
            target.name = n;
        }
        if let Some(c) = set.get("color").and_then(Value::as_str) {
            parse_color(c).ok_or_else(|| CmdError::Params("`color` must be #RRGGBB".into()))?;
            target.color = c.trim().to_uppercase();
        }
        for (key, field) in [("bold", &mut target.bold), ("underline", &mut target.underline), ("strike", &mut target.strike)] {
            if let Some(b) = set.get(key).and_then(Value::as_bool) {
                *field = b;
            }
        }
        if was_current {
            r.current = r.sets[i].name.clone();
        }
        let names: Vec<&String> = r.sets.iter().map(|x| &x.name).collect();
        if names.iter().enumerate().any(|(j, n)| names[..j].contains(n)) {
            return Err(CmdError::Params("two sets cannot share a name".into()));
        }
    }
    r.store(&mut s.doc);
    Ok(serde_json::to_value(&r).unwrap_or_default())
}

/// `revision.markRevised`: mark the selection (or, with a caret, its paragraph) revised in the
/// current set.
fn mark_revised(s: &mut Session, _: &Value) -> CmdResult {
    let mut r = Revisions::of_doc(&s.doc);
    if r.current_index().is_none() {
        r.start_next();
        r.store(&mut s.doc);
    }
    let set = r.current_index();
    let (mut a, mut b) = s.sel.ordered();
    if a == b {
        a.off = 0;
        b.off = s.doc.para_at(&b).map_or(0, |p| p.len());
        touch_with(&mut s.doc, &a, set);
    }
    s.doc.format_range(&a, &b, &|c| c.revised = set)?;
    Ok(json!({"set": r.current}))
}

fn touch_with(doc: &mut Document, at: &Pos, set: Option<u32>) {
    if let Ok(p) = doc.para_mut(at.story, &at.path) {
        p.mark.revised = set;
        p.touch();
    }
}

/// `revision.clearRevised {all?}`: clear revision marks in the selection (or its paragraph), or
/// everywhere. Revision mode stays as it is.
fn clear_revised(s: &mut Session, v: &Value) -> CmdResult {
    let (mut a, mut b) = s.sel.ordered();
    if v.get("all").and_then(Value::as_bool) == Some(true) {
        (a, b) = (body_start(), body_end(&s.doc));
    } else if a == b {
        a.off = 0;
        b.off = s.doc.para_at(&b).map_or(0, |p| p.len());
    }
    clear_marks(&mut s.doc, &a, &b, &|_| true)?;
    Ok(Value::Null)
}

/// `revision.rollback {to}`: go back to set `to` (an empty name: the original draft): later sets'
/// marks are cleared and the sets removed. The text is not changed.
fn rollback(s: &mut Session, v: &Value) -> CmdResult {
    let to = p::req_str(v, "to")?;
    let mut r = Revisions::of_doc(&s.doc);
    let keep = if to.is_empty() {
        0
    } else {
        r.sets.iter().position(|x| x.name == to).map(|i| i + 1).ok_or_else(|| CmdError::Params(format!("there is no revision set `{to}`")))?
    };
    let keep_u = keep as u32;
    let (a, b) = (body_start(), body_end(&s.doc));
    clear_marks(&mut s.doc, &a, &b, &|set| set >= keep_u)?;
    r.sets.truncate(keep);
    r.current = to.to_string();
    if keep == 0 {
        r.mode = false;
    }
    r.store(&mut s.doc);
    Ok(json!({"current": r.current}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("revision.settings", "Revision Settings", "Production", settings).pure(),
        CommandSpec::new("revision.mode", "Revision Mode", "Production", mode).params(r#"{"on"?: bool}"#),
        CommandSpec::new("revision.next", "Next Revision", "Production", next),
        CommandSpec::new("revision.setCurrent", "Set Current Revision", "Production", set_current).params(r#"{"name": string}"#),
        CommandSpec::new("revision.configure", "Revision Options", "Production", configure)
            .params(r##"{"mark"?: char, "markAt"?: inches, "colorPages"?: bool, "showMarks"?: bool, "set"?: {"index": number, "name"?, "color"?: "#RRGGBB", "bold"?, "underline"?, "strike"?}}"##),
        CommandSpec::new("revision.markRevised", "Mark Revised", "Production", mark_revised),
        CommandSpec::new("revision.clearRevised", "Clear Revised", "Production", clear_revised).params(r#"{"all"?: bool}"#),
        CommandSpec::new("revision.rollback", "Roll Back Revisions", "Production", rollback).params(r#"{"to": set name, or "" for the original}"#),
    ]
}

#[cfg(test)]
mod tests;
