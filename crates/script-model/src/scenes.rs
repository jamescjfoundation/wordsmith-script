// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scenes are not stored: a scene runs from one Scene Heading to the next (SPEC "Document model
//! and elements"). This derives them from the body's paragraphs.

use jcj_doc::{Document, StoryRef};

use crate::element::{ElementKind, Elements};

/// One scene: its heading and the body blocks it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene {
    /// 0-based position among the script's scenes.
    pub index: usize,
    /// The heading's text.
    pub heading: String,
    /// Index of the heading's block in the body.
    pub start: usize,
    /// Index of the block after the scene's last (the next heading, or the body's length).
    pub end: usize,
}

/// The scenes of `doc`, in order. Text before the first Scene Heading belongs to no scene.
pub fn scenes(doc: &Document) -> Vec<Scene> {
    let elements = Elements::of_doc(doc);
    let Some(blocks) = doc.story(StoryRef::Body) else { return Vec::new() };
    let mut out: Vec<Scene> = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        let Some(p) = block.as_para() else { continue };
        if elements.of_para(p).kind == ElementKind::SceneHeading {
            if let Some(last) = out.last_mut() {
                last.end = i;
            }
            out.push(Scene { index: out.len(), heading: p.text.clone(), start: i, end: blocks.len() });
        }
    }
    out
}

/// The scene that contains body block `block`, if any.
pub fn scene_at(scenes: &[Scene], block: usize) -> Option<&Scene> {
    scenes.iter().find(|s| (s.start..s.end).contains(&block))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::new_script;
    use jcj_doc::para_block;

    fn script(lines: &[(&str, &str)]) -> Document {
        let mut doc = new_script();
        doc.body = lines.iter().map(|(style, text)| para_block(crate::template::paragraph(style, text))).collect();
        doc
    }

    #[test]
    fn scenes_run_from_heading_to_heading() {
        let doc = script(&[
            ("Action", "FADE IN:"),
            ("SceneHeading", "INT. KITCHEN - NIGHT"),
            ("Action", "Rain."),
            ("Character", "ANNA"),
            ("Dialogue", "Hello."),
            ("SceneHeading", "EXT. YARD - DAY"),
            ("Action", "Sun."),
        ]);
        let s = scenes(&doc);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].heading.as_str(), s[0].start, s[0].end), ("INT. KITCHEN - NIGHT", 1, 5));
        assert_eq!((s[1].heading.as_str(), s[1].start, s[1].end), ("EXT. YARD - DAY", 5, 7));
        assert_eq!(scene_at(&s, 0), None, "text before the first heading is in no scene");
        assert_eq!(scene_at(&s, 4).map(|x| x.index), Some(0));
        assert_eq!(scene_at(&s, 6).map(|x| x.index), Some(1));
    }

    #[test]
    fn an_empty_script_has_its_one_empty_heading() {
        let doc = new_script();
        let s = scenes(&doc);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].heading, "");
        let none = script(&[("Action", "Just action.")]);
        assert!(scenes(&none).is_empty());
    }
}
