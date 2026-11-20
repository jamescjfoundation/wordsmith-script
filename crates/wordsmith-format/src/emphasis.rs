// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emphasis as text markers, shared by the native format and Fountain: `**` bold, `*` italic,
//! `_` underline, and `\` escapes for the marker characters themselves.

use wordsmith_doc::Paragraph;
use wordsmith_doc::props::{CharProps, Underline};

/// Text with emphasis markers and escapes for one paragraph (runs → `**`, `*`, `_`).
pub(crate) fn encode(p: &Paragraph) -> String {
    let mut out = String::with_capacity(p.text.len() + 8);
    let (mut b, mut i, mut u) = (false, false, false);
    for (range, props) in p.run_ranges() {
        let nb = props.bold == Some(true);
        let ni = props.italic == Some(true);
        let nu = props.underline.is_some_and(|x| x != Underline::None);
        // Each marker toggles its own style, so order does not matter.
        if nb != b {
            out.push_str("**");
        }
        if ni != i {
            out.push('*');
        }
        if nu != u {
            out.push('_');
        }
        (b, i, u) = (nb, ni, nu);
        for c in p.text.get(range).unwrap_or("").chars() {
            match c {
                '\\' | '*' | '_' => {
                    out.push('\\');
                    out.push(c);
                }
                // Inline objects (pictures, fields) are not kept in a script.
                wordsmith_doc::para::OBJ => {}
                _ => out.push(c),
            }
        }
    }
    if b {
        out.push_str("**");
    }
    if i {
        out.push('*');
    }
    if u {
        out.push('_');
    }
    out
}

/// Which markers are read as plain characters (bold, italic, underline).
type Literal = [bool; 3];

/// One decoding pass; also returns which styles were still open at the end.
fn decode_pass(style: &str, text: &str, literal: Literal) -> (Paragraph, Literal) {
    let mut p = Paragraph::new();
    p.props.style = Some(style.to_string());
    let (mut b, mut i, mut u) = (false, false, false);
    let mut piece = String::new();
    let flush = |p: &mut Paragraph, piece: &mut String, b: bool, i: bool, u: bool| {
        if piece.is_empty() {
            return;
        }
        let props =
            CharProps { bold: b.then_some(true), italic: i.then_some(true), underline: u.then_some(Underline::Single), ..CharProps::default() };
        let at = p.len();
        let _ = p.insert_text(at, piece, &props);
        piece.clear();
    };
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    piece.push(n);
                }
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if literal[0] {
                    piece.push_str("**");
                } else {
                    flush(&mut p, &mut piece, b, i, u);
                    b = !b;
                }
            }
            '*' if literal[1] => piece.push('*'),
            '*' => {
                flush(&mut p, &mut piece, b, i, u);
                i = !i;
            }
            '_' if literal[2] => piece.push('_'),
            '_' => {
                flush(&mut p, &mut piece, b, i, u);
                u = !u;
            }
            _ => piece.push(c),
        }
    }
    flush(&mut p, &mut piece, b, i, u);
    (p, [b, i, u])
}

/// A paragraph of element `style` from marked-up text: markers toggle bold, italic and
/// underline. When `lenient`, a marker kind left unclosed at the end of the paragraph is read as
/// plain characters instead (Fountain: "5 * 3" is not emphasis).
pub(crate) fn decode(style: &str, text: &str, lenient: bool) -> Paragraph {
    let mut literal = [false; 3];
    loop {
        let (p, open) = decode_pass(style, text, literal);
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
