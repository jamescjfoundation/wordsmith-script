// SPDX-License-Identifier: MIT OR Apache-2.0
//! Changing a paragraph's element (SPEC "Typing behavior": a command reformats the paragraph
//! under the cursor; `Mod+0` to `Mod+9` select the ten core elements).

use crate::keys::ScriptRun;
use jcj_doc::{Document, Path, Pos};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, params as p};
use serde_json::{Value, json};

use crate::element::{Casing, ElementDef, ElementKind, Elements};

/// The element of the paragraph at `at` (General when it is not a paragraph).
pub(crate) fn element_of_para_at<'a>(doc: &Document, elements: &'a Elements, at: &Pos) -> &'a ElementDef {
    match doc.para_at(at) {
        Some(p) => elements.of_para(p),
        None => elements.of_kind(ElementKind::General).unwrap_or_else(|| elements.of_para(&jcj_doc::Paragraph::new())),
    }
}

/// Make the paragraph at `path` (body story) element `id`: set its style, case its text, and
/// for a lone empty paragraph, write or remove the brackets of a Parenthetical. `caret` moves
/// the caret into the brackets when they are written.
pub(crate) fn apply_element(s: &mut Session, elements: &Elements, path: &Path, id: &str, caret: bool) -> Result<(), CmdError> {
    let target = elements.get(id).ok_or_else(|| CmdError::Params(format!("no element `{id}`")))?.clone();
    let story = s.sel.focus.story;
    let revising = crate::revisions::typing_set(&s.doc);
    let para = s.doc.para_mut(story, path)?;
    let was = elements.of_para(para).kind;
    if revising.is_some() && para.props.style.as_deref() != Some(target.id.as_str()) && !para.text.is_empty() {
        // In revision mode, changing a paragraph's element is a revision of it.
        para.mark.set_revised(revising);
    }
    para.props.style = Some(target.id.clone());
    if target.casing != Casing::AsTyped {
        recase(para, target.casing);
    }
    let mut caret_at = None;
    if target.kind == ElementKind::Parenthetical && para.text.is_empty() {
        para.insert_text(0, "()", &jcj_doc::props::CharProps::default())?;
        caret_at = Some(1);
    } else if was == ElementKind::Parenthetical && target.kind != ElementKind::Parenthetical && para.text == "()" {
        para.delete(0, 2)?;
        caret_at = Some(0);
    }
    para.touch();
    if caret && let Some(off) = caret_at {
        s.sel = Selection::caret(Pos { story, path: path.clone(), off });
    }
    Ok(())
}

/// Put a paragraph's text in the element's casing, run by run, so each run keeps its formatting
/// even where capitals are longer ("ß" → "SS").
fn recase(para: &mut jcj_doc::Paragraph, casing: Casing) {
    let upper = casing.apply(&para.text);
    if upper == para.text {
        return;
    }
    let mut text = String::with_capacity(upper.len());
    let mut start = 0;
    for run in &mut para.runs {
        let end = (start + run.len).min(para.text.len());
        let piece = casing.apply(para.text.get(start..end).unwrap_or(""));
        run.len = piece.len();
        text.push_str(&piece);
        start = end;
    }
    if let Some(rest) = para.text.get(start..) {
        text.push_str(&casing.apply(rest));
    }
    para.text = text;
}

/// `element.set {element}`: make every paragraph the selection touches element `element` (an id).
pub fn set(s: &mut Session, v: &Value) -> CmdResult {
    let id = p::req_str(v, "element")?.to_string();
    set_to(s, &id)
}

fn set_to(s: &mut Session, id: &str) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    if elements.get(id).is_none() {
        return Err(CmdError::Params(format!("no element `{id}`")));
    }
    let (a, b) = s.sel.ordered();
    let paths = if s.sel.is_collapsed() { vec![a.path.clone()] } else { s.doc.para_paths_in(&a, &b) };
    let single = paths.len() == 1;
    for path in &paths {
        apply_element(s, &elements, path, id, single)?;
    }
    s.clamp_selection();
    sel_result(s)
}

fn set_kind(s: &mut Session, kind: ElementKind) -> CmdResult {
    let id = Elements::of_doc(&s.doc).id_of(kind).to_string();
    set_to(s, &id)
}

/// `element.list`: the script's elements and the caret paragraph's element.
pub fn list(s: &mut Session, _: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    let current = element_of_para_at(&s.doc, &elements, &s.sel.focus).id.clone();
    Ok(json!({"current": current, "elements": elements.all()}))
}

/// One command per built-in element, with the core shortcuts.
pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Format › Element";
    vec![
        CommandSpec::new("element.set", "Change Element", LOC, set).params(r#"{"element": elementId}"#),
        CommandSpec::new("element.list", "Elements", LOC, list).pure(),
        CommandSpec::new("element.general", "General", LOC, |s, _| set_kind(s, ElementKind::General)).key("Mod+0"),
        CommandSpec::new("element.sceneHeading", "Scene Heading", LOC, |s, _| set_kind(s, ElementKind::SceneHeading)).key("Mod+1"),
        CommandSpec::new("element.action", "Action", LOC, |s, _| set_kind(s, ElementKind::Action)).key("Mod+2"),
        CommandSpec::new("element.character", "Character", LOC, |s, _| set_kind(s, ElementKind::Character)).key("Mod+3"),
        CommandSpec::new("element.parenthetical", "Parenthetical", LOC, |s, _| set_kind(s, ElementKind::Parenthetical)).key("Mod+4"),
        CommandSpec::new("element.dialogue", "Dialogue", LOC, |s, _| set_kind(s, ElementKind::Dialogue)).key("Mod+5"),
        CommandSpec::new("element.transition", "Transition", LOC, |s, _| set_kind(s, ElementKind::Transition)).key("Mod+6"),
        CommandSpec::new("element.shot", "Shot", LOC, |s, _| set_kind(s, ElementKind::Shot)).key("Mod+7"),
        CommandSpec::new("element.castList", "Cast List", LOC, |s, _| set_kind(s, ElementKind::CastList)).key("Mod+8"),
        CommandSpec::new("element.newAct", "New Act", LOC, |s, _| set_kind(s, ElementKind::NewAct)).key("Mod+9"),
        // Structure and outline elements on Mod+Shift and a digit (SPEC-2 19 suggests a shifted
        // digit; a bare Shift+digit types a symbol, so Mod is added).
        CommandSpec::new("element.outline1", "Outline 1", LOC, |s, _| set_kind(s, ElementKind::Outline1)).key("Mod+Shift+1"),
        CommandSpec::new("element.outline2", "Outline 2", LOC, |s, _| set_kind(s, ElementKind::Outline2)).key("Mod+Shift+2"),
        CommandSpec::new("element.outline3", "Outline 3", LOC, |s, _| set_kind(s, ElementKind::Outline3)).key("Mod+Shift+3"),
        CommandSpec::new("element.note", "Note", LOC, |s, _| set_kind(s, ElementKind::Note)).key("Mod+Shift+4"),
        CommandSpec::new("element.summary", "Summary", LOC, |s, _| set_kind(s, ElementKind::Summary)).key("Mod+Shift+5"),
        CommandSpec::new("element.sequence", "Sequence", LOC, |s, _| set_kind(s, ElementKind::Sequence)).key("Mod+Shift+6"),
        CommandSpec::new("element.endOfAct", "End of Act", LOC, |s, _| set_kind(s, ElementKind::EndOfAct)).key("Mod+Shift+7"),
    ]
}
