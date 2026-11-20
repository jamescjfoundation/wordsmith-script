// SPDX-License-Identifier: MIT OR Apache-2.0
//! Macros (SPEC-2 5): twenty slots per script. A macro can set the paragraph's element, type
//! its text and start a new paragraph of another element. It runs from its shortcut, from the
//! menu, or from an alias: a word that is replaced by the macro's text when typed. Kept in
//! `app_data` under [`MACROS_KEY`] (undoable, saved with the script).
//!
//! New scripts get a starter set of our own on Mod+Alt+Shift and a digit (Mod+Alt and a digit
//! switch views, decision 0032).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result};

use crate::data::{body_pos, clean_line};
use crate::element::Elements;

/// `app_data` key of the macros.
pub const MACROS_KEY: &str = "wordsmith.macros";
/// Slots a script has.
pub const SLOTS: usize = 20;
pub const MAX_TEXT: usize = 2_000;
pub const MAX_NAME: usize = 60;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Macro {
    pub name: String,
    pub shortcut: Option<String>,
    /// The element the paragraph becomes first (an id), if any.
    pub element: Option<String>,
    /// Typed at the caret; a line break acts as Return.
    pub text: String,
    /// The element of a new paragraph started afterwards, if any.
    pub after: Option<String>,
    /// A word replaced by the macro's text as it is typed.
    pub alias: Option<String>,
    /// The replacement takes the alias's capitals ("Hq" → "Headquarters", "HQ" → "HEADQUARTERS").
    pub smart_replace: bool,
    /// Ask before replacing an alias.
    pub confirm: bool,
    /// The alias matches only as a whole word.
    pub whole_word: bool,
    pub match_case: bool,
    /// Elements the macro works in (ids); empty for all.
    pub active_in: Vec<String>,
}

impl Macro {
    fn repaired(mut self) -> Self {
        self.name = clean_line(&self.name, MAX_NAME);
        self.text = self.text.chars().filter(|c| *c == '\n' || !c.is_control()).take(MAX_TEXT).collect();
        self.alias = self.alias.map(|a| clean_line(&a, MAX_NAME)).filter(|a| !a.is_empty() && !a.contains(char::is_whitespace));
        self.shortcut = self.shortcut.filter(|k| !k.is_empty() && k.len() <= 40);
        self.active_in.truncate(crate::element::MAX_ELEMENTS);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.text.is_empty() && self.element.is_none() && self.after.is_none()
    }
}

/// A script's macro slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Macros {
    pub slots: Vec<Macro>,
}

impl Default for Macros {
    fn default() -> Self {
        starter()
    }
}

/// The starter set: scene intros, times of day, parentheticals and transitions.
pub fn starter() -> Macros {
    let m = |name: &str, element: Option<&str>, text: &str, after: Option<&str>, key: usize| Macro {
        name: name.into(),
        shortcut: Some(format!("Mod+Alt+Shift+{}", key % 10)),
        element: element.map(str::to_string),
        text: text.into(),
        after: after.map(str::to_string),
        ..Macro::default()
    };
    let mut slots = vec![
        m("Interior", Some("SceneHeading"), "INT. ", None, 1),
        m("Exterior", Some("SceneHeading"), "EXT. ", None, 2),
        m("Day", None, " - DAY", Some("Action"), 3),
        m("Night", None, " - NIGHT", Some("Action"), 4),
        m("Continuous", None, " - CONTINUOUS", Some("Action"), 5),
        m("A pause", Some("Parenthetical"), "pause", Some("Dialogue"), 6),
        m("Under breath", Some("Parenthetical"), "under her breath", Some("Dialogue"), 7),
        m("Cut", Some("Transition"), "CUT TO:", Some("SceneHeading"), 8),
        m("Dissolve", Some("Transition"), "DISSOLVE TO:", Some("SceneHeading"), 9),
        m("Fade out", Some("Transition"), "FADE OUT.", None, 10),
    ];
    slots.resize(SLOTS, Macro::default());
    Macros { slots }
}

impl Macros {
    pub fn of_doc(doc: &Document) -> Self {
        let mut m: Macros = match doc.app_data.get(MACROS_KEY) {
            Some(v) => serde_json::from_value((**v).clone()).unwrap_or_else(|_| Macros { slots: Vec::new() }),
            None => starter(),
        };
        m.slots = m.slots.into_iter().take(SLOTS).map(Macro::repaired).collect();
        m.slots.resize(SLOTS, Macro::default());
        m
    }

    pub fn store(&self, doc: &mut Document) {
        crate::data::store(doc, MACROS_KEY, self);
    }

    /// The slot bound to `shortcut`, if any.
    pub fn by_shortcut(&self, shortcut: &str) -> Option<usize> {
        self.slots.iter().position(|m| !m.is_empty() && m.shortcut.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(shortcut)))
    }
}

fn slot_param(v: &Value) -> Result<usize, CmdError> {
    let slot = v.get("slot").and_then(Value::as_u64).ok_or_else(|| CmdError::Params("`slot` (1 to 20) is required".into()))? as usize;
    if !(1..=SLOTS).contains(&slot) {
        return Err(CmdError::Params(format!("`slot` is 1 to {SLOTS}")));
    }
    Ok(slot - 1)
}

fn list(s: &mut Session, _: &Value) -> CmdResult {
    let m = Macros::of_doc(&s.doc);
    let slots: Vec<Value> = m.slots.iter().enumerate().map(|(i, x)| json!({"slot": i + 1, "macro": x, "empty": x.is_empty()})).collect();
    Ok(json!({"slots": slots}))
}

/// `macro.set {slot, …fields}`: change the given fields of a slot.
fn set(s: &mut Session, v: &Value) -> CmdResult {
    let slot = slot_param(v)?;
    let mut all = Macros::of_doc(&s.doc);
    let elements = Elements::of_doc(&s.doc);
    let Some(m) = all.slots.get_mut(slot) else { return Err(CmdError::Params("no such slot".into())) };
    let mut value = serde_json::to_value(&*m).unwrap_or(Value::Null);
    if let (Some(obj), Some(given)) = (value.as_object_mut(), v.as_object()) {
        for (k, x) in given {
            if k != "slot" && obj.contains_key(k) {
                obj.insert(k.clone(), x.clone());
            }
        }
    }
    let mut new: Macro = serde_json::from_value(value).map_err(|e| CmdError::Params(format!("macro: {e}")))?;
    for id in [&new.element, &new.after].into_iter().flatten() {
        if elements.get(id).is_none() {
            return Err(CmdError::Params(format!("no element `{id}`")));
        }
    }
    if new.text.len() > MAX_TEXT {
        return Err(CmdError::Params(format!("a macro's text is at most {MAX_TEXT} characters")));
    }
    new = new.repaired();
    if let Some(k) = &new.shortcut
        && all.slots.iter().enumerate().any(|(i, o)| i != slot && o.shortcut.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(k)))
    {
        return Err(CmdError::Params(format!("another macro already uses {k}")));
    }
    if let Some(sl) = all.slots.get_mut(slot) {
        *sl = new.clone();
    }
    all.store(&mut s.doc);
    Ok(json!(new))
}

fn clear(s: &mut Session, v: &Value) -> CmdResult {
    let slot = slot_param(v)?;
    let mut all = Macros::of_doc(&s.doc);
    if let Some(m) = all.slots.get_mut(slot) {
        *m = Macro::default();
    }
    all.store(&mut s.doc);
    Ok(Value::Null)
}

/// Whether macro `m` works where the caret is.
fn active_here(s: &Session, m: &Macro) -> bool {
    if m.active_in.is_empty() {
        return true;
    }
    let elements = Elements::of_doc(&s.doc);
    let here = crate::commands::element::element_of_para_at(&s.doc, &elements, &s.sel.focus).id.clone();
    m.active_in.contains(&here)
}

/// Play macro `m` at the caret.
pub fn play(s: &mut Session, m: &Macro) -> CmdResult {
    if let Some(id) = &m.element {
        crate::commands::element::set(s, &json!({"element": id}))?;
    }
    if !m.text.is_empty() {
        crate::commands::text::insert(s, &json!({"text": m.text}))?;
    }
    if let Some(after) = &m.after {
        // Return at the end of the paragraph, then the element asked for.
        let block = s.sel.focus.path.last();
        let len = s.doc.body.get(block).and_then(|b| b.as_para()).map_or(0, |p| p.len());
        s.sel = Selection::caret(body_pos(block, len));
        crate::commands::text::new_paragraph(s, &Value::Null)?;
        crate::commands::element::set(s, &json!({"element": after}))?;
    }
    sel_result(s)
}

/// `macro.run {slot}`.
fn run(s: &mut Session, v: &Value) -> CmdResult {
    let slot = slot_param(v)?;
    let m = Macros::of_doc(&s.doc).slots.get(slot).cloned().unwrap_or_default();
    if m.is_empty() {
        return Err(CmdError::Failed(format!("macro {} is empty", slot + 1)));
    }
    if !active_here(s, &m) {
        return Err(CmdError::Failed(format!("“{}” does not work in this element", m.name)));
    }
    play(s, &m)
}

/// The alias just typed before the caret, if a macro has one: (slot, start offset of the alias).
/// Called after a word ends (a space or punctuation was typed).
pub fn alias_before_caret(s: &Session) -> Option<(usize, usize)> {
    let at = &s.sel.focus;
    let para = s.doc.para_at(at)?;
    let before = para.text.get(..at.off)?;
    // The character just typed ended the word.
    let last = before.chars().next_back()?;
    if last.is_alphanumeric() {
        return None;
    }
    let end = before.len() - last.len_utf8();
    let body = before.get(..end)?;
    let start = body.char_indices().rev().take_while(|(_, c)| c.is_alphanumeric() || *c == '\'' || *c == '.').last().map_or(end, |(i, _)| i);
    let word = body.get(start..)?;
    if word.is_empty() {
        return None;
    }
    let all = Macros::of_doc(&s.doc);
    all.slots.iter().enumerate().find_map(|(slot, m)| {
        let alias = m.alias.as_deref()?;
        if !active_here(s, m) {
            return None;
        }
        let eq = |a: &str, b: &str| if m.match_case { a == b } else { a.eq_ignore_ascii_case(b) };
        if eq(word, alias) {
            return Some((slot, start));
        }
        // Part words: the alias ending a longer word.
        let tail_at = end.checked_sub(alias.len())?;
        (!m.whole_word && tail_at > start && body.get(tail_at..).is_some_and(|t| eq(t, alias))).then_some((slot, tail_at))
    })
}

/// Replace the alias at `start..` (up to the break before the caret) with the macro's text.
pub fn expand(s: &mut Session, slot: usize, start: usize) -> CmdResult {
    let m = Macros::of_doc(&s.doc).slots.get(slot).cloned().unwrap_or_default();
    let at = s.sel.focus.clone();
    let para = s.doc.para_at(&at).ok_or_else(|| CmdError::Failed("the caret is not in a paragraph".into()))?;
    let before = para.text.get(..at.off).unwrap_or("");
    let last_len = before.chars().next_back().map_or(0, char::len_utf8);
    let end = at.off.saturating_sub(last_len);
    let typed = para.text.get(start..end).ok_or_else(|| CmdError::Failed("the alias is no longer there".into()))?.to_string();
    let breaker = para.text.get(end..at.off).unwrap_or("").to_string();
    let mut text = m.text.clone();
    if m.smart_replace {
        if typed.chars().all(|c| !c.is_lowercase()) && typed.chars().any(char::is_uppercase) {
            text = text.to_uppercase();
        } else if typed.chars().next().is_some_and(char::is_uppercase) {
            let mut c = text.chars();
            text = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
        }
    }
    s.sel = Selection { anchor: body_pos(at.path.last(), start), focus: body_pos(at.path.last(), at.off) };
    crate::commands::text::insert(s, &json!({"text": format!("{text}{breaker}")}))
}

/// `macro.expand {slot, start}`: replace an alias the writer confirmed.
fn expand_cmd(s: &mut Session, v: &Value) -> CmdResult {
    let slot = slot_param(v)?;
    let start = v.get("start").and_then(Value::as_u64).ok_or_else(|| CmdError::Params("`start` is required".into()))? as usize;
    expand(s, slot, start)
}

/// `macro.copyFrom {macros}`: replace this script's macros with another's (the window reads the
/// other script; this takes its `macro.list` or stored value).
fn copy_from(s: &mut Session, v: &Value) -> CmdResult {
    let given = v.get("macros").cloned().ok_or_else(|| CmdError::Params("`macros` is required".into()))?;
    let mut doc = Document::new();
    doc.app_data.insert(MACROS_KEY.into(), std::sync::Arc::new(given));
    let mut m = Macros::of_doc(&doc);
    // Elements this script does not have are dropped from the copied macros.
    let elements = Elements::of_doc(&s.doc);
    for x in &mut m.slots {
        x.element = x.element.take().filter(|e| elements.get(e).is_some());
        x.after = x.after.take().filter(|e| elements.get(e).is_some());
        x.active_in.retain(|e| elements.get(e).is_some());
    }
    m.store(&mut s.doc);
    Ok(json!({"copied": m.slots.iter().filter(|x| !x.is_empty()).count()}))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Tools › Macros";
    vec![
        CommandSpec::new("macro.list", "Macros", LOC, list).pure(),
        CommandSpec::new("macro.set", "Set Macro", LOC, set).params(r#"{"slot": 1-20, "name"?, "shortcut"?, "element"?, "text"?, "after"?, "alias"?, "smartReplace"?, "confirm"?, "wholeWord"?, "matchCase"?, "activeIn"?}"#),
        CommandSpec::new("macro.clear", "Clear Macro", LOC, clear).params(r#"{"slot": 1-20}"#),
        CommandSpec::new("macro.run", "Run Macro", LOC, run).params(r#"{"slot": 1-20}"#),
        CommandSpec::new("macro.expand", "Replace Alias", LOC, expand_cmd).params(r#"{"slot": 1-20, "start": offset}"#),
        CommandSpec::new("macro.copyFrom", "Copy Macros", LOC, copy_from).params(r#"{"macros": stored macros}"#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::commands::text::body_lines;

    #[test]
    fn starter_macros_write_headings_and_transitions() {
        let mut s = new_session();
        s.run("macro.run", &json!({"slot": 1})).unwrap();
        s.run("text.insert", &json!({"text": "hall"})).unwrap();
        s.run("macro.run", &json!({"slot": 4})).unwrap();
        s.run("text.insert", &json!({"text": "Rain."})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("macro.run", &json!({"slot": 8})).unwrap();
        let lines = body_lines(&s);
        assert_eq!(lines[0], ("SceneHeading".into(), "INT. HALL - NIGHT".into()));
        assert_eq!(lines[1], ("Action".into(), "Rain.".into()));
        assert_eq!(lines[2], ("Transition".into(), "CUT TO:".into()));
        assert_eq!(lines[3].0, "SceneHeading");
        assert_eq!(Macros::of_doc(&s.doc).by_shortcut("Mod+Alt+Shift+8"), Some(7));
    }

    #[test]
    fn a_parenthetical_macro_fills_its_brackets() {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. hall\nRain.\n"})).unwrap();
        s.run("element.character", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "anna"})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("macro.run", &json!({"slot": 6})).unwrap();
        let lines = body_lines(&s);
        assert_eq!(lines[3], ("Parenthetical".into(), "(pause)".into()));
        assert_eq!(lines[4].0, "Dialogue");
    }

    #[test]
    fn aliases_expand_with_smart_capitals() {
        let mut s = new_session();
        s.run("macro.set", &json!({"slot": 11, "name": "HQ", "text": "headquarters", "alias": "hq", "smartReplace": true})).unwrap();
        s.run("text.insert", &json!({"text": "int. hall\nBack at Hq"})).unwrap();
        // The space ends the word and the alias is replaced as it is typed.
        s.run("text.insert", &json!({"text": " "})).unwrap();
        assert_eq!(body_lines(&s)[1].1, "Back at Headquarters ");
        s.run("text.insert", &json!({"text": "HQ"})).unwrap();
        s.run("text.insert", &json!({"text": "."})).unwrap();
        assert_eq!(body_lines(&s)[1].1, "Back at Headquarters HEADQUARTERS.");
        // With confirm on, the window is asked instead.
        s.run("macro.set", &json!({"slot": 11, "confirm": true})).unwrap();
        s.run("text.insert", &json!({"text": " hq"})).unwrap();
        s.run("text.insert", &json!({"text": "!"})).unwrap();
        assert!(s.ui_requests.iter().any(|r| r["open"] == "tool.macroConfirm"));
        let (slot, start) = alias_before_caret(&s).unwrap();
        assert_eq!(slot, 10);
        expand(&mut s, slot, start).unwrap();
        assert!(body_lines(&s)[1].1.ends_with(" headquarters!"));
    }

    #[test]
    fn set_checks_elements_shortcuts_and_slots() {
        let mut s = new_session();
        assert!(s.run("macro.set", &json!({"slot": 0})).is_err());
        assert!(s.run("macro.set", &json!({"slot": 21})).is_err());
        assert!(s.run("macro.set", &json!({"slot": 12, "element": "Nope"})).is_err());
        assert!(s.run("macro.set", &json!({"slot": 12, "shortcut": "Mod+Alt+Shift+1"})).is_err());
        s.run("macro.set", &json!({"slot": 12, "text": "x", "activeIn": ["Dialogue"]})).unwrap();
        assert!(s.run("macro.run", &json!({"slot": 12})).is_err(), "not active in a Scene Heading");
        s.run("macro.clear", &json!({"slot": 1})).unwrap();
        assert!(s.run("macro.run", &json!({"slot": 1})).is_err());
    }

    #[test]
    fn copying_drops_unknown_elements_and_hostile_values_are_bounded() {
        let mut s = new_session();
        let other = json!({"slots": [{"name": "x", "text": "y", "element": "Lyrics", "alias": "two words"}]});
        s.run("macro.copyFrom", &json!({"macros": other})).unwrap();
        let m = Macros::of_doc(&s.doc);
        assert_eq!(m.slots.len(), SLOTS);
        assert_eq!(m.slots[0].element, None);
        assert_eq!(m.slots[0].alias, None);
        let mut doc = Document::new();
        crate::data::store(&mut doc, MACROS_KEY, &json!({"slots": vec![json!({"text": "z".repeat(10_000)}); 500]}));
        let m = Macros::of_doc(&doc);
        assert!(m.slots.len() == SLOTS && m.slots[0].text.len() <= MAX_TEXT);
    }
}
