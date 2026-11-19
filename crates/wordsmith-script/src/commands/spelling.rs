// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spelling commands: list misspellings, suggest, ignore in this script, add to the dictionary,
//! and replace a word.

use serde_json::{Value, json};
use wordsmith_doc::{Path, Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::spelling::{ScriptWords, SpellCache, check, suggest};

fn list(s: &mut Session, _: &Value) -> CmdResult {
    let found: Vec<Value> = check(&s.doc, &mut SpellCache::default())
        .into_iter()
        .map(|m| json!({"block": m.block, "start": m.start, "end": m.end, "word": m.word}))
        .collect();
    Ok(json!({"misspellings": found}))
}

fn suggest_cmd(_: &mut Session, v: &Value) -> CmdResult {
    let word = p::req_str(v, "word")?;
    Ok(json!({"suggestions": suggest(word, 8)}))
}

/// `spelling.ignore {word}`: the word is correct in this script (kept with the script, undoable).
fn ignore(s: &mut Session, v: &Value) -> CmdResult {
    let mut w = ScriptWords::of_doc(&s.doc);
    let added = w.add(p::req_str(v, "word")?);
    w.store(&mut s.doc);
    Ok(json!({"added": added}))
}

/// `spelling.addWord {word}`: add to the dictionary for every script, this session.
fn add_word(_: &mut Session, v: &Value) -> CmdResult {
    let word = p::req_str(v, "word")?.trim();
    if word.is_empty() || word.len() > 100 {
        return Err(CmdError::Params("`word` is 1 to 100 bytes".into()));
    }
    craft_proof::add_word(word);
    Ok(json!({"added": word}))
}

/// `spelling.replace {block, start, end, with}`: replace a misspelled word, checking that the
/// range is still the word that was misspelled (`word`).
fn replace(s: &mut Session, v: &Value) -> CmdResult {
    let block = p::u64(v, "block").ok_or_else(|| CmdError::Params("`block` is required".into()))? as usize;
    let start = p::u64(v, "start").ok_or_else(|| CmdError::Params("`start` is required".into()))? as usize;
    let end = p::u64(v, "end").ok_or_else(|| CmdError::Params("`end` is required".into()))? as usize;
    let word = p::req_str(v, "word")?;
    let with = p::req_str(v, "with")?;
    if with.contains('\n') || with.len() > 200 {
        return Err(CmdError::Params("`with` is one line of at most 200 bytes".into()));
    }
    let path = Path::top(block);
    let para = s.doc.para_mut(StoryRef::Body, &path)?;
    if para.text.get(start..end) != Some(word) {
        return Err(CmdError::Failed(format!("the text there is no longer `{word}`")));
    }
    let props = para.props_at(start).clone();
    para.delete(start, end)?;
    para.insert_text(start, with, &props)?;
    para.touch();
    s.sel = Selection::caret(Pos { story: StoryRef::Body, path, off: start + with.len() });
    sel_result(s)
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Tools › Spelling";
    vec![
        CommandSpec::new("spelling.check", "Check Spelling", LOC, list).pure(),
        CommandSpec::new("spelling.suggest", "Spelling Suggestions", LOC, suggest_cmd).params(r#"{"word": string}"#).pure(),
        CommandSpec::new("spelling.ignore", "Ignore in This Script", LOC, ignore).params(r#"{"word": string}"#),
        CommandSpec::new("spelling.addWord", "Add to Dictionary", LOC, add_word).params(r#"{"word": string}"#).pure(),
        CommandSpec::new("spelling.replace", "Replace Misspelling", LOC, replace)
            .params(r#"{"block": n, "start": n, "end": n, "word": string, "with": string}"#),
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
