// SPDX-License-Identifier: MIT OR Apache-2.0
//! Page locking (SPEC "Production"), decision 0026.
//!
//! Locking marks the first character of every page (`CharProps::page_lock`, a lock id) and
//! records each lock's page label and lock level in `app_data` under [`LOCKS_KEY`]. Pagination
//! then starts a page at every mark and never pulls text back across one, so page numbers stay
//! put while the script is revised. Text that no longer fits flows onto added pages labelled with
//! letters after the locked page before them; locked pages whose text was all deleted leave one
//! placeholder page labelled with their combined labels.
//!
//! The page-label scheme: a locked page keeps its label ("12"). Pages added after it take its
//! label and letters in order: 12A, 12B … 12Z, 12ZA. When a lettered page is itself locked (a
//! second lock level), pages added after it take another letter: after 12A, 12AA, 12AB. A run of
//! deleted locked pages leaves one placeholder labelled first-last ("10-11", "7B-9"), or the one
//! label when a single page went. Pages before the first locked page are numbered 1, 2, 3.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::{Document, Pos, StoryRef};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session};

use crate::paginate::Piece;

/// `app_data` key of the locks.
pub const LOCKS_KEY: &str = "wordsmith.locks";
/// Most locked pages a script keeps.
pub const MAX_LOCKS: usize = 100_000;

/// One locked page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub id: u32,
    pub label: String,
    /// The lock level that made it (1 for the first lock).
    pub level: u32,
}

/// The locked pages, in page order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Locks {
    pub pages: Vec<Lock>,
}

impl Locks {
    pub fn of_doc(doc: &Document) -> Self {
        let mut l: Locks = doc.app_data.get(LOCKS_KEY).and_then(|v| serde_json::from_value((**v).clone()).ok()).unwrap_or_default();
        l.pages.truncate(MAX_LOCKS);
        for p in &mut l.pages {
            p.label = p.label.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(24).collect();
        }
        l
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(LOCKS_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }

    pub fn is_locked(&self) -> bool {
        !self.pages.is_empty()
    }

    pub fn level(&self) -> u32 {
        self.pages.iter().map(|p| p.level).max().unwrap_or(0)
    }
}

/// Letters for the `n`th page added after a locked one (1 → "A", 26 → "Z", 27 → "ZA").
pub fn letters(n: usize) -> String {
    let mut out = "Z".repeat(n.saturating_sub(1) / 26);
    out.push((b'A' + ((n.max(1) - 1) % 26) as u8) as char);
    out
}

/// Where each lock mark is: lock id → (block, byte offset).
pub fn lock_marks(doc: &Document) -> HashMap<u32, (usize, usize)> {
    let mut out = HashMap::new();
    for (block, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        for (range, c) in p.run_ranges() {
            if let Some(id) = c.page_lock {
                out.entry(id).or_insert((block, range.start));
            }
        }
    }
    out
}

/// Where a page starts: its first paragraph piece.
fn page_start(page: &[Piece]) -> Option<(usize, usize)> {
    page.iter().find_map(|p| match p {
        Piece::Para { block, start, .. } => Some((*block, *start)),
        _ => None,
    })
}

/// The pages with placeholders for deleted locked pages put in, and every page's label; `None`
/// when the script is not locked.
pub fn label_pages(doc: &Document, pages: Vec<Vec<Piece>>) -> (Vec<Vec<Piece>>, Option<Vec<String>>) {
    let locks = Locks::of_doc(doc);
    if !locks.is_locked() {
        return (pages, None);
    }
    let marks = lock_marks(doc);
    let at: HashMap<(usize, usize), u32> = marks.iter().map(|(id, pos)| (*pos, *id)).collect();
    let page_of_lock: HashMap<u32, usize> =
        pages.iter().enumerate().filter_map(|(i, pg)| page_start(pg).and_then(|s| at.get(&s)).map(|id| (*id, i))).collect();
    // Placeholders: runs of locked pages missing from the script, before the next one present.
    let mut placeholders: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut run: Vec<&str> = Vec::new();
    for lock in &locks.pages {
        match page_of_lock.get(&lock.id) {
            Some(&page) => {
                if !run.is_empty() {
                    placeholders.entry(page).or_default().push(combined(&run));
                    run.clear();
                }
            }
            None => run.push(&lock.label),
        }
    }
    if !run.is_empty() {
        placeholders.entry(pages.len()).or_default().push(combined(&run));
    }
    let label_of: HashMap<u32, &str> = locks.pages.iter().map(|l| (l.id, l.label.as_str())).collect();
    let mut out_pages = Vec::with_capacity(pages.len() + placeholders.len());
    let mut labels = Vec::with_capacity(pages.len() + placeholders.len());
    let mut base: Option<String> = None;
    let mut added = 0;
    let n = pages.len();
    for (i, page) in pages.into_iter().enumerate() {
        for label in placeholders.remove(&i).unwrap_or_default() {
            out_pages.push(vec![Piece::Blank]);
            labels.push(label);
        }
        let locked = page_start(&page).and_then(|s| at.get(&s)).and_then(|id| label_of.get(id));
        let label = match (locked, &base) {
            (Some(l), _) => {
                base = Some(l.to_string());
                added = 0;
                l.to_string()
            }
            (None, Some(b)) => {
                added += 1;
                format!("{b}{}", letters(added))
            }
            (None, None) => (i + 1).to_string(),
        };
        out_pages.push(page);
        labels.push(label);
    }
    for label in placeholders.remove(&n).unwrap_or_default() {
        out_pages.push(vec![Piece::Blank]);
        labels.push(label);
    }
    (out_pages, Some(labels))
}

fn combined(run: &[&str]) -> String {
    match run {
        [one] => one.to_string(),
        [first, .., last] => format!("{first}-{last}"),
        [] => String::new(),
    }
}

/// `lock.pages {revisionMode?}`: lock the pages as they are now (a further lock adds a level:
/// pages added since get locked with their lettered labels). With `revisionMode`, revision mode
/// is turned on too.
fn lock(s: &mut Session, v: &Value) -> CmdResult {
    let pag = crate::paginate::paginate(&s.doc);
    let mut locks = Locks::of_doc(&s.doc);
    let level = locks.level() + 1;
    let marks = lock_marks(&s.doc);
    let at: HashMap<(usize, usize), u32> = marks.iter().map(|(id, pos)| (*pos, *id)).collect();
    let mut next_id = locks.pages.iter().map(|l| l.id).chain(marks.keys().copied()).max().unwrap_or(0) + 1;
    let (pages, labels) = label_pages(&s.doc, pag.pages.clone());
    let labels = labels.unwrap_or_else(|| (1..=pages.len()).map(|n| n.to_string()).collect());
    let mut ordered: Vec<Lock> = Vec::new();
    let old: HashMap<u32, Lock> = locks.pages.iter().map(|l| (l.id, l.clone())).collect();
    for (page, label) in pages.iter().zip(&labels) {
        if page.iter().any(|p| matches!(p, Piece::Blank)) {
            // A placeholder keeps the locks it stands for.
            continue;
        }
        let Some((block, off)) = page_start(page) else { continue };
        if let Some(id) = at.get(&(block, off)) {
            if let Some(l) = old.get(id) {
                ordered.push(l.clone());
            }
            continue;
        }
        // Mark the page's first character; a page starting with an empty paragraph has none.
        let Some(len) =
            s.doc.body.get(block).and_then(|b| b.as_para()).and_then(|p| p.text.get(off..)).and_then(|t| t.chars().next()).map(char::len_utf8)
        else {
            continue;
        };
        let id = next_id;
        next_id += 1;
        let a = Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(block), off };
        let b = Pos { off: off + len, ..a.clone() };
        s.doc.format_range(&a, &b, &|c| c.page_lock = Some(id))?;
        ordered.push(Lock { id, label: label.clone(), level });
    }
    // Locks of deleted pages stay, so their placeholders keep showing.
    let kept: Vec<u32> = ordered.iter().map(|l| l.id).collect();
    for l in &locks.pages {
        if !kept.contains(&l.id) && !marks.contains_key(&l.id) {
            let at = locks.pages.iter().position(|x| x.id == l.id).unwrap_or(0).min(ordered.len());
            ordered.insert(at, l.clone());
        }
    }
    locks.pages = ordered;
    locks.store(&mut s.doc);
    if v.get("revisionMode").and_then(Value::as_bool) == Some(true) {
        let mut r = crate::revisions::Revisions::of_doc(&s.doc);
        r.mode = true;
        if r.current_index().is_none() {
            r.start_next();
        }
        r.store(&mut s.doc);
    }
    Ok(json!({"pages": locks.pages.len(), "level": level}))
}

/// Remove the lock marks of the locks `drop` picks.
fn remove_marks(doc: &mut Document, drop: &dyn Fn(u32) -> bool) -> Result<(), CmdError> {
    let last = doc.body.len().saturating_sub(1);
    let a = Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(0), off: 0 };
    let b =
        Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(last), off: doc.body.get(last).and_then(|b| b.as_para()).map_or(0, |p| p.len()) };
    doc.format_range(&a, &b, &|c| {
        if c.page_lock.is_some_and(drop) {
            c.page_lock = None;
        }
    })?;
    Ok(())
}

/// `lock.unlockLevel`: remove the newest lock level (its added pages flow freely again).
fn unlock_level(s: &mut Session, _: &Value) -> CmdResult {
    let mut locks = Locks::of_doc(&s.doc);
    let level = locks.level();
    if level == 0 {
        return Err(CmdError::Failed("the pages are not locked".into()));
    }
    let gone: Vec<u32> = locks.pages.iter().filter(|l| l.level == level).map(|l| l.id).collect();
    remove_marks(&mut s.doc, &|id| gone.contains(&id))?;
    locks.pages.retain(|l| l.level != level);
    locks.store(&mut s.doc);
    Ok(json!({"level": level - 1}))
}

/// `lock.unlockAll`: unlock every page; the script is numbered 1, 2, 3 again.
fn unlock_all(s: &mut Session, _: &Value) -> CmdResult {
    remove_marks(&mut s.doc, &|_| true)?;
    Locks::default().store(&mut s.doc);
    Ok(json!({"level": 0}))
}

/// `lock.status`: whether the pages are locked, the level, and each page's label.
fn status(s: &mut Session, _: &Value) -> CmdResult {
    let locks = Locks::of_doc(&s.doc);
    let pag = crate::paginate::paginate(&s.doc);
    Ok(json!({"locked": locks.is_locked(), "level": locks.level(), "labels": pag.labels}))
}

/// The lock marks in `a..b`, for keeping them when that text is deleted.
pub fn marks_in(doc: &Document, a: &Pos, b: &Pos) -> Vec<u32> {
    if a.story != StoryRef::Body || a == b {
        return Vec::new();
    }
    let mut out = Vec::new();
    for block in a.path.last()..=b.path.last() {
        let Some(p) = doc.body.get(block).and_then(|x| x.as_para()) else { continue };
        let from = if block == a.path.last() { a.off } else { 0 };
        let to = if block == b.path.last() { b.off } else { p.len() };
        for (range, c) in p.run_ranges() {
            if let Some(id) = c.page_lock
                && range.start >= from
                && range.start < to
            {
                out.push(id);
            }
        }
    }
    out
}

/// After deleting text that held lock marks, the first of them goes on the character now at
/// `at` (or the next paragraph's first), so the locked page still starts where its text goes on.
pub fn keep_marks(doc: &mut Document, ids: &[u32], at: &Pos) {
    let Some(&id) = ids.first() else { return };
    let mut block = at.path.last();
    let mut off = at.off;
    while let Some(p) = doc.body.get(block).and_then(|b| b.as_para()) {
        if let Some(len) = p.text.get(off..).and_then(|t| t.chars().next()).map(char::len_utf8) {
            if p.props_at(off + len).page_lock.is_none() {
                let a = Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(block), off };
                let b = Pos { off: off + len, ..a.clone() };
                let _ = doc.format_range(&a, &b, &|c| c.page_lock = Some(id));
            }
            return;
        }
        block += 1;
        off = 0;
    }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("lock.pages", "Lock Pages", "Production", lock).params(r#"{"revisionMode"?: bool}"#),
        CommandSpec::new("lock.unlockLevel", "Unlock One Level", "Production", unlock_level),
        CommandSpec::new("lock.unlockAll", "Unlock All Pages", "Production", unlock_all),
        CommandSpec::new("lock.status", "Page Lock Status", "Production", status).pure(),
    ]
}

#[cfg(test)]
mod tests;
