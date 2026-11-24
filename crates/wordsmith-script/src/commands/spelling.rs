// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spelling commands. The core's (`jcj_engine::commands::spelling`): `spelling.suggest`,
//! `spelling.ignore` (the script's own words, `core.spelling`), `spelling.unignore`,
//! `spelling.words`; the engine's `review.addToDictionary` adds to the dictionary.
//!
//! Script's own `spelling.check` replaces the core's with the screenplay rules
//! ([`crate::spelling`]): Character paragraphs are not checked and speakers' names are right
//! everywhere. Each misspelling also gives its body `block`, as Script's did before 0.2, and
//! `spelling.replace` takes either the core's positions or Script's `{block, start, end}`.
//! `spelling.addWord` is kept as another name for `review.addToDictionary`.

use jcj_doc::{Path, Pos, StoryRef};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde_json::{Value, json};

use crate::spelling::{SpellCache, check};

fn list(s: &mut Session, _: &Value) -> CmdResult {
    let found = check(&s.doc, &mut SpellCache::default());
    let truncated = found.len() >= crate::spelling::MAX_REPORTED;
    let list: Vec<Value> = found
        .into_iter()
        .map(|m| {
            let at = |off| jcj_engine::cmd::pos_json(&Pos { story: StoryRef::Body, path: Path::top(m.block), off });
            json!({"start": at(m.start), "end": at(m.end), "block": m.block, "word": m.word})
        })
        .collect();
    Ok(json!({"misspellings": list, "truncated": truncated}))
}

/// `spelling.replace`: the core's, also taking `{block, start, end}` as numbers.
fn replace(s: &mut Session, v: &Value) -> CmdResult {
    let mut v = v.clone();
    if let Some(block) = p::u64(&v, "block").filter(|_| v.get("start").is_some_and(serde_json::Value::is_number)) {
        let block = usize::try_from(block).map_err(|_| CmdError::Params("`block` is too large".into()))?;
        for key in ["start", "end"] {
            let off = p::u64(&v, key).ok_or_else(|| CmdError::Params(format!("`{key}` is required")))?;
            let off = usize::try_from(off).map_err(|_| CmdError::Params(format!("`{key}` is too large")))?;
            if let Some(o) = v.as_object_mut() {
                o.insert(key.into(), jcj_engine::cmd::pos_json(&Pos { story: StoryRef::Body, path: Path::top(block), off }));
            }
        }
    }
    super::aliases::call(s, "spelling.replace", &v)
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Review › Spelling";
    vec![
        CommandSpec::new("spelling.check", "Check Spelling", LOC, list).pure(),
        CommandSpec::new("spelling.replace", "Replace Misspelling", LOC, replace)
            .params(r#"{"start": position, "end": position, "word": string, "with": string} or {"block": n, "start": n, "end": n, "word", "with"}"#),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::commands::{new_session, text::body_lines};

    #[test]
    fn check_ignore_and_replace() {
        let mut s = new_session();
        s.run("element.action", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "A glorp and a dragn."})).unwrap();
        let found = s.run("spelling.check", &json!({})).unwrap();
        let words: Vec<&str> = found["misspellings"].as_array().unwrap().iter().filter_map(|m| m["word"].as_str()).collect();
        assert_eq!(words, ["glorp", "dragn"]);
        s.run("spelling.ignore", &json!({"word": "glorp"})).unwrap();
        let m = &s.run("spelling.check", &json!({})).unwrap()["misspellings"][0];
        assert_eq!(m["word"], "dragn");
        let sug = s.run("spelling.suggest", &json!({"word": "dragn"})).unwrap();
        assert!(sug["suggestions"].as_array().unwrap().iter().any(|x| x == "dragon"), "{sug}");
        let mut params = m.clone();
        params["with"] = json!("dragon");
        s.run("spelling.replace", &params).unwrap();
        assert_eq!(body_lines(&s)[0].1, "A glorp and a dragon.");
        assert!(s.run("spelling.replace", &params).is_err(), "the word is no longer there");
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[0].1, "A glorp and a dragn.");
    }
}
