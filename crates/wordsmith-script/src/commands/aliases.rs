// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's command ids from before 0.2 that the engine or the core now provides under another
//! id (core decision 0007). Each old id stays registered and runs the new command, so command
//! lines, agents and recorded macros that use the old ids keep working. The interface uses the
//! new ids.

use std::collections::HashMap;
use std::sync::OnceLock;

use jcj_engine::{CmdError, CmdResult, CommandSpec, Session};
use serde_json::{Value, json};

type RunFn = fn(&mut Session, &Value) -> CmdResult;

/// The commands below Script's: the core's general commands, the shared features' and the
/// engine's, by id (the first of them with an id wins, as in the registry).
fn below() -> &'static HashMap<&'static str, RunFn> {
    static BELOW: OnceLock<HashMap<&'static str, RunFn>> = OnceLock::new();
    BELOW.get_or_init(|| {
        let mut map = HashMap::new();
        for spec in jcj_engine::commands::specs().into_iter().chain(jcj_features::commands()).chain(jcj_engine::engine_specs()) {
            map.entry(spec.id).or_insert(spec.run);
        }
        map
    })
}

/// Run the core's, the features' or the engine's command `id` (never Script's own command of the
/// same id, so Script's commands can build on the one they replace).
pub fn call(s: &mut Session, id: &str, v: &Value) -> CmdResult {
    let run = below().get(id).copied().ok_or_else(|| CmdError::Unknown(id.to_string()))?;
    run(s, v)
}

/// `v` with `extend: true` added.
fn extended(v: &Value) -> Value {
    let mut v = if v.is_object() { v.clone() } else { json!({}) };
    if let Some(o) = v.as_object_mut() {
        o.insert("extend".into(), json!(true));
    }
    v
}

/// `v` with Script's `elements` filter given as the core's `styles` (element ids are style ids).
pub fn with_styles(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(e) = v.get("elements").cloned()
        && v.get("styles").is_none()
        && let Some(o) = v.as_object_mut()
    {
        o.insert("styles".into(), e);
    }
    v
}

macro_rules! alias {
    ($old:literal, $label:literal, $new:literal) => {
        CommandSpec::new($old, $label, "Script (old names)", |s, v| call(s, $new, v))
    };
    ($old:literal, $label:literal, $new:literal, extend) => {
        CommandSpec::new($old, $label, "Script (old names)", |s, v| call(s, $new, &extended(v)))
    };
}

/// `select.wordAt {pos}`: put the caret at `pos`, then select the word there.
fn select_word_at(s: &mut Session, v: &Value) -> CmdResult {
    let pos = v.get("pos").cloned().ok_or_else(|| CmdError::Params("`pos` is required".into()))?;
    call(s, "caret.set", &json!({"pos": pos}))?;
    call(s, "select.word", &json!({}))
}

/// `format.style {style, on?}`: the engine's `format.<style> {value?}`.
fn format_style(s: &mut Session, v: &Value) -> CmdResult {
    let style = v.get("style").and_then(Value::as_str).ok_or_else(|| CmdError::Params("`style` is required".into()))?;
    let mut params = json!({});
    if let Some(on) = v.get("on")
        && let Some(o) = params.as_object_mut()
    {
        o.insert("value".into(), on.clone());
    }
    match style {
        "strike" | "strikethrough" => call(s, "format.strikethrough", &params),
        "doubleStrike" => call(s, "format.doubleStrikethrough", &params),
        "allCaps" | "caps" => call(s, "format.allCaps", &params),
        "smallCaps" => call(s, "format.smallCaps", &params),
        "superscript" | "raised" => call(s, "format.superscript", &params),
        "subscript" | "lowered" => call(s, "format.subscript", &params),
        "doubleUnderline" => call(s, "format.underline", &json!({"style": "double", "value": params.get("value")})),
        "dottedUnderline" => call(s, "format.underline", &json!({"style": "dotted", "value": params.get("value")})),
        "wordUnderline" | "wordsUnderline" => call(s, "format.underline", &json!({"style": "words", "value": params.get("value")})),
        "bold" | "italic" | "underline" => call(s, &format!("format.{style}"), &params),
        other => Err(CmdError::Params(format!("unknown style `{other}`"))),
    }
}

/// The old ids, each running the command that replaced it.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        alias!("caret.lineStart", "Line Start", "caret.home").pure(),
        alias!("caret.lineEnd", "Line End", "caret.end").pure(),
        alias!("select.left", "Select Left", "caret.left", extend).pure(),
        alias!("select.right", "Select Right", "caret.right", extend).pure(),
        alias!("select.wordLeft", "Select Word Left", "caret.wordLeft", extend).pure(),
        alias!("select.wordRight", "Select Word Right", "caret.wordRight", extend).pure(),
        alias!("select.up", "Select Up", "caret.up", extend).pure(),
        alias!("select.down", "Select Down", "caret.down", extend).pure(),
        alias!("select.lineStart", "Select to Line Start", "caret.home", extend).pure(),
        alias!("select.lineEnd", "Select to Line End", "caret.end", extend).pure(),
        alias!("select.docStart", "Select to Script Start", "caret.docStart", extend).pure(),
        alias!("select.docEnd", "Select to Script End", "caret.docEnd", extend).pure(),
        alias!("caret.to", "Place Caret", "caret.set").params(r#"{"pos": position, "extend"?: bool}"#).pure(),
        CommandSpec::new("select.wordAt", "Select Word", "Script (old names)", select_word_at).params(r#"{"pos": position}"#).pure(),
        alias!("text.toggleCase", "Toggle Case", "format.toggleCapitals"),
        alias!("paragraph.next", "Next Paragraph", "select.nextParagraph").pure(),
        alias!("paragraph.previous", "Previous Paragraph", "select.previousParagraph").pure(),
        CommandSpec::new("format.style", "Text Style", "Script (old names)", format_style).params(r#"{"style": string, "on"?: bool}"#),
        CommandSpec::new("format.lowercase", "Small Letters", "Script (old names)", |s, _| call(s, "format.changeCase", &json!({"mode": "lower"}))),
        CommandSpec::new("replace.one", "Replace", "Script (old names)", |s, v| super::find::replace_one(s, &with_styles(v))),
        CommandSpec::new("replace.all", "Replace All", "Script (old names)", |s, v| super::find::replace_all(s, &with_styles(v))),
        alias!("spelling.addWord", "Add to Dictionary", "review.addToDictionary").params(r#"{"word": string}"#),
    ]
}
