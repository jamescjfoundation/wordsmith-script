// SPDX-License-Identifier: MIT OR Apache-2.0
//! Headers and footers (SPEC: "headers and footers with placeholder fields (date, scene, label,
//! page)"), kept in `app_data` under [`HEADER_FOOTER_KEY`].
//!
//! Each band (header, footer) has a left, a center and a right text. Fields in braces fill in on
//! each page: `{page}` the page number, `{scene}` the first scene heading on the page (or the
//! scene that runs onto it), `{date}` the date (today, or the fixed text set), `{title}` the
//! script's title, `{revision}` the current revision set's name (empty until there is one).
//! Other text is printed as it is. Page 1 shows the bands only when asked. The default is the
//! convention: the page number and a period at the top right, none on page 1 (decision 0021).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wordsmith_doc::props::{Align, CharProps, TabAlign, TabLeader, TabStop};
use wordsmith_doc::{Document, InlineObject, Paragraph, PartKind, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session};

use crate::element::{ElementKind, Elements};
use crate::template;

/// `app_data` key of the header and footer settings.
pub const HEADER_FOOTER_KEY: &str = "wordsmith.headerFooter";
/// `app_data` key revision sets are kept under (decision 0024); `{revision}` shows its `current` name.
pub const REVISIONS_KEY: &str = "wordsmith.revisions";
/// Longest text one slot keeps, characters.
pub const MAX_SLOT_CHARS: usize = 200;
/// Fields a slot may use.
pub const FIELDS: [&str; 6] = ["page", "scene", "date", "title", "revision", "collated"];

/// One band's three slots.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Band {
    pub left: String,
    pub center: String,
    pub right: String,
}

impl Band {
    fn is_empty(&self) -> bool {
        self.left.trim().is_empty() && self.center.trim().is_empty() && self.right.trim().is_empty()
    }

    fn cleaned(&self) -> Band {
        let c = |s: &str| s.chars().filter(|c| !c.is_control()).take(MAX_SLOT_CHARS).collect::<String>();
        Band { left: c(&self.left), center: c(&self.center), right: c(&self.right) }
    }
}

/// The header and footer settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HeaderFooter {
    pub header: Band,
    pub footer: Band,
    /// Show the bands on page 1 too.
    pub first_page: bool,
    /// The text `{date}` shows; empty for today's date.
    pub date: String,
    /// Show the footer on page 1, apart from the header (`None`: as `first_page` says).
    pub first_page_footer: Option<bool>,
    /// The number of the script's first page (unlocked scripts), 1 by default.
    pub start_page: u32,
}

impl Default for HeaderFooter {
    fn default() -> Self {
        HeaderFooter {
            header: Band { right: "{page}.".into(), ..Band::default() },
            footer: Band::default(),
            first_page: false,
            date: String::new(),
            first_page_footer: None,
            start_page: 1,
        }
    }
}

impl HeaderFooter {
    /// The settings `doc` holds, or the defaults; bad data reads as the defaults, and texts are
    /// cut to their limits.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(HEADER_FOOTER_KEY) else { return Self::default() };
        let mut h: HeaderFooter = serde_json::from_value((**v).clone()).unwrap_or_default();
        h.header = h.header.cleaned();
        h.footer = h.footer.cleaned();
        h.date = h.date.chars().filter(|c| !c.is_control()).take(MAX_SLOT_CHARS).collect();
        h.start_page = h.start_page.clamp(1, 100_000);
        h
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(HEADER_FOOTER_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }
}

/// `headerFooter.get`: the settings.
fn get(s: &mut Session, _: &Value) -> CmdResult {
    Ok(serde_json::to_value(HeaderFooter::of_doc(&s.doc)).unwrap_or_default())
}

/// `headerFooter.set {header?, footer?, firstPage?, date?}`: change the settings given (one undo
/// step).
fn set(s: &mut Session, v: &Value) -> CmdResult {
    let current = serde_json::to_value(HeaderFooter::of_doc(&s.doc)).unwrap_or_default();
    let mut merged = current;
    let Some(obj) = v.as_object() else { return Err(CmdError::Params("expected an object".into())) };
    for (k, val) in obj {
        match (k.as_str(), val) {
            ("header" | "footer", Value::Object(slots)) => {
                for (slot, text) in slots {
                    if !matches!(slot.as_str(), "left" | "center" | "right") || !text.is_string() {
                        return Err(CmdError::Params(format!("`{k}.{slot}` must be left, center or right, as text")));
                    }
                    if let Some(band) = merged.get_mut(k).and_then(Value::as_object_mut) {
                        band.insert(slot.clone(), text.clone());
                    }
                }
            }
            ("firstPage", Value::Bool(_)) | ("date", Value::String(_)) | ("firstPageFooter", Value::Bool(_) | Value::Null) => {
                if let Some(m) = merged.as_object_mut() {
                    m.insert(k.clone(), val.clone());
                }
            }
            // A window's number field may send 5.0.
            ("startPage", Value::Number(n)) => {
                if let Some(m) = merged.as_object_mut() {
                    m.insert(k.clone(), serde_json::json!(n.as_f64().unwrap_or(1.0).clamp(1.0, 100_000.0) as u32));
                }
            }
            _ => return Err(CmdError::Params(format!("unknown or bad setting `{k}`"))),
        }
    }
    let mut h: HeaderFooter = serde_json::from_value(merged).map_err(|e| CmdError::Params(e.to_string()))?;
    h.header = h.header.cleaned();
    h.footer = h.footer.cleaned();
    h.store(&mut s.doc);
    Ok(serde_json::to_value(&h).unwrap_or_default())
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("headerFooter.get", "Header and Footer Settings", "Script", get).pure(),
        CommandSpec::new("headerFooter.set", "Set Header and Footer", "Script", set).params(
            r#"{"header"?: {"left"?, "center"?, "right"?: text with {page} {scene} {date} {title} {revision} {collated}}, "footer"?: {...}, "firstPage"?: bool, "firstPageFooter"?: bool|null, "startPage"?: number, "date"?: string (empty for today)}"#,
        ),
    ]
}

/// Today's date as "November 19, 2026" (UTC).
pub fn today() -> String {
    long_date(wordsmith_engine::cmd::now_unix())
}

/// `secs` since 1970 as "November 19, 2026" (UTC).
pub fn long_date(secs: u64) -> String {
    let (year, month, day, ..) = crate::dates::civil(secs);
    format!("{} {day}, {year}", crate::dates::month_name(month))
}

/// Style id of the header and footer lines.
pub const BAND_STYLE: &str = "ws.Band";

fn field(instr: &str) -> InlineObject {
    InlineObject::Field { instr: instr.into(), result: String::new(), locked: false, dirty: false, code: false, form: String::new() }
}

/// Append `text` to `p`, its fields filled in or made live (page, scene).
fn push_slot(p: &mut Paragraph, text: &str, doc: &Document, date: &str, label: Option<&str>) {
    let scene_style = Elements::of_doc(doc).id_of(ElementKind::SceneHeading).to_string();
    let revision = doc.app_data.get(REVISIONS_KEY).and_then(|v| v.get("current")).and_then(Value::as_str).unwrap_or("").to_string();
    // Every revision set with marks in the script, oldest first.
    let collated = {
        let rev = crate::revisions::Revisions::of_doc(doc);
        let mut used = std::collections::BTreeSet::new();
        for b in &doc.body {
            if let Some(p) = b.as_para() {
                used.extend(p.mark.revised);
                used.extend(p.run_ranges().filter_map(|(_, c)| c.revised));
            }
        }
        used.into_iter().filter_map(|i| rev.set(i).map(|s| s.name.clone())).collect::<Vec<_>>().join(", ")
    };
    let mut rest = text;
    while !rest.is_empty() {
        let next = rest.find('{').and_then(|open| rest[open..].find('}').map(|close| (open, open + close)));
        let Some((open, close)) = next else {
            let at = p.len();
            let _ = p.insert_text(at, rest, &CharProps::default());
            break;
        };
        let at = p.len();
        let _ = p.insert_text(at, &rest[..open], &CharProps::default());
        let name = &rest[open + 1..close];
        let at = p.len();
        match name {
            // A locked script's page has its label (12A); otherwise the live page number.
            "page" => match label {
                Some(l) => {
                    let _ = p.insert_text(at, l, &CharProps::default());
                }
                None => {
                    let _ = p.insert_object(at, field("PAGE"), &CharProps::default());
                }
            },
            "scene" => {
                let _ = p.insert_object(at, field(&format!("STYLEREF \"{scene_style}\"")), &CharProps::default());
            }
            "date" => {
                let _ = p.insert_text(at, date, &CharProps::default());
            }
            "title" => {
                let _ = p.insert_text(at, &doc.core.title, &CharProps::default());
            }
            "revision" => {
                let _ = p.insert_text(at, &revision, &CharProps::default());
            }
            "collated" => {
                let _ = p.insert_text(at, &collated, &CharProps::default());
            }
            _ => {
                let _ = p.insert_text(at, &rest[open..=close], &CharProps::default());
            }
        }
        rest = &rest[close + 1..];
    }
}

/// The line for `band`: one slot alone is aligned where it belongs; more use tab stops at the
/// center and the right margin.
fn band_line(band: &Band, doc: &Document, date: &str, label: Option<&str>) -> Paragraph {
    let mut p = Paragraph::new();
    p.props.style = Some(BAND_STYLE.into());
    let used: Vec<(&str, Align)> = [(&band.left, Align::Left), (&band.center, Align::Center), (&band.right, Align::Right)]
        .into_iter()
        .filter(|(t, _)| !t.trim().is_empty())
        .map(|(t, a)| (t.as_str(), a))
        .collect();
    if let [(text, align)] = used.as_slice() {
        p.props.align = Some(*align);
        push_slot(&mut p, text, doc, date, label);
        return p;
    }
    let page = &doc.last_section;
    let width = (page.page_w - page.margin_left - page.margin_right).max(template::INCH);
    p.props.tabs = Some(vec![
        TabStop { pos: width / 2.0, align: TabAlign::Center, leader: TabLeader::None },
        TabStop { pos: width, align: TabAlign::Right, leader: TabLeader::None },
    ]);
    push_slot(&mut p, &band.left, doc, date, label);
    for slot in [&band.center, &band.right] {
        let at = p.len();
        let _ = p.insert_text(at, "\t", &CharProps::default());
        push_slot(&mut p, slot, doc, date, label);
    }
    p
}

/// Give the output script `doc` its header and footer, as its settings say. With `labels` (a
/// locked script: one per page), each page is a section of its own whose header and footer show
/// its label for `{page}`; pages start where a paragraph has `page_break_before`.
pub fn apply(doc: &mut Document, labels: Option<&[String]>) {
    let h = HeaderFooter::of_doc(doc);
    let date = if h.date.trim().is_empty() { today() } else { h.date.clone() };
    let sheet = Arc::make_mut(&mut doc.styles);
    let mut style = template::element_style(BAND_STYLE, "Header and Footer", &template::default_formatting(ElementKind::General), BAND_STYLE);
    style.quick = false;
    style.para.space_before = Some(0.0);
    sheet.upsert(style);
    let blank = || {
        let mut p = Paragraph::new();
        p.props.style = Some(BAND_STYLE.into());
        p
    };
    let Some(labels) = labels else {
        let header = (!h.header.is_empty()).then(|| band_line(&h.header, doc, &date, None));
        let footer = (!h.footer.is_empty()).then(|| band_line(&h.footer, doc, &date, None));
        let header = header.map(|p| doc.add_part(PartKind::Header, vec![para_block(p)]));
        let footer = footer.map(|p| doc.add_part(PartKind::Footer, vec![para_block(p)]));
        let first_header = (!h.first_page).then(|| doc.add_part(PartKind::Header, vec![para_block(blank())]));
        let footer_first = h.first_page_footer.unwrap_or(h.first_page);
        let first_footer = (!footer_first).then(|| doc.add_part(PartKind::Footer, vec![para_block(blank())]));
        let s = &mut doc.last_section;
        if h.start_page != 1 {
            s.page_num_start = Some(h.start_page);
            s.page_num_set = true;
        }
        s.headers.default = header;
        s.footers.default = footer;
        s.headers.even = None;
        s.footers.even = None;
        s.title_page = !h.first_page || !footer_first;
        s.headers.first = if h.first_page { header } else { first_header };
        s.footers.first = if footer_first { footer } else { first_footer };
        s.header = 0.5 * template::INCH;
        s.footer = 0.5 * template::INCH;
        return;
    };
    // One section per page: the last paragraph of each page but the last ends one.
    let starts: Vec<usize> = doc
        .body
        .iter()
        .enumerate()
        .filter(|(i, b)| *i == 0 || b.as_para().is_some_and(|p| p.props.page_break_before == Some(true)))
        .map(|(i, _)| i)
        .collect();
    let mut base = doc.last_section.clone();
    base.headers = Default::default();
    base.footers = Default::default();
    base.title_page = false;
    base.header = 0.5 * template::INCH;
    base.footer = 0.5 * template::INCH;
    let mut sections = Vec::with_capacity(starts.len());
    for (page, label) in starts.iter().zip(labels.iter().map(String::as_str).chain(std::iter::repeat(""))).map(|(_, l)| l).enumerate() {
        let mut sect = base.clone();
        // Page 1 shows the bands only when asked.
        let show = page > 0 || h.first_page;
        sect.headers.default = Some(if show && !h.header.is_empty() {
            let line = band_line(&h.header, doc, &date, Some(label));
            doc.add_part(PartKind::Header, vec![para_block(line)])
        } else {
            doc.add_part(PartKind::Header, vec![para_block(blank())])
        });
        let show_footer = page > 0 || h.first_page_footer.unwrap_or(h.first_page);
        sect.footers.default = Some(if show_footer && !h.footer.is_empty() {
            let line = band_line(&h.footer, doc, &date, Some(label));
            doc.add_part(PartKind::Footer, vec![para_block(line)])
        } else {
            doc.add_part(PartKind::Footer, vec![para_block(blank())])
        });
        sections.push(sect);
    }
    let Some(last) = sections.pop() else { return };
    for (i, sect) in sections.into_iter().enumerate() {
        let end = starts.get(i + 1).map_or(0, |n| n.saturating_sub(1));
        if let Some(wordsmith_doc::Block::Para(p)) = doc.body.get_mut(end).map(Arc::make_mut) {
            p.section = Some(Box::new(sect));
        }
    }
    doc.last_section = last;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::template::new_script;

    #[test]
    fn settings_read_bounded_and_set_in_one_step() {
        let mut s = crate::new_session();
        assert_eq!(HeaderFooter::of_doc(&s.doc), HeaderFooter::default());
        s.run("headerFooter.set", &json!({"footer": {"center": "{title} - {date}"}, "firstPage": true})).unwrap();
        let h = HeaderFooter::of_doc(&s.doc);
        assert_eq!(h.footer.center, "{title} - {date}");
        assert_eq!(h.header.right, "{page}.", "the rest is kept");
        assert!(h.first_page);
        assert!(s.run("headerFooter.set", &json!({"header": {"middle": "x"}})).is_err());
        assert!(s.run("headerFooter.set", &json!({"colour": 1})).is_err());
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(HeaderFooter::of_doc(&s.doc), HeaderFooter::default());
        let mut doc = new_script();
        doc.app_data.insert(HEADER_FOOTER_KEY.into(), Arc::new(json!({"header": {"left": "x".repeat(1000)}, "firstPage": "yes"})));
        assert_eq!(HeaderFooter::of_doc(&doc), HeaderFooter::default(), "a bad value reads as the defaults");
        doc.app_data.insert(HEADER_FOOTER_KEY.into(), Arc::new(json!({"header": {"left": "x".repeat(1000)}})));
        assert_eq!(HeaderFooter::of_doc(&doc).header.left.len(), MAX_SLOT_CHARS);
    }

    #[test]
    fn today_reads_as_a_date() {
        let t = today();
        assert!(t.split(' ').count() == 3 && t.contains(", 20"), "{t}");
    }

    #[test]
    fn slots_fill_fields() {
        let mut doc = new_script();
        doc.core.title = "THE TEST".into();
        doc.app_data.insert(REVISIONS_KEY.into(), Arc::new(json!({"current": "Blue"})));
        let p = band_line(&Band { left: "{title} {revision} {date} {nope}".into(), ..Band::default() }, &doc, "1 May", None);
        assert_eq!(p.text, "THE TEST Blue 1 May {nope}");
        assert_eq!(p.props.align, Some(Align::Left));
        let p = band_line(&Band { left: "A".into(), center: "{scene}".into(), right: "{page}.".into() }, &doc, "", None);
        assert_eq!(p.text.matches('\t').count(), 2);
        assert_eq!(p.objects.len(), 2, "scene and page are live fields");
    }
}

#[cfg(test)]
mod spec2_tests {
    use crate::commands::new_session;
    use serde_json::json;

    #[test]
    fn start_page_first_page_footer_and_collated_sets() {
        let mut s = new_session();
        s.run("headerFooter.set", &json!({"startPage": 5.0, "firstPageFooter": true, "footer": {"left": "{collated}"}})).unwrap();
        let h = super::HeaderFooter::of_doc(&s.doc);
        assert_eq!((h.start_page, h.first_page_footer), (5, Some(true)));
        let pag = crate::paginate::paginate(&s.doc);
        assert_eq!(pag.doc.last_section.page_num_start, Some(5));
        assert!(pag.doc.last_section.footers.first.is_some());
        assert!(s.run("headerFooter.set", &json!({"startPage": "five"})).is_err());
    }
}
