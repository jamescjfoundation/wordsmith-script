// SPDX-License-Identifier: MIT OR Apache-2.0
//! The story board and the outline's lanes (SPEC "Views, outlining, navigation and notes"),
//! decision 0028.
//!
//! One set of beats serves both views: the story board places them on a canvas, the outline
//! editor orders them in lanes. They live in `app_data` under [`STORYBOARD_KEY`], so every change
//! is undoable and saved with the script. A beat sent to the script becomes an Outline 1
//! paragraph (its title) and a Summary paragraph (its body) at the caret; the Outline 1 paragraph
//! is tagged with the beat (`ParaProps::beat`) so the outline can show the pages it covers.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Pos, StoryRef, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result};

use crate::element::{ElementKind, Elements};
use crate::template::paragraph;

/// `app_data` key of the story board.
pub const STORYBOARD_KEY: &str = "wordsmith.storyboard";
/// Limits that keep hostile or runaway data in check.
pub const MAX_BEATS: usize = 5_000;
pub const MAX_LINKS: usize = 20_000;
pub const MAX_LANES: usize = 100;
pub const MAX_TITLE: usize = 300;
pub const MAX_BODY: usize = 10_000;
/// The canvas, points: beats stay inside it.
pub const CANVAS: f32 = 20_000.0;
/// A new beat's size, points.
pub const BEAT_W: f32 = 180.0;
pub const BEAT_H: f32 = 110.0;

/// The beat colors offered (any `#RRGGBB` is kept).
pub const PALETTE: [&str; 8] = ["#FFF2A8", "#FFD1A8", "#F7B5C4", "#D9C2F0", "#B8D4F5", "#B8E6D0", "#D8E8A8", "#E6E6E6"];

/// One beat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Beat {
    pub id: u32,
    pub title: String,
    pub body: String,
    /// `#RRGGBB`.
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// The outline lane it is in, if any, and its place there.
    pub lane: Option<u32>,
    pub order: f32,
}

impl Default for Beat {
    fn default() -> Self {
        Beat {
            id: 0,
            title: String::new(),
            body: String::new(),
            color: PALETTE[0].into(),
            x: 0.0,
            y: 0.0,
            w: BEAT_W,
            h: BEAT_H,
            lane: None,
            order: 0.0,
        }
    }
}

/// A flow line from one beat to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub from: u32,
    pub to: u32,
}

/// An outline lane (acts, sequences …).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lane {
    pub id: u32,
    pub name: String,
    /// 0 to 2: how far the lane is indented under the one above.
    pub indent: u8,
}

impl Default for Lane {
    fn default() -> Self {
        Lane { id: 0, name: "Lane".into(), indent: 0 }
    }
}

/// The story board: beats, links, lanes and the grid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Storyboard {
    pub beats: Vec<Beat>,
    pub links: Vec<Link>,
    pub lanes: Vec<Lane>,
    /// Grid spacing, points.
    pub grid: f32,
    pub snap: bool,
}

impl Default for Storyboard {
    fn default() -> Self {
        Storyboard { beats: Vec::new(), links: Vec::new(), lanes: Vec::new(), grid: 20.0, snap: true }
    }
}

fn clean_text(s: &str, max: usize) -> String {
    s.replace("\r\n", "\n").chars().filter(|c| *c == '\n' || *c == '\t' || !c.is_control()).take(max).collect()
}

fn color_ok(c: &str) -> bool {
    crate::revisions::parse_color(c).is_some()
}

fn finite_in(v: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { default }
}

/// A number field, if it is one.
fn num(v: &Value, key: &str) -> Option<f32> {
    v.get(key).and_then(Value::as_f64).map(|x| x as f32)
}

fn id_of(v: &Value, key: &str) -> Option<u32> {
    v.get(key).and_then(Value::as_u64).and_then(|i| u32::try_from(i).ok())
}

impl Storyboard {
    /// Read the stored value field by field: a bad field costs only itself, and an entry
    /// without an id is skipped.
    fn read(v: &Value) -> Self {
        let d = Self::default();
        let list = |key: &str| v.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
        let text = |x: &Value, key: &str| x.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        let beats = list("beats")
            .iter()
            .take(MAX_BEATS * 2)
            .filter_map(|x| {
                let bd = Beat::default();
                Some(Beat {
                    id: id_of(x, "id")?,
                    title: text(x, "title"),
                    body: text(x, "body"),
                    color: x.get("color").and_then(Value::as_str).unwrap_or(&bd.color).to_string(),
                    x: num(x, "x").unwrap_or(0.0),
                    y: num(x, "y").unwrap_or(0.0),
                    w: num(x, "w").unwrap_or(bd.w),
                    h: num(x, "h").unwrap_or(bd.h),
                    lane: id_of(x, "lane"),
                    order: num(x, "order").unwrap_or(0.0),
                })
            })
            .collect();
        let links = list("links").iter().take(MAX_LINKS * 2).filter_map(|x| Some(Link { from: id_of(x, "from")?, to: id_of(x, "to")? })).collect();
        let lanes = list("lanes")
            .iter()
            .take(MAX_LANES * 2)
            .filter_map(|x| {
                Some(Lane {
                    id: id_of(x, "id")?,
                    name: text(x, "name"),
                    indent: x.get("indent").and_then(Value::as_u64).map_or(0, |i| i.min(2) as u8),
                })
            })
            .collect();
        Storyboard { beats, links, lanes, grid: num(v, "grid").unwrap_or(d.grid), snap: v.get("snap").and_then(Value::as_bool).unwrap_or(d.snap) }
    }

    /// The story board `doc` holds; hostile data is brought within the limits and links to
    /// missing beats are dropped.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(STORYBOARD_KEY) else { return Self::default() };
        let mut b = Self::read(v);
        b.beats.truncate(MAX_BEATS);
        let mut seen = std::collections::HashSet::new();
        b.beats.retain(|x| seen.insert(x.id));
        b.lanes.truncate(MAX_LANES);
        let mut seen_lanes = std::collections::HashSet::new();
        b.lanes.retain(|l| seen_lanes.insert(l.id));
        let lanes: std::collections::HashSet<u32> = b.lanes.iter().map(|l| l.id).collect();
        for l in &mut b.lanes {
            l.name = clean_text(&l.name, 100).replace('\n', " ");
            l.indent = l.indent.min(2);
        }
        for x in &mut b.beats {
            x.title = clean_text(&x.title, MAX_TITLE).replace('\n', " ");
            x.body = clean_text(&x.body, MAX_BODY);
            if !color_ok(&x.color) {
                x.color = PALETTE[0].into();
            }
            x.x = finite_in(x.x, 0.0, CANVAS, 0.0);
            x.y = finite_in(x.y, 0.0, CANVAS, 0.0);
            x.w = finite_in(x.w, 60.0, 1_000.0, BEAT_W);
            x.h = finite_in(x.h, 40.0, 1_000.0, BEAT_H);
            x.order = if x.order.is_finite() { x.order } else { 0.0 };
            if x.lane.is_some_and(|l| !lanes.contains(&l)) {
                x.lane = None;
            }
        }
        let ids: std::collections::HashSet<u32> = b.beats.iter().map(|x| x.id).collect();
        b.links.truncate(MAX_LINKS);
        b.links.retain(|l| l.from != l.to && ids.contains(&l.from) && ids.contains(&l.to));
        b.links.dedup();
        b.grid = finite_in(b.grid, 5.0, 200.0, 20.0);
        b
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(STORYBOARD_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }

    pub fn beat(&self, id: u32) -> Option<&Beat> {
        self.beats.iter().find(|b| b.id == id)
    }

    fn beat_mut(&mut self, id: u32) -> Result<&mut Beat, CmdError> {
        self.beats.iter_mut().find(|b| b.id == id).ok_or_else(|| CmdError::Params(format!("there is no beat {id}")))
    }

    /// `v` on the grid, when snapping is on.
    pub fn snapped(&self, v: f32) -> f32 {
        if self.snap { (v / self.grid).round() * self.grid } else { v }
    }

    fn next_id(&self) -> u32 {
        self.beats.iter().map(|b| b.id).max().map_or(1, |m| m + 1)
    }

    /// The next place in `lane`, after its last beat.
    pub fn end_of_lane(&self, lane: Option<u32>) -> f32 {
        self.beats.iter().filter(|b| b.lane == lane).map(|b| b.order).fold(0.0, f32::max) + 1.0
    }

    /// The beats of `lane` in order.
    pub fn lane_beats(&self, lane: Option<u32>) -> Vec<&Beat> {
        let mut v: Vec<&Beat> = self.beats.iter().filter(|b| b.lane == lane).collect();
        v.sort_by(|a, b| a.order.total_cmp(&b.order).then(a.id.cmp(&b.id)));
        v
    }
}

fn id_param(v: &Value, key: &str) -> Result<u32, CmdError> {
    v.get(key).and_then(Value::as_u64).and_then(|i| u32::try_from(i).ok()).ok_or_else(|| CmdError::Params(format!("`{key}` must be a beat id")))
}

fn f32_param(v: &Value, key: &str) -> Option<f32> {
    v.get(key).and_then(Value::as_f64).map(|x| x as f32).filter(|x| x.is_finite())
}

/// Apply the fields of `v` a beat takes (title, body, color, x, y, w, h).
fn set_fields(b: &mut Beat, v: &Value, board: &Storyboard) -> Result<(), CmdError> {
    if let Some(t) = v.get("title").and_then(Value::as_str) {
        b.title = clean_text(t, MAX_TITLE).replace('\n', " ");
    }
    if let Some(t) = v.get("body").and_then(Value::as_str) {
        b.body = clean_text(t, MAX_BODY);
    }
    if let Some(c) = v.get("color").and_then(Value::as_str) {
        if !color_ok(c) {
            return Err(CmdError::Params("`color` must be #RRGGBB".into()));
        }
        b.color = c.trim().to_uppercase();
    }
    if let Some(x) = f32_param(v, "x") {
        b.x = board.snapped(x).clamp(0.0, CANVAS);
    }
    if let Some(y) = f32_param(v, "y") {
        b.y = board.snapped(y).clamp(0.0, CANVAS);
    }
    if let Some(w) = f32_param(v, "w") {
        b.w = board.snapped(w).clamp(60.0, 1_000.0);
    }
    if let Some(h) = f32_param(v, "h") {
        b.h = board.snapped(h).clamp(40.0, 1_000.0);
    }
    Ok(())
}

fn board_json(b: &Storyboard) -> Value {
    serde_json::to_value(b).unwrap_or_default()
}

/// `storyboard.get`: the beats, links, lanes and grid.
fn get(s: &mut Session, _: &Value) -> CmdResult {
    Ok(board_json(&Storyboard::of_doc(&s.doc)))
}

/// `beat.add {title?, body?, color?, x?, y?, lane?}`: a new beat (at the next free place when no
/// position is given).
fn add(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    if board.beats.len() >= MAX_BEATS {
        return Err(CmdError::Failed(format!("a story board holds at most {MAX_BEATS} beats")));
    }
    let n = board.beats.len() as f32;
    let lane = v.get("lane").and_then(Value::as_u64).and_then(|l| u32::try_from(l).ok());
    if lane.is_some_and(|l| !board.lanes.iter().any(|x| x.id == l)) {
        return Err(CmdError::Params("`lane` must be a lane id".into()));
    }
    let mut b = Beat {
        id: board.next_id(),
        x: board.snapped(40.0 + (n % 6.0) * (BEAT_W + 20.0)),
        y: board.snapped(40.0 + (n / 6.0).floor() * (BEAT_H + 20.0)),
        color: PALETTE[(board.beats.len()) % PALETTE.len()].into(),
        lane,
        order: board.end_of_lane(lane),
        ..Beat::default()
    };
    set_fields(&mut b, v, &board)?;
    let id = b.id;
    board.beats.push(b);
    board.store(&mut s.doc);
    Ok(json!({"id": id}))
}

/// `beat.update {id, title?, body?, color?, x?, y?, w?, h?}`.
fn update(s: &mut Session, v: &Value) -> CmdResult {
    let id = id_param(v, "id")?;
    let mut board = Storyboard::of_doc(&s.doc);
    let snapshot = board.clone();
    set_fields(board.beat_mut(id)?, v, &snapshot)?;
    board.store(&mut s.doc);
    Ok(json!({"id": id}))
}

/// `beat.delete {id}`: delete a beat and its links. Paragraphs sent from it stay in the script.
fn delete(s: &mut Session, v: &Value) -> CmdResult {
    let id = id_param(v, "id")?;
    let mut board = Storyboard::of_doc(&s.doc);
    board.beat_mut(id)?;
    board.beats.retain(|b| b.id != id);
    board.links.retain(|l| l.from != id && l.to != id);
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `beat.link {from, to}`: a flow line between two beats (once; a link back the other way is a
/// different link).
fn link(s: &mut Session, v: &Value) -> CmdResult {
    let (from, to) = (id_param(v, "from")?, id_param(v, "to")?);
    let mut board = Storyboard::of_doc(&s.doc);
    if from == to || board.beat(from).is_none() || board.beat(to).is_none() {
        return Err(CmdError::Params("a link joins two different beats".into()));
    }
    if board.links.len() >= MAX_LINKS {
        return Err(CmdError::Failed(format!("a story board holds at most {MAX_LINKS} links")));
    }
    let l = Link { from, to };
    if !board.links.contains(&l) {
        board.links.push(l);
    }
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `beat.unlink {from, to}`.
fn unlink(s: &mut Session, v: &Value) -> CmdResult {
    let (from, to) = (id_param(v, "from")?, id_param(v, "to")?);
    let mut board = Storyboard::of_doc(&s.doc);
    board.links.retain(|l| !(l.from == from && l.to == to));
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `storyboard.configure {grid?, snap?}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    if let Some(g) = f32_param(v, "grid") {
        if !(5.0..=200.0).contains(&g) {
            return Err(CmdError::Params("`grid` is 5 to 200 points".into()));
        }
        board.grid = g;
    }
    if let Some(b) = v.get("snap").and_then(Value::as_bool) {
        board.snap = b;
    }
    board.store(&mut s.doc);
    Ok(json!({"grid": board.grid, "snap": board.snap}))
}

/// The Outline 1 paragraph sent from beat `id`, if any (body block index).
pub fn sent_paragraph(doc: &Document, id: u32) -> Option<usize> {
    doc.body.iter().position(|b| b.as_para().is_some_and(|p| p.props.beat == Some(id)))
}

/// `beat.toScript {id}`: put the beat in the script before the caret's paragraph: its title as
/// Outline 1, its body (if any) as Summary.
fn to_script(s: &mut Session, v: &Value) -> CmdResult {
    let id = id_param(v, "id")?;
    let board = Storyboard::of_doc(&s.doc);
    let beat = board.beat(id).ok_or_else(|| CmdError::Params(format!("there is no beat {id}")))?.clone();
    if s.sel.focus.story != StoryRef::Body {
        return Err(CmdError::Failed("the caret is not in the script".into()));
    }
    let elements = Elements::of_doc(&s.doc);
    let at = s.sel.focus.path.last().min(s.doc.body.len());
    let title = if beat.title.trim().is_empty() { "Untitled beat".to_string() } else { beat.title.clone() };
    let mut heading = paragraph(elements.id_of(ElementKind::Outline1), &title);
    heading.props.beat = Some(id);
    let mut new = vec![para_block(heading)];
    if !beat.body.trim().is_empty() {
        new.push(para_block(paragraph(elements.id_of(ElementKind::Summary), beat.body.trim())));
    }
    let count = new.len();
    s.doc.body.splice(at..at, new);
    s.sel = Selection::caret(Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(at + count), off: 0 });
    s.clamp_selection();
    sel_result(s)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("storyboard.get", "Story Board", "Outline", get).pure(),
        CommandSpec::new("beat.add", "Add Beat", "Outline", add)
            .params(r##"{"title"?, "body"?, "color"?: "#RRGGBB", "x"?, "y"?, "lane"?: lane id}"##),
        CommandSpec::new("beat.update", "Change Beat", "Outline", update)
            .params(r##"{"id", "title"?, "body"?, "color"?: "#RRGGBB", "x"?, "y"?, "w"?, "h"?}"##),
        CommandSpec::new("beat.delete", "Delete Beat", "Outline", delete).params(r#"{"id": number}"#),
        CommandSpec::new("beat.link", "Link Beats", "Outline", link).params(r#"{"from": id, "to": id}"#),
        CommandSpec::new("beat.unlink", "Unlink Beats", "Outline", unlink).params(r#"{"from": id, "to": id}"#),
        CommandSpec::new("storyboard.configure", "Story Board Grid", "Outline", configure).params(r#"{"grid"?: 5-200, "snap"?: bool}"#),
        CommandSpec::new("beat.toScript", "Send Beat to Script", "Outline", to_script).params(r#"{"id": number}"#),
    ]
}

#[cfg(test)]
mod tests;
