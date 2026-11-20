// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emphasis as text markers, shared by the native format and Fountain: `**` bold, `*` italic,
//! `_` underline, and `\` escapes for the marker characters themselves. The native format (from
//! version 2) also marks notes' anchors: `{n12}` starts note 12's text and `{/n}` ends it; from
//! version 3, revised text (`{r2}` … `{/r}`, set 2) and locked page starts (`{p7}` before the
//! page's first character). `{` in the text is escaped as `\{`.

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

/// [`encode`], with the native format's marks when `marks` (notes, revisions, locked page
/// starts), and `after_note`'s text, if any, written where a note's text ends (Fountain writes the
/// note there, outside any emphasis).
pub(crate) fn encode_with(p: &Paragraph, marks: bool, after_note: &dyn Fn(u32) -> Option<String>) -> String {
    const PLAIN: (bool, bool, bool) = (false, false, false);
    let mut out = String::with_capacity(p.text.len() + 8);
    let mut style = PLAIN;
    let mut note: Option<u32> = None;
    let mut revised: Option<u32> = None;
    // Close a note: its marker, then the text written after it, outside any emphasis.
    let end_note = |out: &mut String, style: &mut (bool, bool, bool), old: u32| {
        if marks {
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
            if marks && let Some(new) = props.note {
                out.push_str(&format!("{{n{new}}}"));
            }
            note = props.note;
        }
        if marks && props.revised != revised {
            match props.revised {
                Some(set) => out.push_str(&format!("{{r{set}}}")),
                None => out.push_str("{/r}"),
            }
            revised = props.revised;
        }
        let next = (props.bold == Some(true), props.italic == Some(true), props.underline.is_some_and(|x| x != Underline::None));
        toggles(&mut out, style, next);
        style = next;
        for (k, c) in p.text.get(range).unwrap_or("").chars().enumerate() {
            // A locked page starts at this run's first character.
            if marks
                && k == 0
                && let Some(id) = props.page_lock
            {
                out.push_str(&format!("{{p{id}}}"));
            }
            match c {
                '\\' | '*' | '_' => {
                    out.push('\\');
                    out.push(c);
                }
                '{' if marks => out.push_str("\\{"),
                // Inline objects (pictures, fields) are not kept in a script.
                wordsmith_doc::para::OBJ => {}
                _ => out.push(c),
            }
        }
    }
    if let Some(old) = note {
        end_note(&mut out, &mut style, old);
    }
    if marks && revised.is_some() {
        out.push_str("{/r}");
    }
    toggles(&mut out, style, PLAIN);
    out
}

/// Which markers are read as plain characters (bold, italic, underline).
type Literal = [bool; 3];

/// A native mark, read just after a `{`.
enum Mark {
    /// `{n12}` / `{/n}`: a note's text starts or ends.
    Note(Option<u32>),
    /// `{r2}` / `{/r}`: revised text (set 2) starts or ends.
    Revised(Option<u32>),
    /// `{p7}`: a locked page starts at the next character.
    Lock(u32),
}

/// The mark at the start of `rest` (just after a `{`), and its length after the `{`.
fn mark_at(rest: &str) -> Option<(Mark, usize)> {
    if rest.starts_with("/n}") {
        return Some((Mark::Note(None), 3));
    }
    if rest.starts_with("/r}") {
        return Some((Mark::Revised(None), 3));
    }
    let kind = rest.chars().next()?;
    let r = rest.get(1..)?;
    let digits: String = r.chars().take_while(char::is_ascii_digit).take(9).collect();
    if digits.is_empty() || !r[digits.len()..].starts_with('}') {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    let len = 1 + digits.len() + 1;
    match kind {
        'n' => Some((Mark::Note(Some(n)), len)),
        'r' => Some((Mark::Revised(Some(n)), len)),
        'p' => Some((Mark::Lock(n), len)),
        _ => None,
    }
}

/// The formatting being read: emphasis and marks.
#[derive(Clone, Copy, Default)]
struct State {
    b: bool,
    i: bool,
    u: bool,
    note: Option<u32>,
    revised: Option<u32>,
}

impl State {
    fn props(self, lock: Option<u32>) -> CharProps {
        CharProps {
            bold: self.b.then_some(true),
            italic: self.i.then_some(true),
            underline: self.u.then_some(Underline::Single),
            note: self.note,
            revised: self.revised,
            page_lock: lock,
            ..CharProps::default()
        }
    }
}

fn flush(p: &mut Paragraph, piece: &mut String, st: State) {
    if piece.is_empty() {
        return;
    }
    let at = p.len();
    let _ = p.insert_text(at, piece, &st.props(None));
    piece.clear();
}

/// One decoding pass; also returns which styles were still open at the end.
fn decode_pass(style: &str, text: &str, literal: Literal, marks: bool) -> (Paragraph, Literal) {
    let mut p = Paragraph::new();
    p.props.style = Some(style.to_string());
    let mut st = State::default();
    let mut lock: Option<u32> = None;
    let mut piece = String::new();
    // A plain character; the one after a lock mark goes in on its own, with the mark.
    let push = |p: &mut Paragraph, piece: &mut String, st: State, lock: &mut Option<u32>, c: char| {
        if let Some(id) = lock.take() {
            flush(p, piece, st);
            let at = p.len();
            let _ = p.insert_text(at, &c.to_string(), &st.props(Some(id)));
        } else {
            piece.push(c);
        }
    };
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if marks
            && c == '{'
            && let Some((mark, len)) = text.get(at + 1..).and_then(mark_at)
        {
            flush(&mut p, &mut piece, st);
            match mark {
                Mark::Note(n) => st.note = n,
                Mark::Revised(r) => st.revised = r,
                Mark::Lock(id) => lock = Some(id),
            }
            for _ in 0..len {
                chars.next();
            }
            continue;
        }
        match c {
            '\\' => {
                if let Some((_, n)) = chars.next() {
                    push(&mut p, &mut piece, st, &mut lock, n);
                }
            }
            '*' if chars.peek().map(|x| x.1) == Some('*') => {
                chars.next();
                if literal[0] {
                    push(&mut p, &mut piece, st, &mut lock, '*');
                    push(&mut p, &mut piece, st, &mut lock, '*');
                } else {
                    flush(&mut p, &mut piece, st);
                    st.b = !st.b;
                }
            }
            '*' if literal[1] => push(&mut p, &mut piece, st, &mut lock, '*'),
            '*' => {
                flush(&mut p, &mut piece, st);
                st.i = !st.i;
            }
            '_' if literal[2] => push(&mut p, &mut piece, st, &mut lock, '_'),
            '_' => {
                flush(&mut p, &mut piece, st);
                st.u = !st.u;
            }
            _ => push(&mut p, &mut piece, st, &mut lock, c),
        }
    }
    flush(&mut p, &mut piece, st);
    (p, [st.b, st.i, st.u])
}

/// A paragraph of element `style` from marked-up text: markers toggle bold, italic and
/// underline. When `lenient`, a marker kind left unclosed at the end of the paragraph is read as
/// plain characters instead (Fountain: "5 * 3" is not emphasis).
pub(crate) fn decode(style: &str, text: &str, lenient: bool) -> Paragraph {
    decode_with(style, text, lenient, false)
}

/// [`decode`], reading the native format's marks when `marks` (notes from version 2;
/// revisions and locked page starts from version 3).
pub(crate) fn decode_with(style: &str, text: &str, lenient: bool, marks: bool) -> Paragraph {
    let mut literal = [false; 3];
    loop {
        let (p, open) = decode_pass(style, text, literal, marks);
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
    fn revision_and_lock_marks_round_trip() {
        let src = r"Old {r0}new **bold{/r}** {p7}Page {r1}{p8}x{/r}";
        let p = super::decode_with("Action", src, false, true);
        assert_eq!(p.text, "Old new bold Page x");
        let runs: Vec<(String, Option<u32>, Option<u32>)> = p.run_ranges().map(|(r, c)| (p.text[r].to_string(), c.revised, c.page_lock)).collect();
        assert!(
            runs.contains(&("new ".into(), Some(0), None))
                && runs.contains(&("P".into(), None, Some(7)))
                && runs.contains(&("x".into(), Some(1), Some(8))),
            "{runs:?}"
        );
        let again = super::encode_with(&p, true, &|_| None);
        assert_eq!(super::decode_with("Action", &again, false, true), p, "{again}");
    }

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
