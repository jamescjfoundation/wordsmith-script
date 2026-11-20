// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scene commands, for the index cards, the scene view and the scenes table: list scenes, move
//! them, set a heading or a summary, and go to one. Scenes are not stored (`crate::scenes`): a
//! move moves the scene's paragraphs, so every view shows the one script, and undo puts them
//! back (decision 0022).

use std::sync::Arc;

use serde_json::{Value, json};
use wordsmith_doc::{Block, Document, Pos, StoryRef, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result};

use crate::element::{ElementKind, Elements};
use crate::paginate::output::speaker;
use crate::scenes::{Scene, scenes};
use crate::template::paragraph;

/// Characters of text a summary or first action shows in a listing.
pub const MAX_PREVIEW_CHARS: usize = 400;
/// Most scenes one move takes.
pub const MAX_MOVED: usize = 10_000;

fn preview(text: &str) -> String {
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > MAX_PREVIEW_CHARS { format!("{}…", t.chars().take(MAX_PREVIEW_CHARS).collect::<String>()) } else { t }
}

/// The first paragraph of `kind` in `scene`, as (block, text).
fn first_of(doc: &Document, elements: &Elements, scene: &Scene, kind: ElementKind) -> Option<(usize, String)> {
    (scene.start + 1..scene.end).find_map(|i| {
        let p = doc.body.get(i)?.as_para()?;
        (elements.of_para(p).kind == kind).then(|| (i, p.text.clone()))
    })
}

/// The speakers in `scene`, in order of first speech.
pub fn speakers(doc: &Document, elements: &Elements, scene: &Scene) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in scene.start..scene.end {
        if let Some(p) = doc.body.get(i).and_then(|b| b.as_para())
            && elements.of_para(p).kind == ElementKind::Character
        {
            let name = speaker(&p.text).trim().to_string();
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// One scene as a listing shows it.
pub fn scene_json(doc: &Document, elements: &Elements, scene: &Scene) -> Value {
    json!({
        "index": scene.index,
        "heading": scene.heading,
        "start": scene.start,
        "end": scene.end,
        "summary": first_of(doc, elements, scene, ElementKind::Summary).map(|(_, t)| preview(&t)).unwrap_or_default(),
        "firstAction": first_of(doc, elements, scene, ElementKind::Action).map(|(_, t)| preview(&t)).unwrap_or_default(),
        "characters": speakers(doc, elements, scene),
        "paragraphs": scene.end - scene.start,
    })
}

/// `scene.list`: every scene, in order.
fn list(s: &mut Session, _: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    Ok(json!(scenes(&s.doc).iter().map(|sc| scene_json(&s.doc, &elements, sc)).collect::<Vec<_>>()))
}

fn scene_param(v: &Value, key: &str, count: usize) -> Result<usize, CmdError> {
    let i = v.get(key).and_then(Value::as_u64).ok_or_else(|| CmdError::Params(format!("`{key}` must be a scene number from 0")))? as usize;
    if i >= count {
        return Err(CmdError::Params(format!("`{key}`: there are only {count} scenes")));
    }
    Ok(i)
}

/// `scene.move {scenes: [i, …] | scene: i, before?: j}`: move the scenes, in script order, to
/// just before scene `j`, or to the end of the script when `before` is left out.
fn move_scenes(s: &mut Session, v: &Value) -> CmdResult {
    let all = scenes(&s.doc);
    let mut chosen: Vec<usize> = match (v.get("scenes"), v.get("scene")) {
        (Some(Value::Array(list)), _) => list
            .iter()
            .take(MAX_MOVED)
            .map(|x| {
                x.as_u64()
                    .map(|i| i as usize)
                    .filter(|i| *i < all.len())
                    .ok_or_else(|| CmdError::Params("`scenes` holds a scene that is not there".into()))
            })
            .collect::<Result<_, _>>()?,
        (_, Some(_)) => vec![scene_param(v, "scene", all.len())?],
        _ => return Err(CmdError::Params("give `scene` or `scenes`".into())),
    };
    chosen.sort_unstable();
    chosen.dedup();
    if chosen.is_empty() {
        return sel_result(s);
    }
    let before = match v.get("before") {
        None | Some(Value::Null) => None,
        Some(_) => Some(scene_param(v, "before", all.len())?),
    };
    if before.is_some_and(|b| chosen.contains(&b)) {
        // Before itself: nothing moves.
        return sel_result(s);
    }
    // Take the chosen scenes' blocks out, then put them back before the target.
    let mut moving: Vec<Arc<Block>> = Vec::new();
    for &i in &chosen {
        let sc = &all[i];
        moving.extend(s.doc.body[sc.start..sc.end].iter().cloned());
    }
    let target_block = before.map_or(s.doc.body.len(), |b| all[b].start);
    let removed_before_target: usize = chosen.iter().map(|&i| &all[i]).filter(|sc| sc.end <= target_block).map(|sc| sc.end - sc.start).sum();
    let mut keep: Vec<Arc<Block>> = Vec::with_capacity(s.doc.body.len());
    for (i, b) in s.doc.body.iter().enumerate() {
        if !chosen.iter().any(|&c| (all[c].start..all[c].end).contains(&i)) {
            keep.push(b.clone());
        }
    }
    let at = target_block - removed_before_target;
    let mut body = keep;
    body.splice(at..at, moving);
    // Scripts have one section (the document's last), so moving paragraphs moves no section
    // break.
    s.doc.body = body;
    s.sel = Selection::caret(Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(at), off: 0 });
    sel_result(s)
}

/// `scene.setHeading {scene, text}`: replace a scene's heading (in the heading's casing).
fn set_heading(s: &mut Session, v: &Value) -> CmdResult {
    let all = scenes(&s.doc);
    let i = scene_param(v, "scene", all.len())?;
    let text = v.get("text").and_then(Value::as_str).ok_or_else(|| CmdError::Params("`text` must be text".into()))?;
    let text: String = text.chars().filter(|c| !c.is_control()).take(1_000).collect();
    let elements = Elements::of_doc(&s.doc);
    let def = elements.of_kind(ElementKind::SceneHeading).cloned();
    let block = all[i].start;
    let Some(old) = s.doc.body.get(block).and_then(|b| b.as_para()) else { return Err(CmdError::Failed("the heading is not a paragraph".into())) };
    let style = old.props.style.clone().unwrap_or_default();
    let cased = def.map_or(text.clone(), |d| d.casing.apply(&text));
    let mut p = paragraph(&style, &cased);
    p.props = old.props.clone();
    s.doc.body[block] = para_block(p);
    sel_result(s)
}

/// `scene.setSummary {scene, text}`: set a scene's summary: its first Summary paragraph, added
/// right after the heading when it has none, removed when the text is empty.
fn set_summary(s: &mut Session, v: &Value) -> CmdResult {
    let all = scenes(&s.doc);
    let i = scene_param(v, "scene", all.len())?;
    let text = v.get("text").and_then(Value::as_str).ok_or_else(|| CmdError::Params("`text` must be text".into()))?;
    let text: String = text.replace("\r\n", "\n").chars().filter(|c| *c == '\n' || !c.is_control()).take(5_000).collect();
    let text = text.trim();
    let elements = Elements::of_doc(&s.doc);
    let id = elements.id_of(ElementKind::Summary).to_string();
    match (first_of(&s.doc, &elements, &all[i], ElementKind::Summary), text.is_empty()) {
        (Some((block, _)), true) => {
            s.doc.body.remove(block);
        }
        (Some((block, _)), false) => s.doc.body[block] = para_block(paragraph(&id, text)),
        (None, false) => s.doc.body.insert(all[i].start + 1, para_block(paragraph(&id, text))),
        (None, true) => {}
    }
    s.clamp_selection();
    sel_result(s)
}

/// `scene.goto {scene}`: put the caret at the start of a scene's heading.
fn goto(s: &mut Session, v: &Value) -> CmdResult {
    let all = scenes(&s.doc);
    let i = scene_param(v, "scene", all.len())?;
    s.sel = Selection::caret(Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(all[i].start), off: 0 });
    sel_result(s)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("scene.list", "List Scenes", "Script", list).pure(),
        CommandSpec::new("scene.move", "Move Scenes", "Script", move_scenes)
            .params(r#"{"scene"?: number, "scenes"?: [number], "before"?: number (to the end when left out)}"#),
        CommandSpec::new("scene.setHeading", "Set Scene Heading", "Script", set_heading).params(r#"{"scene": number, "text": string}"#),
        CommandSpec::new("scene.setSummary", "Set Scene Summary", "Script", set_summary)
            .params(r#"{"scene": number, "text": string (empty removes it)}"#),
        CommandSpec::new("scene.goto", "Go to Scene", "Script", goto).params(r#"{"scene": number}"#).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::text::body_lines;
    use crate::template::new_script;

    fn session() -> Session {
        let mut doc = new_script();
        doc.body = [
            ("Action", "FADE IN:"),
            ("SceneHeading", "INT. A - DAY"),
            ("Action", "One."),
            ("Character", "ANNA"),
            ("Dialogue", "Hi."),
            ("SceneHeading", "INT. B - DAY"),
            ("Action", "Two."),
            ("SceneHeading", "INT. C - DAY"),
            ("Character", "BOB (V.O.)"),
            ("Dialogue", "Three."),
            ("Character", "ANNA"),
            ("Dialogue", "Yes."),
        ]
        .iter()
        .map(|(e, t)| para_block(paragraph(e, t)))
        .collect();
        crate::session_for(doc)
    }

    fn headings(s: &Session) -> Vec<String> {
        scenes(&s.doc).into_iter().map(|sc| sc.heading).collect()
    }

    #[test]
    fn listing_scenes() {
        let mut s = session();
        let list = s.run("scene.list", &json!({})).unwrap();
        assert_eq!(list.as_array().map(Vec::len), Some(3));
        assert_eq!(list[0]["firstAction"], "One.");
        assert_eq!(list[2]["characters"], json!(["BOB", "ANNA"]));
    }

    #[test]
    fn moving_scenes_and_undo() {
        let mut s = session();
        let before = body_lines(&s);
        s.run("scene.move", &json!({"scene": 2, "before": 0})).unwrap();
        assert_eq!(headings(&s), ["INT. C - DAY", "INT. A - DAY", "INT. B - DAY"]);
        assert_eq!(body_lines(&s)[0].1, "FADE IN:", "text before the first scene stays first");
        assert_eq!(s.sel.focus.path.last(), 1, "the caret is at the moved scene");
        s.run("scene.move", &json!({"scene": 0})).unwrap();
        assert_eq!(headings(&s), ["INT. A - DAY", "INT. B - DAY", "INT. C - DAY"], "to the end");
        s.run("scene.move", &json!({"scenes": [2, 0], "before": 1})).unwrap();
        assert_eq!(headings(&s), ["INT. A - DAY", "INT. C - DAY", "INT. B - DAY"], "several, in script order");
        assert_eq!(body_lines(&s).len(), before.len(), "nothing lost");
        s.run("scene.move", &json!({"scene": 1, "before": 1})).unwrap();
        assert_eq!(headings(&s), ["INT. A - DAY", "INT. C - DAY", "INT. B - DAY"], "before itself: no change");
        for _ in 0..3 {
            s.run("edit.undo", &json!({})).unwrap();
        }
        assert_eq!(body_lines(&s), before);
        assert!(s.run("scene.move", &json!({"scene": 9})).is_err());
        assert!(s.run("scene.move", &json!({"scenes": [0, 99]})).is_err());
    }

    #[test]
    fn headings_and_summaries() {
        let mut s = session();
        s.run("scene.setHeading", &json!({"scene": 1, "text": "ext. garden - night"})).unwrap();
        assert_eq!(headings(&s)[1], "EXT. GARDEN - NIGHT");
        s.run("scene.setSummary", &json!({"scene": 0, "text": "Anna says hi."})).unwrap();
        assert_eq!(body_lines(&s)[2], ("Summary".to_string(), "Anna says hi.".to_string()));
        s.run("scene.setSummary", &json!({"scene": 0, "text": "Anna waves."})).unwrap();
        let list = s.run("scene.list", &json!({})).unwrap();
        assert_eq!(list[0]["summary"], "Anna waves.");
        s.run("scene.setSummary", &json!({"scene": 0, "text": ""})).unwrap();
        assert!(body_lines(&s).iter().all(|(e, _)| e != "Summary"));
        s.run("scene.goto", &json!({"scene": 2})).unwrap();
        assert_eq!(s.sel.focus.path.last(), 7);
    }
}
