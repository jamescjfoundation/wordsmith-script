// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading Fountain (https://fountain.io/syntax, version 1.1), the open plain-text screenplay
//! format. M2 needs it for the golden scripts; M4 adds the writer, emphasis and round trips.
//!
//! Recognised: title page (`Key: value`, indented continuation lines), scene headings (`INT`,
//! `EXT`, `EST`, `INT./EXT`, `INT/EXT`, `I/E`, or forced with `.`; `#n#` scene numbers),
//! characters (all capitals with a blank line before and text after, or forced with `@`; `^`
//! dual dialogue is read as ordinary dialogue), parentheticals, dialogue, transitions (capitals
//! ending in `TO:`, or forced with `>`), centered text (`>text<`, read as Action), action
//! (forced with `!`; line breaks kept), lyrics (`~`, read as Action), page breaks (`===`, which
//! become a paragraph that starts a new page), sections (`#`) and synopses (`=`), which are left
//! out until M7's outline, notes (`[[ ]]`) and the boneyard (`/* */`), which are dropped.
//! Emphasis markers (`*`, `**`, `***`, `_`) are removed and the text kept; keeping them as
//! formatting is M4's.
//!
//! The input is hostile: its size and paragraph count are bounded, and anything unclear is read
//! as Action, as the spec says.

use std::collections::BTreeMap;

use wordsmith_doc::{Document, para_block};
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

fn is_upper(line: &str) -> bool {
    line.chars().any(char::is_alphabetic) && !line.chars().any(char::is_lowercase)
}

/// A scene heading's text and number, if `line` is one.
fn scene_heading(line: &str) -> Option<(String, Option<String>)> {
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
    let first_is_key =
        lines.first().is_some_and(|l| l.contains(':') && !l.starts_with(' ') && scene_heading(l).is_none() && !l.trim_end().ends_with("TO:"));
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
    let mut body = Vec::new();
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
            body.push(para_block(p));
            i += 1;
            continue;
        }
        // Sections and synopses: outline material, not shown in the script yet (M7).
        if t.starts_with('#') || (t.starts_with('=') && !t.starts_with("==")) {
            i += 1;
            continue;
        }
        // Scene heading.
        if before_blank
            && !t.starts_with('!')
            && let Some((heading, number)) = scene_heading(t)
        {
            let n = body.len();
            body.push(para_block(paragraph(&id(ElementKind::SceneHeading), &plain(&heading).to_uppercase())));
            if let Some(num) = number {
                scene_numbers.insert(n, num);
            }
            i += 1;
            continue;
        }
        // Centered text and forced transitions.
        if let Some(rest) = t.strip_prefix('>') {
            if let Some(centered) = rest.strip_suffix('<') {
                body.push(para_block(paragraph(&id(ElementKind::Action), &plain(centered.trim()))));
            } else {
                body.push(para_block(paragraph(&id(ElementKind::Transition), &plain(rest.trim()).to_uppercase())));
            }
            i += 1;
            continue;
        }
        // Transition: capitals ending in TO:, blank before and after.
        if before_blank && blank(i + 1) && is_upper(t) && t.ends_with("TO:") {
            body.push(para_block(paragraph(&id(ElementKind::Transition), &plain(t))));
            i += 1;
            continue;
        }
        // Character, then its parentheticals and dialogue.
        let forced_character = t.starts_with('@');
        let name_part = t.trim_start_matches('@').trim_end_matches('^').trim_end();
        let name_only = name_part.split('(').next().unwrap_or("").trim();
        let looks_like_character = forced_character || (is_upper(name_only) && !name_only.is_empty() && !t.starts_with('!'));
        if before_blank && looks_like_character && !blank(i + 1) {
            body.push(para_block(paragraph(&id(ElementKind::Character), &plain(name_part).to_uppercase())));
            i += 1;
            let mut dialogue: Vec<String> = Vec::new();
            let flush = |dialogue: &mut Vec<String>, body: &mut Vec<_>| {
                if !dialogue.is_empty() {
                    body.push(para_block(paragraph(&id(ElementKind::Dialogue), &plain(&dialogue.join("\n")))));
                    dialogue.clear();
                }
            };
            while i < lines.len() && !lines[i].trim().is_empty() || lines.get(i).is_some_and(|l| l == &"  ") {
                let d = lines[i].trim();
                if d.starts_with('(') && d.ends_with(')') {
                    flush(&mut dialogue, &mut body);
                    body.push(para_block(paragraph(&id(ElementKind::Parenthetical), &plain(d))));
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
            body.push(para_block(paragraph(&id(ElementKind::Action), &plain(lyric.trim()))));
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
        body.push(para_block(paragraph(&id(ElementKind::Action), &plain(&action.join("\n")))));
    }
    if body.len() >= MAX_PARAGRAPHS {
        return Err(format!("the script has more than {MAX_PARAGRAPHS} paragraphs"));
    }
    if !body.is_empty() {
        doc.body = body;
    }
    if let Some(title) = title_page.get("title") {
        doc.core.title = plain(title).replace('\n', " ");
    }
    Ok(FountainScript { doc, title_page, scene_numbers })
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
    fn notes_boneyard_sections_and_emphasis_are_removed() {
        let s = read("# Act One\n\n= Anna arrives.\n\n/* cut this\n\nand this */Anna *enters*. [[fix later]] She **stops** _here_.\n\n===\n\nEXT. YARD - DAY\n").unwrap();
        let got = lines(&s);
        assert_eq!(got[0], l("Action", "Anna enters.  She stops here."));
        assert_eq!(got[1].1, "");
        assert_eq!(got[2], l("SceneHeading", "EXT. YARD - DAY"));
        let page_break = s.doc.body.get(1).and_then(|b| b.as_para()).and_then(|p| p.props.page_break_before);
        assert_eq!(page_break, Some(true));
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
