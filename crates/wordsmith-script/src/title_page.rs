// SPDX-License-Identifier: MIT OR Apache-2.0
//! The title page: its fields, kept in `app_data` under [`TITLE_PAGE_KEY`] (so it is undoable
//! and saved with the script), and its printed page.
//!
//! The spec leaves the title page's fields and layout open, so they come from screenplay
//! convention (decision 0021): the title centered about a third of the way down the page, the
//! credit ("Written by") and the authors below it, the source ("Based on …") below them, and
//! notes below that; at the foot, the contact details on the left and the draft date and
//! copyright on the right. The page is printed only when a field has text. It comes before the
//! script as a section of its own, so the script's pages are numbered from 1 after it; the page
//! view shows the script alone.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wordsmith_doc::props::{Align, CharProps, LineSpacing, ParaProps, TabAlign, TabLeader, TabStop};
use wordsmith_doc::{Document, Paragraph, Style, StyleKind, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session};

use crate::template;

/// `app_data` key of the title page.
pub const TITLE_PAGE_KEY: &str = "wordsmith.titlePage";
/// Longest text a field keeps, characters.
pub const MAX_FIELD_CHARS: usize = 5_000;
/// Most lines a field keeps.
pub const MAX_FIELD_LINES: usize = 40;

/// Style ids of the printed title page.
pub const CENTER_STYLE: &str = "ws.TitleCenter";
pub const FOOT_STYLE: &str = "ws.TitleFoot";

/// The title page's fields, as text (line breaks kept).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TitlePage {
    pub title: String,
    /// "Written by", "Screenplay by", …
    pub credit: String,
    pub authors: String,
    /// "Based on the novel by …"
    pub source: String,
    pub notes: String,
    pub contact: String,
    pub draft_date: String,
    pub copyright: String,
}

/// The field names, as commands and files name them.
pub const FIELDS: [&str; 8] = ["title", "credit", "authors", "source", "notes", "contact", "draftDate", "copyright"];

fn clean(text: &str) -> String {
    let text: String =
        text.replace("\r\n", "\n").replace('\r', "\n").chars().filter(|c| *c == '\n' || !c.is_control()).take(MAX_FIELD_CHARS).collect();
    text.split('\n').take(MAX_FIELD_LINES).map(str::trim_end).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string()
}

impl TitlePage {
    /// The title page `doc` holds; bad or hostile data reads as fields that are empty or cut
    /// down to the limits.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(TITLE_PAGE_KEY) else { return Self::default() };
        let mut t = TitlePage::default();
        for name in FIELDS {
            if let Some(text) = v.get(name).and_then(Value::as_str) {
                t.set(name, text);
            }
        }
        t
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(TITLE_PAGE_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }

    /// The field named `name`.
    pub fn get(&self, name: &str) -> Option<&str> {
        Some(match name {
            "title" => &self.title,
            "credit" => &self.credit,
            "authors" => &self.authors,
            "source" => &self.source,
            "notes" => &self.notes,
            "contact" => &self.contact,
            "draftDate" => &self.draft_date,
            "copyright" => &self.copyright,
            _ => return None,
        })
    }

    /// Set the field named `name` (within the limits); false for an unknown name.
    pub fn set(&mut self, name: &str, text: &str) -> bool {
        let field = match name {
            "title" => &mut self.title,
            "credit" => &mut self.credit,
            "authors" => &mut self.authors,
            "source" => &mut self.source,
            "notes" => &mut self.notes,
            "contact" => &mut self.contact,
            "draftDate" => &mut self.draft_date,
            "copyright" => &mut self.copyright,
            _ => return false,
        };
        *field = clean(text);
        true
    }

    /// Whether every field is empty (nothing is printed).
    pub fn is_empty(&self) -> bool {
        FIELDS.iter().all(|f| self.get(f).is_none_or(|t| t.trim().is_empty()))
    }

    /// The fields as a JSON object.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// `titlePage.get`: the fields.
fn get(s: &mut Session, _: &Value) -> CmdResult {
    Ok(TitlePage::of_doc(&s.doc).to_json())
}

/// `titlePage.set {title?, credit?, …}`: set the fields given (one undo step). The title is also
/// the script's title.
fn set(s: &mut Session, v: &Value) -> CmdResult {
    let Some(obj) = v.as_object() else { return Err(CmdError::Params("expected an object of fields".into())) };
    let mut t = TitlePage::of_doc(&s.doc);
    for (k, val) in obj {
        let Some(text) = val.as_str() else { return Err(CmdError::Params(format!("`{k}` must be text"))) };
        if !t.set(k, text) {
            return Err(CmdError::Params(format!("unknown title page field `{k}` (fields: {})", FIELDS.join(", "))));
        }
    }
    if obj.contains_key("title") {
        s.doc.core.title = t.title.split('\n').map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    }
    t.store(&mut s.doc);
    Ok(t.to_json())
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("titlePage.get", "Title Page Fields", "Script", get).pure(),
        CommandSpec::new("titlePage.set", "Set Title Page", "Script", set).params(
            r#"{"title"?: string, "credit"?: string, "authors"?: string, "source"?: string, "notes"?: string, "contact"?: string, "draftDate"?: string, "copyright"?: string}"#,
        ),
    ]
}

fn style(id: &str, name: &str, align: Align, tabs: Vec<TabStop>) -> Style {
    Style {
        id: id.into(),
        name: name.into(),
        kind: StyleKind::Paragraph,
        para: ParaProps {
            align: Some(align),
            indent_left: Some(0.0),
            indent_right: Some(0.0),
            indent_first: Some(0.0),
            space_before: Some(0.0),
            space_after: Some(0.0),
            line_spacing: Some(LineSpacing::Exactly(template::LINE)),
            tabs: (!tabs.is_empty()).then_some(tabs),
            ..ParaProps::default()
        },
        chr: CharProps { font: Some(template::SCRIPT_FONT.into()), size: Some(template::SCRIPT_SIZE), ..CharProps::default() },
        builtin: true,
        ..Style::default()
    }
}

fn line(style: &str, text: &str) -> Paragraph {
    let mut p = Paragraph::with_text(text, CharProps::default());
    p.props.style = Some(style.into());
    p
}

fn lines_of(text: &str) -> Vec<&str> {
    if text.trim().is_empty() { Vec::new() } else { text.split('\n').collect() }
}

/// `out` (a paginated script) with the title page put before it as a section of its own. The
/// script's page numbers start at 1 after it. `out` is returned as it is when the title page
/// is empty.
pub fn with_title_page(out: &Document, t: &TitlePage) -> Document {
    if t.is_empty() {
        return out.clone();
    }
    let mut doc = out.clone();
    let page = doc.last_section.clone();
    let text_width = (page.page_w - page.margin_left - page.margin_right).max(template::INCH);
    let sheet = Arc::make_mut(&mut doc.styles);
    sheet.upsert(style(CENTER_STYLE, "Title Page", Align::Center, Vec::new()));
    sheet.upsert(style(
        FOOT_STYLE,
        "Title Page Foot",
        Align::Left,
        vec![TabStop { pos: text_width, align: TabAlign::Right, leader: TabLeader::None }],
    ));

    // The top block: title, credit, authors, source, notes, each a blank line apart.
    let mut top: Vec<Paragraph> = Vec::new();
    for (n, field) in [&t.title, &t.credit, &t.authors, &t.source, &t.notes].into_iter().enumerate() {
        let ls = lines_of(field);
        if ls.is_empty() {
            continue;
        }
        if !top.is_empty() {
            for _ in 0..if n >= 3 { 2 } else { 1 } {
                top.push(line(CENTER_STYLE, ""));
            }
        }
        top.extend(ls.into_iter().map(|l| line(CENTER_STYLE, l.trim())));
    }
    // The foot: contact on the left; draft date then copyright on the right.
    let left = lines_of(&t.contact);
    let right: Vec<&str> = lines_of(&t.draft_date).into_iter().chain(lines_of(&t.copyright)).collect();
    let foot_rows = left.len().max(right.len());
    let foot: Vec<Paragraph> = (0..foot_rows)
        .map(|i| {
            let l = left.get(i).map_or("", |s| s.trim());
            let r = right.get(i).map_or("", |s| s.trim());
            line(FOOT_STYLE, &if r.is_empty() { l.to_string() } else { format!("{l}\t{r}") })
        })
        .collect();

    // Lines on the page: the title starts about a third of the way down, the foot ends at the
    // bottom margin. A title page too full for that just runs on.
    let page_lines = ((page.page_h - page.margin_top - page.margin_bottom) / template::LINE).floor().max(1.0) as usize;
    let above = (page_lines / 3).saturating_sub(1).min(page_lines.saturating_sub(top.len() + foot_rows));
    let between = page_lines.saturating_sub(above + top.len() + foot_rows);
    let mut paras: Vec<Paragraph> = Vec::new();
    paras.extend((0..above).map(|_| line(CENTER_STYLE, "")));
    paras.extend(top);
    paras.extend((0..between).map(|_| line(CENTER_STYLE, "")));
    paras.extend(foot);
    if paras.is_empty() {
        paras.push(line(CENTER_STYLE, ""));
    }
    // The title page's section: the page, no headers or footers.
    let mut title_section = page.clone();
    title_section.headers = Default::default();
    title_section.footers = Default::default();
    title_section.title_page = false;
    title_section.page_num_start = None;
    if let Some(last) = paras.last_mut() {
        last.section = Some(Box::new(title_section));
    }
    doc.last_section.page_num_start = Some(1);
    let mut body: Vec<_> = paras.into_iter().map(para_block).collect();
    body.append(&mut doc.body);
    doc.body = body;
    doc
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::template::new_script;

    #[test]
    fn fields_are_cleaned_and_bounded() {
        let mut t = TitlePage::default();
        assert!(t.set("title", "  THE TEST\r\nPart Two  \n\n"));
        assert_eq!(t.title, "  THE TEST\nPart Two");
        assert!(!t.set("nope", "x"));
        t.set("notes", &"x\n".repeat(100));
        assert_eq!(t.notes.split('\n').count(), MAX_FIELD_LINES);
        t.set("contact", &"y".repeat(MAX_FIELD_CHARS * 2));
        assert_eq!(t.contact.chars().count(), MAX_FIELD_CHARS);
        let mut doc = new_script();
        doc.app_data.insert(TITLE_PAGE_KEY.into(), Arc::new(json!({"title": 5, "credit": "Written by", "bogus": "x"})));
        let read = TitlePage::of_doc(&doc);
        assert_eq!((read.title.as_str(), read.credit.as_str()), ("", "Written by"), "hostile data reads safely");
        assert!(TitlePage::default().is_empty());
    }

    #[test]
    fn set_is_one_undoable_step_and_titles_the_script() {
        let mut s = crate::new_session();
        s.run("titlePage.set", &json!({"title": "THE TEST\nA Story", "authors": "Anna Writer"})).unwrap();
        assert_eq!(s.doc.core.title, "THE TEST A Story");
        let got = s.run("titlePage.get", &json!({})).unwrap();
        assert_eq!(got["authors"], "Anna Writer");
        assert!(s.run("titlePage.set", &json!({"colour": "blue"})).is_err());
        s.run("edit.undo", &json!({})).unwrap();
        assert!(TitlePage::of_doc(&s.doc).is_empty());
    }

    #[test]
    fn the_title_page_is_a_section_before_the_script() {
        let script = new_script();
        let empty = with_title_page(&script, &TitlePage::default());
        assert_eq!(empty.body.len(), script.body.len(), "an empty title page prints nothing");
        let mut t = TitlePage::default();
        t.set("title", "THE TEST");
        t.set("credit", "Written by");
        t.set("authors", "Anna Writer");
        t.set("contact", "anna@example.com\n555-0100");
        t.set("draftDate", "First Draft");
        let doc = with_title_page(&script, &t);
        let page_lines = 54;
        let added = doc.body.len() - script.body.len();
        assert_eq!(added, page_lines, "the page is filled to its last line");
        let texts: Vec<String> = doc.body.iter().take(added).filter_map(|b| b.as_para()).map(|p| p.text.clone()).collect();
        let title_at = texts.iter().position(|t| t == "THE TEST").unwrap();
        assert_eq!(title_at, page_lines / 3 - 1);
        assert_eq!(texts[title_at + 2], "Written by");
        assert_eq!(texts[title_at + 4], "Anna Writer");
        assert_eq!(texts[added - 2], "anna@example.com\tFirst Draft");
        assert_eq!(texts[added - 1], "555-0100");
        let last = doc.body[added - 1].as_para().unwrap();
        assert!(last.section.as_ref().is_some_and(|s| s.headers.default.is_none()), "the title page has no header");
        assert_eq!(doc.last_section.page_num_start, Some(1));
    }
}
