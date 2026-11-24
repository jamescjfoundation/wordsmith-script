// SPDX-License-Identifier: MIT OR Apache-2.0
//! The reports of SPEC-2 13: scenes, locations, cast, one character, the filtered script, notes
//! and statistics (tags wait for breakdown tagging, decision 0030). Each report is sections of
//! rows with column names, so the window shows it as a table and it saves as text or CSV the
//! same way (`report.run`, `report.text`, `report.csv`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use jcj_doc::Document;
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde_json::{Value, json};

use crate::element::{ElementKind, Elements};
use crate::paginate::output::speaker;
use crate::paginate::{Piece, paginate};
use crate::scenes::{scene_at, scenes};

/// `app_data` key of the words the statistics report flags.
pub const FLAGGED_KEY: &str = "wordsmith.flaggedWords";
/// Words flagged by default: strong language a writer may want to count. Editable per script.
pub const DEFAULT_FLAGGED: [&str; 9] = ["damn", "hell", "crap", "shit", "fuck", "bastard", "bitch", "ass", "goddamn"];
/// A speech this long or longer (words) counts as a monologue.
pub const MONOLOGUE_WORDS: usize = 100;

/// One report: a title, sections of rows, and closing lines.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub title: String,
    pub sections: Vec<Section>,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    pub heading: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Section {
    fn new(heading: &str, columns: &[&str]) -> Self {
        Section { heading: heading.into(), columns: columns.iter().map(|c| (*c).to_string()).collect(), rows: Vec::new() }
    }
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({
            "title": self.title,
            "sections": self.sections.iter().map(|s| json!({"heading": s.heading, "columns": s.columns, "rows": s.rows})).collect::<Vec<_>>(),
            "lines": self.lines,
        })
    }

    /// Plain text with columns lined up.
    pub fn to_text(&self) -> String {
        let mut out = format!("{}\n{}\n", self.title, "=".repeat(self.title.chars().count()));
        for s in &self.sections {
            out.push('\n');
            if !s.heading.is_empty() {
                out.push_str(&format!("{}\n\n", s.heading));
            }
            let mut widths: Vec<usize> = s.columns.iter().map(|c| c.chars().count()).collect();
            for r in &s.rows {
                for (i, c) in r.iter().enumerate() {
                    if let Some(w) = widths.get_mut(i) {
                        *w = (*w).max(c.chars().count().min(60));
                    }
                }
            }
            let line = |cells: &[String]| {
                let mut l = String::new();
                for (i, c) in cells.iter().enumerate() {
                    let w = widths.get(i).copied().unwrap_or(0);
                    let c: String = c.chars().take(60).collect();
                    l.push_str(&c);
                    if i + 1 < cells.len() {
                        l.push_str(&" ".repeat(w.saturating_sub(c.chars().count()) + 2));
                    }
                }
                l.trim_end().to_string()
            };
            out.push_str(&line(&s.columns));
            out.push('\n');
            for r in &s.rows {
                out.push_str(&line(r));
                out.push('\n');
            }
        }
        if !self.lines.is_empty() {
            out.push('\n');
            for l in &self.lines {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    }

    /// CSV: each section's columns and rows, sections apart by a blank line.
    pub fn to_csv(&self) -> String {
        let cell = |c: &str| if c.contains([',', '"', '\n']) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.to_string() };
        let mut out = String::new();
        for (n, s) in self.sections.iter().enumerate() {
            if n > 0 {
                out.push('\n');
            }
            out.push_str(&s.columns.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
            out.push('\n');
            for r in &s.rows {
                out.push_str(&r.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
                out.push('\n');
            }
        }
        out
    }
}

/// Each body block's page label (its first page).
fn pages(doc: &Document) -> (HashMap<usize, String>, usize) {
    let pag = paginate(doc);
    let mut out = HashMap::new();
    for (n, page) in pag.pages.iter().enumerate() {
        let label = pag.labels.as_ref().and_then(|l| l.get(n).cloned()).unwrap_or_else(|| (n + 1).to_string());
        for piece in page {
            if let Piece::Para { block, .. } = piece {
                out.entry(*block).or_insert_with(|| label.clone());
            }
        }
    }
    (out, pag.pages.len())
}

fn words(text: &str) -> usize {
    text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

/// Words of dialogue by speaker (capitals).
pub fn words_by_speaker(doc: &Document) -> BTreeMap<String, usize> {
    let elements = Elements::of_doc(doc);
    let mut out = BTreeMap::new();
    let mut who: Option<String> = None;
    for b in &doc.body {
        let Some(p) = b.as_para() else { continue };
        match elements.of_para(p).kind {
            ElementKind::Character => who = Some(speaker(&p.text).trim().to_uppercase()),
            ElementKind::Dialogue => {
                if let Some(w) = &who {
                    *out.entry(w.clone()).or_insert(0) += words(&p.text);
                }
            }
            ElementKind::Parenthetical => {}
            _ => who = None,
        }
    }
    out
}

/// A heading split into intro, location and time of day by the script's separators.
pub fn heading_parts(doc: &Document, heading: &str) -> (String, String, String) {
    let settings = crate::scene_heading::SceneHeadingSettings::of_doc(doc);
    let h = heading.trim();
    let sep = settings.intro_separator.trim();
    let (intro, rest) = settings
        .intros
        .iter()
        .find_map(|i| {
            let head = format!("{}{sep}", i.trim_end_matches('.'));
            h.get(..head.len()).filter(|x| x.eq_ignore_ascii_case(&head)).map(|_| (i.clone(), h.get(head.len()..).unwrap_or("").trim().to_string()))
        })
        .unwrap_or_default();
    let rest = if intro.is_empty() { h.to_string() } else { rest };
    let tsep = settings.time_separator.trim();
    match rest.rfind(&format!(" {tsep} ")) {
        Some(i) => (intro, rest[..i].trim().to_string(), rest[i + tsep.len() + 2..].trim().to_string()),
        None => (intro, rest.trim().to_string(), String::new()),
    }
}

fn scene_number(numbers: &[String], i: usize) -> String {
    numbers.get(i).cloned().unwrap_or_else(|| (i + 1).to_string())
}

/// The scene report.
pub fn scene_report(doc: &Document, sort: &str) -> Report {
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let pag = paginate(doc);
    let eighths = crate::reports::scene_eighths(doc, &pag);
    let (page_of, last_page) = pages(doc);
    let mut order: Vec<usize> = (0..all.len()).collect();
    let len = |i: &usize| eighths.get(*i).copied().unwrap_or(0);
    match sort {
        "alphabetical" => order.sort_by_key(|i| all.get(*i).map(|s| s.heading.to_uppercase()).unwrap_or_default()),
        "shortest" => order.sort_by_key(len),
        "longest" => order.sort_by_key(|i| std::cmp::Reverse(len(i))),
        // Production order: by location, then intro, then time of day, so a place's scenes sit
        // together as a schedule groups them.
        "production" => order.sort_by_key(|i| {
            let (intro, loc, time) = all.get(*i).map(|s| heading_parts(doc, &s.heading)).unwrap_or_default();
            (loc, intro, time, *i)
        }),
        _ => {}
    }
    let mut sec = Section::new("", &["Scene", "Heading", "Title", "Page", "Length"]);
    for i in &order {
        let Some(sc) = all.get(*i) else { continue };
        let title = crate::scene_data::of_scene(doc, sc).title;
        sec.rows.push(vec![
            scene_number(&numbers, *i),
            sc.heading.clone(),
            title,
            page_of.get(&sc.start).cloned().unwrap_or_default(),
            crate::reports::format_eighths(len(i)),
        ]);
    }
    let mut lines = Vec::new();
    if let (Some(long), Some(short)) = ((0..all.len()).max_by_key(len), (0..all.len()).min_by_key(len)) {
        lines.push(format!("Longest scene: {} ({})", scene_number(&numbers, long), crate::reports::format_eighths(len(&long))));
        lines.push(format!("Shortest scene: {} ({})", scene_number(&numbers, short), crate::reports::format_eighths(len(&short))));
    }
    lines.push(format!("Last page: {last_page}"));
    Report { title: "Scene Report".into(), sections: vec![sec], lines }
}

/// The location report.
pub fn location_report(doc: &Document, sort: &str) -> Report {
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    struct Loc {
        first: usize,
        intros: BTreeSet<String>,
        times: BTreeSet<String>,
        scenes: Vec<String>,
    }
    let mut locs: BTreeMap<String, Loc> = BTreeMap::new();
    let mut blanks: Vec<String> = Vec::new();
    for sc in &all {
        let (intro, loc, time) = heading_parts(doc, &sc.heading);
        let key = loc.to_uppercase();
        if key.is_empty() || time.is_empty() || intro.is_empty() {
            let missing: Vec<&str> = [(intro.is_empty(), "intro"), (key.is_empty(), "location"), (time.is_empty(), "time of day")]
                .iter()
                .filter(|x| x.0)
                .map(|x| x.1)
                .collect();
            blanks.push(format!("Scene {} has no {}: {}", scene_number(&numbers, sc.index), missing.join(" or "), sc.heading));
        }
        let e = locs.entry(key).or_insert_with(|| Loc { first: sc.index, intros: BTreeSet::new(), times: BTreeSet::new(), scenes: Vec::new() });
        if !intro.is_empty() {
            e.intros.insert(intro.to_uppercase());
        }
        if !time.is_empty() {
            e.times.insert(time.to_uppercase());
        }
        e.scenes.push(scene_number(&numbers, sc.index));
    }
    let mut rows: Vec<(String, Loc)> = locs.into_iter().collect();
    match sort {
        "alphabetical" => {}
        "most" => rows.sort_by_key(|(_, l)| std::cmp::Reverse(l.scenes.len())),
        "least" => rows.sort_by_key(|(_, l)| l.scenes.len()),
        _ => rows.sort_by_key(|(_, l)| l.first),
    }
    let mut sec = Section::new("", &["Location", "Scenes", "Intro", "Time of day", "Scene numbers"]);
    for (name, l) in rows {
        sec.rows.push(vec![
            if name.is_empty() { "(none)".into() } else { name },
            l.scenes.len().to_string(),
            l.intros.into_iter().collect::<Vec<_>>().join(", "),
            l.times.into_iter().collect::<Vec<_>>().join(", "),
            l.scenes.join(", "),
        ]);
    }
    Report { title: "Location Report".into(), sections: vec![sec], lines: blanks }
}

/// The cast report.
pub fn cast_report(doc: &Document, sort: &str) -> Report {
    let elements = Elements::of_doc(doc);
    let all = scenes(doc);
    let rows = crate::commands::character::characters(doc);
    let words = words_by_speaker(doc);
    // Non-speaking appearances, from the scenes' own data.
    let mut silent: BTreeMap<String, usize> = BTreeMap::new();
    for sc in &all {
        let (speaking, shown) = crate::scene_data::cast(doc, &elements, sc);
        for n in shown.iter().filter(|n| !speaking.contains(n)) {
            *silent.entry(n.clone()).or_insert(0) += 1;
        }
    }
    let mut people: Vec<(String, usize, usize, usize, usize)> = rows
        .iter()
        .map(|r| (r.name.clone(), r.speeches, words.get(&r.name).copied().unwrap_or(0), r.scenes, silent.get(&r.name).copied().unwrap_or(0)))
        .collect();
    for (n, k) in &silent {
        if !people.iter().any(|p| &p.0 == n) {
            people.push((n.clone(), 0, 0, 0, *k));
        }
    }
    match sort {
        "alphabetical" => people.sort_by(|a, b| a.0.cmp(&b.0)),
        "scenes" => people.sort_by_key(|p| std::cmp::Reverse(p.3 + p.4)),
        "speaking" => people.sort_by_key(|p| std::cmp::Reverse(p.3)),
        _ => {}
    }
    let mut sec = Section::new("", &["Character", "Speeches", "Words", "Speaking scenes", "Non-speaking scenes", "Total scenes"]);
    for p in &people {
        sec.rows.push(vec![p.0.clone(), p.1.to_string(), p.2.to_string(), p.3.to_string(), p.4.to_string(), (p.3 + p.4).to_string()]);
    }
    let mut lines = Vec::new();
    let speakers: Vec<&(String, usize, usize, usize, usize)> = people.iter().filter(|p| p.1 > 0).collect();
    if let (Some(most), Some(least)) = (speakers.iter().max_by_key(|p| p.2), speakers.iter().min_by_key(|p| p.2)) {
        lines.push(format!("Most dialogue: {} ({} words)", most.0, most.2));
        lines.push(format!("Least dialogue: {} ({} words)", least.0, least.2));
    }
    Report { title: "Cast Report".into(), sections: vec![sec], lines }
}

/// The report on one character: scenes, partners, monologues, and optionally every speech.
pub fn character_report(doc: &Document, name: &str, with_dialogue: bool) -> Report {
    let name = name.trim().to_uppercase();
    let elements = Elements::of_doc(doc);
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let (page_of, _) = pages(doc);
    let mut in_scenes = Section::new("Scenes", &["Scene", "Heading", "Page", "Arc"]);
    let mut partners: BTreeMap<String, usize> = BTreeMap::new();
    for sc in &all {
        let (_, shown) = crate::scene_data::cast(doc, &elements, sc);
        if !shown.contains(&name) {
            continue;
        }
        let arc = crate::scene_data::of_scene(doc, sc).arcs.get(&name).cloned().unwrap_or_default();
        in_scenes.rows.push(vec![scene_number(&numbers, sc.index), sc.heading.clone(), page_of.get(&sc.start).cloned().unwrap_or_default(), arc]);
        for other in shown.iter().filter(|o| **o != name) {
            *partners.entry(other.clone()).or_insert(0) += 1;
        }
    }
    let mut partner_rows: Vec<(String, usize)> = partners.into_iter().collect();
    partner_rows.sort_by_key(|p| std::cmp::Reverse(p.1));
    let mut with = Section::new("Scene partners", &["Character", "Scenes together"]);
    with.rows = partner_rows.into_iter().map(|(n, k)| vec![n, k.to_string()]).collect();
    let mut monologues = Section::new(&format!("Monologues ({MONOLOGUE_WORDS} words or more)"), &["Page", "Scene", "Words", "Opening"]);
    let mut speeches = Section::new("Dialogue", &["Page", "Scene", "Speech"]);
    let mut i = 0;
    while i < doc.body.len() {
        let is_cue = doc
            .body
            .get(i)
            .and_then(|b| b.as_para())
            .filter(|p| elements.of_para(p).kind == ElementKind::Character && speaker(&p.text).trim().to_uppercase() == name);
        if is_cue.is_none() {
            i += 1;
            continue;
        }
        let mut text = String::new();
        let mut j = i + 1;
        while let Some(p) = doc
            .body
            .get(j)
            .and_then(|b| b.as_para())
            .filter(|p| matches!(elements.of_para(p).kind, ElementKind::Dialogue | ElementKind::Parenthetical))
        {
            if elements.of_para(p).kind == ElementKind::Dialogue {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&p.text);
            }
            j += 1;
        }
        let scene = scene_at(&all, i).map(|s| scene_number(&numbers, s.index)).unwrap_or_default();
        let page = page_of.get(&i).cloned().unwrap_or_default();
        if words(&text) >= MONOLOGUE_WORDS {
            monologues.rows.push(vec![page.clone(), scene.clone(), words(&text).to_string(), text.chars().take(60).collect()]);
        }
        if with_dialogue {
            speeches.rows.push(vec![page, scene, text]);
        }
        i = j;
    }
    let mut sections = vec![in_scenes, with, monologues];
    if with_dialogue {
        sections.push(speeches);
    }
    Report { title: format!("Character Report: {name}"), sections, lines: Vec::new() }
}

/// The script reduced to the given elements (ids).
pub fn filtered_report(doc: &Document, keep: &[String]) -> Report {
    let elements = Elements::of_doc(doc);
    let mut sec = Section::new("", &["Element", "Text"]);
    for b in &doc.body {
        let Some(p) = b.as_para() else { continue };
        let e = elements.of_para(p);
        if keep.is_empty() || keep.contains(&e.id) {
            sec.rows.push(vec![e.name.clone(), p.text.clone()]);
        }
    }
    Report { title: "Filtered Script".into(), sections: vec![sec], lines: Vec::new() }
}

/// The filtered script as a script: only those elements' paragraphs, laid out as usual.
pub fn filtered_script(doc: &Document, keep: &[String]) -> Document {
    let elements = Elements::of_doc(doc);
    let mut out = doc.clone();
    out.body.retain(|b| b.as_para().is_some_and(|p| keep.contains(&elements.of_para(p).id)));
    out.ensure_nonempty();
    out
}

/// The notes report.
pub fn notes_report(doc: &Document, sort: &str, types: &[String]) -> Report {
    let notes = crate::notes::Notes::of_doc(doc);
    let anchors = crate::notes::anchors(doc);
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let (page_of, _) = pages(doc);
    let mut rows: Vec<(usize, u64, String, Vec<String>)> = Vec::new();
    for (id, n) in &notes.notes {
        if !types.is_empty() && !types.iter().any(|t| t.eq_ignore_ascii_case(&n.kind)) {
            continue;
        }
        let block = anchors.get(id).map(|a| a.start.path.0.first().map_or(0, |b| *b as usize));
        let scene = block.and_then(|b| scene_at(&all, b)).map(|s| scene_number(&numbers, s.index)).unwrap_or_default();
        let page = block.and_then(|b| page_of.get(&b).cloned()).unwrap_or_default();
        rows.push((
            block.unwrap_or(usize::MAX),
            n.modified.max(n.created),
            n.kind.clone(),
            vec![n.title.clone(), n.kind.clone(), n.text.clone(), page, scene, n.author.clone(), date(n.created), date(n.modified)],
        ));
    }
    match sort {
        "date" => rows.sort_by_key(|r| std::cmp::Reverse(r.1)),
        "type" => rows.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0))),
        _ => rows.sort_by_key(|r| r.0),
    }
    let mut sec = Section::new("", &["Title", "Type", "Note", "Page", "Scene", "Writer", "Created", "Modified"]);
    sec.rows = rows.into_iter().map(|r| r.3).collect();
    Report { title: "Notes Report".into(), sections: vec![sec], lines: Vec::new() }
}

/// A Unix time as `YYYY-MM-DD` (UTC), or empty for none.
pub fn date(secs: u64) -> String {
    if secs == 0 {
        return String::new();
    }
    crate::dates::ymd(secs)
}

/// The flagged words of a script.
pub fn flagged_words(doc: &Document) -> Vec<String> {
    match doc.ext.get(FLAGGED_KEY) {
        Some(v) => v
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).take(1_000).collect())
            .unwrap_or_default(),
        None => DEFAULT_FLAGGED.iter().map(|w| (*w).to_string()).collect(),
    }
}

/// The statistics report.
pub fn statistics_report(doc: &Document, sections: &[String]) -> Report {
    let want = |s: &str| sections.is_empty() || sections.iter().any(|x| x == s);
    let elements = Elements::of_doc(doc);
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let (page_of, _) = pages(doc);
    let paras: Vec<(&jcj_doc::Paragraph, usize)> = doc.body.iter().enumerate().filter_map(|(i, b)| b.as_para().map(|p| (p, i))).collect();
    let total_words: usize = paras.iter().map(|(p, _)| words(&p.text)).sum();
    let mut out = Vec::new();
    let mut lines = vec![format!("{total_words} words in {} paragraphs, {} scenes.", paras.len(), all.len())];
    if want("elements") {
        let mut by: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (p, _) in &paras {
            let e = by.entry(elements.of_para(p).name.clone()).or_default();
            e.0 += 1;
            e.1 += words(&p.text);
        }
        let mut sec = Section::new("Elements", &["Element", "Paragraphs", "Share of paragraphs", "Words", "Share of words"]);
        for (name, (n, w)) in by {
            sec.rows.push(vec![name, n.to_string(), pct(n, paras.len()), w.to_string(), pct(w, total_words)]);
        }
        out.push(sec);
    }
    if want("characters") {
        let rows = crate::commands::character::characters(doc);
        let words = words_by_speaker(doc);
        let speeches: usize = rows.iter().map(|r| r.speeches).sum();
        let spoken: usize = words.values().sum();
        let mut sec = Section::new("Characters", &["Character", "Speeches", "Share of speeches", "Words", "Share of dialogue"]);
        for r in rows {
            let w = words.get(&r.name).copied().unwrap_or(0);
            sec.rows.push(vec![r.name.clone(), r.speeches.to_string(), pct(r.speeches, speeches), w.to_string(), pct(w, spoken)]);
        }
        out.push(sec);
    }
    if want("scenes") {
        let mut sec = Section::new("Scenes", &["Scene", "Heading", "Words", "Other scenes here"]);
        let locs: Vec<String> = all.iter().map(|s| heading_parts(doc, &s.heading).1.to_uppercase()).collect();
        for sc in &all {
            let w: usize = (sc.start..sc.end).filter_map(|i| doc.body.get(i).and_then(|b| b.as_para())).map(|p| words(&p.text)).sum();
            let here = locs.get(sc.index).cloned().unwrap_or_default();
            let others: Vec<String> = all
                .iter()
                .filter(|o| o.index != sc.index && !here.is_empty() && locs.get(o.index) == Some(&here))
                .map(|o| scene_number(&numbers, o.index))
                .collect();
            sec.rows.push(vec![scene_number(&numbers, sc.index), sc.heading.clone(), w.to_string(), others.join(", ")]);
        }
        out.push(sec);
    }
    if want("language") {
        let flagged = flagged_words(doc);
        let mut sec = Section::new("Flagged language", &["Word", "Times", "Pages", "Speakers"]);
        let mut found: BTreeMap<String, (usize, BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
        let mut who: Option<String> = None;
        for (p, i) in &paras {
            let kind = elements.of_para(p).kind;
            match kind {
                ElementKind::Character => who = Some(speaker(&p.text).trim().to_uppercase()),
                ElementKind::Dialogue | ElementKind::Parenthetical => {}
                _ => who = None,
            }
            for w in p.text.split(|c: char| !c.is_alphanumeric() && c != '\'') {
                let lw = w.to_lowercase();
                if flagged.contains(&lw) {
                    let e = found.entry(lw).or_default();
                    e.0 += 1;
                    if let Some(pg) = page_of.get(i) {
                        e.1.insert(pg.clone());
                    }
                    if kind == ElementKind::Dialogue
                        && let Some(s) = &who
                    {
                        e.2.insert(s.clone());
                    }
                }
            }
        }
        for (w, (n, pgs, who)) in found {
            sec.rows.push(vec![w, n.to_string(), pgs.into_iter().collect::<Vec<_>>().join(", "), who.into_iter().collect::<Vec<_>>().join(", ")]);
        }
        if sec.rows.is_empty() {
            lines.push("No flagged words.".into());
        }
        out.push(sec);
    }
    Report { title: "Statistics".into(), sections: out, lines }
}

fn pct(n: usize, total: usize) -> String {
    if total == 0 { "0%".into() } else { format!("{:.1}%", n as f64 * 100.0 / total as f64) }
}

/// The report `v` asks for: `{kind, sort?, name?, elements?, types?, sections?, dialogue?}`.
pub fn report(doc: &Document, v: &Value) -> Result<Report, CmdError> {
    let kind = p::req_str(v, "kind")?;
    let sort = p::str(v, "sort").unwrap_or("script");
    let list = |key: &str| -> Vec<String> {
        v.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).take(200).map(str::to_string).collect()).unwrap_or_default()
    };
    Ok(match kind {
        "scenes" => scene_report(doc, sort),
        "locations" => location_report(doc, sort),
        "cast" => cast_report(doc, sort),
        "character" => character_report(doc, p::req_str(v, "name")?, p::bool(v, "dialogue").unwrap_or(false)),
        "filtered" => filtered_report(doc, &list("elements")),
        "notes" => notes_report(doc, sort, &list("types")),
        "statistics" => statistics_report(doc, &list("sections")),
        "representation" => {
            let r = crate::cast::representation(doc, p::req_str(v, "a")?, p::str(v, "b").unwrap_or(p::req_str(v, "a")?));
            let by: Vec<String> =
                r.get("by").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
            let mut cols: Vec<String> = by.clone();
            cols.extend(["Characters", "Share", "Words", "Share of dialogue", "Scenes", "Share of scenes"].map(str::to_string));
            let rows = r
                .get("groups")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|g| {
                    let mut row: Vec<String> = by.iter().map(|b| g[b.as_str()].as_str().unwrap_or("").to_string()).collect();
                    row.extend([
                        g["characters"].to_string(),
                        format!("{}%", g["charactersPct"]),
                        g["words"].to_string(),
                        format!("{}%", g["dialoguePct"]),
                        g["scenes"].to_string(),
                        format!("{}%", g["scenesPct"]),
                    ]);
                    row
                })
                .collect();
            Report { title: "Representation".into(), sections: vec![Section { heading: String::new(), columns: cols, rows }], lines: Vec::new() }
        }
        other => {
            return Err(CmdError::Params(format!(
                "no report `{other}` (scenes, locations, cast, character, filtered, notes, statistics, representation)"
            )));
        }
    })
}

fn run(s: &mut Session, v: &Value) -> CmdResult {
    Ok(report(&s.doc, v)?.to_json())
}

fn text(s: &mut Session, v: &Value) -> CmdResult {
    Ok(json!(report(&s.doc, v)?.to_text()))
}

fn csv(s: &mut Session, v: &Value) -> CmdResult {
    Ok(json!(report(&s.doc, v)?.to_csv()))
}

/// `report.setFlaggedWords {words}`.
fn set_flagged(s: &mut Session, v: &Value) -> CmdResult {
    let list = v.get("words").and_then(Value::as_array).ok_or_else(|| CmdError::Params("`words` is a list".into()))?;
    let words: Vec<String> =
        list.iter().filter_map(Value::as_str).map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty() && w.len() <= 40).take(1_000).collect();
    crate::data::store(&mut s.doc, FLAGGED_KEY, &words);
    Ok(json!(words))
}

pub fn specs() -> Vec<CommandSpec> {
    const PARAMS: &str = r#"{"kind": "scenes"|"locations"|"cast"|"character"|"filtered"|"notes"|"statistics"|"representation", "sort"?, "name"?, "elements"?, "types"?, "sections"?, "dialogue"?, "a"?, "b"?}"#;
    vec![
        CommandSpec::new("report.run", "Report", "Reports", run).params(PARAMS).pure(),
        CommandSpec::new("report.text", "Report as Text", "Reports", text).params(PARAMS).pure(),
        CommandSpec::new("report.csv", "Report as CSV", "Reports", csv).params(PARAMS).pure(),
        CommandSpec::new("report.flaggedWords", "Flagged Words", "Reports", |s, _| Ok(json!(flagged_words(&s.doc)))).pure(),
        CommandSpec::new("report.setFlaggedWords", "Set Flagged Words", "Reports", set_flagged).params(r#"{"words": [string]}"#),
    ]
}

#[cfg(test)]
mod tests;
