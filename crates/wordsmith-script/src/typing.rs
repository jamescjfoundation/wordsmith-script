// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typing options (SPEC-2 3.2): smart quotes and capitalising the first word of a sentence, with
//! a list of abbreviations that do not end a sentence. Both are off by default so typing behaves
//! as it did in v0.1 (decision 0031); a writer turns them on per script. Kept in `app_data`
//! under [`TYPING_KEY`].
//!
//! They act on typed characters only (one character at a time), never on pasted text, so a
//! paste lands exactly as copied.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

use crate::element::{Casing, ElementKind};

/// `app_data` key of the typing options.
pub const TYPING_KEY: &str = "wordsmith.typing";
/// Most abbreviations kept, and the longest one, in characters.
pub const MAX_ABBREVIATIONS: usize = 500;
pub const MAX_ABBREVIATION_LEN: usize = 16;

/// Abbreviations that end in a period without ending the sentence. Our own list.
pub const DEFAULT_ABBREVIATIONS: [&str; 22] = [
    "Mr.", "Mrs.", "Ms.", "Dr.", "St.", "Jr.", "Sr.", "Prof.", "Capt.", "Lt.", "Sgt.", "Col.", "Gen.", "Rev.", "Mt.", "Ave.", "No.", "vs.", "etc.",
    "e.g.", "i.e.", "approx.",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Typing {
    /// Straight quotes become curly ones as they are typed.
    pub smart_quotes: bool,
    /// The first letter of a sentence is put in capitals as it is typed.
    pub auto_capitalize: bool,
    pub abbreviations: Vec<String>,
}

impl Default for Typing {
    fn default() -> Self {
        Typing { smart_quotes: false, auto_capitalize: false, abbreviations: DEFAULT_ABBREVIATIONS.iter().map(|s| (*s).to_string()).collect() }
    }
}

impl Typing {
    pub fn of_doc(doc: &Document) -> Self {
        let mut t: Typing = crate::data::load(doc, TYPING_KEY);
        t.abbreviations.retain(|a| !a.trim().is_empty() && a.chars().count() <= MAX_ABBREVIATION_LEN);
        t.abbreviations.truncate(MAX_ABBREVIATIONS);
        t
    }

    pub fn store(&self, doc: &mut Document) {
        crate::data::store(doc, TYPING_KEY, self);
    }

    /// `typed` (one character) as it should go in after `before`, the paragraph's text up to the
    /// caret, in an element of `kind` and `casing`.
    pub fn transform(&self, typed: &str, before: &str, kind: ElementKind, casing: Casing) -> String {
        let mut chars = typed.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else { return typed.to_string() };
        let prev = before.chars().next_back();
        let opening = prev.is_none_or(|p| p.is_whitespace() || "([{—–-“‘".contains(p));
        if self.smart_quotes && c == '"' {
            return if opening { "“" } else { "”" }.to_string();
        }
        if self.smart_quotes && c == '\'' {
            return if opening { "‘" } else { "’" }.to_string();
        }
        if self.auto_capitalize
            && casing == Casing::AsTyped
            && c.is_lowercase()
            && matches!(kind, ElementKind::Action | ElementKind::Dialogue | ElementKind::General)
            && self.starts_sentence(before)
        {
            return c.to_uppercase().collect();
        }
        typed.to_string()
    }

    /// Whether a letter typed after `before` begins a sentence.
    pub fn starts_sentence(&self, before: &str) -> bool {
        let trimmed = before.trim_end();
        if trimmed.is_empty() {
            return true;
        }
        // A letter typed straight after text is inside a word.
        if trimmed.len() == before.len() {
            return false;
        }
        let end = trimmed.trim_end_matches(['"', '\'', '”', '’', ')']);
        if !end.ends_with(['.', '!', '?']) {
            return false;
        }
        let last_word = end.rsplit(char::is_whitespace).next().unwrap_or("");
        !self.abbreviations.iter().any(|a| a.eq_ignore_ascii_case(last_word))
    }
}

fn settings(s: &mut Session, _: &Value) -> CmdResult {
    Ok(serde_json::to_value(Typing::of_doc(&s.doc)).unwrap_or(Value::Null))
}

/// `typing.configure {smartQuotes?, autoCapitalize?, abbreviations?}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut t = Typing::of_doc(&s.doc);
    if let Some(b) = p::bool(v, "smartQuotes") {
        t.smart_quotes = b;
    }
    if let Some(b) = p::bool(v, "autoCapitalize") {
        t.auto_capitalize = b;
    }
    if let Some(list) = v.get("abbreviations") {
        let list = list.as_array().ok_or_else(|| CmdError::Params("`abbreviations` is a list of strings".into()))?;
        if list.len() > MAX_ABBREVIATIONS {
            return Err(CmdError::Params(format!("at most {MAX_ABBREVIATIONS} abbreviations")));
        }
        t.abbreviations = list.iter().filter_map(Value::as_str).map(str::trim).filter(|a| !a.is_empty()).map(str::to_string).collect();
    }
    t.store(&mut s.doc);
    Ok(json!(Typing::of_doc(&s.doc)))
}

fn toggle(s: &mut Session, smart: bool) -> CmdResult {
    let mut t = Typing::of_doc(&s.doc);
    if smart {
        t.smart_quotes = !t.smart_quotes;
    } else {
        t.auto_capitalize = !t.auto_capitalize;
    }
    t.store(&mut s.doc);
    Ok(json!(t))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("typing.settings", "Typing Options", "Format", settings).pure(),
        CommandSpec::new("typing.configure", "Set Typing Options", "Format", configure)
            .params(r#"{"smartQuotes"?: bool, "autoCapitalize"?: bool, "abbreviations"?: [string]}"#),
        CommandSpec::new("typing.smartQuotes", "Smart Quotes", "Format", |s, _| toggle(s, true)),
        CommandSpec::new("typing.autoCapitalize", "Capitalize Sentences", "Format", |s, _| toggle(s, false)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Typing {
        Typing { smart_quotes: true, auto_capitalize: true, ..Typing::default() }
    }

    #[test]
    fn off_by_default_and_typing_is_unchanged() {
        let t = Typing::default();
        assert_eq!(t.transform("\"", "", ElementKind::Action, Casing::AsTyped), "\"");
        assert_eq!(t.transform("a", "Done. ", ElementKind::Action, Casing::AsTyped), "a");
    }

    #[test]
    fn quotes_open_and_close() {
        let t = on();
        assert_eq!(t.transform("\"", "She said ", ElementKind::Action, Casing::AsTyped), "“");
        assert_eq!(t.transform("\"", "She said “hi", ElementKind::Action, Casing::AsTyped), "”");
        assert_eq!(t.transform("'", "don", ElementKind::Dialogue, Casing::AsTyped), "’");
        assert_eq!(t.transform("'", "(", ElementKind::Dialogue, Casing::AsTyped), "‘");
    }

    #[test]
    fn sentences_start_with_capitals_but_not_after_abbreviations() {
        let t = on();
        assert_eq!(t.transform("t", "", ElementKind::Action, Casing::AsTyped), "T");
        assert_eq!(t.transform("t", "He runs. ", ElementKind::Action, Casing::AsTyped), "T");
        assert_eq!(t.transform("t", "Really?\" ", ElementKind::Dialogue, Casing::AsTyped), "T");
        assert_eq!(t.transform("s", "Mr. ", ElementKind::Action, Casing::AsTyped), "s");
        assert_eq!(t.transform("t", "He runs ", ElementKind::Action, Casing::AsTyped), "t");
        assert_eq!(t.transform("t", "word", ElementKind::Action, Casing::AsTyped), "t");
        // Only in prose elements; never in a pasted run of text.
        assert_eq!(t.transform("t", "", ElementKind::Parenthetical, Casing::AsTyped), "t");
        assert_eq!(t.transform("two", "", ElementKind::Action, Casing::AsTyped), "two");
    }

    #[test]
    fn hostile_settings_are_bounded() {
        let mut doc = Document::new();
        let long: Vec<String> = (0..2000).map(|i| format!("a{i}.")).chain(["x".repeat(100)]).collect();
        crate::data::store(&mut doc, TYPING_KEY, &json!({"abbreviations": long, "smartQuotes": "yes"}));
        let t = Typing::of_doc(&doc);
        assert!(t.abbreviations.len() <= MAX_ABBREVIATIONS);
        crate::data::store(&mut doc, TYPING_KEY, &json!({"abbreviations": long}));
        let t = Typing::of_doc(&doc);
        assert!(t.abbreviations.len() <= MAX_ABBREVIATIONS && t.abbreviations.iter().all(|a| a.len() <= MAX_ABBREVIATION_LEN));
    }
}
