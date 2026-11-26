// SPDX-License-Identifier: MIT OR Apache-2.0
//! Autocomplete lists (SPEC "Typing behavior"; decision 0015): Characters, Character
//! Extensions, Scene Intros, Locations, Times of day and Transitions. The lists fill themselves
//! from the script, can be edited and rebuilt, and each can be switched off. Suggestions are only
//! ever offered: nothing is inserted until the writer accepts one.
//!
//! Lists live in `app_data` under [`AUTOCOMPLETE_KEY`] (undoable, per script), except Scene
//! Intros, which are the scene-heading settings' intros so Tab's separators and the list always
//! agree.

use std::collections::BTreeMap;

use jcj_doc::{Document, Paragraph, Pos};
use serde::{Deserialize, Serialize};

use crate::element::{ElementKind, Elements};
use crate::scene_heading::SceneHeadingSettings;

/// `app_data` key of the autocomplete lists.
pub const AUTOCOMPLETE_KEY: &str = "wordsmith.autocomplete";
/// Most entries a list keeps, and the longest entry, in bytes. Bounds hostile files.
pub const MAX_ENTRIES: usize = 5_000;
pub const MAX_ENTRY_LEN: usize = 100;
/// Most suggestions offered at once.
pub const MAX_SUGGESTIONS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    Characters,
    Extensions,
    Intros,
    Locations,
    Times,
    Transitions,
}

impl ListKind {
    pub const ALL: [ListKind; 6] =
        [ListKind::Characters, ListKind::Extensions, ListKind::Intros, ListKind::Locations, ListKind::Times, ListKind::Transitions];

    pub fn name(self) -> &'static str {
        match self {
            ListKind::Characters => "Characters",
            ListKind::Extensions => "Character Extensions",
            ListKind::Intros => "Scene Intros",
            ListKind::Locations => "Locations",
            ListKind::Times => "Times of Day",
            ListKind::Transitions => "Transitions",
        }
    }

    pub fn parse(s: &str) -> Option<ListKind> {
        Self::ALL.into_iter().find(|k| serde_json::to_value(k).ok().and_then(|v| v.as_str().map(|x| x == s)).unwrap_or(false))
    }
}

/// How a list's suggestions are ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    #[default]
    Alphabetical,
    /// The order the writer arranged.
    Custom,
    /// Characters with the most speeches first (other lists: alphabetical).
    Speeches,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct List {
    pub enabled: bool,
    pub order: Order,
    pub entries: Vec<String>,
}

impl Default for List {
    fn default() -> Self {
        List { enabled: true, order: Order::Alphabetical, entries: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Autocomplete {
    pub lists: BTreeMap<ListKind, List>,
    /// Tab on a Character with a name starts an extension, " (", instead of doing nothing.
    pub tab_inserts_extension: bool,
    /// On an empty Character in a two-person exchange, offer the other speaker.
    pub guess_next_speaker: bool,
}

fn list_of(entries: &[&str]) -> List {
    List { entries: entries.iter().map(|s| s.to_string()).collect(), ..List::default() }
}

impl Default for Autocomplete {
    fn default() -> Self {
        let mut lists = BTreeMap::new();
        lists.insert(ListKind::Characters, List::default());
        lists.insert(ListKind::Extensions, list_of(&["V.O.", "O.S.", "O.C.", "CONT'D"]));
        // Intros' entries are the scene-heading settings'; this keeps only the switch and order.
        lists.insert(ListKind::Intros, List::default());
        lists.insert(ListKind::Locations, List::default());
        lists.insert(ListKind::Times, list_of(&["DAY", "NIGHT", "MORNING", "AFTERNOON", "EVENING", "LATER", "CONTINUOUS", "MOMENTS LATER"]));
        lists.insert(
            ListKind::Transitions,
            list_of(&["CUT TO:", "DISSOLVE TO:", "FADE IN:", "FADE OUT.", "FADE TO BLACK.", "MATCH CUT TO:", "SMASH CUT TO:"]),
        );
        Autocomplete { lists, tab_inserts_extension: true, guess_next_speaker: true }
    }
}

/// A term cleaned for a list: trimmed, one line, within the length limit.
fn clean_term(t: &str) -> Option<String> {
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty() && t.len() <= MAX_ENTRY_LEN).then_some(t)
}

impl Autocomplete {
    /// The lists a document holds, repaired; the defaults when it holds none or they do not parse.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.ext.get(AUTOCOMPLETE_KEY) else { return Self::default() };
        let d = Self::default();
        // Field by field, so one bad value loses only itself.
        let mut a = d.clone();
        if let Some(lists) = v.get("lists").and_then(serde_json::Value::as_object) {
            for (name, list) in lists {
                if let Some(kind) = ListKind::parse(name)
                    && let Ok(list) = serde_json::from_value::<List>(list.clone())
                {
                    a.lists.insert(kind, list);
                }
            }
        }
        if let Some(b) = v.get("tabInsertsExtension").and_then(serde_json::Value::as_bool) {
            a.tab_inserts_extension = b;
        }
        if let Some(b) = v.get("guessNextSpeaker").and_then(serde_json::Value::as_bool) {
            a.guess_next_speaker = b;
        }
        for kind in ListKind::ALL {
            let list = a.lists.entry(kind).or_insert_with(|| d.lists.get(&kind).cloned().unwrap_or_default());
            let mut seen = std::collections::HashSet::new();
            list.entries = list.entries.iter().filter_map(|e| clean_term(e)).filter(|e| seen.insert(e.to_uppercase())).take(MAX_ENTRIES).collect();
        }
        a
    }

    pub fn store(&self, doc: &mut Document) {
        let _ = doc.ext.set(AUTOCOMPLETE_KEY, serde_json::to_value(self).unwrap_or_default());
    }

    pub fn list(&self, kind: ListKind) -> List {
        self.lists.get(&kind).cloned().unwrap_or_default()
    }

    /// A list's entries as the document has them (Scene Intros from the scene-heading settings).
    pub fn entries(&self, doc: &Document, kind: ListKind) -> Vec<String> {
        match kind {
            ListKind::Intros => SceneHeadingSettings::of_doc(doc).intros,
            _ => self.list(kind).entries,
        }
    }

    /// Add `term` to a list unless it is there already (ignoring case). Returns whether it was added.
    pub fn add(&mut self, doc: &mut Document, kind: ListKind, term: &str) -> bool {
        let Some(term) = clean_term(term) else { return false };
        if kind == ListKind::Intros {
            let mut s = SceneHeadingSettings::of_doc(doc);
            if s.intros.iter().any(|e| e.eq_ignore_ascii_case(&term)) || s.intros.len() >= crate::scene_heading::MAX_INTROS {
                return false;
            }
            s.intros.push(term.to_uppercase());
            s.store(doc);
            return true;
        }
        let list = self.lists.entry(kind).or_default();
        if list.entries.len() >= MAX_ENTRIES || list.entries.iter().any(|e| e.eq_ignore_ascii_case(&term)) {
            return false;
        }
        list.entries.push(term);
        true
    }

    pub fn remove(&mut self, doc: &mut Document, kind: ListKind, term: &str) -> bool {
        if kind == ListKind::Intros {
            let mut s = SceneHeadingSettings::of_doc(doc);
            let before = s.intros.len();
            s.intros.retain(|e| !e.eq_ignore_ascii_case(term.trim()));
            let removed = s.intros.len() != before;
            s.store(doc);
            return removed;
        }
        let list = self.lists.entry(kind).or_default();
        let before = list.entries.len();
        list.entries.retain(|e| !e.eq_ignore_ascii_case(term.trim()));
        list.entries.len() != before
    }
}

/// Where the caret is, for suggestions: the list and the typed prefix it would complete.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub list: ListKind,
    /// Byte offset in the paragraph where the prefix starts.
    pub start: usize,
    pub prefix: String,
}

/// The suggestion context at `pos`, if its paragraph and place call for a list.
pub fn context(doc: &Document, pos: &Pos) -> Option<Context> {
    let p = doc.para_at(pos)?;
    let elements = Elements::of_doc(doc);
    let off = pos.off.min(p.text.len());
    let before = p.text.get(..off)?;
    let ctx = |list, start: usize| {
        let start = start.min(off);
        let raw = p.text.get(start..off).unwrap_or("");
        let trimmed = raw.trim_start();
        Context { list, start: start + (raw.len() - trimmed.len()), prefix: trimmed.to_string() }
    };
    match elements.of_para(p).kind {
        ElementKind::Character => match before.rfind('(') {
            Some(open) if !before.get(open..).unwrap_or("").contains(')') => Some(ctx(ListKind::Extensions, open + 1)),
            Some(_) => None,
            None => Some(ctx(ListKind::Characters, 0)),
        },
        ElementKind::SceneHeading => {
            let s = SceneHeadingSettings::of_doc(doc);
            let time_sep = s.time_separator.trim_end();
            let intro_sep = s.intro_separator.trim_end();
            if let Some(i) = before.rfind(time_sep).filter(|_| !time_sep.is_empty()) {
                Some(ctx(ListKind::Times, i + time_sep.len()))
            } else if let Some(i) =
                before.find(intro_sep).filter(|_| !intro_sep.is_empty()).filter(|&i| s.intro_of(before.get(..i).unwrap_or("")).is_some())
            {
                Some(ctx(ListKind::Locations, i + intro_sep.len()))
            } else {
                Some(ctx(ListKind::Intros, 0))
            }
        }
        ElementKind::Transition => Some(ctx(ListKind::Transitions, 0)),
        _ => None,
    }
}

/// How many speeches each speaker has, by upper-case name.
fn speech_counts(doc: &Document) -> BTreeMap<String, usize> {
    let elements = Elements::of_doc(doc);
    let mut out = BTreeMap::new();
    for p in doc.body.iter().filter_map(|b| b.as_para()) {
        if elements.of_para(p).kind == ElementKind::Character {
            let who = crate::paginate::output::speaker(&p.text).to_uppercase();
            if !who.is_empty() {
                *out.entry(who).or_insert(0) += 1;
            }
        }
    }
    out
}

/// In a two-person exchange, the speaker who comes next at paragraph `block`: the one before
/// the latest speaker in this scene.
pub fn next_speaker(doc: &Document, block: usize) -> Option<String> {
    let elements = Elements::of_doc(doc);
    let mut speakers: Vec<String> = Vec::new();
    for p in doc.body.iter().take(block).rev().filter_map(|b| b.as_para()) {
        match elements.of_para(p).kind {
            ElementKind::SceneHeading => break,
            ElementKind::Character => {
                let who = crate::paginate::output::speaker(&p.text).to_string();
                if !who.is_empty() && speakers.last().is_none_or(|l| !l.eq_ignore_ascii_case(&who)) {
                    speakers.push(who);
                }
                if speakers.len() == 2 {
                    break;
                }
            }
            _ => {}
        }
    }
    (speakers.len() == 2).then(|| speakers.swap_remove(1))
}

/// Suggestions for the caret at `pos`: the context and up to [`MAX_SUGGESTIONS`] entries that
/// start with the typed prefix (ignoring case), in the list's order. An empty prefix offers only
/// the next-speaker guess. A switched-off list offers nothing.
pub fn suggest(doc: &Document, pos: &Pos) -> Option<(Context, Vec<String>)> {
    let ctx = context(doc, pos)?;
    let ac = Autocomplete::of_doc(doc);
    let list = ac.list(ctx.list);
    if !list.enabled {
        return None;
    }
    let prefix = ctx.prefix.to_uppercase();
    if prefix.is_empty() {
        if ctx.list == ListKind::Characters && ac.guess_next_speaker {
            let guess = next_speaker(doc, pos.path.last())?;
            return Some((ctx, vec![guess]));
        }
        return None;
    }
    let mut entries: Vec<String> =
        ac.entries(doc, ctx.list).into_iter().filter(|e| e.to_uppercase().starts_with(&prefix) && e.to_uppercase() != prefix).collect();
    match list.order {
        Order::Custom => {}
        Order::Alphabetical => entries.sort_by_key(|e| e.to_uppercase()),
        Order::Speeches if ctx.list == ListKind::Characters => {
            let counts = speech_counts(doc);
            entries.sort_by(|a, b| counts.get(&b.to_uppercase()).cmp(&counts.get(&a.to_uppercase())).then(a.to_uppercase().cmp(&b.to_uppercase())));
        }
        Order::Speeches => entries.sort_by_key(|e| e.to_uppercase()),
    }
    entries.truncate(MAX_SUGGESTIONS);
    (!entries.is_empty()).then_some((ctx, entries))
}

/// The terms a paragraph contributes to the lists.
pub fn terms_of(doc: &Document, p: &Paragraph) -> Vec<(ListKind, String)> {
    let elements = Elements::of_doc(doc);
    let mut out = Vec::new();
    match elements.of_para(p).kind {
        ElementKind::Character => {
            let who = crate::paginate::output::speaker(&p.text);
            if !who.is_empty() {
                out.push((ListKind::Characters, who.to_string()));
            }
            let mut rest = p.text.as_str();
            while let Some(open) = rest.find('(') {
                let after = rest.get(open + 1..).unwrap_or("");
                let Some(close) = after.find(')') else { break };
                if let Some(ext) = after.get(..close) {
                    out.push((ListKind::Extensions, ext.trim().to_string()));
                }
                rest = after.get(close + 1..).unwrap_or("");
            }
        }
        ElementKind::SceneHeading => {
            let s = SceneHeadingSettings::of_doc(doc);
            let intro_sep = s.intro_separator.trim_end();
            let time_sep = s.time_separator.trim_end();
            if let Some(i) = p.text.find(intro_sep).filter(|&i| s.intro_of(p.text.get(..i).unwrap_or("")).is_some()) {
                let rest = p.text.get(i + intro_sep.len()..).unwrap_or("");
                let (place, time) = match rest.rfind(time_sep).filter(|_| !time_sep.is_empty()) {
                    Some(j) => (rest.get(..j).unwrap_or(""), rest.get(j + time_sep.len()..).unwrap_or("")),
                    None => (rest, ""),
                };
                out.push((ListKind::Locations, place.trim().to_string()));
                out.push((ListKind::Times, time.trim().to_string()));
            }
        }
        ElementKind::Transition => out.push((ListKind::Transitions, p.text.trim().to_string())),
        _ => {}
    }
    out.retain(|(_, t)| !t.is_empty());
    out
}

/// Add a paragraph's terms to the lists (when the writer leaves it). Returns whether any was new.
pub fn learn(doc: &mut Document, p: &Paragraph) -> bool {
    let terms = terms_of(doc, p);
    if terms.is_empty() {
        return false;
    }
    let mut ac = Autocomplete::of_doc(doc);
    let mut added = false;
    for (kind, term) in terms {
        added |= ac.add(doc, kind, &term);
    }
    if added {
        ac.store(doc);
    }
    added
}

/// Rebuild the learned lists from the script (SPEC): terms no longer used are dropped, missing
/// ones added. Characters, Locations and Transitions are rebuilt; Extensions and Times keep
/// their entries and gain the script's; Scene Intros are kept as they are.
pub fn rebuild(doc: &mut Document) {
    let mut ac = Autocomplete::of_doc(doc);
    let mut found: BTreeMap<ListKind, Vec<String>> = BTreeMap::new();
    for p in doc.body.iter().filter_map(|b| b.as_para()) {
        for (kind, term) in terms_of(doc, p) {
            let v = found.entry(kind).or_default();
            if !v.iter().any(|e| e.eq_ignore_ascii_case(&term)) {
                v.push(term);
            }
        }
    }
    for kind in [ListKind::Characters, ListKind::Locations, ListKind::Transitions] {
        let list = ac.lists.entry(kind).or_default();
        let script = found.remove(&kind).unwrap_or_default();
        list.entries.retain(|e| script.iter().any(|s| s.eq_ignore_ascii_case(e)));
        for t in script {
            if !list.entries.iter().any(|e| e.eq_ignore_ascii_case(&t)) && list.entries.len() < MAX_ENTRIES {
                list.entries.push(t);
            }
        }
    }
    for (kind, terms) in found {
        if kind == ListKind::Intros {
            continue;
        }
        for t in terms {
            let list = ac.lists.entry(kind).or_default();
            if !list.entries.iter().any(|e| e.eq_ignore_ascii_case(&t)) && list.entries.len() < MAX_ENTRIES {
                list.entries.push(t);
            }
        }
    }
    ac.store(doc);
}

#[cfg(test)]
mod tests;
