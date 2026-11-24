// SPDX-License-Identifier: MIT OR Apache-2.0
//! Characters and the tables' CSV (decision 0022): list the characters, rename one everywhere,
//! and the scenes and characters tables as CSV.
//!
//! A rename changes, in one undo step: every cue of the character (its extension kept, "ANNA
//! (V.O.)" becomes "ANNE (V.O.)"); the name as a whole word or phrase in every other paragraph,
//! in the case style it is written in there (capitals stay capitals, lower case stays lower
//! case, anything else takes the new name as typed); and the Characters autocomplete list.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use wordsmith_doc::{Document, Paragraph, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

use crate::autocomplete::{Autocomplete, ListKind};
use crate::element::{ElementKind, Elements};
use crate::paginate::output::speaker;
use crate::scenes::scenes;

/// Longest name a rename takes, bytes.
pub const MAX_NAME: usize = 100;

/// One character's line in the characters table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterRow {
    pub name: String,
    pub speeches: usize,
    pub scenes: usize,
    /// 0-based index of the first scene the character speaks in, if any.
    pub first_scene: Option<usize>,
}

/// The characters who speak in `doc`, in order of first speech.
pub fn characters(doc: &Document) -> Vec<CharacterRow> {
    let elements = Elements::of_doc(doc);
    let all = scenes(doc);
    let mut rows: Vec<CharacterRow> = Vec::new();
    let mut seen_in: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        if elements.of_para(p).kind != ElementKind::Character {
            continue;
        }
        let name = speaker(&p.text).trim().to_uppercase();
        if name.is_empty() {
            continue;
        }
        let scene = crate::scenes::scene_at(&all, i).map(|s| s.index);
        if !rows.iter().any(|r| r.name == name) {
            rows.push(CharacterRow { name: name.clone(), speeches: 0, scenes: 0, first_scene: scene });
        }
        let Some(row) = rows.iter_mut().find(|r| r.name == name) else { continue };
        row.speeches += 1;
        let list = seen_in.entry(name).or_default();
        if let Some(sc) = scene
            && !list.contains(&sc)
        {
            list.push(sc);
            row.scenes += 1;
        }
    }
    rows
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte ranges of `name` in `text` as a whole word or phrase, ignoring case.
fn whole_matches(text: &str, name: &str) -> Vec<(usize, usize)> {
    let lower = text.to_lowercase();
    let needle = name.to_lowercase();
    // Lower-casing can change lengths for some scripts; only use positions when it did not.
    if needle.is_empty() || lower.len() != text.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = lower.get(from..).and_then(|r| r.find(&needle)) {
        let (a, b) = (from + at, from + at + needle.len());
        let before = text.get(..a).and_then(|t| t.chars().next_back());
        let after = text.get(b..).and_then(|t| t.chars().next());
        // "Anna's" is a match; "Annabel" and "Joanna" are not.
        if !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char) {
            out.push((a, b));
        }
        from = b;
    }
    out
}

/// The new name in the case style the old one was written in.
fn in_style_of(written: &str, to: &str) -> String {
    let letters: Vec<char> = written.chars().filter(|c| c.is_alphabetic()).collect();
    if !letters.is_empty() && letters.iter().all(|c| c.is_uppercase()) {
        to.to_uppercase()
    } else if !letters.is_empty() && letters.iter().all(|c| c.is_lowercase()) {
        to.to_lowercase()
    } else {
        to.to_string()
    }
}

/// `p` with each range replaced, keeping the formatting at each.
fn replace_ranges(p: &Paragraph, ranges: &[(usize, usize)], with: impl Fn(&str) -> String) -> Paragraph {
    let mut q = p.clone();
    for &(a, b) in ranges.iter().rev() {
        let old = p.text.get(a..b).unwrap_or("").to_string();
        // The formatting of the range's first character (`props_at` gives the one before).
        let first = p.text.get(a..).and_then(|t| t.chars().next()).map_or(0, char::len_utf8);
        let props = q.props_at(a + first).clone();
        if q.delete(a, b).is_ok() {
            let _ = q.insert_text(a, &with(&old), &props);
        }
    }
    q.touch();
    q
}

/// `character.list`: the characters table.
fn list(s: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(
        characters(&s.doc)
            .into_iter()
            .map(|r| json!({"name": r.name, "speeches": r.speeches, "scenes": r.scenes, "firstScene": r.first_scene}))
            .collect::<Vec<_>>()
    ))
}

/// `character.rename {from, to}`: rename a character everywhere (one undo step).
fn rename(s: &mut Session, v: &Value) -> CmdResult {
    let from = p::req_str(v, "from")?.split_whitespace().collect::<Vec<_>>().join(" ");
    let to = p::req_str(v, "to")?.split_whitespace().collect::<Vec<_>>().join(" ");
    if from.is_empty() || to.is_empty() || to.len() > MAX_NAME || to.contains(['(', ')']) {
        return Err(CmdError::Params(format!("names must be 1 to {MAX_NAME} bytes, without brackets")));
    }
    let elements = Elements::of_doc(&s.doc);
    let (mut cues, mut mentions) = (0, 0);
    for b in s.doc.body.iter_mut() {
        let Some(p) = b.as_para() else { continue };
        if elements.of_para(p).kind == ElementKind::Character {
            let name = speaker(&p.text);
            if name.trim().eq_ignore_ascii_case(&from) || name.trim().to_lowercase() == from.to_lowercase() {
                let start = p.text.find(name.trim()).unwrap_or(0);
                let end = start + name.trim().len();
                *b = para_block(replace_ranges(p, &[(start, end)], |_| to.to_uppercase()));
                cues += 1;
            }
            continue;
        }
        let ranges = whole_matches(&p.text, &from);
        if !ranges.is_empty() {
            mentions += ranges.len();
            *b = para_block(replace_ranges(p, &ranges, |old| in_style_of(old, &to)));
        }
    }
    if cues + mentions == 0 {
        return Err(CmdError::Params(format!("`{from}` does not appear in the script")));
    }
    let mut ac = Autocomplete::of_doc(&s.doc);
    if ac.entries(&s.doc, ListKind::Characters).iter().any(|e| e.eq_ignore_ascii_case(&from)) {
        ac.remove(&mut s.doc, ListKind::Characters, &from);
        ac.add(&mut s.doc, ListKind::Characters, &to.to_uppercase());
        ac.store(&mut s.doc);
    }
    s.clamp_selection();
    crate::cast::rename(&mut s.doc, &from, &to);
    Ok(json!({"cues": cues, "mentions": mentions}))
}

/// A CSV field (RFC 4180): quoted when it holds a comma, quote or line break. A field that
/// starts with `=`, `+`, `-` or `@` gets a leading `'`, so a spreadsheet does not run it as a
/// formula.
pub fn csv_field(text: &str) -> String {
    let text = if text.starts_with(['=', '+', '-', '@']) { format!("'{text}") } else { text.to_string() };
    if text.contains([',', '"', '\n', '\r']) { format!("\"{}\"", text.replace('"', "\"\"")) } else { text }
}

fn csv(rows: &[Vec<String>]) -> String {
    rows.iter().map(|r| r.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(",") + "\r\n").collect()
}

/// The scenes table as CSV: number, heading, page, speakers, summary.
pub fn scenes_csv(doc: &Document) -> String {
    let elements = Elements::of_doc(doc);
    let pag = crate::paginate::paginate(doc);
    let page_of = |block: usize| pag.map.iter().position(|m| m.is_some_and(|m| m.block == block)).and_then(|out| page_of_output(&pag, out));
    let mut rows = vec![vec!["Scene".to_string(), "Heading".into(), "Page".into(), "Length".into(), "Characters".into(), "Summary".into()]];
    let eighths = crate::reports::scene_eighths(doc, &pag);
    let numbers = crate::scene_numbers::numbers(doc);
    for sc in scenes(doc) {
        let v = super::scene::scene_json(doc, &elements, &sc);
        rows.push(vec![
            numbers.get(sc.index).cloned().unwrap_or_else(|| (sc.index + 1).to_string()),
            sc.heading.clone(),
            page_of(sc.start).map(|p| p.to_string()).unwrap_or_default(),
            eighths.get(sc.index).map(|e| crate::reports::format_eighths(*e)).unwrap_or_default(),
            super::scene::speakers(doc, &elements, &sc).join(", "),
            v.get("summary").and_then(Value::as_str).unwrap_or("").to_string(),
        ]);
    }
    csv(&rows)
}

/// The page (from 1) output paragraph `out` is on.
pub fn page_of_output(pag: &crate::paginate::Paginated, out: usize) -> Option<usize> {
    let mut seen = 0;
    for (n, page) in pag.pages.iter().enumerate() {
        seen += page.len();
        if out < seen {
            return Some(n + 1);
        }
    }
    None
}

/// The characters table as CSV: name, speeches, scenes, first scene.
pub fn characters_csv(doc: &Document) -> String {
    let mut rows = vec![vec!["Character".to_string(), "Speeches".into(), "Scenes".into(), "First scene".into()]];
    for r in characters(doc) {
        rows.push(vec![r.name, r.speeches.to_string(), r.scenes.to_string(), r.first_scene.map(|i| (i + 1).to_string()).unwrap_or_default()]);
    }
    csv(&rows)
}

/// `tables.csv {table}`: a table as CSV text.
fn table_csv(s: &mut Session, v: &Value) -> CmdResult {
    match p::req_str(v, "table")? {
        "scenes" => Ok(json!(scenes_csv(&s.doc))),
        "characters" => Ok(json!(characters_csv(&s.doc))),
        other => Err(CmdError::Params(format!("unknown table `{other}` (scenes, characters)"))),
    }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("character.list", "List Characters", "Script", list).pure(),
        CommandSpec::new("character.rename", "Rename Character", "Script", rename).params(r#"{"from": string, "to": string}"#),
        CommandSpec::new("tables.csv", "Table as CSV", "Script", table_csv).params(r#"{"table": "scenes" | "characters"}"#).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::text::body_lines;
    use crate::template::{new_script, paragraph};

    /// A paragraph where `**` toggles bold.
    fn styled(element: &str, text: &str) -> Paragraph {
        let mut p = paragraph(element, "");
        for (i, piece) in text.split("**").enumerate() {
            let props = wordsmith_doc::props::CharProps { bold: (i % 2 == 1).then_some(true), ..Default::default() };
            let at = p.len();
            let _ = p.insert_text(at, piece, &props);
        }
        p
    }

    fn session() -> Session {
        let mut doc = new_script();
        doc.body = [
            ("SceneHeading", "INT. A - DAY"),
            ("Action", "ANNA enters. Anna's coat drips. Annabel and Joanna wait. anna, again."),
            ("Character", "ANNA (V.O.)"),
            ("Dialogue", "I'm **Anna**."),
            ("Character", "BOB"),
            ("Dialogue", "Hi, Anna."),
            ("SceneHeading", "INT. B - DAY"),
            ("Character", "ANNA"),
            ("Dialogue", "Bye."),
        ]
        .iter()
        .map(|(e, t)| para_block(styled(e, t)))
        .collect();
        crate::session_for(doc)
    }

    #[test]
    fn the_characters_table() {
        let s = session();
        let rows = characters(&s.doc);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].name.as_str(), rows[0].speeches, rows[0].scenes, rows[0].first_scene), ("ANNA", 2, 2, Some(0)));
        assert_eq!((rows[1].name.as_str(), rows[1].speeches), ("BOB", 1));
    }

    #[test]
    fn rename_everywhere_in_one_step() {
        let mut s = session();
        let before = body_lines(&s);
        let r = s.run("character.rename", &json!({"from": "Anna", "to": "Zoe"})).unwrap();
        assert_eq!(r, json!({"cues": 2, "mentions": 5}));
        let after = body_lines(&s);
        assert_eq!(after[1].1, "ZOE enters. Zoe's coat drips. Annabel and Joanna wait. zoe, again.");
        assert_eq!(after[2].1, "ZOE (V.O.)", "the extension is kept");
        assert_eq!(after[3].1, "I'm Zoe.");
        assert_eq!(after[5].1, "Hi, Zoe.");
        assert_eq!(after[7].1, "ZOE");
        // Formatting at the name is kept.
        let p = s.doc.body[3].as_para().unwrap();
        assert_eq!(p.props_at(5).bold, Some(true), "the new name is bold");
        assert_eq!(p.props_at(4).bold, None, "the space before is not");
        assert!(s.run("character.rename", &json!({"from": "Nobody", "to": "X"})).is_err());
        assert!(s.run("character.rename", &json!({"from": "Bob", "to": "Bob (O.S.)"})).is_err());
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(body_lines(&s), before, "one undo step");
    }

    #[test]
    fn csv_is_escaped() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a, b"), "\"a, b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_field("two\nlines"), "\"two\nlines\"");
        let s = session();
        let scenes = scenes_csv(&s.doc);
        assert!(scenes.starts_with("Scene,Heading,Page,Length,Characters,Summary\r\n1,INT. A - DAY,1,2/8,\"ANNA, BOB\",\r\n"), "{scenes}");
        assert_eq!(characters_csv(&s.doc), "Character,Speeches,Scenes,First scene\r\nANNA,2,2,1\r\nBOB,1,1,1\r\n");
    }
}

/// `app_data` key of the Tables panel's own tabs (SPEC-2 10: add, rename and remove tabs).
pub const TABLE_TABS_KEY: &str = "wordsmith.tableTabs";

/// A tab the writer added to the Tables panel: a table shown with a filter and chosen columns.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TableTab {
    pub name: String,
    /// `scenes`, `characters` or `notes`.
    pub table: String,
    pub filter: String,
    /// Columns left out.
    pub hidden: Vec<String>,
}

/// The script's own Tables tabs.
pub fn table_tabs(doc: &Document) -> Vec<TableTab> {
    let tabs: Vec<TableTab> = crate::data::load(doc, TABLE_TABS_KEY);
    tabs.into_iter()
        .take(50)
        .filter(|t| matches!(t.table.as_str(), "scenes" | "characters" | "notes"))
        .map(|mut t| {
            t.name = crate::data::clean_line(&t.name, 40);
            t.filter = crate::data::clean_line(&t.filter, 200);
            t.hidden.truncate(50);
            t
        })
        .filter(|t| !t.name.is_empty())
        .collect()
}

fn set_table_tabs(s: &mut Session, v: &Value) -> CmdResult {
    let tabs: Vec<TableTab> =
        serde_json::from_value(v.get("tabs").cloned().unwrap_or(Value::Null)).map_err(|e| CmdError::Params(format!("`tabs`: {e}")))?;
    crate::data::store(&mut s.doc, TABLE_TABS_KEY, &tabs);
    Ok(json!(table_tabs(&s.doc)))
}

pub fn table_tab_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("tables.tabs", "Table Tabs", "View", |s, _| Ok(json!(table_tabs(&s.doc)))).pure(),
        CommandSpec::new("tables.setTabs", "Set Table Tabs", "View", set_table_tabs).params(r#"{"tabs": [{"name", "table", "filter", "hidden"}]}"#),
    ]
}

#[cfg(test)]
mod tab_tests {
    use super::*;

    #[test]
    fn table_tabs_round_trip_and_are_checked() {
        let mut s = crate::commands::new_session();
        let out = s.run("tables.setTabs", &json!({"tabs": [{"name": "Leads", "table": "characters", "filter": "ANNA"}, {"name": "Bad", "table": "props"}, {"name": "", "table": "notes"}]})).unwrap();
        assert_eq!(out.as_array().map(Vec::len), Some(1));
        assert_eq!(table_tabs(&s.doc)[0].filter, "ANNA");
        assert!(s.run("tables.setTabs", &json!({"tabs": 3})).is_err());
    }
}
