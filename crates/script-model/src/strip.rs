// SPDX-License-Identifier: MIT OR Apache-2.0
//! The structure strip (SPEC-2 9.3): the script's structure as lanes of blocks along its pages,
//! one lane per chosen structure element (Outline 1, acts, Sequence… by default) and one for
//! the scenes. A block runs from its paragraph to the next of the same or a broader rank, and
//! is as long as the pages it spans. Which elements feed the lanes is kept in `app_data` under
//! [`STRIP_KEY`].

use jcj_doc::Document;
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::element::{ElementKind, Elements};
use crate::paginate::{Piece, paginate};

/// `app_data` key of the strip's lanes.
pub const STRIP_KEY: &str = "wordsmith.strip";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StripSettings {
    /// Element ids feeding the lanes, broadest first. The scenes lane is always last.
    pub lanes: Vec<String>,
}

impl Default for StripSettings {
    fn default() -> Self {
        StripSettings { lanes: vec!["Outline1".into(), "NewAct".into(), "Sequence".into()] }
    }
}

/// One block of a lane: its label, where it starts and ends (pages, fractional, from 0), its
/// color and the body block to go to.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub label: String,
    pub start: f32,
    pub end: f32,
    pub color: String,
    pub block: usize,
}

/// Each body block's position as a fraction of pages (page index + how far down it starts).
fn positions(doc: &Document) -> (Vec<f32>, usize) {
    let pag = paginate(doc);
    let mut at = vec![f32::NAN; doc.body.len()];
    for (n, page) in pag.pages.iter().enumerate() {
        let page_lines: usize = page.iter().map(|p| if let Piece::Para { lines, .. } = p { *lines } else { 1 }).sum::<usize>().max(1);
        let mut seen = 0usize;
        for piece in page {
            if let Piece::Para { block, lines, .. } = piece {
                if let Some(slot) = at.get_mut(*block)
                    && slot.is_nan()
                {
                    *slot = n as f32 + seen as f32 / page_lines as f32;
                }
                seen += lines;
            } else {
                seen += 1;
            }
        }
    }
    // Blocks not printed (hidden outline paragraphs) take the place of the next printed one.
    let total = pag.pages.len();
    let mut next = total as f32;
    for x in at.iter_mut().rev() {
        if x.is_nan() {
            *x = next;
        } else {
            next = *x;
        }
    }
    (at, total)
}

/// The strip's lanes: (lane name, blocks), the scenes lane last, and the page count.
pub fn lanes(doc: &Document) -> (Vec<(String, Vec<Block>)>, usize) {
    let settings: StripSettings = crate::data::load(doc, STRIP_KEY);
    let elements = Elements::of_doc(doc);
    let (at, total) = positions(doc);
    let end_of_script = total as f32;
    let rank_of = |i: usize| doc.body.get(i).and_then(|b| b.as_para()).and_then(|p| elements.of_para(p).kind.structure_rank());
    let mut out = Vec::new();
    for id in &settings.lanes {
        let Some(def) = elements.get(id) else { continue };
        let my_rank = def.kind.structure_rank().unwrap_or(0);
        let mut blocks = Vec::new();
        for (i, b) in doc.body.iter().enumerate() {
            let Some(p) = b.as_para() else { continue };
            if p.props.style.as_deref() != Some(def.id.as_str()) {
                continue;
            }
            // Ends where the next paragraph of this rank or broader starts.
            let end =
                (i + 1..doc.body.len()).find(|&j| rank_of(j).is_some_and(|r| r <= my_rank)).and_then(|j| at.get(j).copied()).unwrap_or(end_of_script);
            let label = if p.text.trim().is_empty() { def.name.clone() } else { p.text.clone() };
            blocks.push(Block { label, start: at.get(i).copied().unwrap_or(0.0), end, color: String::new(), block: i });
        }
        out.push((def.name.clone(), blocks));
    }
    let scenes = crate::scenes::scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let scene_blocks = scenes
        .iter()
        .map(|sc| {
            let data = crate::scene_data::of_scene(doc, sc);
            let color = crate::scene_data::SCENE_COLORS.iter().find(|(n, _)| *n == data.color).map_or(String::new(), |(_, hex)| (*hex).to_string());
            let number = numbers.get(sc.index).cloned().unwrap_or_else(|| (sc.index + 1).to_string());
            let label = if data.title.is_empty() { format!("{number} {}", sc.heading) } else { format!("{number} {}", data.title) };
            Block {
                label,
                start: at.get(sc.start).copied().unwrap_or(0.0),
                end: at.get(sc.end).copied().unwrap_or(end_of_script),
                color,
                block: sc.start,
            }
        })
        .collect();
    out.push(("Scenes".into(), scene_blocks));
    (out, total)
}

fn get(s: &mut Session, _: &Value) -> CmdResult {
    let (lanes, pages) = lanes(&s.doc);
    let lanes: Vec<Value> = lanes
        .into_iter()
        .map(|(name, blocks)| {
            json!({"name": name, "blocks": blocks.into_iter().map(|b| json!({"label": b.label, "start": b.start, "end": b.end, "color": b.color, "block": b.block})).collect::<Vec<_>>()})
        })
        .collect();
    Ok(json!({"pages": pages, "lanes": lanes, "settings": crate::data::load::<StripSettings>(&s.doc, STRIP_KEY)}))
}

/// `strip.configure {lanes}`: the elements feeding the lanes (structure elements only).
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    let list = v.get("lanes").and_then(Value::as_array).ok_or_else(|| CmdError::Params("`lanes` is a list of element ids".into()))?;
    let mut lanes = Vec::new();
    for x in list.iter().take(8) {
        let id = x.as_str().unwrap_or("");
        let def = elements.get(id).ok_or_else(|| CmdError::Params(format!("no element `{id}`")))?;
        if def.kind.structure_rank().is_none() || def.kind == ElementKind::SceneHeading {
            return Err(CmdError::Params(format!("`{id}` is not a structure element")));
        }
        if !lanes.contains(&def.id) {
            lanes.push(def.id.clone());
        }
    }
    let settings = StripSettings { lanes };
    crate::data::store(&mut s.doc, STRIP_KEY, &settings);
    Ok(json!(settings))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("strip.get", "Structure Strip", "View", get).pure(),
        CommandSpec::new("strip.configure", "Structure Strip Lanes", "View", configure).params(r#"{"lanes": [structureElementId]}"#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::template::paragraph;
    use jcj_doc::para_block;

    #[test]
    fn lanes_follow_acts_sequences_and_scenes() {
        let mut s = new_session();
        let mut body = vec![("NewAct", "ACT ONE"), ("Sequence", "THE ARRIVAL"), ("SceneHeading", "INT. HALL - DAY"), ("Action", "Rain.")];
        body.extend([("SceneHeading", "EXT. YARD - DAY"), ("Action", "Sun."), ("NewAct", "ACT TWO"), ("SceneHeading", "INT. ATTIC - NIGHT")]);
        s.doc.body = body.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
        let (lanes, pages) = lanes(&s.doc);
        assert_eq!(pages, 2, "act two starts a page");
        let acts = &lanes.iter().find(|l| l.0 == "New Act").unwrap_or(&lanes[0]).1;
        assert_eq!(acts.len(), 2);
        assert_eq!((acts[0].start, acts[0].end), (0.0, 1.0));
        assert_eq!(acts[1].end, 2.0);
        let seq = &lanes.iter().find(|l| l.0 == "Sequence").unwrap_or(&lanes[0]).1;
        assert_eq!(seq[0].end, 1.0, "a sequence ends at the next act");
        let scenes = &lanes.last().unwrap_or(&lanes[0]).1;
        assert_eq!(scenes.len(), 3);
        assert!(scenes[0].end <= scenes[1].start + 1e-4);
        assert_eq!(scenes[2].block, 7);
    }

    #[test]
    fn lanes_are_configurable() {
        let mut s = new_session();
        s.run("strip.configure", &json!({"lanes": ["Outline2", "Sequence"]})).unwrap();
        assert_eq!(s.run("strip.get", &json!({})).unwrap()["lanes"].as_array().map(Vec::len), Some(3));
        assert!(s.run("strip.configure", &json!({"lanes": ["Action"]})).is_err());
        assert!(s.run("strip.configure", &json!({"lanes": ["Nope"]})).is_err());
    }
}
