// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenplay pagination (SPEC "Formatting, page geometry and pagination";
//! `docs/decisions/0012-pagination.md`).
//!
//! The layout engine breaks lines; this module decides pages. It reads each paragraph's lines
//! from a layout of the script ([`measure`]), fills pages under the screenplay rules ([`plan`]),
//! and builds an output script with explicit page breaks, split paragraphs and the (MORE),
//! (CONT'D) and CONTINUED lines ([`output`]). The output is what the page view shows, PDF
//! export writes and printing prints; a map from its paragraphs back to the source keeps the
//! caret working on the script itself.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wordsmith_doc::Document;

pub mod measure;
pub mod output;
pub mod plan;

pub use output::{Paginated, Paginator, paginate};
pub use plan::{Piece, plan};

/// `app_data` key of the more-and-continued settings.
pub const MORES_KEY: &str = "wordsmith.moresContinueds";

/// Longest more or continued text, in bytes. Bounds hostile files.
pub const MAX_MARK_LEN: usize = 40;

/// The (MORE) and (CONT'D) texts and which of them are used (SPEC "Mores and continueds": one
/// settings place, each text editable).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MoresContinueds {
    /// Speeches split across a page: (MORE) at the foot, the name with (CONT'D) at the head.
    pub dialogue: bool,
    /// The same speaker again within a scene, after only action between: (CONT'D) on the name.
    pub character: bool,
    /// Scenes split across a page: (CONTINUED) at the foot, CONTINUED: at the head.
    pub scene: bool,
    /// Add the scene's page count to the head mark: CONTINUED: (2).
    pub scene_page_count: bool,
    pub more: String,
    pub contd: String,
    pub continued_top: String,
    pub continued_bottom: String,
}

impl Default for MoresContinueds {
    fn default() -> Self {
        MoresContinueds {
            dialogue: true,
            character: true,
            scene: false,
            scene_page_count: false,
            more: "(MORE)".into(),
            contd: "(CONT'D)".into(),
            continued_top: "CONTINUED:".into(),
            continued_bottom: "(CONTINUED)".into(),
        }
    }
}

impl MoresContinueds {
    /// The settings a document holds, or the defaults; texts over the limit or empty fall back.
    pub fn of_doc(doc: &Document) -> Self {
        let Some(v) = doc.app_data.get(MORES_KEY) else { return Self::default() };
        let mut s: MoresContinueds = serde_json::from_value((**v).clone()).unwrap_or_default();
        let d = Self::default();
        for (field, default) in
            [(&mut s.more, d.more), (&mut s.contd, d.contd), (&mut s.continued_top, d.continued_top), (&mut s.continued_bottom, d.continued_bottom)]
        {
            if field.trim().is_empty() || field.len() > MAX_MARK_LEN || field.contains('\n') {
                *field = default;
            }
        }
        s
    }

    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(MORES_KEY.to_string(), Arc::new(serde_json::to_value(self).unwrap_or_default()));
    }
}

/// `moresContinueds.settings` and `moresContinueds.configure {dialogue?, character?, scene?,
/// scenePageCount?, more?, contd?, continuedTop?, continuedBottom?}`: the one place the
/// (MORE) and (CONT'D) texts and switches are set (SPEC "Mores and continueds").
pub fn specs() -> Vec<wordsmith_engine::CommandSpec> {
    use serde_json::Value;
    use wordsmith_engine::{CmdError, CommandSpec};
    fn configure(s: &mut wordsmith_engine::Session, v: &Value) -> wordsmith_engine::CmdResult {
        let mut m = MoresContinueds::of_doc(&s.doc);
        for (key, slot) in
            [("dialogue", &mut m.dialogue), ("character", &mut m.character), ("scene", &mut m.scene), ("scenePageCount", &mut m.scene_page_count)]
        {
            if let Some(x) = v.get(key) {
                *slot = x.as_bool().ok_or_else(|| CmdError::Params(format!("`{key}` must be true or false")))?;
            }
        }
        for (key, slot) in
            [("more", &mut m.more), ("contd", &mut m.contd), ("continuedTop", &mut m.continued_top), ("continuedBottom", &mut m.continued_bottom)]
        {
            if let Some(x) = v.get(key) {
                let text = x.as_str().map(str::trim).filter(|t| !t.is_empty() && t.len() <= MAX_MARK_LEN && !t.contains('\n'));
                *slot = text.ok_or_else(|| CmdError::Params(format!("`{key}` is 1 to {MAX_MARK_LEN} bytes on one line")))?.to_string();
            }
        }
        m.store(&mut s.doc);
        Ok(serde_json::to_value(&m).unwrap_or_default())
    }
    const LOC: &str = "Format › Pages";
    vec![
        CommandSpec::new("moresContinueds.settings", "More and Continued Settings", LOC, |s, _| Ok(serde_json::to_value(MoresContinueds::of_doc(&s.doc)).unwrap_or_default())).pure(),
        CommandSpec::new("moresContinueds.configure", "Change More and Continued Settings", LOC, configure).params(
            r#"{"dialogue"?: bool, "character"?: bool, "scene"?: bool, "scenePageCount"?: bool, "more"?: string, "contd"?: string, "continuedTop"?: string, "continuedBottom"?: string}"#,
        ),
    ]
}

/// Split rules (SPEC leaves them to convention; decision 0012).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    /// Fewest lines of a split paragraph left at the foot of a page.
    pub min_before: usize,
    /// Fewest lines of a split paragraph carried to the next page.
    pub min_after: usize,
}

impl Default for Rules {
    fn default() -> Self {
        Rules { min_before: 2, min_after: 2 }
    }
}

#[cfg(test)]
mod tests;
