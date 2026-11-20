// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emphasis as text markers, shared by the native format and Fountain: `**` bold, `*` italic,
//! `_` underline, and `\` escapes for the marker characters themselves. The native format (from
//! version 2) also marks notes' anchors: `{n12}` starts note 12's text and `{/n}` ends it, with
//! `{` escaped as `\{`.

use wordsmith_doc::Paragraph;
use wordsmith_doc::props::{CharProps, Underline};

/// Text with emphasis markers and escapes for one paragraph (runs → `**`, `*`, `_`).
#[cfg(test)]
pub(crate) fn encode(p: &Paragraph) -> String {
    encode_with(p, false, &|_| None)
}

/// Emphasis markers that take the styles from `from` to `to` (each marker toggles its own).
fn toggles(out: &mut String, from: (bool, bool, bool), to: (bool, bool, bool)) {
    if from.0 != to.0 {
        out.push_str("**");
    }
    if from.1 != to.1 {
        out.push('*');
    }
    if from.2 != to.2 {
        out.push('_');
    }
}

/// [`encode`], with note markers when `notes` (the native format), and `after_note`'s text, if
/// any, written where a note's text ends (Fountain writes the note there, outside any emphasis).
pub(crate) fn encode_with(p: &Paragraph, notes: bool, after_note: &dyn Fn(u32) -> Option<String>) -> String {
    const PLAIN: (bool, bool, bool) = (false, false, false);
    let mut out = String::with_capacity(p.text.len() + 8);
    let mut style = PLAIN;
    let mut note: Option<u32> = None;
    // Close a note: its marker, then the text written after it, outside any emphasis.
    let end_note = |out: &mut String, style: &mut (bool, bool, bool), old: u32| {
        if notes {
            out.push_str("{/n}");
        }
        if let Some(t) = after_note(old) {
            toggles(out, *style, PLAIN);
            *style = PLAIN;
            out.push_str(&t);
        }
    };
    for (range, props) in p.run_ranges() {
        if props.note != note {
            if let Some(old) = note {
                end_note(&mut out, &mut style, old);
            }
            if notes && let Some(new) = props.note {
                out.push_str(&format!("{{n{new}}}"));
            }
            note = props.note;
        }
        let next = (props.bold == Some(true), props.italic == Some(true), props.underline.is_some_and(|x| x != Underline::None));
        toggles(&mut out, style, next);
        style = next;
        for c in p.text.get(range).unwrap_or("").chars() {
            match c {
                '\\' | '*' | '_' => {
                    out.push('\\');
                    out.push(c);
                }
                '{' if notes => out.push_str("\\{"),
                // Inline objects (pictures, fields) are not kept in a script.
                wordsmith_doc::para::OBJ => {}
                _ => out.push(c),
            }
        }
    }
    if let Some(old) = note {
        end_note(&mut out, &mut style, old);
    }
    toggles(&mut out, style, PLAIN);
    out
}

/// Which markers are read as plain characters (bold, italic, underline).
type Literal = [bool; 3];

/// A note marker at the start of `rest` (just after a `{`): the note it opens (`Some(Some(id))`)
/// or closes (`Some(None)`), and its length after the `{`.
fn note_marker(rest: &str) -> Option<(Option<u32>, usize)> {
    if rest.starts_with("/n}") {
        return Some((None, 3));
    }
    let r = rest.strip_prefix('n')?;
    let digits: String = r.chars().take_while(char::is_ascii_digit).take(9).collect();
    if digits.is_empty() || !r[digits.len()..].starts_with('}') {
        return None;
    }
    Some((Some(digits.parse().ok()?), 1 + digits.len() + 1))
}

/// One decoding pass; also returns which styles were still open at the end.
fn decode_pass(style: &str, text: &str, literal: Literal, notes: bool) -> (Paragraph, Literal) {
    let mut p = Paragraph::new();
    p.props.style = Some(style.to_string());
    let (mut b, mut i, mut u) = (false, false, false);
    let mut note: Option<u32> = None;
    let mut piece = String::new();
    let flush = |p: &mut Paragraph, piece: &mut String, b: bool, i: bool, u: bool, note: Option<u32>| {
        if piece.is_empty() {
            return;
        }
        let props =
            CharProps { bold: b.then_some(true), italic: i.then_some(true), underline: u.then_some(Underline::Single), note, ..CharProps::default() };
        let at = p.len();
        let _ = p.insert_text(at, piece, &props);
        piece.clear();
    };
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if notes
            && c == '{'
            && let Some((next, len)) = text.get(at + 1..).and_then(note_marker)
        {
            flush(&mut p, &mut piece, b, i, u, note);
            note = next;
            for _ in 0..len {
                chars.next();
            }
            continue;
        }
        match c {
            '\\' => {
                if let Some((_, n)) = chars.next() {
                    piece.push(n);
                }
            }
            '*' if chars.peek().map(|x| x.1) == Some('*') => {
                chars.next();
                if literal[0] {
                    piece.push_str("**");
                } else {
                    flush(&mut p, &mut piece, b, i, u, note);
                    b = !b;
                }
            }
            '*' if literal[1] => piece.push('*'),
            '*' => {
                flush(&mut p, &mut piece, b, i, u, note);
                i = !i;
            }
            '_' if literal[2] => piece.push('_'),
            '_' => {
                flush(&mut p, &mut piece, b, i, u, note);
                u = !u;
            }
            _ => piece.push(c),
        }
    }
    flush(&mut p, &mut piece, b, i, u, note);
    (p, [b, i, u])
}

/// A paragraph of element `style` from marked-up text: markers toggle bold, italic and
/// underline. When `lenient`, a marker kind left unclosed at the end of the paragraph is read as
/// plain characters instead (Fountain: "5 * 3" is not emphasis).
pub(crate) fn decode(style: &str, text: &str, lenient: bool) -> Paragraph {
    decode_with(style, text, lenient, false)
}

/// [`decode`], reading note markers when `notes` (the native format from version 2).
pub(crate) fn decode_with(style: &str, text: &str, lenient: bool, notes: bool) -> Paragraph {
    let mut literal = [false; 3];
    loop {
        let (p, open) = decode_pass(style, text, literal, notes);
        if !lenient || !open.iter().any(|o| *o) {
            return p;
        }
        for k in 0..3 {
            literal[k] |= open[k];
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn note_marks_round_trip_in_the_native_form() {
        let p = super::decode_with("Action", r"Rain {n3}hits **the**{/n} window \{n1} {oops}", false, true);
        assert_eq!(p.text, "Rain hits the window {n1} {oops}");
        let noted: Vec<(String, Option<u32>)> = p.run_ranges().map(|(r, c)| (p.text[r].to_string(), c.note)).collect();
        assert!(noted.contains(&("hits ".to_string(), Some(3))) && noted.contains(&("the".to_string(), Some(3))), "{noted:?}");
        let again = super::encode_with(&p, true, &|_| None);
        assert_eq!(again, r"Rain {n3}hits **the{/n}** window \{n1} \{oops}");
        assert_eq!(super::decode_with("Action", &again, false, true), p);
        // Without notes (Fountain), the marks are not written, and braces are plain.
        assert_eq!(super::encode(&p), "Rain hits **the** window {n1} {oops}");
        assert_eq!(super::encode_with(&p, false, &|id| Some(format!(" [[note {id}]]"))), "Rain hits **the** [[note 3]] window {n1} {oops}");
    }

    use super::*;

    fn styled(p: &Paragraph) -> Vec<(String, bool, bool, bool)> {
        p.run_ranges()
            .map(|(r, c)| {
                (
                    p.text.get(r).unwrap_or("").to_string(),
                    c.bold == Some(true),
                    c.italic == Some(true),
                    c.underline.is_some_and(|x| x != Underline::None),
                )
            })
            .collect()
    }

    #[test]
    fn markers_round_trip() {
        let p = decode("Action", r"plain **bold** *it* _un_ ***both*** \*star\_", false);
        assert_eq!(p.text, "plain bold it un both *star_");
        assert_eq!(decode("Action", &encode(&p), false).run_ranges().count(), p.run_ranges().count());
        assert_eq!(styled(&decode("Action", &encode(&p), false)), styled(&p));
    }

    #[test]
    fn lenient_reads_unclosed_markers_as_text() {
        assert_eq!(decode("Action", "5 * 3 is *fifteen*", true).text, "5 * 3 is *fifteen*", "an odd count: all literal");
        assert_eq!(decode("Action", "5 * 3", true).text, "5 * 3");
        let p = decode("Action", "snake_case and **bold**", true);
        assert_eq!(p.text, "snake_case and bold");
        assert!(styled(&p).iter().any(|s| s.0 == "bold" && s.1));
        assert_eq!(decode("Action", "a ** b", true).text, "a ** b");
        assert_eq!(decode("Action", "a * b", false).text, "a  b", "strict mode toggles");
    }
}
