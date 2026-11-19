// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pagination rules on small scripts. A page holds 54 lines (9 in at 6 lines per inch). General
//! paragraphs have no space before, so `n` single-line General paragraphs use exactly `n` lines.

use wordsmith_doc::{Document, StoryRef, para_block};
use wordsmith_layout::LayoutCache;

use super::{MoresContinueds, Paginated, Paginator, Piece, paginate};
use crate::template::{new_script, paragraph};

const PAGE_LINES: usize = 54;

fn script(lines: &[(&str, &str)]) -> Document {
    let mut doc = new_script();
    doc.body = lines.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
    doc
}

/// `n` lines of General filler, then `rest`.
fn after_filler(n: usize, rest: &[(&str, &str)]) -> Document {
    let mut lines: Vec<(&str, &str)> = vec![("General", "filler"); n];
    lines.extend_from_slice(rest);
    script(&lines)
}

fn run(doc: &Document) -> Paginated {
    let p = paginate(doc);
    assert!(p.matches_plan, "the layout disagrees with the plan: {} planned, {} laid out", p.pages.len(), p.layout.pages.len());
    p
}

/// Each output page's paragraphs as (style, text).
fn pages(p: &Paginated) -> Vec<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut i = 0;
    let body = p.doc.story(StoryRef::Body).cloned().unwrap_or_default();
    for page in &p.pages {
        let mut v = Vec::new();
        for _ in page {
            if let Some(par) = body.get(i).and_then(|b| b.as_para()) {
                v.push((par.props.style.clone().unwrap_or_default(), par.text.clone()));
            }
            i += 1;
        }
        out.push(v);
    }
    out
}

fn texts(page: &[(String, String)]) -> Vec<&str> {
    page.iter().map(|(_, t)| t.as_str()).filter(|t| *t != "filler").collect()
}

#[test]
fn a_short_script_is_one_page() {
    let p = run(&script(&[("SceneHeading", "INT. ROOM - DAY"), ("Action", "Quiet.")]));
    assert_eq!(p.pages.len(), 1);
    assert_eq!(texts(&pages(&p)[0]), ["INT. ROOM - DAY", "Quiet."]);
    let empty = run(&new_script());
    assert_eq!(empty.pages.len(), 1);
}

#[test]
fn a_page_holds_54_lines_and_space_before_is_dropped_at_the_top() {
    let p = run(&after_filler(PAGE_LINES, &[("Action", "Next page.")]));
    assert_eq!(p.pages.len(), 2);
    assert_eq!(texts(&pages(&p)[1]), ["Next page."]);
    // The Action's blank line is not carried to the top of page 2.
    let first_on_2 = p.doc.body.get(PAGE_LINES).and_then(|b| b.as_para()).and_then(|q| q.props.space_before);
    assert_eq!(first_on_2, Some(0.0));
    // 53 lines leave room for one more General line, but not for an Action and its blank line.
    let p = run(&after_filler(PAGE_LINES - 1, &[("General", "last line")]));
    assert_eq!(p.pages.len(), 1);
    let p = run(&after_filler(PAGE_LINES - 1, &[("Action", "needs two lines")]));
    assert_eq!(p.pages.len(), 2);
}

#[test]
fn a_scene_heading_is_never_left_at_the_foot() {
    // Room for the heading (2 lines) but not for any of the action after it.
    let p = run(&after_filler(PAGE_LINES - 3, &[("SceneHeading", "INT. HALL - NIGHT"), ("Action", "Footsteps.")]));
    assert_eq!(p.pages.len(), 2);
    assert_eq!(texts(&pages(&p)[1]), ["INT. HALL - NIGHT", "Footsteps."]);
    // With room for both, both stay.
    let p = run(&after_filler(PAGE_LINES - 4, &[("SceneHeading", "INT. HALL - NIGHT"), ("Action", "Footsteps.")]));
    assert_eq!(p.pages.len(), 1);
}

#[test]
fn a_character_keeps_its_first_line_of_dialogue() {
    let p = run(&after_filler(PAGE_LINES - 2, &[("Character", "ANNA"), ("Dialogue", "Hello.")]));
    assert_eq!(p.pages.len(), 2);
    assert_eq!(texts(&pages(&p)[1]), ["ANNA", "Hello."]);
    // And its Parenthetical.
    let p = run(&after_filler(PAGE_LINES - 3, &[("Character", "ANNA"), ("Parenthetical", "(softly)"), ("Dialogue", "Hello.")]));
    assert_eq!(texts(&pages(&p)[1]), ["ANNA", "(softly)", "Hello."]);
}

/// Dialogue 35 characters wide: each sentence below is one line.
const LONG_SPEECH: &str = "One two three four five six seven. Eight nine ten eleven twelve thirteen. Fourteen fifteen sixteen seventeen. Eighteen nineteen twenty twenty-one. Twenty-two twenty-three twenty-four. Twenty-five twenty-six twenty-seven.";

#[test]
fn dialogue_splits_after_a_sentence_with_more_and_contd() {
    // Room for the Character's blank line and name, 3 dialogue lines and (MORE).
    let p = run(&after_filler(PAGE_LINES - 6, &[("Character", "ANNA"), ("Dialogue", LONG_SPEECH), ("Action", "She stops.")]));
    assert_eq!(p.pages.len(), 2);
    let pg = pages(&p);
    let one = texts(&pg[0]);
    assert_eq!(one.first(), Some(&"ANNA"));
    assert_eq!(one.last(), Some(&"(MORE)"));
    let part = one[one.len() - 2];
    assert!(part.ends_with('.'), "the split is after a sentence: {part:?}");
    assert!(LONG_SPEECH.starts_with(part));
    let two = texts(&pg[1]);
    assert_eq!(two[0], "ANNA (CONT'D)");
    let rest = two[1];
    assert_eq!(format!("{part} {rest}"), LONG_SPEECH, "nothing is lost or repeated");
    assert_eq!(two[2], "She stops.");
    assert!(p.pages[0].contains(&Piece::More));
}

#[test]
fn dialogue_with_no_sentence_end_in_reach_moves_whole() {
    // About eight lines of dialogue and a single sentence end, at its very end.
    let speech = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four twenty-five twenty-six twenty-seven twenty-eight.";
    let p = run(&after_filler(PAGE_LINES - 6, &[("Character", "BOB"), ("Dialogue", speech)]));
    let pg = pages(&p);
    assert_eq!(texts(&pg[1]), ["BOB", speech], "the speech and its name move together");
    assert!(!texts(&pg[0]).contains(&"(MORE)"));
}

#[test]
fn a_speech_broken_between_paragraphs_gets_more_and_contd() {
    let p = run(&after_filler(
        PAGE_LINES - 4,
        &[("Character", "ANNA"), ("Dialogue", "First line."), ("Parenthetical", "(beat)"), ("Dialogue", "Second line.")],
    ));
    let pg = pages(&p);
    assert_eq!(texts(&pg[0]).last(), Some(&"(MORE)"));
    assert_eq!(texts(&pg[1]), ["ANNA (CONT'D)", "(beat)", "Second line."]);
}

#[test]
fn action_splits_after_a_sentence_with_two_lines_each_side() {
    let action = "The door opens slowly and a cold wind blows in from the dark yard. Leaves scatter across the floor and the candle flickers twice. Somewhere a dog barks and then falls silent again. Nobody moves.";
    let p = run(&after_filler(PAGE_LINES - 3, &[("Action", action)]));
    let pg = pages(&p);
    let one = texts(&pg[0]);
    let two = texts(&pg[1]);
    assert_eq!(one.len(), 1);
    assert!(one[0].ends_with('.'));
    assert_eq!(format!("{} {}", one[0], two[0]), action);
    assert!(!one.contains(&"(MORE)"), "action takes no (MORE)");
}

#[test]
fn automatic_contd_within_a_scene_only() {
    let p = run(&script(&[
        ("SceneHeading", "INT. A - DAY"),
        ("Character", "ANNA"),
        ("Dialogue", "One."),
        ("Action", "She turns."),
        ("Character", "ANNA"),
        ("Dialogue", "Two."),
        ("Character", "BOB"),
        ("Dialogue", "Three."),
        ("Character", "ANNA (V.O.)"),
        ("Dialogue", "Four."),
        ("SceneHeading", "EXT. B - DAY"),
        ("Character", "ANNA"),
        ("Dialogue", "Five."),
    ]));
    let names: Vec<String> = pages(&p).concat().into_iter().filter(|(s, _)| s == "Character").map(|(_, t)| t).collect();
    assert_eq!(names, ["ANNA", "ANNA (CONT'D)", "BOB", "ANNA (V.O.)", "ANNA"]);
    // The option can be switched off.
    let mut doc = script(&[("Character", "ANNA"), ("Dialogue", "One."), ("Action", "x"), ("Character", "ANNA"), ("Dialogue", "Two.")]);
    MoresContinueds { character: false, ..Default::default() }.store(&mut doc);
    let names: Vec<String> = pages(&run(&doc)).concat().into_iter().filter(|(s, _)| s == "Character").map(|(_, t)| t).collect();
    assert_eq!(names, ["ANNA", "ANNA"]);
}

#[test]
fn scene_continueds_are_an_option() {
    let lines: Vec<(&str, &str)> =
        std::iter::once(("SceneHeading", "INT. LONG - DAY")).chain(std::iter::repeat_n(("General", "filler"), 70)).collect();
    let mut doc = script(&lines);
    let off = run(&doc);
    assert!(!pages(&off).concat().iter().any(|(_, t)| t.contains("CONTINUED")));
    MoresContinueds { scene: true, scene_page_count: true, ..Default::default() }.store(&mut doc);
    let on = run(&doc);
    let pg = pages(&on);
    assert_eq!(texts(&pg[0]).last(), Some(&"(CONTINUED)"));
    assert_eq!(texts(&pg[1]).first(), Some(&"CONTINUED: (2)"));
}

#[test]
fn start_new_page_starts_one() {
    let p = run(&script(&[("Action", "Act one ends."), ("NewAct", "ACT TWO"), ("SceneHeading", "INT. X - DAY")]));
    assert_eq!(p.pages.len(), 2);
    assert_eq!(texts(&pages(&p)[1]), ["ACT TWO", "INT. X - DAY"]);
}

#[test]
fn pages_are_numbered_top_right_from_page_two() {
    let p = run(&after_filler(PAGE_LINES * 2, &[]));
    let s = &p.doc.last_section;
    assert!(s.title_page, "page 1 uses the empty first-page header");
    let header = s.headers.default.and_then(|id| p.doc.parts.get(&id)).and_then(|part| part.blocks.first()).and_then(|b| b.as_para()).unwrap();
    assert_eq!(header.props.style.as_deref(), Some(super::output::PAGE_NUMBER_STYLE));
    assert!(header.objects.iter().any(|o| matches!(o, wordsmith_doc::InlineObject::Field { instr, .. } if instr == "PAGE")));
    assert!(header.text.ends_with('.'));
    let first = s.headers.first.and_then(|id| p.doc.parts.get(&id)).and_then(|part| part.blocks.first()).and_then(|b| b.as_para()).unwrap();
    assert!(first.text.is_empty());
}

#[test]
fn a_paragraph_longer_than_a_page_still_paginates() {
    // No sentence ends at all: split at line breaks as a last resort.
    let huge = "word ".repeat(60 * 70);
    let p = run(&script(&[("Action", huge.trim_end())]));
    assert!(p.pages.len() >= 2);
    let joined: String = pages(&p).concat().into_iter().map(|(_, t)| t).collect::<Vec<_>>().join(" ");
    assert_eq!(joined.split_whitespace().count(), 60 * 70, "every word is kept");
}

#[test]
fn source_and_output_positions_map_both_ways() {
    let p = run(&after_filler(PAGE_LINES - 6, &[("Character", "ANNA"), ("Dialogue", LONG_SPEECH)]));
    let dialogue_block = PAGE_LINES - 6 + 1;
    let far = wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(dialogue_block), off: LONG_SPEECH.len() - 3 };
    let out = p.to_output(&far).unwrap();
    let back = p.to_source(&out).unwrap();
    assert_eq!(back, far);
    let shown = p.doc.para_at(&out).unwrap();
    assert!(shown.text.ends_with("twenty-seven."));
}

/// A feature-length script: scenes of action and dialogue, about 150 pages.
pub(crate) fn feature(scenes: usize) -> Document {
    let mut lines: Vec<(&str, String)> = Vec::new();
    for n in 0..scenes {
        lines.push(("SceneHeading", format!("INT. ROOM {n} - DAY")));
        lines.push(("Action", "The light is low and the room is quiet. Somebody has left the radio on, very softly.".into()));
        for k in 0..4 {
            lines.push(("Character", if k % 2 == 0 { "ANNA".into() } else { "BOB".into() }));
            lines.push(("Dialogue", "I keep thinking we missed something obvious. Tell me again what you saw.".into()));
        }
        lines.push(("Transition", "CUT TO:".into()));
    }
    let mut doc = new_script();
    doc.body = lines.iter().map(|(e, t)| para_block(paragraph(e, t))).collect();
    doc
}

#[test]
fn a_feature_length_script_paginates_and_matches_the_layout() {
    let doc = feature(300);
    let mut pager = Paginator::new();
    let t0 = std::time::Instant::now();
    let p = pager.paginate(&doc);
    let cold = t0.elapsed();
    let t1 = std::time::Instant::now();
    let again = pager.paginate(&doc);
    let warm = t1.elapsed();
    eprintln!("feature: {} pages, paginate cold {cold:?}, warm {warm:?}", p.pages.len());
    assert!(p.matches_plan && again.matches_plan);
    assert!((120..=200).contains(&p.pages.len()), "{} pages", p.pages.len());
}

#[test]
#[ignore = "timing breakdown, run by hand with --ignored --nocapture"]
fn timing_breakdown() {
    use super::measure::{MeasureCache, measure_cached, print_layout};
    let doc = feature(300);
    let mut cache = LayoutCache::new();
    let mut mc = MeasureCache::default();
    let _ = measure_cached(&doc, &mut mc);
    let t_source = std::time::Duration::ZERO;
    let t = std::time::Instant::now();
    let m = measure_cached(&doc, &mut mc);
    let t_measure = t.elapsed();
    let marks = MoresContinueds::default();
    let t = std::time::Instant::now();
    let pages = super::plan(&doc, &m, &marks, super::Rules::default());
    let t_plan = t.elapsed();
    let t = std::time::Instant::now();
    let (out, _) = super::output::build(&doc, &pages, &marks);
    let t_build = t.elapsed();
    let t = std::time::Instant::now();
    let _ = print_layout(&out, &mut cache);
    let t_out = t.elapsed();
    eprintln!("source layout {t_source:?}, measure {t_measure:?}, plan {t_plan:?}, build {t_build:?}, output layout {t_out:?}");
}

#[test]
fn measuring_paragraph_by_paragraph_matches_the_full_layout() {
    use super::measure::{MeasureCache, measure, measure_cached, print_layout};
    let mut doc = feature(20);
    doc.body.push(para_block(paragraph("Dialogue", LONG_SPEECH)));
    doc.body.push(para_block(paragraph("Transition", "SMASH CUT TO:")));
    let full = measure(&doc, &print_layout(&doc, &mut LayoutCache::new()));
    let mut cache = MeasureCache::default();
    assert_eq!(measure_cached(&doc, &mut cache), full);
    // Again, from the cache; then after an edit to one paragraph.
    assert_eq!(measure_cached(&doc, &mut cache), full);
    let mut edited = doc.clone();
    edited.body[3] = para_block(paragraph("Action", "Changed, and now a different length entirely so it wraps onto a second line of the page."));
    let full = measure(&edited, &print_layout(&edited, &mut LayoutCache::new()));
    assert_eq!(measure_cached(&edited, &mut cache), full);
}
