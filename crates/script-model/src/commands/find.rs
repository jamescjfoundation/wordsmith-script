// SPDX-License-Identifier: MIT OR Apache-2.0
//! Find and replace are the core's (`jcj_engine::commands::find`: `find.all`, `find.next`,
//! `find.previous`, `find.replace`, `find.replaceAll`). Matches lie within one paragraph; "match
//! case" and "whole word" are options; a search may be limited to some elements.
//!
//! Script's rules on top: a replacement in an uppercase element is put in capitals, as typing
//! there would be (decision 0010), and the old `elements` filter still works (element ids are
//! paragraph style ids, the core's `styles`). Replace All is one undo step.

use jcj_doc::{Document, Pos};
use jcj_engine::commands::find::{Query, replace_all_with, replace_range};
use jcj_engine::{CmdResult, CommandSpec, Session, cmd::sel_result, params as p};
use serde_json::{Value, json};

pub use jcj_engine::commands::find::{MAX_MATCHES, MAX_QUERY, find_all};

use super::aliases::{call, with_styles};
use crate::element::Elements;

/// `with` cased for the paragraph at `pos`.
fn cased(doc: &Document, pos: &Pos, with: &str) -> String {
    let elements = Elements::of_doc(doc);
    doc.para_at(pos).map_or(with.to_string(), |p| elements.of_para(p).casing.apply(with))
}

fn replacement(v: &Value) -> Result<String, jcj_engine::CmdError> {
    let with = p::req_str(v, "with")?;
    if with.len() > MAX_QUERY || with.contains(['\n', '\r']) {
        return Err(jcj_engine::CmdError::Params(format!("`with` is at most {MAX_QUERY} bytes on one line")));
    }
    Ok(with.to_string())
}

/// `find.replace`: replace the selected match (if the selection is one) in the element's casing,
/// then select the next.
pub fn replace_one(s: &mut Session, v: &Value) -> CmdResult {
    let v = with_styles(v);
    let q = Query::from_params(&v)?;
    let with = replacement(&v)?;
    let (a, b) = s.sel.ordered();
    let replaced = find_all(&s.doc, &q).iter().any(|(ma, mb)| *ma == a && *mb == b);
    if replaced {
        let text = cased(&s.doc, &a, &with);
        replace_range(s, &a, &b, &text)?;
    }
    let _ = call(s, "find.next", &v);
    let mut r = sel_result(s)?;
    if let Some(o) = r.as_object_mut() {
        o.insert("replaced".into(), json!(replaced));
    }
    Ok(r)
}

/// `find.replaceAll`: every match, each in its element's casing, as one undo step.
pub fn replace_all(s: &mut Session, v: &Value) -> CmdResult {
    let v = with_styles(v);
    let q = Query::from_params(&v)?;
    let with = replacement(&v)?;
    let n = replace_all_with(s, &q, &|doc, at| cased(doc, at, &with))?;
    Ok(json!({"replaced": n}))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Edit › Find";
    const FIND: &str = r#"{"query": string, "matchCase"?: bool, "wholeWord"?: bool, "styles"?: [elementId]}"#;
    const REPLACE: &str = r#"{"query": string, "with": string, "matchCase"?: bool, "wholeWord"?: bool, "styles"?: [elementId]}"#;
    vec![
        CommandSpec::new("find.all", "Find All", LOC, |s, v| call(s, "find.all", &with_styles(v))).params(FIND).pure(),
        CommandSpec::new("find.next", "Find Next", LOC, |s, v| call(s, "find.next", &with_styles(v))).params(FIND).pure(),
        CommandSpec::new("find.previous", "Find Previous", LOC, |s, v| call(s, "find.previous", &with_styles(v))).params(FIND).pure(),
        CommandSpec::new("find.replace", "Replace", LOC, replace_one).params(REPLACE),
        CommandSpec::new("find.replaceAll", "Replace All", LOC, replace_all).params(REPLACE),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::commands::{new_session, text::body_lines};

    #[test]
    fn find_next_and_previous_wrap() {
        let mut s = new_session();
        s.run("element.action", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "one dog\ntwo dog\nthree dog"})).unwrap();
        s.run("caret.docStart", &json!({})).unwrap();
        let blocks = |s: &mut jcj_engine::Session| s.sel.focus.path.last();
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
