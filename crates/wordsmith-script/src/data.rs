// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and writing a settings group kept in the document's `app_data` (decision 0008).
//! A value that does not parse gives the defaults, so a hostile file never fails to open;
//! each group's own `repaired` step bounds what it read.

use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use wordsmith_doc::{Document, Path, Pos, StoryRef};

/// The group stored under `key`, or its defaults.
pub fn load<T: DeserializeOwned + Default>(doc: &Document, key: &str) -> T {
    doc.app_data.get(key).and_then(|v| serde_json::from_value((**v).clone()).ok()).unwrap_or_default()
}

/// Store `value` under `key`.
pub fn store<T: Serialize>(doc: &mut Document, key: &str, value: &T) {
    doc.app_data.insert(key.to_string(), Arc::new(serde_json::to_value(value).unwrap_or_default()));
}

/// A position in body paragraph `block`.
pub fn body_pos(block: usize, off: usize) -> Pos {
    Pos { story: StoryRef::Body, path: Path::top(block), off }
}

/// Text cut to `max` characters with control characters dropped.
pub fn clean_line(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}
