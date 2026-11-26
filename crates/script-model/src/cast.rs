// SPDX-License-Identifier: MIT OR Apache-2.0
//! Characters' own data (SPEC-2 12): traits (free text, a default set the writer can add to),
//! a highlight color for their speeches, and the representation chart. Kept in `app_data` under
//! [`CAST_KEY`], by name in capitals. A rename carries a character's data to the new name.

use std::collections::BTreeMap;

use jcj_doc::Document;
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::data::clean_line;
use crate::element::{ElementKind, Elements};
use crate::paginate::output::speaker;

/// `app_data` key of the characters' data.
pub const CAST_KEY: &str = "wordsmith.cast";
pub const MAX_TRAITS: usize = 50;
pub const MAX_PEOPLE: usize = 5_000;
pub const MAX_VALUE: usize = 200;

/// The traits every script starts with.
pub const DEFAULT_TRAITS: [&str; 7] = ["Age", "Disability", "Ethnicity", "Gender", "Occupation", "Orientation", "Role"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Person {
    pub traits: BTreeMap<String, String>,
    /// `#RRGGBB` for the character's speeches.
    pub highlight: Option<String>,
    /// Whether their speeches are highlighted now.
    pub highlight_on: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cast {
    pub traits: Vec<String>,
    pub people: BTreeMap<String, Person>,
}

impl Default for Cast {
    fn default() -> Self {
        Cast { traits: DEFAULT_TRAITS.iter().map(|t| (*t).to_string()).collect(), people: BTreeMap::new() }
    }
}

impl Cast {
    pub fn of_doc(doc: &Document) -> Self {
        let mut c: Cast = match doc.ext.get(CAST_KEY) {
            Some(v) => serde_json::from_value(v.clone()).unwrap_or_default(),
            None => Cast::default(),
        };
        c.traits = c.traits.iter().map(|t| clean_line(t, 60)).filter(|t| !t.is_empty()).take(MAX_TRAITS).collect();
        c.traits.dedup();
        c.people = std::mem::take(&mut c.people)
            .into_iter()
            .take(MAX_PEOPLE)
            .map(|(k, mut p)| {
                p.traits = p
                    .traits
                    .into_iter()
                    .take(MAX_TRAITS)
                    .map(|(t, v)| (clean_line(&t, 60), clean_line(&v, MAX_VALUE)))
                    .filter(|(t, v)| !t.is_empty() && !v.is_empty())
                    .collect();
                p.highlight = p.highlight.filter(|h| crate::revisions::parse_color(h).is_some());
                (clean_line(&k, 100).to_uppercase(), p)
            })
            .filter(|(k, _)| !k.is_empty())
            .collect();
        c
    }

    pub fn store(&self, doc: &mut Document) {
        crate::data::store(doc, CAST_KEY, self);
    }
}

/// Carry `from`'s data to `to` after a rename.
pub fn rename(doc: &mut Document, from: &str, to: &str) {
    let mut c = Cast::of_doc(doc);
    if let Some(p) = c.people.remove(&from.trim().to_uppercase()) {
        c.people.insert(to.trim().to_uppercase(), p);
        c.store(doc);
    }
}

/// Every speech of a highlighted character: (first block, last block, `#RRGGBB`). A speech is
/// the Character paragraph and the Dialogue and Parentheticals after it.
pub fn highlighted_speeches(doc: &Document) -> Vec<(usize, usize, String)> {
    let c = Cast::of_doc(doc);
    let on: BTreeMap<&str, &str> =
        c.people.iter().filter(|(_, p)| p.highlight_on).map(|(k, p)| (k.as_str(), p.highlight.as_deref().unwrap_or("#FFF176"))).collect();
    if on.is_empty() {
        return Vec::new();
    }
    let elements = Elements::of_doc(doc);
    let kinds: Vec<ElementKind> = doc.body.iter().map(|b| b.as_para().map_or(ElementKind::General, |p| elements.of_para(p).kind)).collect();
    let mut out = Vec::new();
    for (i, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        if kinds.get(i) != Some(&ElementKind::Character) {
            continue;
        }
        let who = speaker(&p.text).trim().to_uppercase();
        let Some(color) = on.get(who.as_str()) else { continue };
        let mut end = i;
        while matches!(kinds.get(end + 1), Some(ElementKind::Dialogue | ElementKind::Parenthetical)) {
            end += 1;
        }
        out.push((i, end, (*color).to_string()));
    }
    out
}

fn traits(s: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(Cast::of_doc(&s.doc).traits))
}

/// `character.setTraits {traits}`: the list of traits (values of removed traits are kept).
fn set_traits(s: &mut Session, v: &Value) -> CmdResult {
    let list = v.get("traits").and_then(Value::as_array).ok_or_else(|| CmdError::Params("`traits` is a list of names".into()))?;
    if list.len() > MAX_TRAITS {
        return Err(CmdError::Params(format!("at most {MAX_TRAITS} traits")));
    }
    let mut c = Cast::of_doc(&s.doc);
    c.traits = list.iter().filter_map(Value::as_str).map(|t| clean_line(t, 60)).filter(|t| !t.is_empty()).collect();
    c.traits.dedup();
    c.store(&mut s.doc);
    Ok(json!(c.traits))
}

/// `character.get {name}`.
fn get(s: &mut Session, v: &Value) -> CmdResult {
    let name = p::req_str(v, "name")?.trim().to_uppercase();
    let c = Cast::of_doc(&s.doc);
    Ok(json!({"name": name, "traits": c.traits, "person": c.people.get(&name).cloned().unwrap_or_default()}))
}

/// `character.set {name, traits?: {trait: value}, highlight?: "#RRGGBB"|null, highlightOn?}`.
fn set(s: &mut Session, v: &Value) -> CmdResult {
    let name = clean_line(p::req_str(v, "name")?, 100).to_uppercase();
    if name.is_empty() {
        return Err(CmdError::Params("`name` is empty".into()));
    }
    let mut c = Cast::of_doc(&s.doc);
    if c.people.len() >= MAX_PEOPLE && !c.people.contains_key(&name) {
        return Err(CmdError::Failed(format!("at most {MAX_PEOPLE} characters keep data")));
    }
    let person = c.people.entry(name.clone()).or_default();
    if let Some(t) = v.get("traits").and_then(Value::as_object) {
        for (k, x) in t {
            let value = clean_line(x.as_str().unwrap_or(""), MAX_VALUE);
            if value.is_empty() {
                person.traits.remove(k);
            } else {
                person.traits.insert(clean_line(k, 60), value);
            }
        }
    }
    match v.get("highlight") {
        None => {}
        Some(Value::Null) => person.highlight = None,
        Some(Value::String(h)) if crate::revisions::parse_color(h).is_some() => person.highlight = Some(h.to_uppercase()),
        Some(_) => return Err(CmdError::Params("`highlight` is #RRGGBB or null".into())),
    }
    if let Some(on) = p::bool(v, "highlightOn") {
        person.highlight_on = on;
    }
    let out = person.clone();
    c.store(&mut s.doc);
    Ok(json!({"name": name, "person": out}))
}

/// The representation chart (SPEC-2 12): characters grouped by two traits, with each group's
/// share of the characters, of the dialogue (words) and of scene appearances. Blank values are
/// grouped as "Unassigned".
pub fn representation(doc: &Document, a: &str, b: &str) -> Value {
    let c = Cast::of_doc(doc);
    let rows = crate::commands::character::characters(doc);
    let words = crate::reports_more::words_by_speaker(doc);
    let total_chars = rows.len().max(1) as f64;
    let total_words = words.values().sum::<usize>().max(1) as f64;
    let total_scenes = rows.iter().map(|r| r.scenes).sum::<usize>().max(1) as f64;
    let value = |name: &str, t: &str| {
        c.people
            .get(name)
            .and_then(|p| p.traits.iter().find(|(k, _)| k.eq_ignore_ascii_case(t)).map(|(_, v)| v.clone()))
            .unwrap_or_else(|| "Unassigned".into())
    };
    let mut groups: BTreeMap<(String, String), (usize, usize, usize)> = BTreeMap::new();
    for r in &rows {
        let g = groups.entry((value(&r.name, a), value(&r.name, b))).or_default();
        g.0 += 1;
        g.1 += words.get(&r.name).copied().unwrap_or(0);
        g.2 += r.scenes;
    }
    let pct = |n: usize, t: f64| (n as f64 * 1000.0 / t).round() / 10.0;
    let list: Vec<Value> = groups
        .into_iter()
        .map(|((x, y), (n, w, sc))| json!({a: x, b: y, "characters": n, "charactersPct": pct(n, total_chars), "words": w, "dialoguePct": pct(w, total_words), "scenes": sc, "scenesPct": pct(sc, total_scenes)}))
        .collect();
    json!({"by": [a, b], "groups": list})
}

fn representation_cmd(s: &mut Session, v: &Value) -> CmdResult {
    let a = p::req_str(v, "a")?.to_string();
    let b = p::str(v, "b").unwrap_or(&a).to_string();
    Ok(representation(&s.doc, &a, &b))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Script › Characters";
    vec![
        CommandSpec::new("character.traits", "Character Traits", LOC, traits).pure(),
        CommandSpec::new("character.setTraits", "Set Character Traits", LOC, set_traits).params(r#"{"traits": [string]}"#),
        CommandSpec::new("character.get", "Character Details", LOC, get).params(r#"{"name": string}"#).pure(),
        CommandSpec::new("character.set", "Set Character Details", LOC, set)
            .params(r##"{"name": string, "traits"?: {trait: value}, "highlight"?: "#RRGGBB" | null, "highlightOn"?: bool}"##),
        CommandSpec::new("report.representation", "Representation Chart", "Reports", representation_cmd)
            .params(r#"{"a": trait, "b"?: trait}"#)
            .pure(),
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
            ("Dialogue", "Hi there you."),
            ("Character", "BEN"),
            ("Parenthetical", "(quiet)"),
            ("Dialogue", "Hello."),
            ("Action", "They wait."),
        ]
        .iter()
        .map(|(e, t)| para_block(paragraph(e, t)))
        .collect();
        s
    }

    #[test]
    fn traits_and_highlights() {
        let mut s = session();
        assert_eq!(s.run("character.traits", &json!({})).unwrap().as_array().map(Vec::len), Some(7));
        s.run("character.set", &json!({"name": "ben", "traits": {"Age": "40", "Role": "Lead"}, "highlight": "#ffcc00", "highlightOn": true}))
            .unwrap();
        let got = s.run("character.get", &json!({"name": "BEN"})).unwrap();
        assert_eq!(got["person"]["traits"]["Age"], "40");
        assert_eq!(highlighted_speeches(&s.doc), vec![(3, 5, "#FFCC00".to_string())]);
        s.run("character.set", &json!({"name": "ben", "traits": {"Age": ""}})).unwrap();
        assert!(s.run("character.get", &json!({"name": "BEN"})).unwrap()["person"]["traits"].get("Age").is_none());
        assert!(s.run("character.set", &json!({"name": "ben", "highlight": "gold"})).is_err());
        s.run("character.rename", &json!({"from": "BEN", "to": "BENJAMIN"})).unwrap();
        assert_eq!(s.run("character.get", &json!({"name": "BENJAMIN"})).unwrap()["person"]["traits"]["Role"], "Lead");
    }

    #[test]
    fn representation_groups_blanks_as_unassigned() {
        let mut s = session();
        s.run("character.set", &json!({"name": "ANNA", "traits": {"Gender": "Woman", "Role": "Lead"}})).unwrap();
        let r = s.run("report.representation", &json!({"a": "Gender", "b": "Role"})).unwrap();
        let groups = r["groups"].as_array().cloned().unwrap_or_default();
        assert_eq!(groups.len(), 2);
        let anna = groups.iter().find(|g| g["Gender"] == "Woman").cloned().unwrap_or_default();
        assert_eq!((anna["characters"].as_u64(), anna["charactersPct"].as_f64(), anna["words"].as_u64()), (Some(1), Some(50.0), Some(3)));
        assert!(groups.iter().any(|g| g["Gender"] == "Unassigned"));
    }
}
