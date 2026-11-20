// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editing commands from SPEC-2 3.2 and 10: the Enter/Tab hint, reset to element, toggle case,
//! select scene, page breaks, new scenes, deleting a paragraph (the reformat tool's delete) and
//! going to a page.

use serde_json::{Value, json};
use wordsmith_doc::props::CharProps;
use wordsmith_doc::{Path, Pos, StoryRef, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use super::element::element_of_para_at;
use crate::data::body_pos;
use crate::element::{ElementKind, Elements};

/// `element.hint`: the caret paragraph's element and what Return and Tab make from it.
pub fn hint(s: &mut Session, _: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    let here = element_of_para_at(&s.doc, &elements, &s.sel.focus).clone();
    let name = |id: Option<&str>| id.and_then(|i| elements.get(i)).map(|d| d.name.clone());
    let enter = s.doc.styles.get(&here.id).and_then(|st| st.next.clone());
    let enter = name(enter.as_deref()).unwrap_or_else(|| here.name.clone());
    let tab = if here.kind == ElementKind::Character && crate::autocomplete::Autocomplete::of_doc(&s.doc).tab_inserts_extension {
        Some("extension".to_string())
    } else {
        name(here.tab.as_deref())
    };
    Ok(json!({"element": here.name, "enter": enter, "tab": tab}))
}

/// `element.reset`: give the paragraphs the selection touches their element's own formatting,
/// dropping formatting set by hand. Notes, revision marks, locks, scene numbers and bookmarks
/// stay.
fn reset(s: &mut Session, _: &Value) -> CmdResult {
    let (a, b) = s.sel.ordered();
    let paths = if s.sel.is_collapsed() { vec![a.path.clone()] } else { s.doc.para_paths_in(&a, &b) };
    for path in paths {
        let para = s.doc.para_mut(a.story, &path)?;
        let keep = para.props.clone();
        para.props = wordsmith_doc::props::ParaProps {
            style: keep.style,
            scene_number: keep.scene_number,
            omitted: keep.omitted,
            beat: keep.beat,
            bookmark: keep.bookmark,
            dual: keep.dual,
            ..Default::default()
        };
        let len = para.len();
        let start = Pos { story: a.story, path: path.clone(), off: 0 };
        let end = Pos { off: len, ..start.clone() };
        s.doc.format_range(&start, &end, &|c: &mut CharProps| {
            *c = CharProps { note: c.note, revised: c.revised, page_lock: c.page_lock, ..CharProps::default() };
        })?;
    }
    s.pending = None;
    sel_result(s)
}

/// `text.toggleCase`: the selection in capitals, or in small letters if it already is.
fn toggle_case(s: &mut Session, _: &Value) -> CmdResult {
    if s.sel.is_collapsed() {
        return Err(CmdError::Failed("select some text first".into()));
    }
    let (a, b) = s.sel.ordered();
    let selected = s.selected_text();
    let to_upper = selected.chars().any(char::is_lowercase);
    for path in s.doc.para_paths_in(&a, &b) {
        let para = s.doc.para_mut(a.story, &path)?;
        let from = if path == a.path { a.off } else { 0 };
        let to = if path == b.path { b.off } else { para.len() };
        let Some(piece) = para.text.get(from..to) else { continue };
        let changed: String = if to_upper { piece.to_uppercase() } else { piece.to_lowercase() };
        if changed == piece {
            continue;
        }
        if changed.len() == piece.len() {
            // Same length (nearly always): change the letters in place, runs untouched.
            para.text.replace_range(from..to, &changed);
        } else {
            let props = para.props_at(from + piece.chars().next().map_or(0, char::len_utf8)).clone();
            para.delete(from, to)?;
            para.insert_text(from, &changed, &props)?;
        }
        para.touch();
    }
    s.clamp_selection();
    sel_result(s)
}

/// `select.scene`: select the whole scene the caret is in.
fn select_scene(s: &mut Session, _: &Value) -> CmdResult {
    let all = crate::scenes::scenes(&s.doc);
    let block = s.sel.focus.path.last();
    let sc = crate::scenes::scene_at(&all, block).ok_or_else(|| CmdError::Failed("the caret is not in a scene".into()))?;
    let last = sc.end.saturating_sub(1).max(sc.start);
    let len = s.doc.body.get(last).and_then(|b| b.as_para()).map_or(0, |p| p.len());
    s.sel = Selection { anchor: body_pos(sc.start, 0), focus: body_pos(last, len) };
    sel_result(s)
}

/// `text.pageBreak`: start a new page at the caret (splitting the paragraph when the caret is
/// inside it). On a paragraph that already starts a page, at its start, the break is removed.
fn page_break(s: &mut Session, _: &Value) -> CmdResult {
    let at = super::text::delete_selection(s)?;
    let para = s.doc.para_at(&at).ok_or_else(|| CmdError::Failed("the caret is not in a paragraph".into()))?;
    let path = if at.off == 0 {
        if para.props.page_break_before == Some(true) {
            let p = s.doc.para_mut(at.story, &at.path)?;
            p.props.page_break_before = None;
            p.touch();
            return sel_result(s);
        }
        at.path.clone()
    } else {
        let new = s.doc.split_paragraph(&at)?;
        crate::scene_numbers::after_split(&mut s.doc, &new);
        s.sel = Selection::caret(new.clone());
        new.path
    };
    let p = s.doc.para_mut(at.story, &path)?;
    p.props.page_break_before = Some(true);
    p.touch();
    sel_result(s)
}

/// `scene.insert`: a new, empty Scene Heading after the caret's scene, with the caret in it.
fn insert_scene(s: &mut Session, _: &Value) -> CmdResult {
    let all = crate::scenes::scenes(&s.doc);
    let block = s.sel.focus.path.last();
    let at = crate::scenes::scene_at(&all, block).map_or(s.doc.body.len(), |sc| sc.end).min(s.doc.body.len());
    let id = Elements::of_doc(&s.doc).id_of(ElementKind::SceneHeading).to_string();
    s.doc.body.insert(at, para_block(crate::template::paragraph(&id, "")));
    s.sel = Selection::caret(body_pos(at, 0));
    sel_result(s)
}

/// `paragraph.delete`: delete the caret's paragraph.
fn delete_paragraph(s: &mut Session, _: &Value) -> CmdResult {
    let block = s.sel.focus.path.last();
    crate::checker::delete_block(s, block)?;
    s.clamp_selection();
    sel_result(s)
}

/// `paragraph.next` / `paragraph.previous`: select the next (previous) paragraph whole, for
/// stepping through the script (the reformat tool).
fn step_paragraph(s: &mut Session, forward: bool) -> CmdResult {
    let here = s.sel.focus.path.last();
    let n = s.doc.body.len();
    let to = if forward { (here + 1).min(n.saturating_sub(1)) } else { here.saturating_sub(1) };
    let len = s.doc.body.get(to).and_then(|b| b.as_para()).map_or(0, |p| p.len());
    s.sel = Selection { anchor: body_pos(to, 0), focus: body_pos(to, len) };
    sel_result(s)
}

/// The body block a page starts with: `page` is a page number (1-based) or, in a locked
/// script, a page label ("12A").
pub fn page_start(doc: &wordsmith_doc::Document, page: &str) -> Option<usize> {
    let pag = crate::paginate::paginate(doc);
    let page = page.trim();
    let index = pag
        .labels
        .as_ref()
        .and_then(|l| l.iter().position(|x| x.eq_ignore_ascii_case(page)))
        .or_else(|| page.parse::<usize>().ok().and_then(|n| n.checked_sub(1)))?;
    let pieces = pag.pages.get(index)?;
    pieces.iter().find_map(|pc| match pc {
        crate::paginate::Piece::Para { block, .. } => Some(*block),
        _ => None,
    })
}

/// `goto.page {page}`.
fn goto_page(s: &mut Session, v: &Value) -> CmdResult {
    let page = match v.get("page") {
        Some(Value::Number(n)) => n.to_string(),
        _ => p::req_str(v, "page")?.to_string(),
    };
    let block = page_start(&s.doc, &page).ok_or_else(|| CmdError::Failed(format!("there is no page {page}")))?;
    s.sel = Selection::caret(body_pos(block, 0));
    sel_result(s)
}

/// `goto.scene {scene}`: the scene by number (as shown, "12A", or its position from 1).
fn goto_scene(s: &mut Session, v: &Value) -> CmdResult {
    let want = match v.get("scene") {
        Some(Value::Number(n)) => n.to_string(),
        _ => p::req_str(v, "scene")?.trim().to_string(),
    };
    let all = crate::scenes::scenes(&s.doc);
    let numbers = crate::scene_numbers::numbers(&s.doc);
    let by_number = all.iter().find(|sc| numbers.get(sc.index).is_some_and(|n| n.eq_ignore_ascii_case(&want)));
    let by_position = want.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).and_then(|i| all.get(i));
    let sc = by_number.or(by_position).ok_or_else(|| CmdError::Failed(format!("there is no scene {want}")))?;
    s.sel = Selection::caret(body_pos(sc.start, 0));
    sel_result(s)
}

fn has_selection(s: &Session) -> Option<&'static str> {
    if s.sel.is_collapsed() { Some("nothing is selected") } else { None }
}

fn in_scene(s: &Session) -> Option<&'static str> {
    let all = crate::scenes::scenes(&s.doc);
    if crate::scenes::scene_at(&all, s.sel.focus.path.last()).is_none() { Some("the caret is not in a scene") } else { None }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("element.hint", "Return and Tab Hint", "Format › Element", hint).pure(),
        CommandSpec::new("element.reset", "Reset to Element", "Format › Element", reset).key("Mod+Shift+R"),
        CommandSpec::new("text.toggleCase", "Toggle Case", "Format", toggle_case).key("Mod+Shift+K").when(has_selection),
        CommandSpec::new("select.scene", "Select Scene", "Edit", select_scene).key("Mod+Shift+A").when(in_scene).pure(),
        CommandSpec::new("text.pageBreak", "Page Break", "Insert", page_break).key("Mod+Shift+Enter"),
        CommandSpec::new("scene.insert", "New Scene", "Insert", insert_scene).key("Mod+Shift+N"),
        CommandSpec::new("paragraph.delete", "Delete Paragraph", "Edit", delete_paragraph),
        CommandSpec::new("paragraph.next", "Next Paragraph", "Edit", |s, _| step_paragraph(s, true)).pure(),
        CommandSpec::new("paragraph.previous", "Previous Paragraph", "Edit", |s, _| step_paragraph(s, false)).pure(),
        CommandSpec::new("goto.page", "Go to Page", "Go", goto_page).params(r#"{"page": number | label}"#).pure(),
        CommandSpec::new("goto.scene", "Go to Scene Number", "Go", goto_scene).params(r#"{"scene": number | label}"#).pure(),
    ]
}

/// The body paragraph at `block`, for tests.
pub fn para_text(s: &Session, block: usize) -> String {
    s.doc.para(StoryRef::Body, &Path::top(block)).map(|p| p.text.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;

    /// A script from lines; a line in capitals starting INT. or EXT. is a Scene Heading.
    fn script(text: &str) -> Session {
        let mut s = new_session();
        s.doc.body = text
            .lines()
            .map(|l| {
                let e = if l.starts_with("INT.") || l.starts_with("EXT.") { "SceneHeading" } else { "Action" };
                para_block(crate::template::paragraph(e, l))
            })
            .collect();
        s
    }

    #[test]
    fn hint_names_return_and_tab() {
        let mut s = script("INT. HALL - DAY\nRain.");
        s.sel = Selection::caret(body_pos(1, 0));
        let h = s.run("element.hint", &json!({})).unwrap();
        assert_eq!((h["element"].as_str(), h["enter"].as_str(), h["tab"].as_str()), (Some("Action"), Some("Action"), Some("Character")));
        s.sel = Selection::caret(body_pos(0, 0));
        let h = s.run("element.hint", &json!({})).unwrap();
        assert_eq!(h["enter"], "Action");
    }

    #[test]
    fn toggle_case_flips_and_keeps_runs() {
        let mut s = script("INT. HALL - DAY\nRain falls.");
        s.sel = Selection { anchor: body_pos(1, 0), focus: body_pos(1, 4) };
        s.run("text.toggleCase", &json!({})).unwrap();
        assert_eq!(para_text(&s, 1), "RAIN falls.");
        s.run("text.toggleCase", &json!({})).unwrap();
        assert_eq!(para_text(&s, 1), "rain falls.");
    }

    #[test]
    fn select_scene_and_insert_scene() {
        let mut s = script("INT. HALL - DAY\nRain.\nEXT. YARD - NIGHT\nSun.");
        s.sel = Selection::caret(body_pos(1, 2));
        s.run("select.scene", &json!({})).unwrap();
        assert_eq!(s.selected_text(), "INT. HALL - DAY\nRain.");
        s.sel = Selection::caret(body_pos(1, 2));
        s.run("scene.insert", &json!({})).unwrap();
        assert_eq!(s.sel.focus.path.last(), 2);
        s.run("text.insert", &json!({"text": "int. attic - day"})).unwrap();
        assert_eq!(crate::scenes::scenes(&s.doc).len(), 3);
        assert_eq!(para_text(&s, 2), "INT. ATTIC - DAY");
    }

    #[test]
    fn page_break_splits_and_toggles() {
        let mut s = script("INT. HALL - DAY\nRain falls.");
        s.sel = Selection::caret(body_pos(1, 5));
        s.run("text.pageBreak", &json!({})).unwrap();
        assert_eq!(para_text(&s, 2), "falls.");
        assert_eq!(crate::paginate::paginate(&s.doc).pages.len(), 2);
        s.run("text.pageBreak", &json!({})).unwrap();
        assert_eq!(crate::paginate::paginate(&s.doc).pages.len(), 1);
    }

    #[test]
    fn goto_page_and_scene() {
        let mut s = script("INT. HALL - DAY\nRain.\nEXT. YARD - NIGHT\nSun.");
        s.sel = Selection::caret(body_pos(3, 0));
        s.run("text.pageBreak", &json!({})).unwrap();
        s.sel = Selection::caret(body_pos(0, 0));
        s.run("goto.page", &json!({"page": 2})).unwrap();
        assert_eq!(s.sel.focus.path.last(), 3);
        assert!(s.run("goto.page", &json!({"page": "9"})).is_err());
        s.run("goto.scene", &json!({"scene": 2})).unwrap();
        assert_eq!(s.sel.focus.path.last(), 2);
    }

    #[test]
    fn reset_drops_hand_formatting_but_keeps_marks() {
        let mut s = script("INT. HALL - DAY\nRain.");
        s.sel = Selection { anchor: body_pos(1, 0), focus: body_pos(1, 4) };
        s.run("bookmark.add", &json!({})).unwrap();
        let start = body_pos(1, 0);
        let end = body_pos(1, 4);
        s.doc
            .format_range(&start, &end, &|c: &mut CharProps| {
                c.bold = Some(true);
                c.revised = Some(1);
            })
            .unwrap();
        s.run("element.reset", &json!({})).unwrap();
        let p = s.doc.para(StoryRef::Body, &Path::top(1)).unwrap();
        assert_eq!(p.props_at(1).bold, None);
        assert_eq!(p.props_at(1).revised, Some(1));
        assert!(p.props.bookmark.is_some());
    }

    #[test]
    fn delete_and_step_paragraphs() {
        let mut s = script("INT. HALL - DAY\nRain.\nSun.");
        s.sel = Selection::caret(body_pos(0, 0));
        s.run("paragraph.next", &json!({})).unwrap();
        assert_eq!(s.selected_text(), "Rain.");
        s.run("paragraph.delete", &json!({})).unwrap();
        assert_eq!(para_text(&s, 1), "Sun.");
    }
}
