// SPDX-License-Identifier: MIT OR Apache-2.0
//! Templates (SPEC-2 2 and 19): new documents to start from, in categories. Every template here
//! is our own, from public format conventions; none follows a named method or a company's house
//! style. A template carries the element settings, page, header and footer, lists, macros and
//! starting text a document begins with.
//!
//! Two document kinds (SPEC-2 1): a Script (the default) and a Text document for prose such as
//! a treatment or a series bible, kept in `app_data` under [`KIND_KEY`]. A document saved as a
//! template is flagged under [`TEMPLATE_KEY`]; opening it starts a new, untitled document.

use std::sync::Arc;

use serde_json::{Value, json};
use wordsmith_doc::{Document, para_block};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Session, p};

use crate::element::ElementKind;
use crate::template::{new_script, paragraph};

/// `app_data` key of the document kind ("script" or "text").
pub const KIND_KEY: &str = "wordsmith.kind";
/// `app_data` key that marks a document saved as a template.
pub const TEMPLATE_KEY: &str = "wordsmith.isTemplate";

/// A built-in template: its id, name, category and a line about it.
pub struct TemplateInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub about: &'static str,
}

/// Every built-in template, by category.
pub const TEMPLATES: [TemplateInfo; 12] = [
    TemplateInfo { id: "screenplay", name: "Screenplay", category: "Screenplays", about: "A feature or short film script." },
    TemplateInfo {
        id: "screenplay-guided",
        name: "Screenplay, with notes",
        category: "Screenplays",
        about: "A first scene with notes on what each element is for.",
    },
    TemplateInfo {
        id: "tv-single",
        name: "Single-camera episode",
        category: "Television",
        about: "An hour or half-hour episode in acts, with a teaser.",
    },
    TemplateInfo {
        id: "tv-multi",
        name: "Multi-camera sitcom",
        category: "Television",
        about: "Double-spaced dialogue, action in capitals, lettered scenes.",
    },
    TemplateInfo { id: "stage", name: "Stage play", category: "Stage", about: "Names centred, stage directions set in." },
    TemplateInfo { id: "audio", name: "Audio drama", category: "Audio and prose", about: "Sound and music cues, voices." },
    TemplateInfo { id: "treatment", name: "Treatment", category: "Audio and prose", about: "A prose telling of the story (a text document)." },
    TemplateInfo { id: "bible", name: "Series bible", category: "Audio and prose", about: "Characters, world and episodes (a text document)." },
    TemplateInfo { id: "three-acts", name: "Three acts", category: "Outlines and structure", about: "A script outline in three acts." },
    TemplateInfo { id: "four-acts", name: "Four acts", category: "Outlines and structure", about: "A script outline in four acts." },
    TemplateInfo { id: "five-acts", name: "Five acts", category: "Outlines and structure", about: "A script outline in five acts." },
    TemplateInfo { id: "sequences", name: "Acts and sequences", category: "Outlines and structure", about: "Three acts, each in sequences." },
];

/// Whether `doc` is a text document.
pub fn is_text(doc: &Document) -> bool {
    doc.app_data.get(KIND_KEY).and_then(|v| v.as_str()) == Some("text")
}

fn set(doc: &mut Document, key: &str, v: Value) {
    doc.app_data.insert(key.to_string(), Arc::new(v));
}

fn body(doc: &mut Document, lines: &[(ElementKind, &str)]) {
    let elements = crate::element::Elements::of_doc(doc);
    doc.body = lines.iter().map(|(k, t)| para_block(paragraph(elements.id_of(*k), t))).collect();
    doc.ensure_nonempty();
}

/// Change element `kind`'s settings in `doc` the way `element.configure` would.
fn configure(doc: &mut Document, kind: ElementKind, v: Value) {
    let mut s = crate::session_for(doc.clone());
    let mut params = v;
    params["element"] = json!(kind.id());
    if s.run("element.configure", &params).is_ok() {
        *doc = s.doc;
    }
}

fn acts(doc: &mut Document, acts: &[&str], sequences: &[&[&str]]) {
    use ElementKind as K;
    let mut lines: Vec<(K, String)> = Vec::new();
    for (i, act) in acts.iter().enumerate() {
        lines.push((K::Outline1, (*act).to_string()));
        lines.push((K::Summary, "What happens in this act, in a few lines.".into()));
        for seq in sequences.get(i).copied().unwrap_or(&[]) {
            lines.push((K::Outline2, (*seq).to_string()));
        }
    }
    lines.push((K::SceneHeading, String::new()));
    let refs: Vec<(K, &str)> = lines.iter().map(|(k, t)| (*k, t.as_str())).collect();
    body(doc, &refs);
    // The outline shows in the script, so the acts can be seen while writing.
    set(doc, crate::outline::OUTLINE_KEY, json!({"show": true}));
}

/// A new document from built-in template `id`.
pub fn build(id: &str) -> Option<Document> {
    use ElementKind as K;
    let mut doc = new_script();
    match id {
        "screenplay" => {}
        "screenplay-guided" => {
            set(&mut doc, crate::outline::OUTLINE_KEY, json!({"show": true}));
            body(
                &mut doc,
                &[
                    (K::Note, "A scene starts with a heading: inside or outside, where, and when. Press Tab after INT to add the period."),
                    (K::SceneHeading, "INT. KITCHEN - NIGHT"),
                    (K::Note, "Action: what we see and hear, in the present tense. Return starts more action; Tab makes a character's name."),
                    (K::Action, "Rain runs down the window. A kettle starts to whistle."),
                    (K::Note, "A character's name in capitals, then what they say. Tab in dialogue makes a parenthetical."),
                    (K::Character, "MAYA"),
                    (K::Parenthetical, "(not looking up)"),
                    (K::Dialogue, "Leave it. It always does that."),
                    (K::Note, "Notes like these are outline paragraphs: they never print. Hide them with View › Show Outline."),
                ],
            );
        }
        "tv-single" => {
            body(
                &mut doc,
                &[
                    (K::NewAct, "TEASER"),
                    (K::SceneHeading, ""),
                    (K::EndOfAct, "END OF TEASER"),
                    (K::NewAct, "ACT ONE"),
                    (K::SceneHeading, ""),
                    (K::EndOfAct, "END OF ACT ONE"),
                ],
            );
        }
        "tv-multi" => {
            configure(&mut doc, K::Dialogue, json!({"lineSpacing": 2.0}));
            configure(&mut doc, K::Action, json!({"uppercase": true}));
            configure(&mut doc, K::SceneHeading, json!({"underline": true}));
            configure(&mut doc, K::CastList, json!({"uppercase": true}));
            body(&mut doc, &[(K::NewAct, "ACT ONE"), (K::SceneHeading, "INT. LIVING ROOM - DAY"), (K::CastList, "(CAST)"), (K::Action, "")]);
            set(&mut doc, crate::scene_numbers::SCENE_NUMBERS_KEY, json!({"show": true}));
        }
        "stage" => {
            configure(&mut doc, K::Character, json!({"left": 1.5, "right": 7.5, "align": "center"}));
            configure(&mut doc, K::Dialogue, json!({"left": 1.5, "right": 7.5}));
            configure(&mut doc, K::Parenthetical, json!({"left": 3.0, "right": 6.0}));
            configure(&mut doc, K::Action, json!({"left": 3.5, "right": 7.5, "italic": true}));
            configure(&mut doc, K::SceneHeading, json!({"align": "center", "underline": true}));
            body(&mut doc, &[(K::NewAct, "ACT ONE"), (K::SceneHeading, "SCENE 1"), (K::Action, "")]);
        }
        "audio" => {
            configure(&mut doc, K::Transition, json!({"name": "Sound Cue", "align": "left", "left": 1.5, "right": 7.5}));
            configure(&mut doc, K::Shot, json!({"name": "Music Cue", "bold": true}));
            body(
                &mut doc,
                &[
                    (K::SceneHeading, "SCENE 1. A BUSY STATION"),
                    (K::Transition, "SOUND: TRAINS, A CROWD, A WHISTLE."),
                    (K::Character, ""),
                    (K::Dialogue, ""),
                ],
            );
        }
        "treatment" | "bible" => {
            set(&mut doc, KIND_KEY, json!("text"));
            configure(&mut doc, K::General, json!({"lineSpacing": 1.5, "spaceBefore": 1.0}));
            configure(&mut doc, K::Outline1, json!({"bold": true}));
            set(&mut doc, crate::outline::OUTLINE_KEY, json!({"show": true}));
            set(&mut doc, crate::paginate::MORES_KEY, json!({"dialogue": false, "character": false, "scene": false}));
            let lines: &[(K, &str)] = if id == "treatment" {
                &[(K::Outline1, "Title"), (K::General, "The story in a few lines."), (K::Outline1, "The story"), (K::General, "")]
            } else {
                &[
                    (K::Outline1, "The world"),
                    (K::General, ""),
                    (K::Outline1, "The characters"),
                    (K::General, ""),
                    (K::Outline1, "The episodes"),
                    (K::General, ""),
                ]
            };
            body(&mut doc, lines);
        }
        "three-acts" => acts(&mut doc, &["Act One", "Act Two", "Act Three"], &[]),
        "four-acts" => acts(&mut doc, &["Act One", "Act Two", "Act Three", "Act Four"], &[]),
        "five-acts" => acts(&mut doc, &["Act One", "Act Two", "Act Three", "Act Four", "Act Five"], &[]),
        "sequences" => acts(
            &mut doc,
            &["Act One", "Act Two", "Act Three"],
            &[&["Sequence 1", "Sequence 2"], &["Sequence 3", "Sequence 4", "Sequence 5", "Sequence 6"], &["Sequence 7", "Sequence 8"]],
        ),
        _ => return None,
    }
    Some(doc)
}

/// A document opened from a template file, as a new one: the template flag removed.
pub fn from_template(mut doc: Document) -> Document {
    doc.app_data.remove(TEMPLATE_KEY);
    doc
}

fn list(_: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(TEMPLATES.iter().map(|t| json!({"id": t.id, "name": t.name, "category": t.category, "about": t.about})).collect::<Vec<_>>()))
}

/// `file.newFromTemplate {template}`: a new document from a built-in template.
fn new_from(s: &mut Session, v: &Value) -> CmdResult {
    let id = p::req_str(v, "template")?;
    let doc = build(id).ok_or_else(|| CmdError::Params(format!("no template `{id}`")))?;
    if s.dirty && v.get("discard").and_then(Value::as_bool) != Some(true) {
        return Err(CmdError::Failed("save the script first, or ask to discard its changes".into()));
    }
    s.set_document(doc);
    s.path = None;
    s.file_name = TEMPLATES.iter().find(|t| t.id == id).map(|t| t.name.to_string());
    Ok(json!({"template": id}))
}

fn kind(s: &mut Session, _: &Value) -> CmdResult {
    Ok(json!(if is_text(&s.doc) { "text" } else { "script" }))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("template.list", "Templates", "File", list).pure(),
        CommandSpec::new("file.newFromTemplate", "New from Template", "File", new_from).params(r#"{"template": id, "discard"?: bool}"#),
        CommandSpec::new("document.kind", "Document Kind", "File", kind).pure(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new_session;

    #[test]
    fn every_template_builds_and_paginates() {
        for t in &TEMPLATES {
            let doc = build(t.id).unwrap_or_else(new_script);
            let pag = crate::paginate::paginate(&doc);
            assert!(!pag.pages.is_empty(), "{}", t.id);
            assert_eq!(is_text(&doc), matches!(t.id, "treatment" | "bible"), "{}", t.id);
        }
        assert!(build("nope").is_none());
    }

    #[test]
    fn templates_set_their_elements() {
        let doc = build("tv-multi").unwrap_or_else(new_script);
        let mut s = crate::session_for(doc);
        let d = s.run("element.settings", &json!({"element": "Dialogue"})).unwrap();
        assert_eq!(d["lineSpacing"].as_f64(), Some(2.0));
        let doc = build("sequences").unwrap_or_else(new_script);
        let outline: Vec<String> =
            doc.body.iter().filter_map(|b| b.as_para()).filter(|p| p.props.style.as_deref() == Some("Outline2")).map(|p| p.text.clone()).collect();
        assert_eq!(outline.len(), 8);
    }

    #[test]
    fn new_from_template_refuses_over_unsaved_changes() {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "x"})).unwrap();
        s.dirty = true;
        assert!(s.run("file.newFromTemplate", &json!({"template": "stage"})).is_err());
        s.run("file.newFromTemplate", &json!({"template": "stage", "discard": true})).unwrap();
        assert_eq!(s.file_name.as_deref(), Some("Stage play"));
        assert_eq!(s.run("document.kind", &json!({})).unwrap(), "script");
        let mut t = new_session();
        t.run("file.newFromTemplate", &json!({"template": "bible"})).unwrap();
        assert_eq!(t.run("document.kind", &json!({})).unwrap(), "text");
    }
}
