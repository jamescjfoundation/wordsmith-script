// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editing commands from SPEC-2 3.2 and 10: the Enter/Tab hint, reset to element, toggle case,
//! select scene, page breaks, new scenes, deleting a paragraph (the reformat tool's delete) and
//! going to a page.

use crate::keys::ScriptPara;
use jcj_doc::props::CharProps;
use jcj_doc::{Path, Pos, StoryRef, para_block};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, params as p};
use serde_json::{Value, json};

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
        // The paragraph's extension data (scene number, omitted, beat, bookmark, dual) stays.
        para.props = jcj_doc::props::ParaProps { style: keep.style, ext: keep.ext, ..Default::default() };
        let len = para.len();
        let start = Pos { story: a.story, path: path.clone(), off: 0 };
        let end = Pos { off: len, ..start.clone() };
        s.doc.format_range(&start, &end, &|c: &mut CharProps| {
            // Marks (notes, revision marks, page locks) are extension data: they stay.
            *c = CharProps { ext: c.ext.clone(), ..CharProps::default() };
        })?;
    }
    s.pending = None;
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

/// The body block a page starts with: `page` is a page number (1-based) or, in a locked
/// script, a page label ("12A").
pub fn page_start(doc: &jcj_doc::Document, page: &str) -> Option<usize> {
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

fn in_scene(s: &Session) -> Option<&'static str> {
    let all = crate::scenes::scenes(&s.doc);
    if crate::scenes::scene_at(&all, s.sel.focus.path.last()).is_none() { Some("the caret is not in a scene") } else { None }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("element.hint", "Return and Tab Hint", "Format › Element", hint).pure(),
        CommandSpec::new("element.reset", "Reset to Element", "Format › Element", reset).key("Mod+Shift+R"),
        CommandSpec::new("select.scene", "Select Scene", "Edit", select_scene).key("Mod+Shift+A").when(in_scene).pure(),
        CommandSpec::new("text.pageBreak", "Page Break", "Insert", page_break).key("Mod+Shift+Enter"),
        CommandSpec::new("scene.insert", "New Scene", "Insert", insert_scene).key("Mod+Shift+N"),
        CommandSpec::new("paragraph.delete", "Delete Paragraph", "Edit", delete_paragraph),
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
    use crate::keys::ScriptRun;

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
                c.set_revised(Some(1));
            })
            .unwrap();
        s.run("element.reset", &json!({})).unwrap();
        let p = s.doc.para(StoryRef::Body, &Path::top(1)).unwrap();
        assert_eq!(p.props_at(1).bold, None);
        assert_eq!(p.props_at(1).revised(), Some(1));
        assert!(p.props.bookmark().is_some());
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

/// `dialogue.dual`: set the speech at the caret beside the one before it, or back under it.
fn dual(s: &mut Session, _: &Value) -> CmdResult {
    let block = s.sel.focus.path.last();
    let start = crate::paginate::dual::speech_start(&s.doc, block);
    let on = s.doc.body.get(start).and_then(|b| b.as_para()).is_some_and(|p| p.props.dual() == Some(true));
    if !on && let Some(why) = crate::paginate::dual::why_not(&s.doc, block) {
        return Err(CmdError::Failed(why.into()));
    }
    let para = s.doc.para_mut(StoryRef::Body, &Path::top(start))?;
    para.props.set_dual(if on { None } else { Some(true) });
    para.touch();
    Ok(json!({"dual": !on}))
}

pub fn dual_specs() -> Vec<CommandSpec> {
    vec![CommandSpec::new("dialogue.dual", "Dual Dialogue", "Format", dual).key("Mod+Shift+D")]
}

#[cfg(test)]
mod dual_tests {
    use super::*;
    use crate::commands::new_session;
    use crate::paginate::{Piece, paginate};
    use crate::template::paragraph;

    fn speeches() -> Session {
        let mut s = new_session();
        s.doc.body = [
            ("SceneHeading", "INT. HALL - DAY"),
            ("Character", "ANNA"),
            ("Dialogue", "We go now."),
            ("Character", "BEN"),
            ("Parenthetical", "(at once)"),
            ("Dialogue", "No, we stay here a while longer, because the rain will stop and then we can walk home dry."),
            ("Action", "They glare."),
        ]
        .iter()
        .map(|(e, t)| para_block(paragraph(e, t)))
        .collect();
        s
    }

    #[test]
    fn the_second_speech_goes_beside_the_first() {
        let mut s = speeches();
        s.sel = Selection::caret(body_pos(5, 3));
        assert_eq!(s.run("dialogue.dual", &json!({})).unwrap()["dual"], true);
        assert_eq!(crate::paginate::dual::pairs(&s.doc).len(), 1);
        let pag = paginate(&s.doc);
        let printed: Vec<usize> = pag.pages[0].iter().filter_map(|p| if let Piece::Para { block, .. } = p { Some(*block) } else { None }).collect();
        assert_eq!(printed, vec![0, 1, 2, 6], "the second speech is drawn in a box, not as paragraphs");
        let anna = pag.map.iter().position(|m| m.is_some_and(|m| m.block == 1)).unwrap_or(0);
        let obj = pag.doc.body[anna].as_para().map(|p| p.objects.len()).unwrap_or(0);
        assert_eq!(obj, 1, "the second speech's box sits on the first speech's name");
        // Toggled off from anywhere in the second speech.
        s.sel = Selection::caret(body_pos(4, 0));
        assert_eq!(s.run("dialogue.dual", &json!({})).unwrap()["dual"], false);
        assert!(crate::paginate::dual::pairs(&s.doc).is_empty());
    }

    #[test]
    fn dual_needs_a_speech_just_before() {
        let mut s = speeches();
        s.sel = Selection::caret(body_pos(2, 0));
        let err = s.run("dialogue.dual", &json!({})).unwrap_err().to_string();
        assert!(err.contains("no speech straight before"), "{err}");
        s.sel = Selection::caret(body_pos(6, 0));
        assert!(s.run("dialogue.dual", &json!({})).unwrap_err().to_string().contains("second of two speeches"));
    }
}
