// SPDX-License-Identifier: MIT OR Apache-2.0
//! Property tests: generated scripts survive the native format and Fountain.

use proptest::prelude::*;
use wordsmith_doc::props::{CharProps, Underline};
use wordsmith_doc::{Document, Paragraph, para_block};
use wordsmith_format::{fountain, native};
use wordsmith_script::{ElementKind, Elements};

fn config() -> ProptestConfig {
    // No regression files in the source tree; a failure prints its case.
    ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() }
}

/// One paragraph of a generated script: element, text pieces with (bold, italic, underline), and
/// paragraph attributes.
#[derive(Clone, Debug)]
struct GenPara {
    kind: usize,
    pieces: Vec<(String, bool, bool, bool)>,
    page_break: bool,
    scene_number: Option<String>,
    bookmark: Option<u32>,
    dual: bool,
}

/// Text that may hold every character the native format escapes or marks: emphasis markers,
/// backslashes, braces, `@` and `=` at a line start, line breaks (also two in a row), and
/// letters outside ASCII.
fn native_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            4 => "[a-zA-Z ]{1,6}",
            1 => Just("*".to_string()),
            1 => Just("**".to_string()),
            1 => Just("_".to_string()),
            1 => Just("\\".to_string()),
            1 => Just("{".to_string()),
            1 => Just("}".to_string()),
            1 => Just("{n1}".to_string()),
            1 => Just("\n".to_string()),
            1 => Just("\n@".to_string()),
            1 => Just("\n=".to_string()),
            1 => Just("é漢🎬".to_string()),
        ],
        0..6,
    )
    .prop_map(|v| v.concat())
}

fn gen_para() -> impl Strategy<Value = GenPara> {
    (
        0..ElementKind::BUILTIN.len(),
        proptest::collection::vec((native_text(), any::<bool>(), any::<bool>(), any::<bool>()), 0..4),
        any::<bool>(),
        proptest::option::of("[0-9A-Z.-]{1,12}"),
        proptest::option::of(1u32..50),
        any::<bool>(),
    )
        .prop_map(|(kind, pieces, page_break, scene_number, bookmark, dual)| GenPara { kind, pieces, page_break, scene_number, bookmark, dual })
}

fn build(paras: &[GenPara]) -> Document {
    let mut doc = wordsmith_script::new_script();
    let elements = Elements::of_doc(&doc);
    doc.body = paras
        .iter()
        .map(|g| {
            let id = elements.id_of(ElementKind::BUILTIN[g.kind]).to_string();
            let mut p = Paragraph::with_text("", CharProps::default()).styled(&id);
            for (text, b, i, u) in &g.pieces {
                let c = CharProps {
                    bold: b.then_some(true),
                    italic: i.then_some(true),
                    underline: u.then_some(Underline::Single),
                    ..CharProps::default()
                };
                let _ = p.insert_text(p.len(), text, &c);
            }
            p.props.page_break_before = g.page_break.then_some(true);
            p.props.scene_number = g.scene_number.clone();
            p.props.bookmark = g.bookmark;
            p.props.dual = g.dual.then_some(true);
            para_block(p)
        })
        .collect();
    doc.ensure_nonempty();
    doc
}

/// Element, text, each character's (bold, italic, underline), and the paragraph attributes.
type Shape = (String, String, Vec<(bool, bool, bool)>, Option<bool>, Option<String>, Option<u32>, Option<bool>);

fn shape(doc: &Document) -> Vec<Shape> {
    let e = Elements::of_doc(doc);
    doc.body
        .iter()
        .filter_map(|b| b.as_para())
        .map(|p| {
            let styles = p
                .text
                .char_indices()
                .map(|(i, _)| {
                    let c = p.props_at(i + 1);
                    (c.bold == Some(true), c.italic == Some(true), c.underline.is_some_and(|u| u != Underline::None))
                })
                .collect();
            (e.of_para(p).id.clone(), p.text.clone(), styles, p.props.page_break_before, p.props.scene_number.clone(), p.props.bookmark, p.props.dual)
        })
        .collect()
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn generated_scripts_survive_the_native_format(paras in proptest::collection::vec(gen_para(), 0..12)) {
        let doc = build(&paras);
        let text = native::write(&doc).expect("writes");
        let back = native::read(&text).expect("reads what it wrote");
        prop_assert_eq!(shape(&back), shape(&doc));
        prop_assert_eq!(native::write(&back).expect("writes"), text, "saving again gives the same bytes");
    }
}

/// A speech or another core element of a script Fountain can carry.
#[derive(Clone, Debug)]
enum Piece {
    Heading(String),
    Action(String),
    Speech(String, Vec<(bool, String)>),
    Transition(String),
}

fn words(upper: bool) -> impl Strategy<Value = String> {
    let word = if upper { "[A-Z]{1,8}" } else { "[a-z]{1,8}" };
    proptest::collection::vec(word, 1..5).prop_map(|w| w.join(" "))
}

fn piece() -> impl Strategy<Value = Piece> {
    prop_oneof![
        (prop_oneof![Just("INT."), Just("EXT."), Just("INT./EXT.")], words(true), prop_oneof![Just("DAY"), Just("NIGHT")])
            .prop_map(|(p, place, time)| Piece::Heading(format!("{p} {place} - {time}"))),
        (words(false), proptest::collection::vec(words(false), 0..3)).prop_map(|(first, more)| {
            let mut lines = vec![format!("He {first}.")];
            lines.extend(more.into_iter().map(|l| format!("She {l}.")));
            Piece::Action(lines.join("\n"))
        }),
        (words(true), proptest::collection::vec((any::<bool>(), words(false)), 1..4)).prop_map(|(name, lines)| {
            // Fountain has no way to keep two Dialogue paragraphs in a row apart: they are lines of one.
            let mut out: Vec<(bool, String)> = Vec::new();
            for (paren, l) in lines {
                match out.last_mut() {
                    Some((false, prev)) if !paren => prev.push_str(&format!("\nWell, {l}.")),
                    _ => out.push((paren, if paren { format!("({l})") } else { format!("Well, {l}.") })),
                }
            }
            Piece::Speech(name, out)
        }),
        words(true).prop_map(|w| Piece::Transition(format!("{w} TO:"))),
    ]
}

fn fountain_doc(pieces: &[Piece]) -> Document {
    let mut doc = wordsmith_script::new_script();
    let e = Elements::of_doc(&doc);
    let para = |k: ElementKind, t: &str| para_block(wordsmith_script::template::paragraph(e.id_of(k), t));
    let mut body = Vec::new();
    for p in pieces {
        match p {
            Piece::Heading(t) => body.push(para(ElementKind::SceneHeading, t)),
            Piece::Action(t) => body.push(para(ElementKind::Action, t)),
            Piece::Transition(t) => body.push(para(ElementKind::Transition, t)),
            Piece::Speech(name, lines) => {
                body.push(para(ElementKind::Character, name));
                for (paren, l) in lines {
                    body.push(para(if *paren { ElementKind::Parenthetical } else { ElementKind::Dialogue }, l));
                }
            }
        }
    }
    doc.body = body;
    doc.ensure_nonempty();
    doc
}

fn lines(doc: &Document) -> Vec<(String, String)> {
    wordsmith_script::commands::text::body_lines(&wordsmith_script::session_for(doc.clone()))
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn generated_scripts_survive_fountain(pieces in proptest::collection::vec(piece(), 1..12)) {
        let doc = fountain_doc(&pieces);
        let text = fountain::write(&doc);
        let back = fountain::read(&text).expect("reads what it wrote").doc;
        prop_assert_eq!(lines(&back), lines(&doc), "{}", text);
        prop_assert_eq!(fountain::write(&back), text, "writing again gives the same text");
    }
}
