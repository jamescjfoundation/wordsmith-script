// SPDX-License-Identifier: MIT OR Apache-2.0
//! The watermark (SPEC-2 7): text set diagonally behind every page, its strength adjustable
//! (70 % by default), with a recipient's name for tracking copies; and, for sides, a
//! character's name at the top of every page. Kept in `app_data` under [`WATERMARK_KEY`].
//! Pictures as watermarks are not built (`docs/gaps.md`).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_doc::para::{Anchor, Float, ShapeKind, Wrap};
use wordsmith_doc::props::{Align, CharProps, LineSpacing, Rgb, TextColor};
use wordsmith_doc::{Document, InlineObject, Paragraph, PartKind, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

use crate::data::clean_line;
use crate::template::{INCH, SCRIPT_FONT};

/// `app_data` key of the watermark.
pub const WATERMARK_KEY: &str = "wordsmith.watermark";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Watermark {
    pub on: bool,
    pub text: String,
    /// 0 to 100: how strong the text is.
    pub opacity: u8,
    /// Who this copy is for.
    pub recipient: String,
    /// A name printed at the top of every page (sides); set for one printing, not kept.
    pub page_name: String,
}

impl Default for Watermark {
    fn default() -> Self {
        Watermark { on: false, text: String::new(), opacity: 70, recipient: String::new(), page_name: String::new() }
    }
}

impl Watermark {
    pub fn of_doc(doc: &Document) -> Self {
        let mut w: Watermark = crate::data::load(doc, WATERMARK_KEY);
        w.text = clean_line(&w.text, 60);
        w.recipient = clean_line(&w.recipient, 80);
        w.page_name = clean_line(&w.page_name, 80);
        w.opacity = w.opacity.min(100);
        w
    }

    pub fn store(&self, doc: &mut Document) {
        crate::data::store(doc, WATERMARK_KEY, self);
    }

    fn shows(&self) -> bool {
        self.on && (!self.text.is_empty() || !self.recipient.is_empty())
    }
}

fn line(text: &str, size: f32, bold: bool, color: Rgb, align: Align) -> std::sync::Arc<wordsmith_doc::Block> {
    let c = CharProps {
        font: Some(SCRIPT_FONT.into()),
        size: Some(size),
        bold: bold.then_some(true),
        color: Some(TextColor::Rgb(color)),
        ..CharProps::default()
    };
    let mut p = Paragraph::with_text(text, c);
    p.props.align = Some(align);
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    p.props.line_spacing = Some(LineSpacing::Exactly(size * 1.15));
    para_block(p)
}

fn shape(out: &mut Document, blocks: Vec<std::sync::Arc<wordsmith_doc::Block>>, rect: (f32, f32, f32, f32), rot: f32, behind: bool) -> InlineObject {
    let (x, y, w, h) = rect;
    let story = out.add_part(PartKind::TextBox, blocks);
    if let Some(part) = out.parts.get_mut(&story) {
        part.body.inset = Some([0; 4]);
    }
    let float = Float {
        wrap: if behind { Wrap::BehindText } else { Wrap::InFrontOfText },
        h_rel: Anchor::Page,
        v_rel: Anchor::Page,
        x,
        y,
        rot,
        ..Float::default()
    };
    InlineObject::Shape {
        kind: ShapeKind::Rectangle,
        w,
        h,
        fill: None,
        stroke: None,
        stroke_width: 0.0,
        float,
        story: Some(story),
        freeform: None,
        effects: Default::default(),
        extra: Default::default(),
    }
}

/// Put the watermark (and page name) on every page of the output script `out`: pages start at
/// the first paragraph and at each paragraph that breaks the page.
pub fn apply(out: &mut Document, w: &Watermark) {
    if !w.shows() && w.page_name.is_empty() {
        return;
    }
    let starts: Vec<usize> = out
        .body
        .iter()
        .enumerate()
        .filter(|(i, b)| *i == 0 || b.as_para().is_some_and(|p| p.props.page_break_before == Some(true)))
        .map(|(i, _)| i)
        .collect();
    let (pw, ph) = (out.last_section.page_w, out.last_section.page_h);
    // Stronger is darker: 100 % is a mid grey, 0 % white.
    let g = (255.0 - f32::from(w.opacity) * 1.1).clamp(130.0, 255.0) as u8;
    let grey = Rgb(g, g, g);
    for at in starts {
        let mut objs = Vec::new();
        if w.shows() {
            let (bw, bh) = (7.5 * INCH, 2.2 * INCH);
            let mut blocks = Vec::new();
            if !w.text.is_empty() {
                blocks.push(line(&w.text, 66.0, true, grey, Align::Center));
            }
            if !w.recipient.is_empty() {
                blocks.push(line(&w.recipient, 22.0, false, grey, Align::Center));
            }
            objs.push(shape(out, blocks, ((pw - bw) / 2.0, (ph - bh) / 2.0, bw, bh), 315.0, true));
        }
        if !w.page_name.is_empty() {
            let blocks = vec![line(&w.page_name, 10.0, true, Rgb(0, 0, 0), Align::Left)];
            objs.push(shape(out, blocks, (1.5 * INCH, 0.5 * INCH, 3.0 * INCH, 14.0), 0.0, false));
        }
        if let Some(wordsmith_doc::Block::Para(p)) = out.body.get_mut(at).map(std::sync::Arc::make_mut) {
            for obj in objs {
                let end = p.len();
                let _ = p.insert_object(end, obj, &CharProps::default());
            }
        }
    }
}

fn get(s: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(Watermark::of_doc(&s.doc)))
}

/// `watermark.set {on?, text?, opacity?, recipient?}`.
fn set(s: &mut Session, v: &Value) -> CmdResult {
    let mut w = Watermark::of_doc(&s.doc);
    if let Some(b) = p::bool(v, "on") {
        w.on = b;
    }
    if let Some(t) = p::str(v, "text") {
        w.text = clean_line(t, 60);
    }
    if let Some(r) = p::str(v, "recipient") {
        w.recipient = clean_line(r, 80);
    }
    if let Some(o) = v.get("opacity") {
        let o = o.as_u64().filter(|o| *o <= 100).ok_or_else(|| CmdError::Params("`opacity` is 0 to 100".into()))?;
        w.opacity = o as u8;
    }
    w.page_name.clear();
    w.store(&mut s.doc);
    Ok(json!(w))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("watermark.get", "Watermark", "Production", get).pure(),
        CommandSpec::new("watermark.set", "Set Watermark", "Production", set)
            .params(r#"{"on"?: bool, "text"?: string, "opacity"?: 0-100, "recipient"?: string}"#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;

    #[test]
    fn every_page_gets_the_watermark() {
        let mut s = new_session();
        s.doc.body = (0..70).map(|i| para_block(crate::template::paragraph("Action", &format!("Line {i}.")))).collect();
        s.run("watermark.set", &json!({"on": true, "text": "DRAFT", "recipient": "Copy 7: J. Doe", "opacity": 40})).unwrap();
        let pag = crate::paginate::paginate(&s.doc);
        assert_eq!(pag.pages.len(), 3);
        let with: usize = pag
            .doc
            .body
            .iter()
            .filter_map(|b| b.as_para())
            .map(|p| p.objects.iter().filter(|o| matches!(o, InlineObject::Shape { float, .. } if float.rot == 315.0)).count())
            .sum();
        assert_eq!(with, 3);
        assert!(s.run("watermark.set", &json!({"opacity": 101})).is_err());
        let bytes = crate::export::pdf(&s.doc).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}
