// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's data on paragraphs and runs, kept in the model's extension data (core decision 0006).
//!
//! Before 0.2 these were fields Script added to its own copy of WordCraft's model. They are now
//! typed keys in Script's `script.` namespace, and the marks the shared document features own
//! (bookmarks, notes, revision marks, page locks) use the core's `core.` keys. The model carries,
//! copies, compares, undoes and saves them without knowing what they mean.
//!
//! [`ScriptPara`] and [`ScriptRun`] read and write them as the old fields read: `None` when not
//! set, and setting `None` removes the key.

use jcj_doc::ExtKey;
use jcj_doc::props::{CharProps, ParaProps};

pub use jcj_features::keys::{BOOKMARK, NOTE, PAGE_LOCK, REVISED};

/// A scene heading's locked scene number ("12A").
pub const SCENE_NUMBER: ExtKey<String> = ExtKey::new("script.scene_number");
/// An omitted scene's placeholder heading: the id its text is kept under.
pub const OMITTED: ExtKey<u32> = ExtKey::new("script.omitted");
/// The story board beat an outline paragraph was sent from.
pub const BEAT: ExtKey<u32> = ExtKey::new("script.beat");
/// A scene heading's id for the scene's own data (title, color, cast, arc beats).
pub const SCENE_ID: ExtKey<u32> = ExtKey::new("script.scene_id");
/// The second speech of a dual-dialogue pair starts at this Character paragraph.
pub const DUAL: ExtKey<bool> = ExtKey::new("script.dual");
/// Attributes a `.wss` file had on this paragraph that this version does not know (written by a
/// newer version), kept so saving gives them back.
pub const EXTRA_ATTRS: ExtKey<Vec<String>> = ExtKey::new("script.extra_attrs");

fn put<T: serde::Serialize + serde::de::DeserializeOwned>(key: ExtKey<T>, on: &mut impl jcj_doc::HasExt, v: Option<T>) {
    match v {
        // The keys are constants in a valid namespace, so setting cannot fail.
        Some(v) => {
            let _ = key.set(on, &v);
        }
        None => key.remove(on),
    }
}

/// Script's paragraph data.
pub trait ScriptPara {
    fn scene_number(&self) -> Option<String>;
    fn set_scene_number(&mut self, v: Option<String>);
    fn omitted(&self) -> Option<u32>;
    fn set_omitted(&mut self, v: Option<u32>);
    fn beat(&self) -> Option<u32>;
    fn set_beat(&mut self, v: Option<u32>);
    fn bookmark(&self) -> Option<u32>;
    fn set_bookmark(&mut self, v: Option<u32>);
    fn scene_id(&self) -> Option<u32>;
    fn set_scene_id(&mut self, v: Option<u32>);
    fn dual(&self) -> Option<bool>;
    fn set_dual(&mut self, v: Option<bool>);
    fn extra_attrs(&self) -> Vec<String>;
    fn set_extra_attrs(&mut self, v: Vec<String>);
}

impl ScriptPara for ParaProps {
    fn scene_number(&self) -> Option<String> {
        SCENE_NUMBER.get_or_none(self)
    }
    fn set_scene_number(&mut self, v: Option<String>) {
        put(SCENE_NUMBER, self, v);
    }
    fn omitted(&self) -> Option<u32> {
        OMITTED.get_or_none(self)
    }
    fn set_omitted(&mut self, v: Option<u32>) {
        put(OMITTED, self, v);
    }
    fn beat(&self) -> Option<u32> {
        BEAT.get_or_none(self)
    }
    fn set_beat(&mut self, v: Option<u32>) {
        put(BEAT, self, v);
    }
    fn bookmark(&self) -> Option<u32> {
        BOOKMARK.get_or_none(self)
    }
    fn set_bookmark(&mut self, v: Option<u32>) {
        put(BOOKMARK, self, v);
    }
    fn scene_id(&self) -> Option<u32> {
        SCENE_ID.get_or_none(self)
    }
    fn set_scene_id(&mut self, v: Option<u32>) {
        put(SCENE_ID, self, v);
    }
    fn dual(&self) -> Option<bool> {
        DUAL.get_or_none(self)
    }
    fn set_dual(&mut self, v: Option<bool>) {
        put(DUAL, self, v);
    }
    fn extra_attrs(&self) -> Vec<String> {
        EXTRA_ATTRS.get_or_none(self).unwrap_or_default()
    }
    fn set_extra_attrs(&mut self, v: Vec<String>) {
        put(EXTRA_ATTRS, self, (!v.is_empty()).then_some(v));
    }
}

/// Script's marks on runs of text (and on a paragraph's mark, `Paragraph::mark`: a revision set
/// that deleted text in the paragraph).
pub trait ScriptRun {
    /// The note this text is anchored to.
    fn note(&self) -> Option<u32>;
    fn set_note(&mut self, v: Option<u32>);
    /// The revision set this text was revised in.
    fn revised(&self) -> Option<u32>;
    fn set_revised(&mut self, v: Option<u32>);
    /// A locked page starts at this character (the lock's id).
    fn page_lock(&self) -> Option<u32>;
    fn set_page_lock(&mut self, v: Option<u32>);
}

impl ScriptRun for CharProps {
    fn note(&self) -> Option<u32> {
        NOTE.get_or_none(self)
    }
    fn set_note(&mut self, v: Option<u32>) {
        put(NOTE, self, v);
    }
    fn revised(&self) -> Option<u32> {
        REVISED.get_or_none(self)
    }
    fn set_revised(&mut self, v: Option<u32>) {
        put(REVISED, self, v);
    }
    fn page_lock(&self) -> Option<u32> {
        PAGE_LOCK.get_or_none(self)
    }
    fn set_page_lock(&mut self, v: Option<u32>) {
        put(PAGE_LOCK, self, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_read_as_they_were_written() {
        let mut p = ParaProps::default();
        assert_eq!(p.scene_number(), None);
        p.set_scene_number(Some("12A".into()));
        p.set_dual(Some(true));
        p.set_extra_attrs(vec!["future=1".into()]);
        assert_eq!(p.scene_number().as_deref(), Some("12A"));
        assert_eq!(p.dual(), Some(true));
        assert_eq!(p.extra_attrs(), vec!["future=1".to_string()]);
        p.set_scene_number(None);
        p.set_extra_attrs(Vec::new());
        assert_eq!(p.scene_number(), None);
        assert!(!EXTRA_ATTRS.is_set(&p));
        let mut c = CharProps::default();
        c.set_note(Some(3));
        c.set_page_lock(Some(7));
        assert_eq!((c.note(), c.revised(), c.page_lock()), (Some(3), None, Some(7)));
        assert_eq!(c.ext.get("core.note"), Some(&serde_json::json!(3)));
    }
}
