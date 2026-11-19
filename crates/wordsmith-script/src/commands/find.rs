// SPDX-License-Identifier: MIT OR Apache-2.0
//! Find and replace. Matches lie within one paragraph; "match case" and "whole word" are
//! options. A replacement in an uppercase element is put in capitals, as typing there would be
//! (decision 0010). Replace All is one undo step.

use serde_json::{Value, json};
use wordsmith_doc::{Path, Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::element::Elements;

/// Longest search text, bytes.
pub const MAX_QUERY: usize = 1_000;
/// Most matches `find.all` lists and `replace.all` replaces.
pub const MAX_MATCHES: usize = 100_000;

/// What to look for.
pub struct Query {
    pub text: String,
    pub match_case: bool,
    pub whole_word: bool,
}

impl Query {
    pub fn from_params(v: &Value) -> Result<Query, CmdError> {
        let text = p::req_str(v, "query")?;
        if text.is_empty() || text.len() > MAX_QUERY || text.contains('\n') {
            return Err(CmdError::Params(format!("`query` is 1 to {MAX_QUERY} bytes on one line")));
        }
        Ok(Query {
            text: text.to_string(),
            match_case: p::bool(v, "matchCase").unwrap_or(false),
            whole_word: p::bool(v, "wholeWord").unwrap_or(false),
        })
    }
}

fn same(a: char, b: char, case: bool) -> bool {
    if case { a == b } else { a == b || a.to_lowercase().eq(b.to_lowercase()) }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’'
}

/// Byte ranges of `q` in `text`, not overlapping.
pub fn find_in(text: &str, q: &Query) -> Vec<(usize, usize)> {
    let needle: Vec<char> = q.text.chars().collect();
    let hay: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        let hit = needle.iter().enumerate().all(|(k, &n)| hay.get(i + k).is_some_and(|&(_, c)| same(c, n, q.match_case)));
        if hit {
            let start = hay.get(i).map_or(0, |h| h.0);
            let end = hay.get(i + needle.len()).map_or(text.len(), |h| h.0);
            let before_ok = i == 0 || hay.get(i - 1).is_none_or(|&(_, c)| !is_word(c));
            let after_ok = hay.get(i + needle.len()).is_none_or(|&(_, c)| !is_word(c));
            if !q.whole_word || (before_ok && after_ok) {
                out.push((start, end));
                i += needle.len();
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Every match in the body, in order, as positions.
pub fn find_all(s: &Session, q: &Query) -> Vec<(Pos, Pos)> {
    let mut out = Vec::new();
    for (i, b) in s.doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        for (a, e) in find_in(&p.text, q) {
            let pos = |off| Pos { story: StoryRef::Body, path: Path::top(i), off };
            out.push((pos(a), pos(e)));
            if out.len() >= MAX_MATCHES {
                return out;
            }
        }
    }
    out
}

fn all(s: &mut Session, v: &Value) -> CmdResult {
    let q = Query::from_params(v)?;
    let found: Vec<Value> = find_all(s, &q).into_iter().map(|(a, b)| json!({"block": a.path.last(), "start": a.off, "end": b.off})).collect();
    Ok(json!({"count": found.len(), "matches": found}))
}

/// Select the next (or previous) match from the selection, wrapping round the script.
fn step(s: &mut Session, v: &Value, forward: bool) -> CmdResult {
    let q = Query::from_params(v)?;
    let found = find_all(s, &q);
    if found.is_empty() {
        return Err(CmdError::Failed(format!("`{}` was not found", q.text)));
    }
    let (a, b) = s.sel.ordered();
    // If the selection is itself a match, move past it; otherwise start at the caret.
    let on_match = found.iter().any(|(ma, mb)| *ma == a && *mb == b);
    let pick = if forward {
        found.iter().find(|(m, _)| if on_match { *m >= b && *m != a } else { *m >= a }).or_else(|| found.first())
    } else {
        found.iter().rev().find(|(m, _)| *m < a).or_else(|| found.last())
    };
    let Some((ma, mb)) = pick.cloned() else { return sel_result(s) };
    s.sel = Selection { anchor: ma, focus: mb };
    let mut r = sel_result(s)?;
    if let Some(o) = r.as_object_mut() {
        o.insert("count".into(), json!(found.len()));
    }
    Ok(r)
}

/// `with` cased for the paragraph at `pos`.
fn cased(s: &Session, pos: &Pos, with: &str) -> String {
    let elements = Elements::of_doc(&s.doc);
    s.doc.para_at(pos).map_or(with.to_string(), |p| elements.of_para(p).casing.apply(with))
}

fn replacement(v: &Value) -> Result<String, CmdError> {
    let with = p::req_str(v, "with")?;
    if with.len() > MAX_QUERY || with.contains('\n') {
        return Err(CmdError::Params(format!("`with` is at most {MAX_QUERY} bytes on one line")));
    }
    Ok(with.to_string())
}

/// `replace.one`: replace the selected match (if the selection is one), then select the next.
fn replace_one(s: &mut Session, v: &Value) -> CmdResult {
    let q = Query::from_params(v)?;
    let with = replacement(v)?;
    let (a, b) = s.sel.ordered();
    if find_all(s, &q).iter().any(|(ma, mb)| *ma == a && *mb == b) {
        let text = cased(s, &a, &with);
        let para = s.doc.para_mut(a.story, &a.path)?;
        let props = para.props_at(a.off).clone();
        para.delete(a.off, b.off)?;
        para.insert_text(a.off, &text, &props)?;
        para.touch();
        s.sel = Selection::caret(Pos { off: a.off + text.len(), ..a });
    }
    // Then on to the next match, if any.
    let _ = step(s, v, true);
    sel_result(s)
}

/// `replace.all`: every match, as one undo step. Returns how many.
fn replace_all(s: &mut Session, v: &Value) -> CmdResult {
    let q = Query::from_params(v)?;
    let with = replacement(v)?;
    let found = find_all(s, &q);
    // Back to front, so earlier offsets stay right.
    for (a, b) in found.iter().rev() {
        let text = cased(s, a, &with);
        let para = s.doc.para_mut(a.story, &a.path)?;
        let props = para.props_at(a.off).clone();
        para.delete(a.off, b.off)?;
        para.insert_text(a.off, &text, &props)?;
        para.touch();
    }
    s.clamp_selection();
    Ok(json!({"replaced": found.len()}))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Edit › Find";
    const PARAMS: &str = r#"{"query": string, "matchCase"?: bool, "wholeWord"?: bool}"#;
    const REPLACE: &str = r#"{"query": string, "with": string, "matchCase"?: bool, "wholeWord"?: bool}"#;
    vec![
        CommandSpec::new("find.all", "Find All", LOC, all).params(PARAMS).pure(),
        CommandSpec::new("find.next", "Find Next", LOC, |s, v| step(s, v, true)).params(PARAMS).pure(),
        CommandSpec::new("find.previous", "Find Previous", LOC, |s, v| step(s, v, false)).params(PARAMS).pure(),
        CommandSpec::new("replace.one", "Replace", LOC, replace_one).params(REPLACE),
        CommandSpec::new("replace.all", "Replace All", LOC, replace_all).params(REPLACE),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Query, find_in};
    use crate::commands::{new_session, text::body_lines};

    fn q(text: &str, match_case: bool, whole_word: bool) -> Query {
        Query { text: text.into(), match_case, whole_word }
    }

    #[test]
    fn matching_rules() {
        assert_eq!(find_in("Anna and anna and ANNAbel", &q("anna", false, false)), [(0, 4), (9, 13), (18, 22)]);
        assert_eq!(find_in("Anna and anna and ANNAbel", &q("anna", true, false)), [(9, 13)]);
        assert_eq!(find_in("Anna and anna and ANNAbel", &q("anna", false, true)), [(0, 4), (9, 13)]);
        assert_eq!(find_in("aaaa", &q("aa", false, false)), [(0, 2), (2, 4)], "no overlaps");
        assert_eq!(find_in("Café CAFÉ", &q("café", false, false)), [(0, 5), (6, 11)], "non-ASCII case");
        assert!(find_in("short", &q("longer text", false, false)).is_empty());
    }

    #[test]
    fn find_next_and_previous_wrap() {
        let mut s = new_session();
        s.run("element.action", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "one dog\ntwo dog\nthree dog"})).unwrap();
        s.run("caret.docStart", &json!({})).unwrap();
        let blocks = |s: &mut wordsmith_engine::Session| s.sel.focus.path.last();
        s.run("find.next", &json!({"query": "dog"})).unwrap();
        assert_eq!((blocks(&mut s), s.selected_text()), (0, "dog".into()));
        s.run("find.next", &json!({"query": "dog"})).unwrap();
        assert_eq!(blocks(&mut s), 1);
        s.run("find.next", &json!({"query": "dog"})).unwrap();
        s.run("find.next", &json!({"query": "dog"})).unwrap();
        assert_eq!(blocks(&mut s), 0, "wraps to the start");
        s.run("find.previous", &json!({"query": "dog"})).unwrap();
        assert_eq!(blocks(&mut s), 2, "and back to the end");
        assert!(s.run("find.next", &json!({"query": "cat"})).is_err());
        assert!(s.run("find.next", &json!({"query": ""})).is_err());
    }

    #[test]
    fn replace_one_and_all() {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. dog house - day\nThe dog barks. Dogma.\n"})).unwrap();
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("find.next", &json!({"query": "dog", "wholeWord": true})).unwrap();
        s.run("replace.one", &json!({"query": "dog", "with": "cat", "wholeWord": true})).unwrap();
        assert_eq!(body_lines(&s)[0].1, "INT. CAT HOUSE - DAY", "capitals in a scene heading");
        let r = s.run("replace.all", &json!({"query": "dog", "with": "cat", "wholeWord": true})).unwrap();
        assert_eq!(r["replaced"], 1);
        assert_eq!(body_lines(&s)[1].1, "The cat barks. Dogma.");
        assert_eq!(s.undo_label(), Some("Replace All"));
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[1].1, "The dog barks. Dogma.", "one undo step");
        let r = s.run("replace.all", &json!({"query": "dog", "with": "", "matchCase": true})).unwrap();
        assert_eq!(r["replaced"], 1);
        assert_eq!(body_lines(&s)[1].1, "The  barks. Dogma.");
    }
}
