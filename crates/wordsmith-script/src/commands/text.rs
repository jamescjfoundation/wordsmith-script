// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typing: text, Return, Tab, Backspace and Delete, with the screenplay rules from
//! `docs/SPEC.md` ("Typing behavior") and `docs/decisions/0010-typing-rules.md`.

use serde_json::Value;
use wordsmith_doc::props::CharProps;
use wordsmith_doc::{Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, Selection, Session, cmd::sel_result, p};

use super::element::{apply_element, element_of_para_at};
use crate::element::{ElementKind, Elements};
use crate::scene_heading::SceneHeadingSettings;

/// Longest text one `text.insert` takes, in bytes. Bounds hostile callers.
pub const MAX_INSERT: usize = 1 << 20;
/// Most paragraphs one `text.insert` may create (line breaks in the text).
pub const MAX_INSERT_PARAGRAPHS: usize = 10_000;

/// Delete the selection, if any, and leave a caret where it was.
pub(crate) fn delete_selection(s: &mut Session) -> Result<Pos, CmdError> {
    if s.sel.is_collapsed() {
        return Ok(s.sel.focus.clone());
    }
    let (a, b) = s.sel.ordered();
    let at = s.doc.delete_range(&a, &b)?;
    s.sel = Selection::caret(at.clone());
    Ok(at)
}

/// The formatting new text takes at `at`: what is waiting at the caret, else the text before it.
fn typing_props(s: &Session, at: &Pos) -> CharProps {
    if let Some(p) = &s.pending {
        return p.clone();
    }
    s.doc.para_at(at).map(|p| p.props_at(at.off).clone()).unwrap_or_default()
}

/// Insert `text` at the caret in the caret paragraph's casing (no line breaks).
fn insert_line(s: &mut Session, text: &str) -> Result<(), CmdError> {
    if text.is_empty() {
        return Ok(());
    }
    let at = s.sel.focus.clone();
    let elements = Elements::of_doc(&s.doc);
    let casing = element_of_para_at(&s.doc, &elements, &at).casing;
    let props = typing_props(s, &at);
    let end = s.doc.insert_text(&at, &casing.apply(text), &props)?;
    s.sel = Selection::caret(end);
    Ok(())
}

/// `text.insert {text}`: type at the caret, replacing the selection. Uppercase elements store
/// capitals. Line breaks act as Return; tabs become spaces; other control characters are dropped.
pub fn insert(s: &mut Session, v: &Value) -> CmdResult {
    let text = p::req_str(v, "text")?;
    if text.len() > MAX_INSERT {
        return Err(CmdError::Params(format!("`text` is longer than {MAX_INSERT} bytes")));
    }
    let clean: String = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| *c == '\n' || !c.is_control())
        .collect();
    if clean.matches('\n').count() > MAX_INSERT_PARAGRAPHS {
        return Err(CmdError::Params(format!("`text` has more than {MAX_INSERT_PARAGRAPHS} lines")));
    }
    delete_selection(s)?;
    for (i, line) in clean.split('\n').enumerate() {
        if i > 0 {
            return_key(s)?;
        }
        insert_line(s, line)?;
    }
    sel_result(s)
}

/// Return: at the end of a paragraph, a new paragraph of the element's next element; in the
/// middle, a split that keeps the element.
fn return_key(s: &mut Session) -> Result<(), CmdError> {
    let mut at = delete_selection(s)?;
    let elements = Elements::of_doc(&s.doc);
    let Some(para) = s.doc.para_at(&at) else { return Err(CmdError::Failed("the caret is not in a paragraph".into())) };
    let element = elements.of_para(para).clone();
    // In "(beat)" the caret usually sits before the closing bracket: Return there is the end.
    if element.kind == ElementKind::Parenthetical && para.text.ends_with(')') && at.off + 1 == para.len() {
        at.off = para.len();
    }
    let at_end = at.off >= para.len();
    // The paragraph being left teaches the autocomplete lists its names and places.
    let left = para.clone();
    crate::autocomplete::learn(&mut s.doc, &left);
    let next_id = s.doc.styles.get(&element.id).and_then(|st| st.next.clone()).filter(|n| elements.get(n).is_some()).unwrap_or(element.id.clone());
    let new = s.doc.split_paragraph(&at)?;
    s.sel = Selection::caret(new.clone());
    if at_end {
        apply_element(s, &elements, &new.path, &next_id, true)?;
    }
    Ok(())
}

/// `text.newParagraph`: the Return key.
pub fn new_paragraph(s: &mut Session, _: &Value) -> CmdResult {
    return_key(s)?;
    sel_result(s)
}

/// `text.tab`: the Tab key (SPEC Tab table, scene-heading separators).
pub fn tab(s: &mut Session, _: &Value) -> CmdResult {
    let at = s.sel.focus.clone();
    s.sel = Selection::caret(at.clone());
    let elements = Elements::of_doc(&s.doc);
    let Some(para) = s.doc.para_at(&at) else { return sel_result(s) };
    let element = elements.of_para(para).clone();
    let text = para.text.clone();
    let blank = text.trim().is_empty();
    if element.kind == ElementKind::SceneHeading && !blank {
        scene_heading_tab(s, &at, &text)?;
        return sel_result(s);
    }
    if element.kind == ElementKind::Character
        && !blank
        && !text.contains('(')
        && at.off >= text.trim_end().len()
        && crate::autocomplete::Autocomplete::of_doc(&s.doc).tab_inserts_extension
    {
        // Tab after a name starts its extension (SPEC option): "ANNA (".
        let para = s.doc.para_mut(at.story, &at.path)?;
        let len = para.text.trim_end().len();
        let tail = para.len();
        para.delete(len, tail)?;
        para.insert_text(len, " (", &CharProps::default())?;
        para.touch();
        s.sel = Selection::caret(Pos { off: len + 2, ..at.clone() });
        return sel_result(s);
    }
    let Some(target) = element.tab.clone() else { return sel_result(s) };
    if element.tab_only_when_blank && !blank {
        return sel_result(s);
    }
    apply_element(s, &elements, &at.path, &target, true)?;
    sel_result(s)
}

/// Tab inside a Scene Heading, at its end: after a known intro ("int", "EXT."), the intro in
/// capitals and the intro separator; after an intro, its separator and a location, the time
/// separator. Anywhere else nothing happens (`docs/decisions/0010-typing-rules.md`).
fn scene_heading_tab(s: &mut Session, at: &Pos, text: &str) -> Result<(), CmdError> {
    if at.off < text.len() {
        return Ok(());
    }
    let settings = SceneHeadingSettings::of_doc(&s.doc);
    let trimmed = text.trim_end();
    let new_text = if let Some(intro) = settings.intro_of(trimmed) {
        format!("{}{}", intro.trim_end_matches('.').to_uppercase(), settings.intro_separator)
    } else if let Some(rest) = after_intro(trimmed, &settings)
        && !rest.trim().is_empty()
        && !rest.contains(settings.time_separator.trim_end())
    {
        format!("{trimmed}{}", settings.time_separator)
    } else {
        return Ok(());
    };
    let para = s.doc.para_mut(at.story, &at.path)?;
    let len = para.len();
    para.delete(0, len)?;
    para.insert_text(0, &new_text, &CharProps::default())?;
    para.touch();
    s.sel = Selection::caret(Pos { off: new_text.len(), ..at.clone() });
    Ok(())
}

/// The heading text after its intro and intro separator ("KITCHEN" in "INT. KITCHEN"), if it
/// starts with one.
fn after_intro<'a>(text: &'a str, settings: &SceneHeadingSettings) -> Option<&'a str> {
    let sep = settings.intro_separator.trim_end();
    settings.intros.iter().find_map(|intro| {
        let head = format!("{}{sep}", intro.trim_end_matches('.'));
        let start = text.get(..head.len())?;
        start.eq_ignore_ascii_case(&head).then(|| text.get(head.len()..)).flatten()
    })
}

/// `text.backspace`: delete the selection, or the character before the caret; at the start of a
/// paragraph, join it to the one before (which keeps its element).
pub fn backspace(s: &mut Session, _: &Value) -> CmdResult {
    if !s.sel.is_collapsed() {
        delete_selection(s)?;
        return sel_result(s);
    }
    let at = s.sel.focus.clone();
    let Some(para) = s.doc.para_at(&at) else { return sel_result(s) };
    if at.off > 0 {
        let from = Pos { off: para.backspace_from(at.off), ..at.clone() };
        let end = s.doc.delete_range(&from, &at)?;
        s.sel = Selection::caret(end);
    } else if let Some(prev) = s.doc.prev_para(at.story, &at.path) {
        let prev_len = s.doc.para(at.story, &prev).map_or(0, |p| p.len());
        let from = Pos { story: at.story, path: prev, off: prev_len };
        let end = s.doc.delete_range(&from, &at)?;
        s.sel = Selection::caret(end);
    }
    sel_result(s)
}

/// `text.delete`: delete the selection, or the character after the caret; at the end of a
/// paragraph, join the next one to it.
pub fn delete(s: &mut Session, _: &Value) -> CmdResult {
    if !s.sel.is_collapsed() {
        delete_selection(s)?;
        return sel_result(s);
    }
    let at = s.sel.focus.clone();
    let Some(para) = s.doc.para_at(&at) else { return sel_result(s) };
    if at.off < para.len() {
        let to = Pos { off: para.next_boundary(at.off), ..at.clone() };
        let end = s.doc.delete_range(&at, &to)?;
        s.sel = Selection::caret(end);
    } else if let Some(next) = s.doc.next_para(at.story, &at.path) {
        let to = Pos { story: at.story, path: next, off: 0 };
        let end = s.doc.delete_range(&at, &to)?;
        s.sel = Selection::caret(end);
    }
    sel_result(s)
}

/// The body's paragraphs as (element id, text), for tests and `document.inspect`.
pub fn body_lines(s: &Session) -> Vec<(String, String)> {
    let elements = Elements::of_doc(&s.doc);
    s.doc
        .story(StoryRef::Body)
        .map(|blocks| blocks.iter().filter_map(|b| b.as_para()).map(|p| (elements.of_para(p).id.clone(), p.text.clone())).collect())
        .unwrap_or_default()
}
