// SPDX-License-Identifier: MIT OR Apache-2.0
//! Character formatting by hand (SPEC-2 6): bold, italic, the underlines, strike, raised and
//! lowered text, capitals and small capitals, small letters, a highlighter and text color. On a
//! selection each command applies to it; at a caret it applies to what is typed next.

use serde_json::{Value, json};
use wordsmith_doc::props::{CharProps, Highlight, Rgb, TextColor, Underline, VertAlign};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, cmd::sel_result, p};

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

/// The styles `format.style` toggles.
pub const STYLES: [&str; 11] =
    ["bold", "italic", "underline", "doubleUnderline", "wordUnderline", "dottedUnderline", "strike", "superscript", "subscript", "caps", "smallCaps"];

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

/// Set `style` on or off in `c`.
fn set_style(c: &mut CharProps, style: &str, on: bool) {
    let underline = |kind| if on { Some(kind) } else { None };
    match style {
        "bold" => c.bold = on.then_some(true),
        "italic" => c.italic = on.then_some(true),
        "underline" => c.underline = underline(Underline::Single),
        "doubleUnderline" => c.underline = underline(Underline::Double),
        "wordUnderline" => c.underline = underline(Underline::Words),
        "dottedUnderline" => c.underline = underline(Underline::Dotted),
        "strike" => c.strike = on.then_some(true),
        "superscript" => c.vert_align = on.then_some(VertAlign::Superscript),
        "subscript" => c.vert_align = on.then_some(VertAlign::Subscript),
        "caps" => c.caps = on.then_some(true),
        "smallCaps" => c.small_caps = on.then_some(true),
        _ => {}
    }
}

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

/// The formatting at the start of the selection (or what typing will take).
fn current(s: &Session) -> CharProps {
    if let Some(p) = &s.pending {
        return p.clone();
    }
    let (a, b) = s.sel.ordered();
    let off = if a == b {
        a.off
    } else {
        a.off + s.doc.para_at(&a).and_then(|p| p.text.get(a.off..)).and_then(|t| t.chars().next()).map_or(0, char::len_utf8)
    };
    s.doc.para_at(&a).map(|p| p.props_at(off).clone()).unwrap_or_default()
}

/// `format.style {style, on?}`: toggle a style (or set it on or off).
fn style(s: &mut Session, v: &Value) -> CmdResult {
    let name = p::req_str(v, "style")?.to_string();
    if !STYLES.contains(&name.as_str()) {
        return Err(CmdError::Params(format!("`style` is one of {}", STYLES.join(", "))));
    }
    let on = p::bool(v, "on").unwrap_or_else(|| !is_on(&current(s), &name));
    apply(s, &move |c| set_style(c, &name, on))
}

fn toggle(s: &mut Session, name: &'static str) -> CmdResult {
    let on = !is_on(&current(s), name);
    apply(s, &move |c| set_style(c, name, on))
}

/// `format.plain`: remove every style set by hand (marks stay).
fn plain(s: &mut Session, _: &Value) -> CmdResult {
    apply(s, &|c| {
        for st in STYLES {
            set_style(c, st, false);
        }
        c.highlight = None;
        c.shading = None;
        c.color = None;
    })
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

/// `format.color {color}`: text color, `#RRGGBB` or `auto`.
fn color(s: &mut Session, v: &Value) -> CmdResult {
    let given = p::req_str(v, "color")?;
    let tc = TextColor::parse(given.trim()).ok_or_else(|| CmdError::Params("`color` is #RRGGBB or auto".into()))?;
    let set = if tc == TextColor::Auto { None } else { Some(tc) };
    apply(s, &move |c| c.color = set)
}

/// `format.lowercase`: the selection in small letters (a change to the text itself).
fn lowercase(s: &mut Session, _: &Value) -> CmdResult {
    if s.sel.is_collapsed() {
        return Err(CmdError::Failed("select some text first".into()));
    }
    let (a, b) = s.sel.ordered();
    for path in s.doc.para_paths_in(&a, &b) {
        let para = s.doc.para_mut(a.story, &path)?;
        let from = if path == a.path { a.off } else { 0 };
        let to = if path == b.path { b.off } else { para.len() };
        let Some(piece) = para.text.get(from..to) else { continue };
        let low = piece.to_lowercase();
        if low.len() == piece.len() && low != piece {
            para.text.replace_range(from..to, &low);
            para.touch();
        }
    }
    sel_result(s)
}

/// `format.state`: which styles are on at the selection, for menus and the toolbar.
fn state(s: &mut Session, _: &Value) -> CmdResult {
    let c = current(s);
    let on: Vec<&str> = STYLES.iter().copied().filter(|st| is_on(&c, st)).collect();
    Ok(json!({"styles": on}))
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Format › Style";
    vec![
        CommandSpec::new("format.bold", "Bold", LOC, |s, _| toggle(s, "bold")).key("Mod+B"),
        CommandSpec::new("format.italic", "Italic", LOC, |s, _| toggle(s, "italic")).key("Mod+I"),
        CommandSpec::new("format.underline", "Underline", LOC, |s, _| toggle(s, "underline")).key("Mod+U"),
        CommandSpec::new("format.style", "Text Style", LOC, style).params(r#"{"style": string, "on"?: bool}"#),
        CommandSpec::new("format.plain", "Plain Text", LOC, plain).key("Mod+Shift+T"),
        CommandSpec::new("format.highlight", "Highlight", LOC, highlight).params(r##"{"color": name | "#RRGGBB" | "none"}"##),
        CommandSpec::new("format.color", "Text Color", LOC, color).params(r##"{"color": "#RRGGBB" | "auto"}"##),
        CommandSpec::new("format.lowercase", "Small Letters", LOC, lowercase),
        CommandSpec::new("format.state", "Text Styles Here", LOC, state).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;
    use crate::data::body_pos;
    use wordsmith_engine::Selection;

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
        assert_eq!(at(&s, 2).bold, None);
        for st in STYLES {
            s.run("format.style", &json!({"style": st, "on": true})).unwrap();
            assert!(is_on(&at(&s, 2), st), "{st}");
        }
        s.run("format.plain", &json!({})).unwrap();
        assert!(s.run("format.state", &json!({})).unwrap()["styles"].as_array().is_some_and(Vec::is_empty));
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
        assert_eq!(at(&s, 1).color, None);
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
