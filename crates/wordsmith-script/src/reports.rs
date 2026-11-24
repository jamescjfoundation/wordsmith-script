// SPDX-License-Identifier: MIT OR Apache-2.0
//! Production reports (SPEC "Production"): scene lengths in eighths of a page and a character
//! report. Decision 0027.
//!
//! A scene's length is the height its text takes on the printed pages (its lines, with the space
//! before each paragraph that is not at the top of a page), in eighths of a page's text height,
//! rounded up, at least 1/8: the convention a schedule uses. Written as "1 3/8", "5/8", "2".

use serde_json::{Value, json};
use wordsmith_doc::Document;
use wordsmith_engine::{CmdResult, CommandSpec, Session};

use crate::commands::character::csv_field;
use crate::element::{ElementKind, Elements};
use crate::paginate::{Paginated, Piece};
use crate::scenes::{scene_at, scenes};
use crate::template::LINE;

/// Each scene's length in eighths of a page, from a pagination of the script.
pub fn scene_eighths(doc: &Document, pag: &Paginated) -> Vec<u32> {
    let all = scenes(doc);
    let s = &doc.last_section;
    let page_h = (s.page_h - s.margin_top - s.margin_bottom).max(LINE);
    let mut heights = vec![0.0f32; all.len()];
    for page in &pag.pages {
        for piece in page {
            let Piece::Para { block, lines, space_before, .. } = piece else { continue };
            let Some(sc) = scene_at(&all, *block) else { continue };
            let sb = if *space_before {
                doc.body.get(*block).and_then(|b| b.as_para()).map_or(0.0, |p| doc.styles.resolve_para(&p.props).space_before)
            } else {
                0.0
            };
            if let Some(h) = heights.get_mut(sc.index) {
                *h += *lines as f32 * LINE + sb;
            }
        }
    }
    heights.into_iter().map(|h| ((h / page_h * 8.0) - 0.001).ceil().max(1.0) as u32).collect()
}

/// Eighths as a schedule writes them: "1 3/8", "5/8", "2".
pub fn format_eighths(e: u32) -> String {
    match (e / 8, e % 8) {
        (0, r) => format!("{r}/8"),
        (w, 0) => w.to_string(),
        (w, r) => format!("{w} {r}/8"),
    }
}

/// One character's line in the character report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterLine {
    pub name: String,
    pub speeches: usize,
    pub words: usize,
    pub scenes: usize,
    pub first_scene: Option<String>,
    pub last_scene: Option<String>,
    pub pages: usize,
}

/// The character report: every speaking character, in order of first speech, with speeches,
/// words spoken (dialogue only), scenes, first and last scene (by number) and the pages they
/// speak on.
pub fn character_report(doc: &Document, pag: &Paginated) -> Vec<CharacterLine> {
    let elements = Elements::of_doc(doc);
    let all = scenes(doc);
    let numbers = crate::scene_numbers::numbers(doc);
    let rows = crate::commands::character::characters(doc);
    // Each block's page, from the plan.
    let mut page_of = std::collections::HashMap::new();
    for (n, page) in pag.pages.iter().enumerate() {
        for piece in page {
            if let Piece::Para { block, .. } = piece {
                page_of.entry(*block).or_insert(n);
            }
        }
    }
    let mut out: Vec<CharacterLine> = rows
        .iter()
        .map(|r| CharacterLine {
            name: r.name.clone(),
            speeches: r.speeches,
            words: 0,
            scenes: r.scenes,
            first_scene: None,
            last_scene: None,
            pages: 0,
        })
        .collect();
    let mut pages: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); out.len()];
    let mut speaker: Option<usize> = None;
    for (i, b) in doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        match elements.of_para(p).kind {
            ElementKind::Character => {
                let name = crate::paginate::output::speaker(&p.text).trim().to_uppercase();
                speaker = out.iter().position(|c| c.name == name);
                if let Some(c) = speaker {
                    let number = scene_at(&all, i).and_then(|sc| numbers.get(sc.index)).cloned();
                    if let Some(row) = out.get_mut(c) {
                        if row.first_scene.is_none() {
                            row.first_scene = number.clone();
                        }
                        if number.is_some() {
                            row.last_scene = number;
                        }
                    }
                    if let (Some(pg), Some(set)) = (page_of.get(&i), pages.get_mut(c)) {
                        set.insert(*pg);
                    }
                }
            }
            ElementKind::Dialogue => {
                if let Some(c) = speaker {
                    if let Some(row) = out.get_mut(c) {
                        row.words += p.text.split_whitespace().count();
                    }
                    if let (Some(pg), Some(set)) = (page_of.get(&i), pages.get_mut(c)) {
                        set.insert(*pg);
                    }
                }
            }
            ElementKind::Parenthetical => {}
            _ => speaker = None,
        }
    }
    for (c, set) in out.iter_mut().zip(pages) {
        c.pages = set.len();
    }
    out
}

/// The character report as CSV.
pub fn character_report_csv(doc: &Document) -> String {
    let pag = crate::paginate::paginate(doc);
    let mut out = String::from("Character,Speeches,Words,Scenes,First scene,Last scene,Pages\r\n");
    for c in character_report(doc, &pag) {
        let fields = [
            c.name,
            c.speeches.to_string(),
            c.words.to_string(),
            c.scenes.to_string(),
            c.first_scene.unwrap_or_default(),
            c.last_scene.unwrap_or_default(),
            c.pages.to_string(),
        ];
        out.push_str(&fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

/// `report.characters`: the character report as rows and as CSV.
fn characters(s: &mut Session, _: &Value) -> CmdResult {
    let pag = crate::paginate::paginate(&s.doc);
    let rows: Vec<Value> = character_report(&s.doc, &pag)
        .into_iter()
        .map(|c| {
            json!({"name": c.name, "speeches": c.speeches, "words": c.words, "scenes": c.scenes, "firstScene": c.first_scene, "lastScene": c.last_scene, "pages": c.pages})
        })
        .collect();
    Ok(json!({"rows": rows, "csv": character_report_csv(&s.doc)}))
}

/// `report.sceneLengths`: each scene's number, heading and length in eighths.
fn scene_lengths(s: &mut Session, _: &Value) -> CmdResult {
    let pag = crate::paginate::paginate(&s.doc);
    let numbers = crate::scene_numbers::numbers(&s.doc);
    let lengths = scene_eighths(&s.doc, &pag);
    Ok(json!(
        scenes(&s.doc)
            .iter()
            .map(|sc| {
                let e = lengths.get(sc.index).copied().unwrap_or(1);
                json!({"number": numbers.get(sc.index), "heading": sc.heading, "eighths": e, "length": format_eighths(e)})
            })
            .collect::<Vec<_>>()
    ))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("report.characters", "Character Report", "Production", characters).pure(),
        CommandSpec::new("report.sceneLengths", "Scene Lengths", "Production", scene_lengths).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use wordsmith_doc::para_block;

    use super::*;
    use crate::template::{new_script, paragraph};

    #[test]
    fn eighths_are_written_as_a_schedule_writes_them() {
        assert_eq!(format_eighths(1), "1/8");
        assert_eq!(format_eighths(8), "1");
        assert_eq!(format_eighths(11), "1 3/8");
        assert_eq!(format_eighths(16), "2");
    }

    fn script() -> Document {
        let mut doc = new_script();
        let mut body = vec![para_block(paragraph("SceneHeading", "INT. A - DAY")), para_block(paragraph("Action", "Short."))];
        body.push(para_block(paragraph("SceneHeading", "INT. B - DAY")));
        // About a page and a half: 40 one-line Action paragraphs a blank line apart.
        body.extend((0..40).map(|i| para_block(paragraph("Action", &format!("Line {i}.")))));
        body.push(para_block(paragraph("Character", "ANNA (V.O.)")));
        body.push(para_block(paragraph("Dialogue", "Two words.")));
        body.push(para_block(paragraph("Character", "BOB")));
        body.push(para_block(paragraph("Dialogue", "One more word.")));
        body.push(para_block(paragraph("Character", "ANNA")));
        body.push(para_block(paragraph("Parenthetical", "(beat)")));
        body.push(para_block(paragraph("Dialogue", "Three words here.")));
        doc.body = body;
        doc
    }

    #[test]
    fn scene_lengths_in_eighths() {
        let doc = script();
        let pag = crate::paginate::paginate(&doc);
        let e = scene_eighths(&doc, &pag);
        // Scene 1: three lines of 54, rounded up to 1/8. Scene 2: 80 lines of action and a few
        // of speech, a little over a page and a half.
        assert_eq!(e[0], 1);
        assert!((13..=14).contains(&e[1]), "{e:?}");
    }

    #[test]
    fn the_character_report() {
        let doc = script();
        let pag = crate::paginate::paginate(&doc);
        let r = character_report(&doc, &pag);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].name.as_str(), r[0].speeches, r[0].words, r[0].scenes), ("ANNA", 2, 5, 1));
        assert_eq!((r[0].first_scene.as_deref(), r[0].last_scene.as_deref(), r[0].pages), (Some("2"), Some("2"), 1));
        assert_eq!((r[1].name.as_str(), r[1].words), ("BOB", 3));
        let csv = character_report_csv(&doc);
        assert!(csv.starts_with("Character,Speeches,Words,Scenes,First scene,Last scene,Pages\r\nANNA,2,5,1,2,2,1\r\n"), "{csv}");
    }
}
