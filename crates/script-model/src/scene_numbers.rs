// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scene numbers (SPEC "Production"), decision 0025.
//!
//! Unlocked, scenes are numbered 1, 2, 3 … in script order, so numbers follow every change.
//! Locking stores each heading's number on it (`ParaProps::scene_number`); from then on a scene
//! keeps its number wherever it moves, and a scene added after the lock takes the number of the
//! numbered scene before it with a letter (12A, then 12B; after 12A, 12B; when that is taken,
//! 12AA), or, before the first numbered scene, a letter before its number (A1). Omitting a scene
//! keeps its heading as an OMITTED placeholder with its number; its text is kept and can be
//! recalled. Settings (`wordsmith.sceneNumbers`): show the numbers, and whether they are locked.

use crate::keys::ScriptPara;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use jcj_doc::{Document, Paragraph, para_block};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, cmd::sel_result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::element::{ElementKind, Elements};
use crate::scenes::scenes;

/// `app_data` key of the settings, and of omitted scenes' text.
pub const SCENE_NUMBERS_KEY: &str = "wordsmith.sceneNumbers";
pub const OMITTED_KEY: &str = "wordsmith.omitted";
/// The placeholder heading's text.
pub const OMITTED: &str = "OMITTED";
/// Longest scene number kept.
pub const MAX_NUMBER: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SceneNumbers {
    /// Print and show the numbers beside the headings.
    pub show: bool,
    /// Scenes keep the numbers stored on their headings.
    pub locked: bool,
    /// The first number (unlocked numbering), 1 by default (SPEC-2 11.1).
    pub start: u32,
    /// Start again from `start` every this many scenes; 0 never.
    pub restart_every: u32,
    /// A scene added after the lock takes a letter before the next number (A2) instead of
    /// after the one before (1A).
    pub prefix: bool,
    /// Where numbers show: "both", "left" or "right".
    pub sides: String,
    /// The numbers' own font; empty for the script's.
    pub font: String,
}

impl Default for SceneNumbers {
    fn default() -> Self {
        SceneNumbers { show: false, locked: false, start: 1, restart_every: 0, prefix: false, sides: "both".into(), font: String::new() }
    }
}

impl SceneNumbers {
    pub fn of_doc(doc: &Document) -> Self {
        let mut s: SceneNumbers = doc.ext.get(SCENE_NUMBERS_KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        s.start = s.start.clamp(1, 100_000);
        s.restart_every = s.restart_every.min(10_000);
        if !matches!(s.sides.as_str(), "both" | "left" | "right") {
            s.sides = "both".into();
        }
        s.font = s.font.chars().filter(|c| !c.is_control()).take(64).collect();
        s
    }

    /// The number of the `i`th scene (from 0) when the numbers are not locked.
    pub fn unlocked(&self, i: usize) -> String {
        let i = i as u32;
        let n = if self.restart_every > 0 { i % self.restart_every } else { i };
        (self.start + n).to_string()
    }

    pub fn store(&self, doc: &mut Document) {
        let _ = doc.ext.set(SCENE_NUMBERS_KEY, serde_json::to_value(self).unwrap_or_default());
    }
}

/// A scene number as it is kept, if it is one: capital letters, digits, `-` and `.`, from 1 to
/// [`MAX_NUMBER`] bytes, after trimming and capitalising. Every way a number reaches a script
/// (typing one, locking, Fountain, the native file) goes through this, so a number that is
/// stored is one the file keeps.
pub fn clean(n: &str) -> Option<String> {
    let n = n.trim().to_uppercase();
    (!n.is_empty() && n.len() <= MAX_NUMBER && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')).then_some(n)
}

/// The next letter suffix: "A" → "B", … "Z" → "ZA".
fn next_letters(s: &str) -> String {
    match s.chars().last() {
        Some(c) if c.is_ascii_uppercase() && c != 'Z' => format!("{}{}", s.get(..s.len() - 1).unwrap_or(""), (c as u8 + 1) as char),
        _ => format!("{s}A"),
    }
}

/// Letter suffixes shortest first: A … Z, AA … ZZ, AAA …
fn letter_tails() -> impl Iterator<Item = String> {
    (0u64..).map(|mut i| {
        let mut out = Vec::new();
        loop {
            out.push(b'A' + (i % 26) as u8);
            if i < 26 {
                break;
            }
            i = i / 26 - 1;
        }
        out.iter().rev().map(|b| char::from(*b)).collect()
    })
}

/// How many candidates a free number is looked for among before the last resort.
const MAX_TRIES: usize = 10_000;

/// A number not in `used` that [`clean`] keeps: `base` with letters after it, shortest first, or
/// failing that the smallest whole number not in use.
fn free_after(base: &str, used: &HashSet<String>) -> String {
    letter_tails()
        .take(MAX_TRIES)
        .map(|tail| format!("{base}{tail}"))
        .take_while(|c| c.len() <= MAX_NUMBER)
        .find(|c| !used.contains(c))
        .unwrap_or_else(|| (1u64..).map(|n| n.to_string()).find(|c| !used.contains(c)).unwrap_or_default())
}

/// A number for a scene added after `prev` (stored or given), not in `used`, never longer than
/// [`MAX_NUMBER`].
fn after(prev: &str, used: &HashSet<String>) -> String {
    // After 12 comes 12A; after 12A comes 12B; when that is taken, 12AA, 12AB …
    let digits = prev.trim_end_matches(|c: char| c.is_ascii_uppercase());
    let first =
        if digits.len() < prev.len() { format!("{digits}{}", next_letters(prev.get(digits.len()..).unwrap_or(""))) } else { format!("{prev}A") };
    if !used.contains(&first) && first.len() <= MAX_NUMBER {
        return first;
    }
    if prev.len() < MAX_NUMBER {
        let candidate = free_after(prev, used);
        if candidate.starts_with(prev) {
            return candidate;
        }
    }
    // A long run of added scenes: letters after the number's own digits instead.
    free_after(digits, used)
}

/// A number for a scene added before the first numbered scene `first`, not in `used`.
fn before(first: &str, used: &HashSet<String>) -> String {
    let mut letters = "A".to_string();
    while used.contains(&format!("{letters}{first}")) && letters.len() < 6 {
        letters = next_letters(&letters);
    }
    let n = format!("{letters}{first}");
    if n.len() <= MAX_NUMBER && !used.contains(&n) { n } else { free_after(first.trim_end_matches(|c: char| c.is_ascii_uppercase()), used) }
}

/// Each scene's number as it is shown, in script order.
pub fn numbers(doc: &Document) -> Vec<String> {
    let settings = SceneNumbers::of_doc(doc);
    let all = scenes(doc);
    let stored: Vec<Option<String>> =
        all.iter().map(|sc| doc.body.get(sc.start).and_then(|b| b.as_para()).and_then(|p| p.props.scene_number()).and_then(|n| clean(&n))).collect();
    if !settings.locked {
        return (0..all.len()).map(|i| settings.unlocked(i)).collect();
    }
    let mut used: HashSet<String> = stored.iter().flatten().cloned().collect();
    let mut out: Vec<String> = Vec::with_capacity(all.len());
    for (i, s) in stored.iter().enumerate() {
        // The next stored number from this scene on.
        let next_stored = || stored.iter().skip(i).flatten().next().cloned();
        let n = match s {
            Some(n) => n.clone(),
            // With prefix letters, an added scene takes a letter before the next number.
            None if settings.prefix && next_stored().is_some() => before(&next_stored().unwrap_or_default(), &used),
            None => match out.last() {
                Some(prev) => after(prev, &used),
                None => {
                    let first = next_stored().unwrap_or_else(|| "1".into());
                    before(&first, &used)
                }
            },
        };
        used.insert(n.clone());
        out.push(n);
    }
    out
}

/// `scene.numbers`: each scene's number as shown, and the settings.
fn list(s: &mut Session, _: &Value) -> CmdResult {
    let settings = SceneNumbers::of_doc(&s.doc);
    Ok(json!({"numbers": numbers(&s.doc), "show": settings.show, "locked": settings.locked}))
}

/// `scene.showNumbers {show?}`: show or hide the numbers (or the other way).
fn show(s: &mut Session, v: &Value) -> CmdResult {
    let mut settings = SceneNumbers::of_doc(&s.doc);
    settings.show = v.get("show").and_then(Value::as_bool).unwrap_or(!settings.show);
    settings.store(&mut s.doc);
    Ok(json!({"show": settings.show}))
}

fn set_number(doc: &mut Document, block: usize, n: Option<String>) {
    if let Some(jcj_doc::Block::Para(p)) = doc.body.get_mut(block).map(Arc::make_mut) {
        p.props.set_scene_number(n);
        p.touch();
    }
}

/// `scene.lockNumbers {lock?}`: lock the numbers as they are shown now (added scenes take
/// letters from then on), or unlock them (numbers follow the order again).
fn lock(s: &mut Session, v: &Value) -> CmdResult {
    let mut settings = SceneNumbers::of_doc(&s.doc);
    let lock = v.get("lock").and_then(Value::as_bool).unwrap_or(!settings.locked);
    let shown = numbers(&s.doc);
    for (sc, n) in scenes(&s.doc).iter().zip(shown) {
        set_number(&mut s.doc, sc.start, lock.then_some(n));
    }
    settings.locked = lock;
    if lock {
        settings.show = true;
    }
    settings.store(&mut s.doc);
    Ok(json!({"locked": lock}))
}

/// `scene.renumber {fromCaret?, keepExisting?}`: number the scenes again from the start
/// number, keeping the lock as it is. `fromCaret` leaves scenes before the caret's alone and
/// carries on from the number before; `keepExisting` numbers only scenes without a stored
/// number.
fn renumber(s: &mut Session, v: &Value) -> CmdResult {
    let settings = SceneNumbers::of_doc(&s.doc);
    let all = scenes(&s.doc);
    let shown = numbers(&s.doc);
    let from = if v.get("fromCaret").and_then(Value::as_bool) == Some(true) {
        crate::scenes::scene_at(&all, s.sel.focus.path.last()).map_or(0, |sc| sc.index)
    } else {
        0
    };
    let keep = v.get("keepExisting").and_then(Value::as_bool) == Some(true);
    // Carry on from the number before (its digits), or the start.
    let mut n: u32 = from
        .checked_sub(1)
        .and_then(|i| shown.get(i))
        .and_then(|x| x.trim_end_matches(|c: char| !c.is_ascii_digit()).trim_start_matches(|c: char| !c.is_ascii_digit()).parse().ok())
        .map_or(settings.start, |x: u32| x + 1);
    for sc in all.iter().skip(from) {
        let has = s.doc.body.get(sc.start).and_then(|b| b.as_para()).is_some_and(|p| p.props.scene_number().is_some());
        if keep && has {
            continue;
        }
        set_number(&mut s.doc, sc.start, settings.locked.then(|| n.to_string()));
        n += 1;
    }
    Ok(json!({"numbers": numbers(&s.doc)}))
}

/// `scene.configureNumbers {start?, restartEvery?, prefix?, sides?, font?}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut settings = SceneNumbers::of_doc(&s.doc);
    if let Some(n) = v.get("start").and_then(Value::as_u64) {
        settings.start = (n as u32).clamp(1, 100_000);
    }
    if let Some(n) = v.get("restartEvery").and_then(Value::as_u64) {
        settings.restart_every = (n as u32).min(10_000);
    }
    if let Some(b) = v.get("prefix").and_then(Value::as_bool) {
        settings.prefix = b;
    }
    if let Some(side) = v.get("sides").and_then(Value::as_str) {
        if !matches!(side, "both" | "left" | "right") {
            return Err(CmdError::Params("`sides` is both, left or right".into()));
        }
        settings.sides = side.into();
    }
    if let Some(f) = v.get("font").and_then(Value::as_str) {
        settings.font = f.trim().chars().take(64).collect();
    }
    settings.store(&mut s.doc);
    Ok(serde_json::to_value(&settings).unwrap_or_default())
}

/// `scene.setNumber {scene?, number}`: give one scene a number by hand. The numbers are locked
/// first if they are not, so it stays.
fn set_one(s: &mut Session, v: &Value) -> CmdResult {
    let number =
        v.get("number").and_then(Value::as_str).and_then(clean).ok_or_else(|| CmdError::Params("`number` is letters and digits, up to 12".into()))?;
    let sc = scene_param(s, v)?;
    let shown = numbers(&s.doc);
    if shown.iter().enumerate().any(|(i, n)| i != sc.index && *n == number) {
        return Err(CmdError::Params(format!("another scene is already {number}")));
    }
    if !SceneNumbers::of_doc(&s.doc).locked {
        lock(s, &json!({"lock": true}))?;
    }
    set_number(&mut s.doc, sc.start, Some(number));
    Ok(json!({"numbers": numbers(&s.doc)}))
}

fn scene_param(s: &Session, v: &Value) -> Result<crate::scenes::Scene, CmdError> {
    let all = scenes(&s.doc);
    // Without `scene`, the scene the caret is in.
    if v.get("scene").is_none() {
        return crate::scenes::scene_at(&all, s.sel.focus.path.last()).cloned().ok_or_else(|| CmdError::Failed("the caret is not in a scene".into()));
    }
    v.get("scene")
        .and_then(Value::as_u64)
        .and_then(|i| all.get(i as usize))
        .cloned()
        .ok_or_else(|| CmdError::Params(format!("`scene` must be a scene number from 0 to {}", all.len().saturating_sub(1))))
}

fn omitted_store(doc: &Document) -> BTreeMap<String, Value> {
    doc.ext.get(OMITTED_KEY).and_then(|v| v.as_object().cloned()).map(|m| m.into_iter().collect()).unwrap_or_default()
}

/// `scene.omit {scene}`: replace a scene with an OMITTED heading that keeps its number. Its text
/// is kept with the script and can be recalled.
fn omit(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    let heading =
        s.doc.body.get(sc.start).and_then(|b| b.as_para()).cloned().ok_or_else(|| CmdError::Failed("the heading is not a paragraph".into()))?;
    if heading.props.omitted().is_some() {
        return Err(CmdError::Failed("the scene is already omitted".into()));
    }
    let number = numbers(&s.doc).get(sc.index).cloned();
    let paras: Vec<Paragraph> = s.doc.body.get(sc.start..sc.end).unwrap_or_default().iter().filter_map(|b| b.as_para().cloned()).collect();
    let mut store = omitted_store(&s.doc);
    let id = store.keys().filter_map(|k| k.parse::<u32>().ok()).max().map_or(1, |m| m + 1);
    store.insert(id.to_string(), serde_json::to_value(&paras).map_err(|e| CmdError::Failed(e.to_string()))?);
    let _ = s.doc.ext.set(OMITTED_KEY, Value::Object(store.into_iter().collect()));
    let mut placeholder = Paragraph::with_text(OMITTED, Default::default());
    placeholder.props.style = heading.props.style.clone();
    placeholder.props.set_omitted(Some(id));
    // Locked or not, the placeholder holds the number the scene had.
    placeholder.props.set_scene_number(number.filter(|_| SceneNumbers::of_doc(&s.doc).locked));
    s.doc.body.splice(sc.start..sc.end, [para_block(placeholder)]);
    s.clamp_selection();
    sel_result(s)
}

/// `scene.recall {scene}`: bring an omitted scene back where its OMITTED heading is.
fn recall(s: &mut Session, v: &Value) -> CmdResult {
    let sc = scene_param(s, v)?;
    let id = s
        .doc
        .body
        .get(sc.start)
        .and_then(|b| b.as_para())
        .and_then(|p| p.props.omitted())
        .ok_or_else(|| CmdError::Params("the scene is not omitted".into()))?;
    let mut store = omitted_store(&s.doc);
    let paras: Vec<Paragraph> = store
        .remove(&id.to_string())
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or_else(|| CmdError::Failed("the omitted scene's text is missing".into()))?;
    if paras.is_empty() {
        return Err(CmdError::Failed("the omitted scene's text is missing".into()));
    }
    let _ = s.doc.ext.set(OMITTED_KEY, Value::Object(store.into_iter().collect()));
    s.doc.body.splice(sc.start..sc.start + 1, paras.into_iter().map(para_block));
    s.clamp_selection();
    sel_result(s)
}

/// After a paragraph is split in two, only the first part keeps its scene number, OMITTED mark and
/// beat.
pub fn after_split(doc: &mut Document, new: &jcj_doc::Pos) {
    if let Ok(p) = doc.para_mut(new.story, &new.path)
        && (p.props.scene_number().is_some()
            || p.props.omitted().is_some()
            || p.props.beat().is_some()
            || p.props.bookmark().is_some()
            || p.props.dual().is_some()
            || p.props.scene_id().is_some()
            || !p.props.extra_attrs().is_empty())
    {
        p.props.set_scene_id(None);
        p.props.set_bookmark(None);
        p.props.set_dual(None);
        p.props.set_scene_number(None);
        p.props.set_omitted(None);
        // A newer version's attributes stay with the first part, as the known ones do.
        p.props.set_extra_attrs(Vec::new());
        // A beat's paragraph stays one paragraph's.
        p.props.set_beat(None);
    }
}

/// Whether `p` is an omitted scene's placeholder.
pub fn is_omitted(p: &Paragraph) -> bool {
    p.props.omitted().is_some()
}

/// The element id of scene headings in `doc`.
pub fn heading_id(doc: &Document) -> String {
    Elements::of_doc(doc).id_of(ElementKind::SceneHeading).to_string()
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("scene.numbers", "Scene Numbers", "Production", list).pure(),
        CommandSpec::new("scene.showNumbers", "Show Scene Numbers", "Production", show).params(r#"{"show"?: bool}"#),
        CommandSpec::new("scene.lockNumbers", "Lock Scene Numbers", "Production", lock).params(r#"{"lock"?: bool}"#),
        CommandSpec::new("scene.renumber", "Renumber Scenes", "Production", renumber).params(r#"{"fromCaret"?: bool, "keepExisting"?: bool}"#),
        CommandSpec::new("scene.configureNumbers", "Scene Number Options", "Production", configure)
            .params(r#"{"start"?: number, "restartEvery"?: number, "prefix"?: bool, "sides"?: "both"|"left"|"right", "font"?: string}"#),
        CommandSpec::new("scene.setNumber", "Set Scene Number", "Production", set_one).params(r#"{"scene"?: index, "number": string}"#),
        CommandSpec::new("scene.omit", "Omit Scene", "Production", omit).params(r#"{"scene"?: number (the caret's scene when left out)}"#),
        CommandSpec::new("scene.recall", "Recall Omitted Scene", "Production", recall)
            .params(r#"{"scene"?: number (the caret's scene when left out)}"#),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::commands::text::body_lines;
    use crate::template::{new_script, paragraph};

    fn session(headings: &[&str]) -> Session {
        let mut doc = new_script();
        doc.body = headings.iter().flat_map(|h| [para_block(paragraph("SceneHeading", h)), para_block(paragraph("Action", "Something."))]).collect();
        crate::session_for(doc)
    }

    fn add_scene_before(s: &mut Session, block: usize, heading: &str) {
        s.doc.body.insert(block, para_block(paragraph("SceneHeading", heading)));
    }

    #[test]
    fn start_restart_prefix_and_one_number_by_hand() {
        let mut s = session(&["INT. A - DAY", "INT. B - DAY", "INT. C - DAY", "INT. D - DAY"]);
        s.run("scene.configureNumbers", &json!({"start": 10, "restartEvery": 3})).unwrap();
        assert_eq!(numbers(&s.doc), ["10", "11", "12", "10"]);
        s.run("scene.configureNumbers", &json!({"restartEvery": 0, "prefix": true})).unwrap();
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        // A scene added between 11 and 12 takes A12 (a letter before the next number).
        add_scene_before(&mut s, 4, "INT. NEW - DAY");
        assert_eq!(numbers(&s.doc), ["10", "11", "A12", "12", "13"]);
        s.run("scene.setNumber", &json!({"scene": 2, "number": "11X"})).unwrap();
        assert_eq!(numbers(&s.doc)[2], "11X");
        assert!(s.run("scene.setNumber", &json!({"scene": 0, "number": "13"})).is_err(), "unique");
        // Renumber from the caret keeps the scenes before it.
        s.sel = jcj_engine::Selection::caret(crate::data::body_pos(4, 0));
        s.run("scene.renumber", &json!({"fromCaret": true})).unwrap();
        assert_eq!(numbers(&s.doc), ["10", "11", "12", "13", "14"]);
        assert!(s.run("scene.configureNumbers", &json!({"sides": "middle"})).is_err());
        s.run("scene.configureNumbers", &json!({"sides": "left", "font": "Inter"})).unwrap();
        let pag = crate::paginate::paginate(&s.doc);
        let heading = pag.doc.body[0].as_para().map(|p| p.objects.len()).unwrap_or(0);
        assert_eq!(heading, 1, "one side only");
    }

    #[test]
    fn letters_after_numbers() {
        let used = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<HashSet<String>>();
        assert_eq!(after("12", &used(&["12"])), "12A");
        assert_eq!(after("12A", &used(&["12", "12A"])), "12B");
        assert_eq!(after("12A", &used(&["12", "12A", "12B"])), "12AA");
        assert_eq!(before("1", &used(&["1"])), "A1");
        assert_eq!(before("1", &used(&["1", "A1"])), "B1");
        assert_eq!(next_letters("Z"), "ZA");
        // Never longer than a number can be: letters after the digits, then a free number.
        assert_eq!(after("123456789012", &used(&["123456789012"])), "1");
        assert_eq!(after("12345678901Z", &used(&[])), "12345678901A");
        assert_eq!(before("123456789012", &used(&[])), "1");
        assert_eq!(letter_tails().take(28).last().as_deref(), Some("AB"));
    }

    #[test]
    fn every_number_shown_is_one_a_script_keeps() {
        for (first, prefix) in [("1", false), ("1", true), ("123456789012", false), ("123456789012", true), ("12-B.7", false)] {
            let mut s = session(&["INT. A", "INT. B"]);
            s.run("scene.configureNumbers", &json!({"prefix": prefix})).unwrap();
            s.run("scene.setNumber", &json!({"scene": 0, "number": first})).unwrap();
            // A long run of scenes added after the lock, before, between and after.
            for i in 0..120 {
                add_scene_before(&mut s, 2, &format!("INT. MID {i}"));
                add_scene_before(&mut s, 0, &format!("INT. EARLY {i}"));
            }
            let shown = numbers(&s.doc);
            let unique: HashSet<&String> = shown.iter().collect();
            assert_eq!(unique.len(), shown.len(), "{first} {prefix}: unique");
            for n in &shown {
                assert_eq!(clean(n).as_ref(), Some(n), "{first} {prefix}: {n} is kept as it is");
            }
            // Locking stores them all, and they stay what they were.
            s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
            assert_eq!(numbers(&s.doc), shown);
        }
    }

    #[test]
    fn unlocked_numbers_follow_the_order_and_locked_ones_stay() {
        let mut s = session(&["INT. A", "INT. B", "INT. C"]);
        assert_eq!(numbers(&s.doc), ["1", "2", "3"]);
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        assert!(SceneNumbers::of_doc(&s.doc).show, "locking shows them");
        // Scenes added after the lock take letters; a moved scene keeps its number.
        add_scene_before(&mut s, 2, "INT. A2");
        add_scene_before(&mut s, 3, "INT. A3");
        add_scene_before(&mut s, 0, "INT. FIRST");
        assert_eq!(numbers(&s.doc), ["A1", "1", "1A", "1B", "2", "3"]);
        s.run("scene.move", &json!({"scene": 5, "before": 1})).unwrap();
        // A scene added since the lock has no stored number yet: its letters follow its
        // neighbours until the next lock stores them.
        assert_eq!(numbers(&s.doc), ["A3", "3", "1", "1A", "1B", "2"]);
        // Locking again stores the letters; renumbering starts over.
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        s.run("scene.renumber", &json!({})).unwrap();
        assert_eq!(numbers(&s.doc), ["1", "2", "3", "4", "5", "6"]);
        s.run("scene.lockNumbers", &json!({"lock": false})).unwrap();
        assert!(s.doc.body.iter().filter_map(|b| b.as_para()).all(|p| p.props.scene_number().is_none()));
    }

    #[test]
    fn omitted_scenes_keep_their_number_and_come_back() {
        let mut s = session(&["INT. A", "INT. B", "INT. C"]);
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        let before = body_lines(&s);
        s.run("scene.omit", &json!({"scene": 1})).unwrap();
        assert_eq!(body_lines(&s)[2], ("SceneHeading".to_string(), OMITTED.to_string()));
        assert_eq!(body_lines(&s).len(), before.len() - 1, "its text is gone from the script");
        assert_eq!(numbers(&s.doc), ["1", "2", "3"], "the placeholder keeps number 2");
        assert!(s.run("scene.omit", &json!({"scene": 1})).is_err());
        s.run("scene.recall", &json!({"scene": 1})).unwrap();
        assert_eq!(body_lines(&s), before);
        assert_eq!(numbers(&s.doc), ["1", "2", "3"]);
        assert!(s.run("scene.recall", &json!({"scene": 1})).is_err(), "not omitted");
        // Undo puts the placeholder back.
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[2].1, OMITTED);
    }

    #[test]
    fn a_split_heading_does_not_copy_its_number() {
        let mut s = session(&["INT. A - DAY"]);
        s.run("scene.lockNumbers", &json!({"lock": true})).unwrap();
        s.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 6, path: jcj_doc::Path::top(0), ..s.sel.focus.clone() });
        s.run("text.newParagraph", &json!({})).unwrap();
        let stored: Vec<Option<String>> = s.doc.body.iter().filter_map(|b| b.as_para()).map(|p| p.props.scene_number()).collect();
        assert_eq!(stored.iter().flatten().count(), 1);
    }
}
