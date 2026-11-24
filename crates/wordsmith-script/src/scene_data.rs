// SPDX-License-Identifier: MIT OR Apache-2.0
//! A scene's own data (SPEC-2 9.2): a title, a color, characters who are in the scene without
//! speaking, speakers hidden from its cast, and an arc beat per character. Kept in `app_data`
//! under [`SCENE_DATA_KEY`] by id; the id is on the scene's heading paragraph
//! (`ParaProps::scene_id`), so the data moves with the scene when scenes are reordered.

use crate::keys::ScriptPara;
use std::collections::BTreeMap;

use jcj_doc::{Document, Path, StoryRef};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::data::clean_line;
use crate::element::Elements;
use crate::scenes::{Scene, scenes};

/// `app_data` key of the scenes' data.
pub const SCENE_DATA_KEY: &str = "wordsmith.sceneData";
pub const MAX_SCENES: usize = 20_000;
pub const MAX_TITLE: usize = 200;
pub const MAX_ARC: usize = 2_000;
pub const MAX_CAST: usize = 200;

/// Scene colors (none plus seven, our names and values).
pub const SCENE_COLORS: [(&str, &str); 7] = [
    ("Red", "#E57373"),
    ("Orange", "#FFB74D"),
    ("Yellow", "#FFF176"),
    ("Green", "#81C784"),
    ("Blue", "#64B5F6"),
    ("Purple", "#BA68C8"),
    ("Gray", "#B0BEC5"),
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SceneData {
    pub title: String,
    /// One of [`SCENE_COLORS`]' names, or empty.
    pub color: String,
    /// Characters in the scene who do not speak.
    pub non_speaking: Vec<String>,
    /// Speakers left out of the scene's cast list.
    pub hidden: Vec<String>,
    /// Arc beats by character.
    pub arcs: BTreeMap<String, String>,
}

impl SceneData {
    fn repaired(mut self) -> Self {
        self.title = clean_line(&self.title, MAX_TITLE);
        if !SCENE_COLORS.iter().any(|(n, _)| *n == self.color) {
            self.color.clear();
        }
        let names = |v: &mut Vec<String>| {
            *v = v.iter().map(|n| clean_line(n, 100).to_uppercase()).filter(|n| !n.is_empty()).take(MAX_CAST).collect();
            v.dedup();
        };
        names(&mut self.non_speaking);
        names(&mut self.hidden);
        self.arcs = std::mem::take(&mut self.arcs)
            .into_iter()
            .take(MAX_CAST)
            .map(|(k, v)| (clean_line(&k, 100).to_uppercase(), v.chars().filter(|c| *c == '\n' || !c.is_control()).take(MAX_ARC).collect::<String>()))
            .filter(|(k, v)| !k.is_empty() && !v.trim().is_empty())
            .collect();
        self
    }

    pub fn is_empty(&self) -> bool {
        *self == SceneData::default()
    }
}

/// Every scene's data, by id.
pub fn all(doc: &Document) -> BTreeMap<u32, SceneData> {
    let raw: BTreeMap<String, SceneData> = crate::data::load(doc, SCENE_DATA_KEY);
    raw.into_iter().take(MAX_SCENES).filter_map(|(k, v)| k.parse().ok().map(|id| (id, v.repaired()))).collect()
}

fn store(doc: &mut Document, data: &BTreeMap<u32, SceneData>) {
    let raw: BTreeMap<String, &SceneData> = data.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.to_string(), v)).collect();
    crate::data::store(doc, SCENE_DATA_KEY, &raw);
}

/// The data of `scene`.
pub fn of_scene(doc: &Document, scene: &Scene) -> SceneData {
    of_scene_in(&all(doc), doc, scene)
}

/// The data of `scene` from `data` (every scene's, read once with [`all`]).
pub fn of_scene_in(data: &BTreeMap<u32, SceneData>, doc: &Document, scene: &Scene) -> SceneData {
    let id = doc.body.get(scene.start).and_then(|b| b.as_para()).and_then(|p| p.props.scene_id());
    id.and_then(|id| data.get(&id).cloned()).unwrap_or_default()
}

/// The id of `scene`'s data, given to its heading now if it has none.
fn ensure_id(doc: &mut Document, scene: &Scene, data: &BTreeMap<u32, SceneData>) -> Result<u32, CmdError> {
    if let Some(id) = doc.body.get(scene.start).and_then(|b| b.as_para()).and_then(|p| p.props.scene_id()) {
        // Two headings with one id (a copied heading): the second gets its own.
        let shared = doc.body.iter().filter(|b| b.as_para().and_then(|p| p.props.scene_id()) == Some(id)).count() > 1;
        if !shared {
            return Ok(id);
        }
    }
    let used: Vec<u32> = doc.body.iter().filter_map(|b| b.as_para().and_then(|p| p.props.scene_id())).collect();
    let id = data.keys().chain(used.iter()).max().map_or(1, |m| m.saturating_add(1));
    let para = doc.para_mut(StoryRef::Body, &Path::top(scene.start))?;
    para.props.set_scene_id(Some(id));
    Ok(id)
}

/// Change `scene`'s data with `f`.
fn update(s: &mut Session, scene: &Scene, f: impl FnOnce(&mut SceneData)) -> Result<SceneData, CmdError> {
    let mut data = all(&s.doc);
    let id = ensure_id(&mut s.doc, scene, &data)?;
    let entry = data.entry(id).or_default();
    f(entry);
    let out = entry.clone().repaired();
    *entry = out.clone();
    store(&mut s.doc, &data);
    Ok(out)
}

/// The speakers of a scene and its cast as shown (speakers not hidden, then the non-speaking).
pub fn cast(doc: &Document, elements: &Elements, scene: &Scene) -> (Vec<String>, Vec<String>) {
    cast_with(&of_scene(doc, scene), doc, elements, scene)
}

/// [`cast`], given the scene's data.
pub fn cast_with(data: &SceneData, doc: &Document, elements: &Elements, scene: &Scene) -> (Vec<String>, Vec<String>) {
    let speaking: Vec<String> = crate::commands::scene::speakers(doc, elements, scene).into_iter().map(|n| n.to_uppercase()).collect();
    let mut shown: Vec<String> = speaking.iter().filter(|n| !data.hidden.contains(n)).cloned().collect();
    for n in &data.non_speaking {
        if !shown.contains(n) {
            shown.push(n.clone());
        }
    }
    (speaking, shown)
}

fn scene_param(s: &Session, v: &Value) -> Result<Scene, CmdError> {
    let all = scenes(&s.doc);
    let i = match v.get("scene").and_then(Value::as_u64) {
        Some(i) => i as usize,
        None => crate::scenes::scene_at(&all, s.sel.focus.path.last())
            .map(|sc| sc.index)
            .ok_or_else(|| CmdError::Params("give `scene`, or put the caret in a scene".into()))?,
    };
    all.get(i).cloned().ok_or_else(|| CmdError::Params(format!("there is no scene {i}")))
}

/// `scene.properties {scene?}`: the scene at the caret (or given by index).
fn properties(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    let elements = Elements::of_doc(&s.doc);
    let data = of_scene(&s.doc, &sc);
    let (speaking, shown) = cast(&s.doc, &elements, &sc);
    let summary = crate::commands::scene::scene_json(&s.doc, &elements, &sc).get("summary").cloned().unwrap_or_default();
    Ok(json!({
        "scene": sc.index,
        "heading": sc.heading,
        "title": data.title,
        "color": data.color,
        "summary": summary,
        "speaking": speaking,
        "cast": shown,
        "nonSpeaking": data.non_speaking,
        "hidden": data.hidden,
        "arcs": data.arcs,
    }))
}

/// `scene.setProperties {scene?, title?, color?, arc?: {character, text}}`.
fn set_properties(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    if let Some(c) = p::str(v, "color")
        && !c.is_empty()
        && !SCENE_COLORS.iter().any(|(n, _)| n.eq_ignore_ascii_case(c))
    {
        return Err(CmdError::Params(format!("`color` is empty or one of {}", SCENE_COLORS.map(|c| c.0).join(", "))));
    }
    let title = p::str(v, "title").map(str::to_string);
    let color =
        p::str(v, "color").map(|c| SCENE_COLORS.iter().find(|(n, _)| n.eq_ignore_ascii_case(c)).map_or(String::new(), |(n, _)| (*n).to_string()));
    let arc = v.get("arc").map(|a| (a["character"].as_str().unwrap_or("").trim().to_uppercase(), a["text"].as_str().unwrap_or("").to_string()));
    if arc.as_ref().is_some_and(|(c, _)| c.is_empty()) {
        return Err(CmdError::Params("`arc` needs a `character`".into()));
    }
    update(s, &sc, |d| {
        if let Some(t) = title {
            d.title = t;
        }
        if let Some(c) = color {
            d.color = c;
        }
        if let Some((who, text)) = arc {
            if text.trim().is_empty() {
                d.arcs.remove(&who);
            } else {
                d.arcs.insert(who, text);
            }
        }
    })?;
    properties(s, &json!({"scene": sc.index}))
}

/// `scene.addCharacter {scene?, name}`: a character in the scene who does not speak (or a hidden
/// speaker shown again).
fn add_character(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    let name = clean_line(p::req_str(v, "name")?, 100).to_uppercase();
    if name.is_empty() {
        return Err(CmdError::Params("`name` is empty".into()));
    }
    let elements = Elements::of_doc(&s.doc);
    let (speaking, _) = cast(&s.doc, &elements, &sc);
    update(s, &sc, |d| {
        d.hidden.retain(|n| *n != name);
        if !speaking.contains(&name) && !d.non_speaking.contains(&name) {
            d.non_speaking.push(name.clone());
        }
    })?;
    // The name joins the Characters autocomplete list.
    let _ = s.run("autocomplete.add", &json!({"list": "characters", "term": name}));
    properties(s, &json!({"scene": sc.index}))
}

/// `scene.removeCharacter {scene?, name}`: a non-speaking character leaves; a speaker is hidden.
fn remove_character(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    let name = p::req_str(v, "name")?.trim().to_uppercase();
    let elements = Elements::of_doc(&s.doc);
    let (speaking, _) = cast(&s.doc, &elements, &sc);
    update(s, &sc, |d| {
        d.non_speaking.retain(|n| *n != name);
        if speaking.contains(&name) && !d.hidden.contains(&name) {
            d.hidden.push(name.clone());
        }
    })?;
    properties(s, &json!({"scene": sc.index}))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Script › Scene";
    vec![
        CommandSpec::new("scene.properties", "Scene Properties", LOC, properties).params(r#"{"scene"?: index}"#).pure(),
        CommandSpec::new("scene.setProperties", "Set Scene Properties", LOC, set_properties)
            .params(r#"{"scene"?: index, "title"?: string, "color"?: string, "arc"?: {"character": string, "text": string}}"#),
        CommandSpec::new("scene.addCharacter", "Add Character to Scene", LOC, add_character).params(r#"{"scene"?: index, "name": string}"#),
        CommandSpec::new("scene.removeCharacter", "Remove Character from Scene", LOC, remove_character)
            .params(r#"{"scene"?: index, "name": string}"#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::template::paragraph;
    use jcj_doc::para_block;

    fn session() -> Session {
        let mut s = new_session();
        s.doc.body = [
            ("SceneHeading", "INT. HALL - DAY"),
            ("Character", "ANNA"),
            ("Dialogue", "Hi."),
            ("Character", "BEN"),
            ("Dialogue", "Hello."),
            ("SceneHeading", "EXT. YARD - NIGHT"),
            ("Action", "Wind."),
        ]
        .iter()
        .map(|(e, t)| para_block(paragraph(e, t)))
        .collect();
        s
    }

    #[test]
    fn title_color_and_arcs_follow_the_scene_when_it_moves() {
        let mut s = session();
        s.run(
            "scene.setProperties",
            &json!({"scene": 0, "title": "Arrival", "color": "blue", "arc": {"character": "anna", "text": "Arrives nervous."}}),
        )
        .unwrap();
        let p = s.run("scene.properties", &json!({"scene": 0})).unwrap();
        assert_eq!((p["title"].as_str(), p["color"].as_str(), p["arcs"]["ANNA"].as_str()), (Some("Arrival"), Some("Blue"), Some("Arrives nervous.")));
        s.run("scene.move", &json!({"scene": 0})).unwrap();
        assert_eq!(s.run("scene.properties", &json!({"scene": 1})).unwrap()["title"], "Arrival");
        assert_eq!(s.run("scene.properties", &json!({"scene": 0})).unwrap()["title"], "");
        assert!(s.run("scene.setProperties", &json!({"scene": 0, "color": "plaid"})).is_err());
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(s.run("scene.properties", &json!({"scene": 0})).unwrap()["title"], "Arrival");
    }

    #[test]
    fn non_speaking_and_hidden_characters() {
        let mut s = session();
        s.run("scene.addCharacter", &json!({"scene": 0, "name": "waiter"})).unwrap();
        let p = s.run("scene.removeCharacter", &json!({"scene": 0, "name": "BEN"})).unwrap();
        assert_eq!(p["cast"], json!(["ANNA", "WAITER"]));
        assert_eq!(p["speaking"], json!(["ANNA", "BEN"]));
        let p = s.run("scene.addCharacter", &json!({"scene": 0, "name": "Ben"})).unwrap();
        assert_eq!(p["cast"], json!(["ANNA", "BEN", "WAITER"]));
        let p = s.run("scene.removeCharacter", &json!({"scene": 0, "name": "WAITER"})).unwrap();
        assert_eq!(p["nonSpeaking"], json!([]));
    }

    #[test]
    fn a_copied_heading_gets_its_own_id_and_hostile_data_is_bounded() {
        let mut s = session();
        s.run("scene.setProperties", &json!({"scene": 0, "title": "One"})).unwrap();
        let copy = s.doc.body[0].clone();
        s.doc.body.push(copy);
        let n = scenes(&s.doc).len();
        s.run("scene.setProperties", &json!({"scene": n - 1, "title": "Copy"})).unwrap();
        assert_eq!(s.run("scene.properties", &json!({"scene": 0})).unwrap()["title"], "One");
        let mut doc = Document::new();
        crate::data::store(&mut doc, SCENE_DATA_KEY, &json!({"1": {"title": "x".repeat(10_000), "color": "Mauve", "arcs": {"": "x"}}}));
        let d = all(&doc);
        assert!(d[&1].title.chars().count() <= MAX_TITLE && d[&1].color.is_empty() && d[&1].arcs.is_empty());
    }
}
