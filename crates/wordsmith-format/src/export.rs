// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exports: plain text, RTF and HTML. PDF export is `wordsmith_script::export` (M2) and Fountain
//! is [`crate::fountain::write`].
//!
//! Each export places paragraphs as the script's own element styles say (indents, alignment,
//! space before), so a script whose elements were changed exports as it looks. None of them
//! has mores or continueds (SPEC: "RTF and plain-text exports drop mores and continueds"): they
//! belong to printed pages, and these formats are continuous. The plain-text and RTF exports
//! read back as the same elements through [`crate::import`].

use wordsmith_doc::props::{Align, Underline};
use wordsmith_doc::{Document, Paragraph};
use wordsmith_script::Elements;

/// Points per character at 10 characters per inch, the plain-text grid.
const CHAR_PT: f32 = 7.2;
/// One line, points (6 lines per inch).
const LINE_PT: f32 = 12.0;

fn paragraphs(doc: &Document) -> impl Iterator<Item = &Paragraph> {
    doc.body.iter().filter_map(|b| b.as_para())
}

/// A paragraph's text without inline objects (pictures and fields are not part of a script).
fn clean(text: &str) -> String {
    text.chars().filter(|c| *c != wordsmith_doc::para::OBJ).collect()
}

/// Lines of `text` wrapped at `width` characters on spaces (a longer word is broken).
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for hard in text.split('\n') {
        let mut line = String::new();
        let mut len = 0;
        for word in hard.split(' ').filter(|w| !w.is_empty()) {
            let mut word: Vec<char> = word.chars().collect();
            while word.len() > width {
                if len > 0 {
                    out.push(std::mem::take(&mut line));
                    len = 0;
                }
                out.push(word.drain(..width).collect());
            }
            if len > 0 && len + 1 + word.len() > width {
                out.push(std::mem::take(&mut line));
                len = 0;
            }
            if len > 0 {
                line.push(' ');
                len += 1;
            }
            line.extend(word.iter());
            len += word.len();
        }
        out.push(line);
    }
    out
}

/// Blank lines a paragraph's space before stands for.
fn blank_lines(space_before: f32) -> usize {
    (space_before / LINE_PT).round().clamp(0.0, 10.0) as usize
}

/// Plain text: each element at its indent in spaces (10 characters per inch), wrapped at its
/// width, right-aligned and centered elements placed so; blank lines for space before; a form
/// feed before a paragraph that starts a new page.
pub fn text(doc: &Document) -> String {
    let page = &doc.last_section;
    let column_pt = (page.page_w - page.margin_left - page.margin_right).max(CHAR_PT);
    let mut out = String::new();
    for (n, p) in paragraphs(doc).enumerate() {
        let r = doc.styles.resolve_para(&p.props);
        let left = (r.indent_left.max(0.0) / CHAR_PT).round() as usize;
        let width = ((column_pt - r.indent_left.max(0.0) - r.indent_right.max(0.0)) / CHAR_PT).floor().max(1.0) as usize;
        if r.page_break_before && n > 0 {
            out.push('\u{c}');
            out.push('\n');
        } else if n > 0 {
            for _ in 0..blank_lines(r.space_before) {
                out.push('\n');
            }
        }
        for line in wrap(&clean(&p.text), width) {
            let pad = match r.align {
                Align::Right => width.saturating_sub(line.chars().count()),
                Align::Center => width.saturating_sub(line.chars().count()) / 2,
                _ => 0,
            };
            if !line.is_empty() {
                out.extend(std::iter::repeat_n(' ', left + pad));
                out.push_str(&line);
            }
            out.push('\n');
        }
    }
    out
}

/// The paragraph's runs with their bold, italic and underline (the element's style, then the
/// run's own formatting).
fn styled_runs(doc: &Document, p: &Paragraph) -> Vec<(String, bool, bool, bool)> {
    let style = p.props.style.as_deref();
    p.run_ranges()
        .map(|(range, props)| {
            let c = doc.styles.resolve_char(style, props);
            (clean(p.text.get(range).unwrap_or("")), c.bold, c.italic, c.underline != Underline::None)
        })
        .filter(|(t, ..)| !t.is_empty())
        .collect()
}

fn twips(pt: f32) -> i32 {
    (pt * 20.0).round().clamp(-1_000_000.0, 1_000_000.0) as i32
}

/// RTF, through craft-rtf: one paragraph per element with its indents, alignment and space
/// before; bold, italic and underline kept; Courier Prime 12 pt.
pub fn rtf(doc: &Document) -> String {
    let blocks = paragraphs(doc)
        .enumerate()
        .map(|(n, p)| {
            let r = doc.styles.resolve_para(&p.props);
            let mut runs: Vec<craft_rtf::Run> = styled_runs(doc, p)
                .into_iter()
                .map(|(text, bold, italic, underline)| craft_rtf::Run {
                    text,
                    fmt: craft_rtf::CharFmt { bold, italic, underline, half_points: Some(24), ..craft_rtf::CharFmt::default() },
                })
                .collect();
            if r.page_break_before && n > 0 {
                runs.insert(0, craft_rtf::Run { text: craft_rtf::PAGE_BREAK.to_string(), fmt: craft_rtf::CharFmt::default() });
            }
            craft_rtf::Block::Para(craft_rtf::Para {
                runs,
                outline_level: None,
                left_indent: twips(r.indent_left),
                right_indent: twips(r.indent_right),
                first_line_indent: twips(r.indent_first),
                space_before: if n == 0 { 0 } else { twips(r.space_before) },
                space_after: twips(r.space_after),
                align: Some(match r.align {
                    Align::Center => craft_rtf::Align::Center,
                    Align::Right => craft_rtf::Align::Right,
                    Align::Justify | Align::Distribute => craft_rtf::Align::Justified,
                    Align::Left => craft_rtf::Align::Left,
                }),
            })
        })
        .collect();
    craft_rtf::write(&craft_rtf::Document { blocks, pictures_left_out: 0 }, wordsmith_script::template::SCRIPT_FONT)
}

/// Text escaped for HTML.
fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\n' => out.push_str("<br>"),
            _ => out.push(c),
        }
    }
    out
}

/// A CSS class for an element id: `el-` and the id in lower case, other characters as `-`.
fn class_of(id: &str) -> String {
    let body: String = id.chars().take(64).map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("el-{body}")
}

/// HTML: one `<p>` per paragraph with its element's class, and a small embedded stylesheet made
/// from the script's element styles (indents, alignment, space before, the element's bold,
/// italic and underline). Emphasis inside a paragraph is `<b>`, `<i>` and `<u>`.
pub fn html(doc: &Document) -> String {
    let elements = Elements::of_doc(doc);
    let page = &doc.last_section;
    let title = if doc.core.title.trim().is_empty() { "Script".to_string() } else { doc.core.title.trim().to_string() };
    let mut css = format!(
        "body {{ background: #fff; color: #000; margin: 0; }}\n\
         .script {{ box-sizing: content-box; max-width: {:.2}in; margin: 1in auto; padding: 0 1em; font-family: \"Courier Prime\", \"Courier New\", Courier, monospace; font-size: 12pt; line-height: 12pt; }}\n\
         .script p {{ margin: 0; }}\n\
         .page-break {{ break-before: page; }}\n",
        ((page.page_w - page.margin_left - page.margin_right) / 72.0).max(1.0)
    );
    let mut used: Vec<&str> = paragraphs(doc).filter_map(|p| p.props.style.as_deref()).collect();
    used.sort_unstable();
    used.dedup();
    for id in used {
        let r = doc.styles.resolve_para(&wordsmith_doc::props::ParaProps { style: Some(id.to_string()), ..Default::default() });
        let c = doc.styles.resolve_char(Some(id), &wordsmith_doc::props::CharProps::default());
        let align = match r.align {
            Align::Center => "center",
            Align::Right => "right",
            Align::Justify | Align::Distribute => "justify",
            Align::Left => "left",
        };
        css.push_str(&format!(
            ".{} {{ margin-left: {:.2}in; margin-right: {:.2}in; padding-top: {:.0}pt; text-align: {align};{}{}{} }}\n",
            class_of(id),
            r.indent_left / 72.0,
            r.indent_right / 72.0,
            r.space_before.max(0.0),
            if c.bold { " font-weight: bold;" } else { "" },
            if c.italic { " font-style: italic;" } else { "" },
            if c.underline != Underline::None { " text-decoration: underline;" } else { "" },
        ));
    }
    let mut body = String::new();
    for (n, p) in paragraphs(doc).enumerate() {
        let id = elements.of_para(p).id.clone();
        let mut class = class_of(&id);
        if p.props.page_break_before == Some(true) && n > 0 {
            class.push_str(" page-break");
        }
        body.push_str(&format!("<p class=\"{class}\">"));
        for (range, props) in p.run_ranges() {
            let text = esc(&clean(p.text.get(range).unwrap_or("")));
            if text.is_empty() {
                continue;
            }
            let (b, i, u) = (props.bold == Some(true), props.italic == Some(true), props.underline.is_some_and(|x| x != Underline::None));
            body.push_str(&format!(
                "{}{}{}{text}{}{}{}",
                if b { "<b>" } else { "" },
                if i { "<i>" } else { "" },
                if u { "<u>" } else { "" },
                if u { "</u>" } else { "" },
                if i { "</i>" } else { "" },
                if b { "</b>" } else { "" },
            ));
        }
        body.push_str("</p>\n");
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"Wordsmith Script\">\n<title>{}</title>\n<style>\n{css}</style>\n</head>\n<body>\n<div class=\"script\">\n{body}</div>\n</body>\n</html>\n",
        esc(&title)
    )
}

#[cfg(test)]
mod tests {
    use wordsmith_doc::para_block;
    use wordsmith_script::template::new_script;

    use super::*;

    fn lines(doc: &Document) -> Vec<(String, String)> {
        let e = Elements::of_doc(doc);
        paragraphs(doc).map(|p| (e.of_para(p).id.clone(), p.text.clone())).collect()
    }

    fn script() -> Document {
        let mut doc = new_script();
        doc.core.title = "Fish & <Chips>".into();
        doc.body = [
            ("SceneHeading", "INT. KITCHEN - NIGHT"),
            ("Action", "Rain **hammers** the window, on and on, long enough that the line has to wrap at the margin of the page."),
            ("Character", "ANNA (V.O.)"),
            ("Parenthetical", "(quietly)"),
            ("Dialogue", "Did you *hear* that? It sounded like the door, or the _wind_, or something worse than either."),
            ("Character", "BOB"),
            ("Dialogue", "No."),
            ("Transition", "CUT TO:"),
            ("SceneHeading", "EXT. YARD - DAY"),
            ("Action", "Quiet. \"Done\" & <dusted>."),
        ]
        .iter()
        .map(|(e, t)| para_block(crate::emphasis::decode(e, t, false)))
        .collect();
        doc
    }

    #[test]
    fn plain_text_is_laid_out_in_columns_and_reads_back() {
        let doc = script();
        let out = text(&doc);
        let rows: Vec<&str> = out.lines().collect();
        assert_eq!(rows[0], "INT. KITCHEN - NIGHT");
        assert_eq!(rows[1], "");
        assert!(rows.contains(&"                      ANNA (V.O.)"), "{out}");
        assert!(rows.contains(&"                (quietly)"), "{out}");
        assert!(rows.iter().any(|r| r.starts_with("          Did you hear that?")), "{out}");
        let cut = rows.iter().find(|r| r.ends_with("CUT TO:")).unwrap();
        assert_eq!(cut.len(), 60, "right-aligned to the margin: {cut:?}");
        assert!(rows.iter().all(|r| r.chars().count() <= 60), "nothing past the right margin");
        assert!(!out.contains("(MORE)") && !out.contains("CONT'D"));
        assert_eq!(lines(&crate::import::text(&out, true).unwrap()), lines(&doc));
    }

    #[test]
    fn page_breaks_become_form_feeds() {
        let mut doc = script();
        let mut p = crate::emphasis::decode("SceneHeading", "EXT. YARD - DAY", false);
        p.props.page_break_before = Some(true);
        doc.body[8] = para_block(p);
        let out = text(&doc);
        assert!(out.contains("\u{c}\nEXT. YARD - DAY"), "{out}");
        assert_eq!(lines(&crate::import::text(&out, true).unwrap()), lines(&doc));
    }

    #[test]
    fn rtf_keeps_emphasis_and_reads_back_as_the_same_elements() {
        let doc = script();
        let out = rtf(&doc);
        let read = craft_rtf::read(out.as_bytes()).unwrap();
        let bold: Vec<String> = read.paras().flat_map(|p| p.runs.iter().filter(|r| r.fmt.bold).map(|r| r.text.clone())).collect();
        assert_eq!(bold, ["hammers"]);
        let italic: Vec<String> = read.paras().flat_map(|p| p.runs.iter().filter(|r| r.fmt.italic).map(|r| r.text.clone())).collect();
        assert_eq!(italic, ["hear"]);
        assert!(read.paras().any(|p| p.runs.iter().any(|r| r.fmt.underline && r.text == "wind")));
        assert!(!out.contains("(MORE)") && !out.contains("CONT'D"));
        assert_eq!(lines(&crate::import::rtf(out.as_bytes(), true).unwrap()), lines(&doc));
    }

    #[test]
    fn html_is_escaped_balanced_and_styled() {
        let out = html(&script());
        assert!(out.contains("<title>Fish &amp; &lt;Chips&gt;</title>"));
        assert!(out.contains("Quiet. &quot;Done&quot; &amp; &lt;dusted&gt;."));
        assert!(out.contains("<p class=\"el-character\">ANNA (V.O.)</p>"));
        assert!(out.contains("Rain <b>hammers</b> the window"));
        assert!(out.contains("<i>hear</i>") && out.contains("<u>wind</u>"));
        assert!(out.contains(".el-dialogue { margin-left: 1.00in; margin-right: 1.50in;"), "{out}");
        assert!(out.contains(".el-transition {") && out.contains("text-align: right;"));
        for (open, close) in [
            ("<p ", "</p>"),
            ("<b>", "</b>"),
            ("<i>", "</i>"),
            ("<u>", "</u>"),
            ("<div ", "</div>"),
            ("<html ", "</html>"),
            ("<head>", "</head>"),
            ("<body>", "</body>"),
            ("<style>", "</style>"),
            ("<title>", "</title>"),
        ] {
            assert_eq!(out.matches(open).count(), out.matches(close).count(), "{open} balanced");
        }
    }

    #[test]
    fn wrapping_and_hostile_text() {
        assert_eq!(wrap("aaa bbb ccc", 7), ["aaa bbb", "ccc"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("a\n\nb", 10), ["a", "", "b"]);
        assert_eq!(wrap("", 0), [""]);
        assert_eq!(class_of("My Element!"), "el-my-element-");
        let mut doc = new_script();
        doc.body = vec![para_block(wordsmith_script::template::paragraph("Unknown", "<script>alert(1)</script>"))];
        let out = html(&doc);
        assert!(!out.contains("<script>"));
        let _ = (text(&doc), rtf(&doc));
    }
}
