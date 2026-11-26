// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spelling with the core's `jcj-proof` (decision 0014; core decision 0007 says which word lists
//! it uses). Screenplay rules on top (SPEC: "ignoring character names and scene
//! intros"): Character paragraphs are skipped; words in capitals are taken as correct (scene
//! headings, transitions, names on first appearance); speakers' names and the Characters list
//! are correct wherever they appear; and the script's own word list (`Ignore`, `Add to script`)
//! is honoured. Results are cached per paragraph revision, so after an edit only the edited
//! paragraphs are checked again.

use std::collections::{HashMap, HashSet};

use jcj_doc::{Document, Paragraph};

use crate::autocomplete::{Autocomplete, ListKind};
use crate::element::{ElementKind, Elements};

/// Most misspellings reported at once.
pub const MAX_REPORTED: usize = 5_000;

/// The script's own correct words (`Ignore`, `Add to script`): the core's document words
/// (`core.spelling`), which `spelling.ignore` adds to. Before 0.2 they were `wordsmith.spelling`.
pub use jcj_engine::commands::spelling::{DOCUMENT_WORDS, DocumentWords as ScriptWords};

/// A misspelled word: body block, byte range in its paragraph, and the word.
#[derive(Clone, Debug, PartialEq)]
pub struct Misspelling {
    pub block: usize,
    pub start: usize,
    pub end: usize,
    pub word: String,
}

/// Words that are correct in this script whatever the dictionary says (lower case).
fn script_correct(doc: &Document) -> HashSet<String> {
    let mut out: HashSet<String> = ScriptWords::of(doc).words.iter().map(|w| w.to_lowercase()).collect();
    let elements = Elements::of_doc(doc);
    // Every speaker's name, each word of it, and the Characters list.
    let names = doc
        .body
        .iter()
        .filter_map(|b| b.as_para())
        .filter(|p| elements.of_para(p).kind == ElementKind::Character)
        .map(|p| crate::paginate::output::speaker(&p.text).to_string())
        .chain(Autocomplete::of_doc(doc).entries(doc, ListKind::Characters));
    for name in names {
        for part in name.split(|c: char| !c.is_alphanumeric() && c != '\'') {
            if !part.is_empty() {
                out.insert(part.to_lowercase());
            }
        }
    }
    out
}

/// Each paragraph's raw dictionary misses, kept per paragraph revision and dictionary
/// generation.
#[derive(Default)]
pub struct SpellCache {
    issues: HashMap<(u64, u64), Vec<(usize, usize)>>,
}

const MAX_CACHED: usize = 200_000;

fn raw_issues(p: &Paragraph) -> Vec<(usize, usize)> {
    jcj_proof::check(&p.text, None, &jcj_proof::SpellingOptions::default(), false)
        .into_iter()
        .filter(|i| matches!(i.kind, jcj_proof::IssueKind::Spelling))
        .map(|i| (i.range.start, i.range.end))
        .collect()
}

/// The script's misspellings, in order (at most [`MAX_REPORTED`]).
pub fn check(doc: &Document, cache: &mut SpellCache) -> Vec<Misspelling> {
    let generation = jcj_proof::dictionary_generation();
    if cache.issues.len() > MAX_CACHED {
        cache.issues.clear();
    }
    let elements = Elements::of_doc(doc);
    let correct = script_correct(doc);
    let mut out = Vec::new();
    for (block, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        // Character names and their extensions are never checked.
        if matches!(elements.of_para(p).kind, ElementKind::Character) {
            continue;
        }
        let issues = cache.issues.entry((p.rev, generation)).or_insert_with(|| raw_issues(p));
        for &(start, end) in issues.iter() {
            let Some(word) = p.text.get(start..end) else { continue };
            if correct.contains(&word.to_lowercase()) {
                continue;
            }
            out.push(Misspelling { block, start, end, word: word.to_string() });
            if out.len() >= MAX_REPORTED {
                return out;
            }
        }
    }
    out
}

/// Suggestions for a word, best first.
pub fn suggest(word: &str, max: usize) -> Vec<String> {
    jcj_proof::suggestions(word, max.min(20), None)
}

#[cfg(test)]
mod tests {
    use jcj_doc::para_block;
    use serde_json::json;

    use super::*;
    use crate::template::{new_script, paragraph};

    fn script(lines: &[(&str, &str)]) -> Document {
        let mut doc = new_script();
        doc.body = lines.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
        doc
    }

    fn words(doc: &Document) -> Vec<String> {
        check(doc, &mut SpellCache::default()).into_iter().map(|m| m.word).collect()
    }

    #[test]
    fn misspellings_are_found_in_action_and_dialogue() {
        let doc = script(&[("Action", "The dog barkd at the moon."), ("Character", "ANNA"), ("Dialogue", "I beleive you.")]);
        assert_eq!(words(&doc), ["barkd", "beleive"]);
    }

    #[test]
    fn names_intros_and_capitals_are_not_checked() {
        let doc = script(&[
            ("SceneHeading", "INT. XQZTL'S KITCHN - NITE"),
            ("Character", "ZORBLAX (V.O.)"),
            ("Dialogue", "Zorblax is here, and Qwyneth too."),
            ("Character", "QWYNETH"),
            ("Action", "Zorblax waves. THWACKO!"),
        ]);
        assert!(words(&doc).is_empty(), "{:?}", words(&doc));
    }

    #[test]
    fn the_script_word_list_and_suggestions() {
        let mut doc = script(&[("Action", "A glorp appears.")]);
        assert_eq!(words(&doc), ["glorp"]);
        let mut w = ScriptWords::of(&doc);
        assert!(w.add("Glorp"));
        assert!(!w.add("glorp"), "once");
        w.store(&mut doc).unwrap();
        assert!(words(&doc).is_empty());
        assert!(suggest("beleive", 5).iter().any(|s| s == "believe"));
        let _ = doc.ext.set(DOCUMENT_WORDS.key(), json!({"words": 3}));
        assert_eq!(ScriptWords::of(&doc), ScriptWords::default(), "hostile data reads as empty");
    }

    #[test]
    fn the_cache_keeps_results_until_a_paragraph_changes() {
        let mut doc = script(&[("Action", "Wrongg word."), ("Action", "Fine words.")]);
        let mut cache = SpellCache::default();
        assert_eq!(check(&doc, &mut cache).len(), 1);
        let n = cache.issues.len();
        assert_eq!(check(&doc, &mut cache).len(), 1);
        assert_eq!(cache.issues.len(), n, "nothing new to check");
        doc.body[1] = para_block(paragraph("Action", "Anothr one."));
        assert_eq!(check(&doc, &mut cache).iter().map(|m| (m.block, m.word.as_str())).collect::<Vec<_>>(), [(0, "Wrongg"), (1, "Anothr")]);
    }
}
