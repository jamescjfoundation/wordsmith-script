// SPDX-License-Identifier: MIT OR Apache-2.0
//! Headers and footers (SPEC: "headers and footers with placeholder fields (date, scene, label,
//! page)") are the core's (`jcj_features::header_footer`, document data `core.header_footer`).
//!
//! Each band (header, footer) has a left, a center and a right text. Fields in braces fill in on
//! each page: `{page}` the page number, `{date}` the date (today, or the fixed text set),
//! `{title}` the script's title, `{revision}` the current revision set's name and `{collated}`
//! the sets used, from the core; and Script's own `{scene}`, the first scene heading on the page
//! (or the scene that runs onto it). The default is the convention: the page number and a period
//! at the top right, none on page 1 (decision 0021).
//!
//! Script sets the bands in its own type: the output gets Header and Footer paragraph styles in
//! the script's Courier, with no space before.

use std::sync::Arc;

use jcj_doc::Document;
use jcj_features::header_footer::{FieldSource, FieldValue};

pub use jcj_features::dates::{long_date, today};
pub use jcj_features::header_footer::{Band, HEADER_FOOTER, HeaderFooter, MAX_SLOT_CHARS, band_line};

use crate::element::{ElementKind, Elements};
use crate::template;

/// Fields a slot may use: the core's and `{scene}`.
pub const FIELDS: [&str; 6] = ["page", "scene", "date", "title", "revision", "collated"];

/// The paragraph styles of the header and footer lines.
pub const BAND_STYLES: [&str; 2] = ["Header", "Footer"];

/// Script's own field: `{scene}`, the scene heading in force on the page.
#[derive(Clone, Copy, Debug, Default)]
pub struct SceneField;

impl FieldSource for SceneField {
    fn field(&self, name: &str, doc: &Document) -> Option<FieldValue> {
        (name == "scene").then(|| {
            let style = Elements::of_doc(doc).id_of(ElementKind::SceneHeading).to_string();
            FieldValue::Live(format!("STYLEREF \"{style}\""))
        })
    }

    fn names(&self) -> Vec<String> {
        vec!["scene".into()]
    }
}

/// Give the output script `doc` its header and footer, as its settings say (labels: a locked
/// script's page labels, one per page).
pub fn apply(doc: &mut Document, labels: Option<&[String]>) {
    let sheet = Arc::make_mut(&mut doc.styles);
    for (id, name) in BAND_STYLES.into_iter().zip(["Header", "Footer"]) {
        let mut style = template::element_style(id, name, &template::default_formatting(ElementKind::General), id);
        style.quick = false;
        style.para.space_before = Some(0.0);
        sheet.upsert(style);
    }
    jcj_features::header_footer::apply(doc, labels, &SceneField);
}

#[cfg(test)]
mod tests {
    use jcj_doc::props::Align;
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
        let _ = doc.ext.set(HEADER_FOOTER.key(), json!({"header": {"left": "x".repeat(1000)}, "firstPage": "yes"}));
        assert_eq!(HeaderFooter::of_doc(&doc), HeaderFooter::default(), "a bad value reads as the defaults");
        let _ = doc.ext.set(HEADER_FOOTER.key(), json!({"header": {"left": "x".repeat(1000)}}));
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
        let mut rev = crate::revisions::Revisions::of_doc(&doc);
        rev.start_next();
        rev.store(&mut doc).unwrap();
        let name = crate::revisions::Revisions::of_doc(&doc).current;
        assert!(!name.is_empty());
        let h = HeaderFooter { date: "1 May".into(), ..HeaderFooter::default() };
        h.store(&mut doc).unwrap();
        let p = band_line(&doc, &Band { left: "{title} {revision} {date} {nope}".into(), ..Band::default() }, None, &SceneField);
        assert_eq!(p.text, format!("THE TEST {name} 1 May {{nope}}"));
        assert_eq!(p.props.align, Some(Align::Left));
        let p = band_line(&doc, &Band { left: "A".into(), center: "{scene}".into(), right: "{page}.".into() }, None, &SceneField);
        assert_eq!(p.text.matches('\t').count(), 2);
        assert_eq!(p.objects.len(), 2, "scene and page are live fields");
    }

    #[test]
    fn start_page_first_page_footer_and_collated_sets() {
        let mut s = crate::new_session();
        s.run("headerFooter.set", &json!({"startPage": 5.0, "firstPageFooter": true, "footer": {"left": "{collated}"}})).unwrap();
        let h = HeaderFooter::of_doc(&s.doc);
        assert_eq!((h.start_page, h.first_page_footer), (5, Some(true)));
        let pag = crate::paginate::paginate(&s.doc);
        assert_eq!(pag.doc.last_section.page_num_start, Some(5));
        assert!(pag.doc.last_section.footers.first.is_some());
        assert!(s.run("headerFooter.set", &json!({"startPage": "five"})).is_err());
    }
}
