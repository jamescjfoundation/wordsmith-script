// SPDX-License-Identifier: MIT OR Apache-2.0
//! The format checker (SPEC-2 14): five rules, each issue fixable or left to the writer.
//!
//! 1. A paragraph starts with spaces: fix removes them.
//! 2. A paragraph is empty: fix deletes it.
//! 3. A Character has no speech after it: fix deletes the Character.
//! 4. The same character speaks twice in a row: fixed by hand.
//! 5. Dialogue does not follow a Character or Parenthetical: fixed by hand.
//!
//! The "before printing" choice (off, ask, scan) is kept in `app_data` under [`CHECKER_KEY`];
//! the window acts on it.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Path, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::data::body_pos;
use crate::element::{ElementKind, Elements};

/// `app_data` key of the checker's settings.
pub const CHECKER_KEY: &str = "wordsmith.checker";
/// Most issues one run reports.
pub const MAX_ISSUES: usize = 10_000;

/// What happens before printing or exporting a PDF.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BeforePrint {
    #[default]
    Off,
    /// Offer to check first.
    Ask,
    /// Check, and show the checker when there are issues.
    Scan,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CheckerSettings {
    pub before_print: BeforePrint,
}

/// The kinds of issue, in the order of the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rule {
    LeadingSpace,
    Empty,
    CharacterWithoutSpeech,
    SameSpeakerTwice,
    DialogueWithoutSpeaker,
}

impl Rule {
    pub fn message(self) -> &'static str {
        match self {
            Rule::LeadingSpace => "This paragraph starts with spaces.",
            Rule::Empty => "This paragraph is empty.",
            Rule::CharacterWithoutSpeech => "This character name has no speech after it.",
            Rule::SameSpeakerTwice => "The same character speaks twice in a row.",
            Rule::DialogueWithoutSpeaker => "This dialogue has no character name before it.",
        }
    }

    pub fn fixable(self) -> bool {
        matches!(self, Rule::LeadingSpace | Rule::Empty | Rule::CharacterWithoutSpeech)
    }
}

/// One issue: the body block it is in and the rule it breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Issue {
    pub block: usize,
    pub rule: Rule,
}

/// A speaker's name without its extension: "ANNA (V.O.)" is ANNA.
pub fn speaker(name: &str) -> String {
    name.split('(').next().unwrap_or("").trim().to_uppercase()
}

/// Every issue in the script, in order.
pub fn check(doc: &Document) -> Vec<Issue> {
    let elements = Elements::of_doc(doc);
    let paras: Vec<(ElementKind, &str)> = doc.body.iter().filter_map(|b| b.as_para()).map(|p| (elements.of_para(p).kind, p.text.as_str())).collect();
    let only_one = paras.len() <= 1;
    let mut out = Vec::new();
    let mut last_speaker: Option<String> = None;
    for (i, &(kind, text)) in paras.iter().enumerate() {
        if out.len() >= MAX_ISSUES {
            break;
        }
        let prev = i.checked_sub(1).and_then(|j| paras.get(j)).map(|p| p.0);
        let next = paras.get(i + 1).map(|p| p.0);
        if text.trim().is_empty() {
            if !only_one {
                out.push(Issue { block: i, rule: Rule::Empty });
            }
            continue;
        }
        if text.starts_with(char::is_whitespace) {
            out.push(Issue { block: i, rule: Rule::LeadingSpace });
        }
        match kind {
            ElementKind::Character => {
                if !matches!(next, Some(ElementKind::Dialogue | ElementKind::Parenthetical)) {
                    out.push(Issue { block: i, rule: Rule::CharacterWithoutSpeech });
                }
                let who = speaker(text);
                if last_speaker.as_deref() == Some(who.as_str()) && !who.is_empty() {
                    out.push(Issue { block: i, rule: Rule::SameSpeakerTwice });
                }
                last_speaker = Some(who);
            }
            ElementKind::Dialogue => {
                if !matches!(prev, Some(ElementKind::Character | ElementKind::Parenthetical)) {
                    out.push(Issue { block: i, rule: Rule::DialogueWithoutSpeaker });
                }
            }
            ElementKind::Parenthetical => {}
            // Anything else between two speeches ends the run of one speaker.
            _ => last_speaker = None,
        }
    }
    out
}

fn issues_json(doc: &Document, issues: &[Issue]) -> Value {
    let list: Vec<Value> = issues
        .iter()
        .map(|i| {
            let text = doc.body.get(i.block).and_then(|b| b.as_para()).map(|p| p.text.chars().take(80).collect::<String>()).unwrap_or_default();
            json!({"block": i.block, "rule": i.rule, "message": i.rule.message(), "fixable": i.rule.fixable(), "text": text})
        })
        .collect();
    json!({"count": issues.len(), "issues": list})
}

fn run(s: &mut Session, _: &Value) -> CmdResult {
    Ok(issues_json(&s.doc, &check(&s.doc)))
}

/// `check.fix {block, rule}`: fix one issue, if it is still there and can be fixed.
fn fix(s: &mut Session, v: &Value) -> CmdResult {
    let block = v.get("block").and_then(Value::as_u64).map(|b| b as usize).ok_or_else(|| CmdError::Params("`block` is required".into()))?;
    let rule: Rule = serde_json::from_value(v.get("rule").cloned().unwrap_or(Value::Null)).map_err(|_| CmdError::Params("unknown `rule`".into()))?;
    if !check(&s.doc).contains(&Issue { block, rule }) {
        return Err(CmdError::Failed("that issue is no longer in the script".into()));
    }
    let path = Path::top(block);
    match rule {
        Rule::LeadingSpace => {
            let para = s.doc.para_mut(StoryRef::Body, &path)?;
            let n = para.text.len() - para.text.trim_start().len();
            para.delete(0, n)?;
            para.touch();
            s.sel = Selection::caret(body_pos(block, 0));
        }
        Rule::Empty | Rule::CharacterWithoutSpeech => {
            delete_block(s, block)?;
        }
        Rule::SameSpeakerTwice | Rule::DialogueWithoutSpeaker => return Err(CmdError::Failed("this issue is fixed by hand".into())),
    }
    s.clamp_selection();
    sel_result(s)
}

/// Delete body paragraph `block` (or empty it when it is the only one).
pub fn delete_block(s: &mut Session, block: usize) -> Result<(), CmdError> {
    let count = s.doc.body.iter().filter(|b| b.as_para().is_some()).count();
    if count <= 1 {
        let para = s.doc.para_mut(StoryRef::Body, &Path::top(block))?;
        let len = para.len();
        para.delete(0, len)?;
        para.touch();
        s.sel = Selection::caret(body_pos(block, 0));
        return Ok(());
    }
    if block >= s.doc.body.len() {
        return Err(CmdError::Params("no such paragraph".into()));
    }
    s.doc.body.remove(block);
    s.sel = Selection::caret(body_pos(block.min(s.doc.body.len().saturating_sub(1)), 0));
    Ok(())
}

/// `check.goto {block}`: select the paragraph an issue is in.
fn goto(s: &mut Session, v: &Value) -> CmdResult {
    let block = v.get("block").and_then(Value::as_u64).map(|b| b as usize).ok_or_else(|| CmdError::Params("`block` is required".into()))?;
    let len = s.doc.body.get(block).and_then(|b| b.as_para()).map(|p| p.len()).ok_or_else(|| CmdError::Params("no such paragraph".into()))?;
    s.sel = Selection { anchor: body_pos(block, 0), focus: body_pos(block, len) };
    sel_result(s)
}

fn settings(s: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(crate::data::load::<CheckerSettings>(&s.doc, CHECKER_KEY)))
}

/// `check.configure {beforePrint: "off" | "ask" | "scan"}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut c: CheckerSettings = crate::data::load(&s.doc, CHECKER_KEY);
    if let Some(b) = p::str(v, "beforePrint") {
        c.before_print = serde_json::from_value(json!(b)).map_err(|_| CmdError::Params("`beforePrint` is off, ask or scan".into()))?;
    }
    crate::data::store(&mut s.doc, CHECKER_KEY, &c);
    Ok(json!(c))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Tools";
    vec![
        CommandSpec::new("check.run", "Check Format", LOC, run).pure(),
        CommandSpec::new("check.fix", "Fix Format Issue", LOC, fix).params(r#"{"block": number, "rule": string}"#),
        CommandSpec::new("check.goto", "Go to Format Issue", LOC, goto).params(r#"{"block": number}"#),
        CommandSpec::new("check.settings", "Format Check Settings", LOC, settings).pure(),
        CommandSpec::new("check.configure", "Set Format Check", LOC, configure).params(r#"{"beforePrint": "off" | "ask" | "scan"}"#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::template::paragraph;
    use wordsmith_doc::para_block;

    fn doc(lines: &[(&str, &str)]) -> Session {
        let mut s = new_session();
        s.doc.body = lines.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
        s
    }

    #[test]
    fn each_rule_is_found() {
        let s = doc(&[
            ("SceneHeading", "INT. HALL - DAY"),
            ("Action", "  Rain."),
            ("Action", ""),
            ("Character", "ANNA"),
            ("Action", "She waits."),
            ("Character", "BEN"),
            ("Dialogue", "Hi."),
            ("Character", "BEN (CONT'D)"),
            ("Dialogue", "Again."),
            ("Dialogue", "And again."),
        ]);
        let found: Vec<(usize, Rule)> = check(&s.doc).into_iter().map(|i| (i.block, i.rule)).collect();
        assert_eq!(
            found,
            vec![
                (1, Rule::LeadingSpace),
                (2, Rule::Empty),
                (3, Rule::CharacterWithoutSpeech),
                (7, Rule::SameSpeakerTwice),
                (9, Rule::DialogueWithoutSpeaker),
            ]
        );
    }

    #[test]
    fn fixes_apply_and_undo() {
        let mut s = doc(&[("SceneHeading", "INT. HALL - DAY"), ("Action", "  Rain."), ("Action", ""), ("Character", "ANNA")]);
        s.run("check.fix", &json!({"block": 1, "rule": "leadingSpace"})).unwrap();
        s.run("check.fix", &json!({"block": 2, "rule": "empty"})).unwrap();
        s.run("check.fix", &json!({"block": 2, "rule": "characterWithoutSpeech"})).unwrap();
        assert!(check(&s.doc).is_empty());
        assert_eq!(s.doc.body.len(), 2);
        assert!(s.run("check.fix", &json!({"block": 9, "rule": "empty"})).is_err());
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc.body.len(), 3);
    }

    #[test]
    fn hand_fixes_are_refused_and_settings_round_trip() {
        let mut s = doc(&[("Action", "x"), ("Dialogue", "Hi.")]);
        assert!(s.run("check.fix", &json!({"block": 1, "rule": "dialogueWithoutSpeaker"})).is_err());
        assert_eq!(s.run("check.configure", &json!({"beforePrint": "ask"})).unwrap()["beforePrint"], "ask");
        assert!(s.run("check.configure", &json!({"beforePrint": "always"})).is_err());
    }
}
