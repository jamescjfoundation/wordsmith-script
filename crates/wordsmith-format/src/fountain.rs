// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and writing Fountain (https://fountain.io/syntax, version 1.1), the open plain-text
//! screenplay format. Decision 0018 records the choices.
//!
//! Recognised: title page (`Key: value`, indented continuation lines), scene headings (`INT`,
//! `EXT`, `EST`, `INT./EXT`, `INT/EXT`, `I/E`, or forced with `.`; `#n#` scene numbers),
//! characters (all capitals with a blank line before and text after, or forced with `@`, which
//! needs no dialogue after it; `^` dual dialogue is read as ordinary dialogue), parentheticals,
//! dialogue, transitions (capitals ending in `TO:`, or forced with `>`), centered text
//! (`>text<`, read as Action), action (forced with `!`; line breaks kept), lyrics (`~`, read as
//! Action), page breaks (`===`: the next paragraph starts a new page), sections (`#` to `###`,
//! Outline 1 to 3) and synopses (`=`, Summary), notes on their own (`[[ ]]`, Note). Notes inside
//! other text and the boneyard (`/* */`) are dropped. Emphasis (`***`, `**`, `*`, `_`) is kept
//! as bold, italic and underline.
//!
//! The input is hostile: its size and paragraph count are bounded, and anything unclear is read
//! as Action, as the spec says.

use std::collections::BTreeMap;

use wordsmith_doc::{Document, Paragraph, para_block};
use wordsmith_script::template::{new_script, paragraph};
use wordsmith_script::{ElementKind, Elements};

/// Largest Fountain text read, bytes.
pub const MAX_FOUNTAIN_BYTES: usize = 32 << 20;
/// Most paragraphs a script may have.
pub const MAX_PARAGRAPHS: usize = 500_000;

/// A Fountain script read into a document, with its title page fields (lower-case keys).
pub struct FountainScript {
    pub doc: Document,
    pub title_page: BTreeMap<String, String>,
    /// Scene numbers (`#12#`), by body paragraph index; numbering itself arrives in M6.
    pub scene_numbers: BTreeMap<usize, String>,
}

const SCENE_PREFIXES: [&str; 6] = ["INT./EXT", "INT/EXT", "I/E", "INT", "EXT", "EST"];

/// Remove the boneyard (`/* ... */`, across lines) and notes (`[[ ... ]]`, within a paragraph).
fn strip_hidden(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(rest.get(..start).unwrap_or(""));
        match rest.get(start + 2..).and_then(|r| r.find("*/")) {
            Some(end) => rest = rest.get(start + 2 + end + 2..).unwrap_or(""),
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    // Notes do not cross a blank line.
    let mut cleaned = String::with_capacity(out.len());
    for (i, block) in out.split("\n\n").enumerate() {
        if i > 0 {
            cleaned.push_str("\n\n");
        }
        // A paragraph that is only a note is kept: it becomes a Note element.
        let whole = block.trim();
        if whole.starts_with("[[") && whole.ends_with("]]") && whole.get(2..).is_some_and(|r| !r.contains("[[")) {
            cleaned.push_str(block);
            continue;
        }
        let mut b = block;
        while let Some(s) = b.find("[[") {
            match b.get(s + 2..).and_then(|r| r.find("]]")) {
                Some(e) => {
                    cleaned.push_str(b.get(..s).unwrap_or(""));
                    b = b.get(s + 2 + e + 2..).unwrap_or("");
                }
                None => break,
            }
        }
        cleaned.push_str(b);
    }
    cleaned
}

/// Text without emphasis markers and with `\` escapes resolved.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '*' | '_' => {}
            _ => out.push(c),
        }
    }
    out
}

/// A paragraph of element `id` with Fountain emphasis kept as formatting.
fn styled(id: &str, text: &str) -> Paragraph {
    crate::emphasis::decode(id, text, true)
}

pub(crate) fn is_upper(line: &str) -> bool {
    line.chars().any(char::is_alphabetic) && !line.chars().any(char::is_lowercase)
}

/// A scene heading's text and number, if `line` is one.
pub(crate) fn scene_heading(line: &str) -> Option<(String, Option<String>)> {
    let (body, number) = match (line.rfind('#'), line.trim_end().ends_with('#')) {
        (Some(_), true) => {
            let t = line.trim_end();
            let inner = t.get(..t.len() - 1).unwrap_or("");
            match inner.rfind('#') {
                Some(open) => (inner.get(..open).unwrap_or("").trim_end().to_string(), Some(inner.get(open + 1..).unwrap_or("").to_string())),
                None => (line.to_string(), None),
            }
        }
        _ => (line.to_string(), None),
    };
    let number = number.filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.'));
    if let Some(forced) = body.strip_prefix('.')
        && forced.chars().next().is_some_and(char::is_alphanumeric)
    {
        return Some((forced.to_string(), number));
    }
    let upper = body.to_uppercase();
    for p in SCENE_PREFIXES {
        if upper.starts_with(p) && upper.get(p.len()..).is_some_and(|rest| rest.starts_with('.') || rest.starts_with(' ')) {
            return Some((body, number));
        }
    }
    None
}

/// `===` was read as an empty paragraph that starts a page; the paragraph after it takes the
/// page break instead, so the script has no stray empty paragraph (one at the very end stays).
fn with_page_breaks(body: Vec<Paragraph>) -> Vec<Paragraph> {
    let mut out: Vec<Paragraph> = Vec::with_capacity(body.len());
    let mut pending: Option<Paragraph> = None;
    for mut p in body {
        if p.text.is_empty() && p.props.page_break_before == Some(true) {
            pending = Some(p);
            continue;
        }
        if pending.take().is_some() {
            p.props.page_break_before = Some(true);
        }
        out.push(p);
    }
    out.extend(pending);
    out
}

/// Read Fountain text into a script.
pub fn read(text: &str) -> Result<FountainScript, String> {
    if text.len() > MAX_FOUNTAIN_BYTES {
        return Err(format!("the file is larger than {} MB", MAX_FOUNTAIN_BYTES >> 20));
    }
    let text = text.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ");
    let text = strip_hidden(text.trim_start_matches('\u{feff}'));
    let lines: Vec<&str> = text.split('\n').collect();

    // Title page: key: value lines at the very top, up to the first blank line.
    let mut title_page = BTreeMap::new();
    let mut i = 0;
    let first_is_key = lines.first().is_some_and(|l| {
        let key = l.split(':').next().unwrap_or("");
        l.contains(':')
            && !key.trim().is_empty()
            && key.chars().all(|c| c.is_alphanumeric() || c == ' ')
            && !l.starts_with(' ')
            && scene_heading(l).is_none()
            && !l.trim_end().ends_with("TO:")
    });
    if first_is_key {
        let mut key = String::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            let l = lines[i];
            if (l.starts_with("   ") || l.starts_with('\t')) && !key.is_empty() {
                let v: &mut String = title_page.entry(key.clone()).or_default();
                if !v.is_empty() {
                    v.push('\n');
                }
                v.push_str(l.trim());
            } else if let Some((k, v)) = l.split_once(':') {
                key = k.trim().to_lowercase();
                title_page.insert(key.clone(), v.trim().to_string());
            }
            i += 1;
        }
    }

    let mut doc = new_script();
    let elements = Elements::of_doc(&doc);
    let id = |k: ElementKind| elements.id_of(k).to_string();
    let mut body: Vec<Paragraph> = Vec::new();
    let mut scene_numbers = BTreeMap::new();
    let blank = |j: usize| lines.get(j).is_none_or(|l| l.trim().is_empty());

    while i < lines.len() && body.len() < MAX_PARAGRAPHS {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        let before_blank = i == 0 || blank(i - 1);
        // Page break.
        if t.len() >= 3 && t.chars().all(|c| c == '=') {
            let mut p = paragraph(&id(ElementKind::General), "");
            p.props.page_break_before = Some(true);
            body.push(p);
            i += 1;
            continue;
        }
        // Sections become outline levels 1 to 3 (deeper sections are level 3); synopses
        // become Summary.
        if let Some(rest) = t.strip_prefix('#') {
            let kind = match rest.chars().take_while(|c| *c == '#').count() {
                0 => ElementKind::Outline1,
                1 => ElementKind::Outline2,
                _ => ElementKind::Outline3,
            };
            body.push(styled(&id(kind), rest.trim_start_matches('#').trim()));
            i += 1;
            continue;
        }
        if let Some(rest) = t.strip_prefix('=')
            && !rest.starts_with('=')
        {
            body.push(styled(&id(ElementKind::Summary), rest.trim()));
            i += 1;
            continue;
        }
        // A note on its own: a Note element (notes inside other text were removed above).
        if t.starts_with("[[") {
            let mut note: Vec<&str> = Vec::new();
            while let Some(l) = lines.get(i).filter(|l| !l.trim().is_empty() || **l == "  ") {
                note.push(l.trim());
                i += 1;
            }
            let joined = note.join("\n");
            let inner = joined.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")).unwrap_or(&joined);
            body.push(styled(&id(ElementKind::Note), inner.trim()));
            continue;
        }
        // Scene heading.
        if before_blank
            && !t.starts_with('!')
            && let Some((heading, number)) = scene_heading(t)
        {
            let n = body.len();
            body.push(paragraph(&id(ElementKind::SceneHeading), &plain(&heading).to_uppercase()));
            if let Some(num) = number {
                scene_numbers.insert(n, num);
            }
            i += 1;
            continue;
        }
        // Centered text and forced transitions.
        if let Some(rest) = t.strip_prefix('>') {
            if let Some(centered) = rest.strip_suffix('<') {
                body.push(styled(&id(ElementKind::Action), centered.trim()));
            } else {
                body.push(paragraph(&id(ElementKind::Transition), &plain(rest.trim()).to_uppercase()));
            }
            i += 1;
            continue;
        }
        // Transition: capitals ending in TO:, blank before and after.
        if before_blank && blank(i + 1) && is_upper(t) && t.ends_with("TO:") && !t.starts_with('@') && !t.starts_with('!') {
            body.push(paragraph(&id(ElementKind::Transition), &plain(t)));
            i += 1;
            continue;
        }
        // Character, then its parentheticals and dialogue.
        let forced_character = t.starts_with('@');
        let name_part = t.trim_start_matches('@').trim_end_matches('^').trim_end();
        let name_only = name_part.split('(').next().unwrap_or("").trim();
        let looks_like_character = forced_character || (is_upper(name_only) && !name_only.is_empty() && !t.starts_with('!'));
        // A forced cue counts even with nothing after it.
        if before_blank && looks_like_character && (forced_character || !blank(i + 1)) {
            body.push(paragraph(&id(ElementKind::Character), &plain(name_part).to_uppercase()));
            i += 1;
            let mut dialogue: Vec<String> = Vec::new();
            let flush = |dialogue: &mut Vec<String>, body: &mut Vec<Paragraph>| {
                if !dialogue.is_empty() {
                    body.push(styled(&id(ElementKind::Dialogue), &dialogue.join("\n")));
                    dialogue.clear();
                }
            };
            while i < lines.len() && !lines[i].trim().is_empty() || lines.get(i).is_some_and(|l| l == &"  ") {
                let d = lines[i].trim();
                if d.starts_with('(') && d.ends_with(')') {
                    flush(&mut dialogue, &mut body);
                    body.push(styled(&id(ElementKind::Parenthetical), d));
                } else {
                    dialogue.push(d.to_string());
                }
                i += 1;
            }
            flush(&mut dialogue, &mut body);
            continue;
        }
        // Lyrics.
        if let Some(lyric) = t.strip_prefix('~') {
            body.push(styled(&id(ElementKind::Action), lyric.trim()));
            i += 1;
            continue;
        }
        // Action: the rest of the paragraph, line breaks kept.
        let mut action: Vec<String> = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            let l = lines[i].trim_end();
            action.push(l.strip_prefix('!').unwrap_or(l).trim_start().to_string());
            i += 1;
        }
        body.push(styled(&id(ElementKind::Action), &action.join("\n")));
    }
    if body.len() >= MAX_PARAGRAPHS {
        return Err(format!("the script has more than {MAX_PARAGRAPHS} paragraphs"));
    }
    if !body.is_empty() {
        doc.body = with_page_breaks(body).into_iter().map(para_block).collect();
    }
    if let Some(title) = title_page.get("title") {
        doc.core.title = plain(title).replace('\n', " ");
    }
    Ok(FountainScript { doc, title_page, scene_numbers })
}

/// Fountain text for a paragraph's contents: emphasis markers, escapes, and `[[` broken so it
/// is not read as a note.
fn marked(p: &Paragraph) -> String {
    crate::emphasis::encode(p).replace("[[", "[\\[")
}

/// One line: line breaks become spaces (headings, cues, transitions, outline lines).
fn one_line(text: &str) -> String {
    text.replace('\n', " ")
}

/// Whether an Action paragraph's first line would be read as something else.
fn action_needs_force(first: &str, multi_line: bool, could_be_title_page: bool) -> bool {
    let t = first.trim();
    let name_only = t.trim_end_matches('^').split('(').next().unwrap_or("").trim();
    t.starts_with(['!', '@', '.', '>', '~', '#', '=', '['])
        || scene_heading(t).is_some()
        || (is_upper(t) && t.ends_with("TO:"))
        || (multi_line && is_upper(name_only))
        || (could_be_title_page && t.contains(':'))
}

/// Action lines: forced with `!` when needed; empty lines and lines starting with `!` are
/// protected with a `!` of their own (the reader removes one from each line).
fn action_block(text: &str, could_be_title_page: bool) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let force = action_needs_force(lines.first().copied().unwrap_or(""), lines.len() > 1, could_be_title_page);
    let mut out = Vec::with_capacity(lines.len());
    for (n, l) in lines.iter().enumerate() {
        let bang = l.trim().is_empty() || l.starts_with('!') || (n == 0 && force);
        out.push(if bang { format!("!{l}") } else { (*l).to_string() });
    }
    out.join("\n")
}

/// Multi-line text inside a block (dialogue, notes): empty lines become two spaces, Fountain's
/// way to keep a blank line inside a paragraph, and a line in brackets is escaped.
fn inner_lines(text: &str) -> String {
    text.split('\n')
        .map(|l| {
            let t = l.trim();
            if t.is_empty() {
                "  ".to_string()
            } else if t.starts_with('(') && t.ends_with(')') {
                // Dialogue in brackets is not a parenthetical.
                format!("\\{l}")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write a script as Fountain.
///
/// Elements Fountain has no form for degrade gently and read back as something close: Shot,
/// Cast List, General and anything custom become Action; New Act and End of Act become centered
/// text (read back as Action); Dialogue or Parentheticals with no cue before them become Action.
/// Two Dialogue paragraphs in a row under one cue read back as one, joined by a line break.
pub fn write(doc: &Document) -> String {
    let elements = Elements::of_doc(doc);
    let mut blocks: Vec<String> = Vec::new();
    let title = one_line(doc.core.title.trim());
    if !title.is_empty() {
        blocks.push(format!("Title: {title}"));
    }
    let paras: Vec<&Paragraph> = doc.body.iter().filter_map(|b| b.as_para()).collect();
    let mut in_speech = false;
    for (n, p) in paras.iter().enumerate() {
        let kind = elements.of_para(p).kind;
        if p.props.page_break_before == Some(true) {
            blocks.push("===".into());
            in_speech = false;
            if p.text.is_empty() && kind == ElementKind::General {
                continue;
            }
        }
        let text = marked(p);
        if text.is_empty() && !matches!(kind, ElementKind::Character) {
            // An empty paragraph keeps its place as empty Action.
            blocks.push("!".into());
            in_speech = false;
            continue;
        }
        match kind {
            ElementKind::Parenthetical | ElementKind::Dialogue if in_speech => {
                if let Some(last) = blocks.last_mut() {
                    last.push('\n');
                    last.push_str(&if kind == ElementKind::Parenthetical { one_line(&text) } else { inner_lines(&text) });
                }
                continue;
            }
            ElementKind::SceneHeading => {
                let t = one_line(&text);
                let plain_ok = scene_heading(&t).is_some_and(|(h, num)| h == t && num.is_none()) && !t.starts_with('.');
                blocks.push(if plain_ok { t } else { format!(".{t}") });
            }
            ElementKind::Character => {
                let t = one_line(&text);
                let name_only = t.split('(').next().unwrap_or("").trim();
                let speech_follows = paras.get(n + 1).is_some_and(|q| {
                    q.props.page_break_before != Some(true) && matches!(elements.of_para(q).kind, ElementKind::Dialogue | ElementKind::Parenthetical)
                });
                let plain_ok = speech_follows
                    && is_upper(name_only)
                    && t.chars().next().is_some_and(char::is_alphanumeric)
                    && scene_heading(&t).is_none()
                    && !t.ends_with("TO:")
                    && !t.ends_with('^');
                blocks.push(if plain_ok { t } else { format!("@{t}") });
                in_speech = true;
                continue;
            }
            ElementKind::Transition => {
                let t = one_line(&text);
                let plain_ok =
                    is_upper(&t) && t.ends_with("TO:") && t.chars().next().is_some_and(char::is_alphanumeric) && scene_heading(&t).is_none();
                blocks.push(if plain_ok { t } else { format!(">{t}") });
            }
            ElementKind::NewAct | ElementKind::EndOfAct => blocks.push(format!(">{}<", one_line(&text))),
            ElementKind::Outline1 => blocks.push(format!("# {}", one_line(&text))),
            ElementKind::Outline2 => blocks.push(format!("## {}", one_line(&text))),
            ElementKind::Outline3 => blocks.push(format!("### {}", one_line(&text))),
            ElementKind::Summary => blocks.push(format!("= {}", one_line(&text))),
            ElementKind::Note => blocks.push(format!("[[{}]]", inner_lines(&text))),
            _ => {
                let could_be_title_page = blocks.is_empty();
                blocks.push(action_block(&text, could_be_title_page));
            }
        }
        in_speech = false;
    }
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &FountainScript) -> Vec<(String, String)> {
        let e = Elements::of_doc(&s.doc);
        s.doc.body.iter().filter_map(|b| b.as_para()).map(|p| (e.of_para(p).id.clone(), p.text.clone())).collect()
    }

    fn l(e: &str, t: &str) -> (String, String) {
        (e.to_string(), t.to_string())
    }

    #[test]
    fn reads_the_core_elements() {
        let s = read(
            "Title: A Test\nCredit: Written by\nAuthor: Someone\n    Second Line\n\nINT. KITCHEN - NIGHT #12A#\n\nRain hits the window.\nThunder.\n\nANNA (V.O.)\n(quietly)\nDid you hear that?\nI did.\n\nBOB ^\nNo.\n\nCUT TO:\n\n.FLASHBACK\n\n!SILENCE.\n\n> THE END <\n\n>SMASH CUT.\n",
        )
        .unwrap();
        assert_eq!(s.title_page.get("title").map(String::as_str), Some("A Test"));
        assert_eq!(s.title_page.get("author").map(String::as_str), Some("Someone\nSecond Line"));
        assert_eq!(s.doc.core.title, "A Test");
        assert_eq!(
            lines(&s),
            vec![
                l("SceneHeading", "INT. KITCHEN - NIGHT"),
                l("Action", "Rain hits the window.\nThunder."),
                l("Character", "ANNA (V.O.)"),
                l("Parenthetical", "(quietly)"),
                l("Dialogue", "Did you hear that?\nI did."),
                l("Character", "BOB"),
                l("Dialogue", "No."),
                l("Transition", "CUT TO:"),
                l("SceneHeading", "FLASHBACK"),
                l("Action", "SILENCE."),
                l("Action", "THE END"),
                l("Transition", "SMASH CUT."),
            ]
        );
        assert_eq!(s.scene_numbers.get(&0).map(String::as_str), Some("12A"));
    }

    #[test]
    fn things_that_look_like_elements_but_are_not() {
        // A capitals line with a blank after it is Action, not a Character; "TO:" inside action
        // is Action; a scene heading needs a blank line before it; "..." does not force.
        let s = read("JUST CAPITALS\n\nHe walks TO: the door.\nINT. NOT A HEADING\n\n...and then\n").unwrap();
        assert_eq!(
            lines(&s),
            vec![l("Action", "JUST CAPITALS"), l("Action", "He walks TO: the door.\nINT. NOT A HEADING"), l("Action", "...and then")]
        );
    }

    #[test]
    fn outline_notes_boneyard_and_emphasis() {
        let s = read(
            "# Act One\n\n## Arrival\n\n#### Deeper\n\n= Anna arrives.\n\n[[A note\non two lines]]\n\n/* cut this\n\nand this */Anna *enters*. [[fix later]] She **stops** _here_.\n\n===\n\nEXT. YARD - DAY\n",
        )
        .unwrap();
        let got = lines(&s);
        assert_eq!(
            got[..6],
            [
                l("Outline1", "Act One"),
                l("Outline2", "Arrival"),
                l("Outline3", "Deeper"),
                l("Summary", "Anna arrives."),
                l("Note", "A note\non two lines"),
                l("Action", "Anna enters.  She stops here."),
            ]
        );
        assert_eq!(got[6], l("SceneHeading", "EXT. YARD - DAY"));
        let page_break = s.doc.body.get(6).and_then(|b| b.as_para()).and_then(|p| p.props.page_break_before);
        assert_eq!(page_break, Some(true));
        // Emphasis is kept as formatting.
        let action = s.doc.body.get(5).and_then(|b| b.as_para()).unwrap();
        let runs: Vec<(String, bool, bool, bool)> = action
            .run_ranges()
            .map(|(r, c)| {
                (
                    action.text[r].to_string(),
                    c.bold == Some(true),
                    c.italic == Some(true),
                    c.underline.is_some_and(|u| u != wordsmith_doc::props::Underline::None),
                )
            })
            .collect();
        assert!(runs.contains(&("enters".into(), false, true, false)), "{runs:?}");
        assert!(runs.contains(&("stops".into(), true, false, false)), "{runs:?}");
        assert!(runs.contains(&("here".into(), false, false, true)), "{runs:?}");
    }

    #[test]
    fn a_forced_cue_needs_no_dialogue_and_a_colon_is_not_a_title_page() {
        let s = read("!Note: this is action.\n\n@McCLANE\n\nHe waits.\n").unwrap();
        assert_eq!(lines(&s), [l("Action", "Note: this is action."), l("Character", "MCCLANE"), l("Action", "He waits.")]);
        assert!(s.title_page.is_empty());
    }

    /// (element, text) of every paragraph, with emphasis spelled out, for round-trip checks.
    fn shape(doc: &Document) -> Vec<(String, String, bool)> {
        let e = Elements::of_doc(doc);
        doc.body
            .iter()
            .filter_map(|b| b.as_para())
            .map(|p| (e.of_para(p).id.clone(), crate::emphasis::encode(p), p.props.page_break_before == Some(true)))
            .collect()
    }

    fn doc_of(paras: &[(&str, &str)]) -> Document {
        let mut doc = new_script();
        doc.body = paras.iter().map(|(e, t)| para_block(crate::emphasis::decode(e, t, false))).collect();
        doc
    }

    #[test]
    fn every_element_fountain_has_round_trips() {
        let mut doc = doc_of(&[
            ("Outline1", "Act One"),
            ("Outline2", "The *meeting*"),
            ("Outline3", "Detail"),
            ("Summary", "Anna meets Bob."),
            ("Note", "Check this\n\nwith the director"),
            ("SceneHeading", "INT. KITCHEN - NIGHT"),
            ("SceneHeading", "FLASHBACK"),
            ("Action", "Rain **hammers** the _window_.\nThunder.\n\nA beat."),
            ("Action", "INT. NOT A HEADING, JUST SHOUTING"),
            ("Action", "BOOM\nThe house shakes."),
            ("Action", "!Bang. 5 \\* 3 = [[15]]"),
            ("Action", "CUT TO: the chase"),
            ("Character", "ANNA (V.O.)"),
            ("Parenthetical", "(quietly)"),
            ("Dialogue", "Did you *hear* that?\n\nI did."),
            ("Parenthetical", "(beat)"),
            ("Dialogue", "Never mind."),
            ("Character", "McCLANE"),
            ("Dialogue", "Yippee."),
            ("Character", "BOB"),
            ("Action", "Bob says nothing."),
            ("Transition", "CUT TO:"),
            ("Transition", "FADE OUT."),
            ("Action", ""),
            ("SceneHeading", "EXT. YARD - DAY"),
        ]);
        doc.core.title = "The Test".into();
        let mut yard = crate::emphasis::decode("SceneHeading", "EXT. YARD - DAY", false);
        yard.props.page_break_before = Some(true);
        doc.body[24] = para_block(yard);
        let text = write(&doc);
        let back = read(&text).unwrap();
        assert_eq!(back.doc.core.title, "The Test");
        let mut want = shape(&doc);
        // Cues are stored in capitals.
        want[17].1 = "MCCLANE".into();
        assert_eq!(shape(&back.doc), want, "\n{text}");
        // And the text is stable from here on (the cue is now in capitals, so it needs no `@`).
        let again = write(&back.doc);
        assert_eq!(write(&read(&again).unwrap().doc), again);
    }

    #[test]
    fn elements_without_a_fountain_form_degrade_gently() {
        let doc = doc_of(&[
            ("Shot", "ANGLE ON ANNA"),
            ("NewAct", "ACT TWO"),
            ("CastList", "ANNA, BOB"),
            ("Dialogue", "Orphan line."),
            ("General", "Plain."),
        ]);
        let back = read(&write(&doc)).unwrap();
        assert_eq!(
            lines(&back),
            [l("Action", "ANGLE ON ANNA"), l("Action", "ACT TWO"), l("Action", "ANNA, BOB"), l("Action", "Orphan line."), l("Action", "Plain.")]
        );
    }

    #[test]
    fn generated_scripts_round_trip() {
        // A small deterministic generator over the elements Fountain keeps exactly.
        struct Rng(u64);
        impl Rng {
            fn next(&mut self, n: u64) -> u64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0 % n
            }
            fn sentence(&mut self, words: &[&str], count: u64) -> String {
                let n = 1 + self.next(count);
                (0..n).map(|_| words[self.next(words.len() as u64) as usize]).collect::<Vec<_>>().join(" ")
            }
        }
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let words = ["rain", "*door*", "**HALL**", "_note_", "Anna", "x\\_y", "5 \\* 3", "(aside)", "INT.", "TO:", "!", "@", "#", "[\\[a]]"];
        for _ in 0..200 {
            let mut paras: Vec<(String, String)> = Vec::new();
            for _ in 0..(1 + rng.next(12)) {
                match rng.next(6) {
                    0 => paras.push((
                        "SceneHeading".into(),
                        format!("INT. {} - DAY", rng.sentence(&words, 2).to_uppercase().replace(['*', '_', '\\'], "")),
                    )),
                    1 | 2 => paras.push(("Action".into(), rng.sentence(&words, 6))),
                    3 => {
                        paras.push(("Character".into(), format!("ANNA{}", if rng.next(2) == 0 { " (O.S.)" } else { "" })));
                        if rng.next(2) == 0 {
                            paras.push(("Parenthetical".into(), "(softly)".into()));
                        }
                        paras.push(("Dialogue".into(), rng.sentence(&words, 5)));
                    }
                    4 => paras.push(("Transition".into(), "CUT TO:".into())),
                    _ => paras.push(("Note".into(), rng.sentence(&words, 3))),
                }
            }
            let refs: Vec<(&str, &str)> = paras.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            let doc = doc_of(&refs);
            let text = write(&doc);
            let back = read(&text).unwrap();
            assert_eq!(shape(&back.doc), shape(&doc), "\n{text}");
        }
    }

    #[test]
    fn hostile_input_is_bounded_and_never_panics() {
        assert!(read(&"x".repeat(MAX_FOUNTAIN_BYTES + 1)).is_err());
        for nasty in [
            "",
            "\n\n\n",
            "/*",
            "[[",
            "#",
            "@",
            "INT.",
            ".",
            "~",
            "(",
            "ANNA\n(",
            "===\n===",
            "\u{feff}Title:",
            "Title: x\n   \n",
            "#1#",
            "INT. A #",
            "ÉCOLE\nbonjour",
        ] {
            let _ = read(nasty);
        }
        let s = read("").unwrap();
        assert_eq!(s.doc.body.len(), 1, "an empty file is an empty script");
    }
}
