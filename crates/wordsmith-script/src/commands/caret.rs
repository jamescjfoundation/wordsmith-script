// SPDX-License-Identifier: MIT OR Apache-2.0
//! Caret movement and selection. `caret.*` moves the caret (collapsing a selection); `select.*`
//! moves only the focus, extending the selection. Lines come from the layout, so Up, Down, Home
//! and End follow what the writer sees.

use serde_json::Value;
use wordsmith_doc::{Document, Pos};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Move {
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
}

/// The position before `at`: the previous character, or the end of the previous paragraph.
fn step_left(doc: &Document, at: &Pos) -> Pos {
    if at.off > 0 {
        let off = doc.para_at(at).map_or(0, |p| p.prev_boundary(at.off));
        return Pos { off, ..at.clone() };
    }
    match doc.prev_para(at.story, &at.path) {
        Some(path) => {
            let off = doc.para(at.story, &path).map_or(0, |p| p.len());
            Pos { story: at.story, path, off }
        }
        None => at.clone(),
    }
}

/// The position after `at`: the next character, or the start of the next paragraph.
fn step_right(doc: &Document, at: &Pos) -> Pos {
    let len = doc.para_at(at).map_or(0, |p| p.len());
    if at.off < len {
        let off = doc.para_at(at).map_or(len, |p| p.next_boundary(at.off));
        return Pos { off, ..at.clone() };
    }
    match doc.next_para(at.story, &at.path) {
        Some(path) => Pos { story: at.story, path, off: 0 },
        None => at.clone(),
    }
}

fn word_left(doc: &Document, at: &Pos) -> Pos {
    let Some(p) = doc.para_at(at) else { return at.clone() };
    if at.off == 0 {
        return step_left(doc, at);
    }
    let mut off = at.off;
    // Skip the spaces and punctuation before the caret, then the word.
    while off > 0 {
        let prev = p.prev_boundary(off);
        if p.text.get(prev..off).is_some_and(|c| c.chars().any(wordsmith_doc::bidi::is_word_char)) {
            break;
        }
        off = prev;
    }
    Pos { off: p.word_start(off), ..at.clone() }
}

fn word_right(doc: &Document, at: &Pos) -> Pos {
    let Some(p) = doc.para_at(at) else { return at.clone() };
    if at.off >= p.len() {
        return step_right(doc, at);
    }
    let mut off = p.word_end(at.off).max(p.next_boundary(at.off));
    // Then past the spaces after it, to the next word's start.
    while off < p.len() {
        let next = p.next_boundary(off);
        if p.text.get(off..next).is_some_and(|c| c.chars().any(wordsmith_doc::bidi::is_word_char)) {
            break;
        }
        off = next;
    }
    Pos { off, ..at.clone() }
}

fn target(s: &mut Session, m: Move, extend: bool) -> Pos {
    let focus = s.sel.focus.clone();
    // With a selection and no Shift, Left and Right go to its ends.
    if !extend && !s.sel.is_collapsed() {
        let (a, b) = s.sel.ordered();
        match m {
            Move::Left => return a,
            Move::Right => return b,
            _ => {}
        }
    }
    match m {
        Move::Left => step_left(&s.doc, &focus),
        Move::Right => step_right(&s.doc, &focus),
        Move::WordLeft => word_left(&s.doc, &focus),
        Move::WordRight => word_right(&s.doc, &focus),
        Move::DocStart => s.doc.start_of(focus.story),
        Move::DocEnd => s.doc.end_of(focus.story),
        Move::Up | Move::Down => {
            let l = s.layout();
            let hint = l.caret(&focus).map_or(0, |c| c.page);
            let dir = if m == Move::Up { -1 } else { 1 };
            match l.vertical(&focus, s.goal_x, dir, hint) {
                Some((pos, x)) => {
                    s.goal_x = Some(x);
                    pos
                }
                // No line above the first or below the last: go to the document's edge.
                None if dir < 0 => s.doc.start_of(focus.story),
                None => s.doc.end_of(focus.story),
            }
        }
        Move::LineStart | Move::LineEnd => {
            let l = s.layout();
            let hint = l.caret(&focus).map_or(0, |c| c.page);
            match l.line_bounds(&focus, hint) {
                Some((a, b)) => {
                    if m == Move::LineStart {
                        a
                    } else {
                        b
                    }
                }
                None => focus,
            }
        }
    }
}

fn go(s: &mut Session, m: Move, extend: bool) -> CmdResult {
    let raw = target(s, m, extend);
    let to = s.doc.clamp(&raw);
    s.sel = if extend { Selection { anchor: s.sel.anchor.clone(), focus: to } } else { Selection::caret(to) };
    sel_result(s)
}

/// `caret.to {pos, extend?}`: put the caret at `pos` (a click), or extend the selection to it
/// (Shift+click, a drag). The position is clamped to the document.
fn caret_to(s: &mut Session, v: &Value) -> CmdResult {
    let pos: Pos =
        v.get("pos").and_then(|p| serde_json::from_value(p.clone()).ok()).ok_or_else(|| CmdError::Params("`pos` must be a position".into()))?;
    let to = s.doc.clamp(&pos);
    let extend = v.get("extend").and_then(Value::as_bool).unwrap_or(false);
    s.sel = if extend { Selection { anchor: s.sel.anchor.clone(), focus: to } } else { Selection::caret(to) };
    sel_result(s)
}

/// `select.wordAt {pos}`: select the word at `pos` (a double-click).
fn select_word_at(s: &mut Session, v: &Value) -> CmdResult {
    let pos: Pos =
        v.get("pos").and_then(|p| serde_json::from_value(p.clone()).ok()).ok_or_else(|| CmdError::Params("`pos` must be a position".into()))?;
    let at = s.doc.clamp(&pos);
    let (a, b) = s.doc.para_at(&at).map_or((at.off, at.off), |p| p.word_at(at.off));
    s.sel = Selection { anchor: Pos { off: a, ..at.clone() }, focus: Pos { off: b, ..at } };
    sel_result(s)
}

fn select_all(s: &mut Session, _: &Value) -> CmdResult {
    let story = s.sel.focus.story;
    s.sel = Selection { anchor: s.doc.start_of(story), focus: s.doc.end_of(story) };
    sel_result(s)
}

/// Caret and selection commands. Shortcuts are portable defaults; the desktop app maps the
/// platform's own (Option and Command on macOS).
pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Navigate";
    vec![
        CommandSpec::new("caret.left", "Left", LOC, |s, _| go(s, Move::Left, false)).key("Left").pure(),
        CommandSpec::new("caret.right", "Right", LOC, |s, _| go(s, Move::Right, false)).key("Right").pure(),
        CommandSpec::new("caret.wordLeft", "Word Left", LOC, |s, _| go(s, Move::WordLeft, false)).key("Mod+Left").pure(),
        CommandSpec::new("caret.wordRight", "Word Right", LOC, |s, _| go(s, Move::WordRight, false)).key("Mod+Right").pure(),
        CommandSpec::new("caret.up", "Up", LOC, |s, _| go(s, Move::Up, false)).key("Up").pure(),
        CommandSpec::new("caret.down", "Down", LOC, |s, _| go(s, Move::Down, false)).key("Down").pure(),
        CommandSpec::new("caret.lineStart", "Line Start", LOC, |s, _| go(s, Move::LineStart, false)).key("Home").pure(),
        CommandSpec::new("caret.lineEnd", "Line End", LOC, |s, _| go(s, Move::LineEnd, false)).key("End").pure(),
        CommandSpec::new("caret.docStart", "Script Start", LOC, |s, _| go(s, Move::DocStart, false)).key("Mod+Home").pure(),
        CommandSpec::new("caret.docEnd", "Script End", LOC, |s, _| go(s, Move::DocEnd, false)).key("Mod+End").pure(),
        CommandSpec::new("select.left", "Select Left", LOC, |s, _| go(s, Move::Left, true)).key("Shift+Left").pure(),
        CommandSpec::new("select.right", "Select Right", LOC, |s, _| go(s, Move::Right, true)).key("Shift+Right").pure(),
        CommandSpec::new("select.wordLeft", "Select Word Left", LOC, |s, _| go(s, Move::WordLeft, true)).key("Mod+Shift+Left").pure(),
        CommandSpec::new("select.wordRight", "Select Word Right", LOC, |s, _| go(s, Move::WordRight, true)).key("Mod+Shift+Right").pure(),
        CommandSpec::new("select.up", "Select Up", LOC, |s, _| go(s, Move::Up, true)).key("Shift+Up").pure(),
        CommandSpec::new("select.down", "Select Down", LOC, |s, _| go(s, Move::Down, true)).key("Shift+Down").pure(),
        CommandSpec::new("select.lineStart", "Select to Line Start", LOC, |s, _| go(s, Move::LineStart, true)).key("Shift+Home").pure(),
        CommandSpec::new("select.lineEnd", "Select to Line End", LOC, |s, _| go(s, Move::LineEnd, true)).key("Shift+End").pure(),
        CommandSpec::new("select.docStart", "Select to Script Start", LOC, |s, _| go(s, Move::DocStart, true)).key("Mod+Shift+Home").pure(),
        CommandSpec::new("select.docEnd", "Select to Script End", LOC, |s, _| go(s, Move::DocEnd, true)).key("Mod+Shift+End").pure(),
        CommandSpec::new("select.all", "Select All", "Edit", select_all).key("Mod+A").pure(),
        CommandSpec::new("caret.to", "Place Caret", LOC, caret_to).params(r#"{"pos": position, "extend"?: bool}"#).pure(),
        CommandSpec::new("select.wordAt", "Select Word", LOC, select_word_at).params(r#"{"pos": position}"#).pure(),
    ]
}
