// SPDX-License-Identifier: MIT OR Apache-2.0
//! Character formatting by hand (SPEC-2 6). Bold, italic, the underlines, strike, raised and
//! lowered text, capitals, small letters and text color are the engine's commands
//! (`format.bold`, `format.underline {style}`, `format.changeCase`, `format.color`,
//! `format.state`, ...; core decision 0007); Script's old ids for them are in [`super::aliases`].
//!
//! Script keeps its highlighter (`format.highlight`), with its own color names: on a selection it
//! applies to it; at a caret, to what is typed next.

use jcj_doc::props::{CharProps, Highlight, Rgb};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, cmd::sel_result, params as p};
use serde_json::Value;

/// The highlighter colors offered by name (our names), and the colors they stand for.
pub const HIGHLIGHTS: [(&str, Highlight); 12] = [
    ("Yellow", Highlight::Yellow),
    ("Lime", Highlight::BrightGreen),
    ("Aqua", Highlight::Turquoise),
    ("Rose", Highlight::Pink),
    ("Sky", Highlight::Blue),
    ("Red", Highlight::Red),
    ("Navy", Highlight::DarkBlue),
    ("Teal", Highlight::Teal),
    ("Green", Highlight::Green),
    ("Violet", Highlight::Violet),
    ("Maroon", Highlight::DarkRed),
    ("Olive", Highlight::DarkYellow),
];

/// Apply `f` to the selection, or to what is typed next at a caret.
fn apply(s: &mut Session, f: &dyn Fn(&mut CharProps)) -> CmdResult {
    if s.sel.is_collapsed() {
        let at = s.sel.focus.clone();
        let mut pending = s.pending.clone().unwrap_or_else(|| s.doc.para_at(&at).map(|p| p.props_at(at.off).clone()).unwrap_or_default());
        f(&mut pending);
        s.pending = Some(pending);
        return sel_result(s);
    }
    let (a, b) = s.sel.ordered();
    s.doc.format_range(&a, &b, f)?;
    sel_result(s)
}

/// A color given as a highlighter name, `#RRGGBB`, or `none`.
fn highlight(s: &mut Session, v: &Value) -> CmdResult {
    let given = p::req_str(v, "color")?.trim().to_string();
    if given.eq_ignore_ascii_case("none") {
        return apply(s, &|c| {
            c.highlight = None;
            c.shading = None;
        });
    }
    if let Some((_, h)) = HIGHLIGHTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(&given)) {
        let h = *h;
        return apply(s, &move |c| {
            c.highlight = Some(h);
            c.shading = None;
        });
    }
    let rgb = Rgb::parse(&given).ok_or_else(|| CmdError::Params("`color` is a highlighter name, #RRGGBB or none".into()))?;
    apply(s, &move |c| {
        c.highlight = None;
        c.shading = Some(rgb);
    })
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Home › Font";
    vec![CommandSpec::new("format.highlight", "Highlight", LOC, highlight).params(r##"{"color": name | "#RRGGBB" | "none"}"##)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::data::body_pos;
    use jcj_doc::props::{TextColor, Underline, VertAlign};
    use jcj_engine::Selection;
    use serde_json::json;

    /// The styles `format.style` toggles.
    const STYLES: [&str; 11] = [
        "bold",
        "italic",
        "underline",
        "doubleUnderline",
        "wordUnderline",
        "dottedUnderline",
        "strike",
        "superscript",
        "subscript",
        "caps",
        "smallCaps",
    ];

    /// Whether `style` is on in `c`.
    fn is_on(c: &CharProps, style: &str) -> bool {
        match style {
            "bold" => c.bold == Some(true),
            "italic" => c.italic == Some(true),
            "underline" => c.underline == Some(Underline::Single),
            "doubleUnderline" => c.underline == Some(Underline::Double),
            "wordUnderline" => c.underline == Some(Underline::Words),
            "dottedUnderline" => c.underline == Some(Underline::Dotted),
            "strike" => c.strike == Some(true),
            "superscript" => c.vert_align == Some(VertAlign::Superscript),
            "subscript" => c.vert_align == Some(VertAlign::Subscript),
            "caps" => c.caps == Some(true),
            "smallCaps" => c.small_caps == Some(true),
            _ => false,
        }
    }

    fn session() -> Session {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. hall\nRain falls."})).unwrap();
        s.sel = Selection { anchor: body_pos(1, 0), focus: body_pos(1, 4) };
        s
    }

    fn at(s: &Session, off: usize) -> CharProps {
        s.doc.para_at(&body_pos(1, off)).map(|p| p.props_at(off).clone()).unwrap_or_default()
    }

    #[test]
    fn styles_toggle_on_a_selection() {
        let mut s = session();
        s.run("format.bold", &json!({})).unwrap();
        assert_eq!(at(&s, 2).bold, Some(true));
        s.run("format.bold", &json!({})).unwrap();
        assert_ne!(at(&s, 2).bold, Some(true), "the engine writes bold off as Some(false)");
        for st in STYLES {
            s.run("format.style", &json!({"style": st, "on": true})).unwrap();
            assert!(is_on(&at(&s, 2), st), "{st}");
        }
        s.run("format.plain", &json!({})).unwrap();
        assert!(STYLES.iter().all(|st| !is_on(&at(&s, 2), st)), "plain text");
        assert!(s.run("format.style", &json!({"style": "blink"})).is_err());
    }

    #[test]
    fn highlight_and_color() {
        let mut s = session();
        s.run("format.highlight", &json!({"color": "violet"})).unwrap();
        assert_eq!(at(&s, 1).highlight, Some(Highlight::Violet));
        s.run("format.highlight", &json!({"color": "#102030"})).unwrap();
        assert_eq!((at(&s, 1).highlight, at(&s, 1).shading), (None, Some(Rgb(0x10, 0x20, 0x30))));
        s.run("format.color", &json!({"color": "#FF0000"})).unwrap();
        assert_eq!(at(&s, 1).color, Some(TextColor::Rgb(Rgb(255, 0, 0))));
        s.run("format.color", &json!({"color": "auto"})).unwrap();
        assert!(matches!(at(&s, 1).color, None | Some(TextColor::Auto)));
        assert!(s.run("format.highlight", &json!({"color": "plaid"})).is_err());
    }

    #[test]
    fn at_a_caret_styles_apply_to_typing() {
        let mut s = session();
        s.sel = Selection::caret(body_pos(1, 11));
        s.run("format.italic", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": " Hard"})).unwrap();
        assert_eq!(at(&s, 13).italic, Some(true));
        assert_eq!(at(&s, 2).italic, None);
    }

    #[test]
    fn lowercase_changes_the_text() {
        let mut s = session();
        s.sel = Selection { anchor: body_pos(0, 0), focus: body_pos(0, 4) };
        s.run("format.lowercase", &json!({})).unwrap();
        assert_eq!(s.doc.para_at(&body_pos(0, 0)).map(|p| p.text.clone()).unwrap_or_default(), "int. HALL");
    }
}
