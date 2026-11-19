// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scene-heading settings: the intros (INT, EXT, I/E by default, editable) and the separators
//! Tab inserts after the intro and after the location (SPEC "Typing behavior": ". " and " - ",
//! stored per script and changeable). Kept in `app_data` under [`SCENE_HEADING_KEY`].

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wordsmith_doc::Document;

/// `app_data` key of the scene-heading settings.
pub const SCENE_HEADING_KEY: &str = "wordsmith.sceneHeading";

/// Most intros kept, and the longest intro or separator, in bytes. Bounds hostile files.
pub const MAX_INTROS: usize = 64;
pub const MAX_PIECE_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SceneHeadingSettings {
    /// Scene intros, in list order (`INT`, `EXT`, `I/E`).
    pub intros: Vec<String>,
    /// Inserted by Tab after the intro.
    pub intro_separator: String,
    /// Inserted by Tab after the location.
    pub time_separator: String,
}

impl Default for SceneHeadingSettings {
    fn default() -> Self {
        SceneHeadingSettings { intros: vec!["INT".into(), "EXT".into(), "I/E".into()], intro_separator: ". ".into(), time_separator: " - ".into() }
    }
}

impl SceneHeadingSettings {
    /// The settings a document holds, or the defaults. Values over the limits are dropped or
    /// replaced by defaults, so a hostile file never fails.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(SCENE_HEADING_KEY) else { return Self::default() };
        let mut s: SceneHeadingSettings = serde_json::from_value((**v).clone()).unwrap_or_default();
        let d = Self::default();
        s.intros.retain(|i| !i.trim().is_empty() && i.len() <= MAX_PIECE_LEN);
        s.intros.truncate(MAX_INTROS);
        if s.intro_separator.is_empty() || s.intro_separator.len() > MAX_PIECE_LEN {
            s.intro_separator = d.intro_separator;
        }
        if s.time_separator.is_empty() || s.time_separator.len() > MAX_PIECE_LEN {
            s.time_separator = d.time_separator;
        }
        s
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(SCENE_HEADING_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }

    /// The intro `text` is, ignoring case and a trailing period (`int`, `INT.`), if it is one.
    pub fn intro_of(&self, text: &str) -> Option<&str> {
        let t = text.trim().trim_end_matches('.');
        self.intros.iter().map(String::as_str).find(|i| i.trim_end_matches('.').eq_ignore_ascii_case(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_follow_the_spec() {
        let s = SceneHeadingSettings::default();
        assert_eq!(s.intros, ["INT", "EXT", "I/E"]);
        assert_eq!(s.intro_separator, ". ");
        assert_eq!(s.time_separator, " - ");
        assert_eq!(s.intro_of("int"), Some("INT"));
        assert_eq!(s.intro_of("I/E."), Some("I/E"));
        assert_eq!(s.intro_of("KITCHEN"), None);
    }

    #[test]
    fn stored_and_hostile_values() {
        let mut doc = Document::new();
        let mut s = SceneHeadingSettings::default();
        s.intros.push("EST".into());
        s.time_separator = " -- ".into();
        s.store(&mut doc);
        assert_eq!(SceneHeadingSettings::of_doc(&doc), s);

        doc.app_data.insert(
            SCENE_HEADING_KEY.into(),
            Arc::new(json!({"intros": ["", "x".repeat(99), "OK"], "introSeparator": "", "timeSeparator": "y".repeat(99)})),
        );
        let s = SceneHeadingSettings::of_doc(&doc);
        assert_eq!(s.intros, ["OK"]);
        assert_eq!((s.intro_separator.as_str(), s.time_separator.as_str()), (". ", " - "));
        doc.app_data.insert(SCENE_HEADING_KEY.into(), Arc::new(json!(42)));
        assert_eq!(SceneHeadingSettings::of_doc(&doc), SceneHeadingSettings::default());
    }
}
