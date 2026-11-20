// SPDX-License-Identifier: MIT OR Apache-2.0
//! The outline editor's lanes (decision 0028): rows of beats, such as acts and the sequences
//! under them, kept with the story board's beats so edits in one view show in the other. Beats
//! sent to the script cover the pages from their Outline 1 paragraph to the next one.

use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session};

use crate::element::{ElementKind, Elements};
use crate::paginate::{Paginated, Piece};
use crate::storyboard::{Lane, MAX_LANES, Storyboard};

/// The structure templates: their names and lanes (name, indent).
pub const TEMPLATES: [(&str, &str); 4] = [
    ("three-act", "Three acts"),
    ("five-act", "Five acts"),
    ("sequences", "Three acts in eight sequences"),
    ("television", "Television: teaser, four acts, tag"),
];

fn template_lanes(name: &str) -> Option<Vec<(String, u8)>> {
    let acts =
        |n: usize| ["Act One", "Act Two", "Act Three", "Act Four", "Act Five"].iter().take(n).map(|a| (a.to_string(), 0u8)).collect::<Vec<_>>();
    Some(match name {
        "three-act" => acts(3),
        "five-act" => acts(5),
        "sequences" => {
            let seq = |a: usize, b: usize| (a..=b).map(|i| (format!("Sequence {i}"), 1u8)).collect::<Vec<_>>();
            let mut v = vec![("Act One".to_string(), 0)];
            v.extend(seq(1, 2));
            v.push(("Act Two".into(), 0));
            v.extend(seq(3, 6));
            v.push(("Act Three".into(), 0));
            v.extend(seq(7, 8));
            v
        }
        "television" => {
            let mut v = vec![("Teaser".to_string(), 0)];
            v.extend(acts(4));
            v.push(("Tag".into(), 0));
            v
        }
        _ => return None,
    })
}

fn lane_param(v: &Value, board: &Storyboard) -> Result<u32, CmdError> {
    let id =
        v.get("id").and_then(Value::as_u64).and_then(|i| u32::try_from(i).ok()).ok_or_else(|| CmdError::Params("`id` must be a lane id".into()))?;
    if !board.lanes.iter().any(|l| l.id == id) {
        return Err(CmdError::Params(format!("there is no lane {id}")));
    }
    Ok(id)
}

fn name_of(v: &Value) -> Result<String, CmdError> {
    let n: String =
        v.get("name").and_then(Value::as_str).unwrap_or("").chars().filter(|c| !c.is_control()).take(100).collect::<String>().trim().to_string();
    if n.is_empty() { Err(CmdError::Params("a lane needs a name".into())) } else { Ok(n) }
}

fn next_lane_id(board: &Storyboard) -> u32 {
    board.lanes.iter().map(|l| l.id).max().map_or(1, |m| m + 1)
}

/// `lane.add {name, indent?}`: a lane at the end.
fn add(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    if board.lanes.len() >= MAX_LANES {
        return Err(CmdError::Failed(format!("an outline holds at most {MAX_LANES} lanes")));
    }
    let id = next_lane_id(&board);
    let indent = v.get("indent").and_then(Value::as_u64).map_or(0, |i| i.min(2) as u8);
    board.lanes.push(Lane { id, name: name_of(v)?, indent });
    board.store(&mut s.doc);
    Ok(json!({"id": id}))
}

/// `lane.rename {id, name}`.
fn rename(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    let id = lane_param(v, &board)?;
    let name = name_of(v)?;
    if let Some(l) = board.lanes.iter_mut().find(|l| l.id == id) {
        l.name = name;
    }
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `lane.move {id, to}`: move a lane to position `to`.
fn move_lane(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    let id = lane_param(v, &board)?;
    let to = v.get("to").and_then(Value::as_u64).map(|t| t as usize).ok_or_else(|| CmdError::Params("`to` must be a position".into()))?;
    let from = board.lanes.iter().position(|l| l.id == id).unwrap_or(0);
    let lane = board.lanes.remove(from);
    board.lanes.insert(to.min(board.lanes.len()), lane);
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `lane.indent {id, indent}`: 0 to 2.
fn indent(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    let id = lane_param(v, &board)?;
    let n = v.get("indent").and_then(Value::as_u64).filter(|n| *n <= 2).ok_or_else(|| CmdError::Params("`indent` is 0, 1 or 2".into()))?;
    if let Some(l) = board.lanes.iter_mut().find(|l| l.id == id) {
        l.indent = n as u8;
    }
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `lane.delete {id}`: delete a lane; its beats stay, in no lane.
fn delete(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    let id = lane_param(v, &board)?;
    board.lanes.retain(|l| l.id != id);
    for b in board.beats.iter_mut().filter(|b| b.lane == Some(id)) {
        b.lane = None;
    }
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `beat.place {id, lane: id | null, index?}`: put a beat in a lane (or none) at position
/// `index` (the end when left out).
fn place(s: &mut Session, v: &Value) -> CmdResult {
    let mut board = Storyboard::of_doc(&s.doc);
    let id = v
        .get("id")
        .and_then(Value::as_u64)
        .and_then(|i| u32::try_from(i).ok())
        .filter(|i| board.beat(*i).is_some())
        .ok_or_else(|| CmdError::Params("`id` must be a beat id".into()))?;
    let lane = match v.get("lane") {
        None | Some(Value::Null) => None,
        Some(l) => Some(
            l.as_u64()
                .and_then(|i| u32::try_from(i).ok())
                .filter(|i| board.lanes.iter().any(|x| x.id == *i))
                .ok_or_else(|| CmdError::Params("`lane` must be a lane id".into()))?,
        ),
    };
    let mut order: Vec<u32> = board.lane_beats(lane).iter().map(|b| b.id).filter(|b| *b != id).collect();
    let index = v.get("index").and_then(Value::as_u64).map_or(order.len(), |i| (i as usize).min(order.len()));
    order.insert(index, id);
    for b in board.beats.iter_mut() {
        if let Some(pos) = order.iter().position(|x| *x == b.id) {
            b.lane = lane;
            b.order = (pos + 1) as f32;
        }
    }
    board.store(&mut s.doc);
    Ok(Value::Null)
}

/// `outline.template {name}`: add the lanes of a structure template after the lanes there are.
fn template(s: &mut Session, v: &Value) -> CmdResult {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    let lanes = template_lanes(name)
        .ok_or_else(|| CmdError::Params(format!("templates: {}", TEMPLATES.iter().map(|t| t.0).collect::<Vec<_>>().join(", "))))?;
    let mut board = Storyboard::of_doc(&s.doc);
    if board.lanes.len() + lanes.len() > MAX_LANES {
        return Err(CmdError::Failed(format!("an outline holds at most {MAX_LANES} lanes")));
    }
    for (n, indent) in lanes {
        let id = next_lane_id(&board);
        board.lanes.push(Lane { id, name: n, indent });
    }
    board.store(&mut s.doc);
    Ok(json!({"lanes": board.lanes.len()}))
}

/// The pages (from 1) each beat sent to the script covers: from its Outline 1 paragraph's page to
/// the page before the next Outline 1 paragraph, or the script's last page.
pub fn beat_pages(doc: &Document, pag: &Paginated) -> std::collections::HashMap<u32, (usize, usize)> {
    let elements = Elements::of_doc(doc);
    let mut first_page = std::collections::HashMap::new();
    let mut last_page = std::collections::HashMap::new();
    for (n, page) in pag.pages.iter().enumerate() {
        for piece in page {
            if let Piece::Para { block, .. } = piece {
                first_page.entry(*block).or_insert(n + 1);
                last_page.insert(*block, n + 1);
            }
        }
    }
    let tops: Vec<usize> = doc
        .body
        .iter()
        .enumerate()
        .filter(|(_, b)| b.as_para().is_some_and(|p| elements.of_para(p).kind == ElementKind::Outline1))
        .map(|(i, _)| i)
        .collect();
    let mut out = std::collections::HashMap::new();
    for (k, &start) in tops.iter().enumerate() {
        let Some(id) = doc.body.get(start).and_then(|b| b.as_para()).and_then(|p| p.props.beat) else { continue };
        let end_block = tops.get(k + 1).map_or(doc.body.len(), |n| *n).saturating_sub(1);
        let from = (start..=end_block).find_map(|b| first_page.get(&b).copied());
        let to = (start..=end_block).rev().find_map(|b| last_page.get(&b).copied());
        if let (Some(a), Some(z)) = (from, to) {
            out.insert(id, (a, z.max(a)));
        }
    }
    out
}

/// `outline.pages`: the pages each sent beat covers.
fn pages(s: &mut Session, _: &Value) -> CmdResult {
    let pag = crate::paginate::paginate(&s.doc);
    let map = beat_pages(&s.doc, &pag);
    let mut v: Vec<Value> = map.into_iter().map(|(id, (a, z))| json!({"beat": id, "from": a, "to": z})).collect();
    v.sort_by_key(|x| x["beat"].as_u64());
    Ok(json!(v))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("lane.add", "Add Lane", "Outline", add).params(r#"{"name": string, "indent"?: 0-2}"#),
        CommandSpec::new("lane.rename", "Rename Lane", "Outline", rename).params(r#"{"id": lane id, "name": string}"#),
        CommandSpec::new("lane.move", "Move Lane", "Outline", move_lane).params(r#"{"id": lane id, "to": position}"#),
        CommandSpec::new("lane.indent", "Indent Lane", "Outline", indent).params(r#"{"id": lane id, "indent": 0-2}"#),
        CommandSpec::new("lane.delete", "Delete Lane", "Outline", delete).params(r#"{"id": lane id}"#),
        CommandSpec::new("beat.place", "Place Beat", "Outline", place).params(r#"{"id": beat id, "lane": lane id | null, "index"?: position}"#),
        CommandSpec::new("outline.template", "Add Structure Template", "Outline", template)
            .params(r#"{"name": "three-act" | "five-act" | "sequences" | "television"}"#),
        CommandSpec::new("outline.pages", "Beat Pages", "Outline", pages).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wordsmith_doc::para_block;

    use super::*;
    use crate::template::paragraph;

    fn board(s: &Session) -> Storyboard {
        Storyboard::of_doc(&s.doc)
    }

    #[test]
    fn lanes_are_added_renamed_moved_indented_and_deleted() {
        let mut s = crate::new_session();
        s.run("outline.template", &json!({"name": "three-act"})).unwrap();
        assert_eq!(board(&s).lanes.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Act One", "Act Two", "Act Three"]);
        let extra = s.run("lane.add", &json!({"name": "B story"})).unwrap()["id"].as_u64().unwrap();
        s.run("lane.move", &json!({"id": extra, "to": 1})).unwrap();
        s.run("lane.rename", &json!({"id": extra, "name": "Romance"})).unwrap();
        s.run("lane.indent", &json!({"id": extra, "indent": 1})).unwrap();
        assert!(s.run("lane.indent", &json!({"id": extra, "indent": 3})).is_err());
        let b = board(&s);
        assert_eq!((b.lanes[1].name.as_str(), b.lanes[1].indent), ("Romance", 1));
        // Beats placed in a lane keep their order; deleting the lane leaves them unassigned.
        let ids: Vec<u64> = (0..3).map(|i| s.run("beat.add", &json!({"title": format!("B{i}")})).unwrap()["id"].as_u64().unwrap()).collect();
        for id in &ids {
            s.run("beat.place", &json!({"id": id, "lane": extra})).unwrap();
        }
        s.run("beat.place", &json!({"id": ids[2], "lane": extra, "index": 0})).unwrap();
        let order: Vec<String> = board(&s).lane_beats(Some(extra as u32)).iter().map(|b| b.title.clone()).collect();
        assert_eq!(order, ["B2", "B0", "B1"]);
        s.run("lane.delete", &json!({"id": extra})).unwrap();
        assert_eq!(board(&s).lane_beats(None).len(), 3);
        assert!(s.run("outline.template", &json!({"name": "hero"})).is_err());
        s.run("outline.template", &json!({"name": "sequences"})).unwrap();
        assert_eq!(board(&s).lanes.iter().filter(|l| l.indent == 1).count(), 8);
    }

    #[test]
    fn sent_beats_cover_their_pages() {
        let mut s = crate::new_session();
        let a = s.run("beat.add", &json!({"title": "Setup"})).unwrap()["id"].as_u64().unwrap();
        let b = s.run("beat.add", &json!({"title": "Payoff"})).unwrap()["id"].as_u64().unwrap();
        // Setup: two pages of action; Payoff: a few lines.
        let mut body: Vec<_> = (0..60).map(|i| para_block(paragraph("Action", &format!("Line {i}.")))).collect();
        body.push(para_block(paragraph("Action", "End.")));
        s.doc.body = body;
        s.sel = wordsmith_engine::Selection::caret(wordsmith_doc::Pos { path: wordsmith_doc::Path::top(0), off: 0, ..s.sel.focus.clone() });
        s.run("beat.toScript", &json!({"id": a})).unwrap();
        let end = s.doc.body.len() - 1;
        s.sel = wordsmith_engine::Selection::caret(wordsmith_doc::Pos { path: wordsmith_doc::Path::top(end), off: 0, ..s.sel.focus.clone() });
        s.run("beat.toScript", &json!({"id": b})).unwrap();
        let pages = s.run("outline.pages", &json!({})).unwrap();
        assert_eq!(pages, json!([{"beat": a, "from": 1, "to": 3}, {"beat": b, "from": 3, "to": 3}]));
    }
}
