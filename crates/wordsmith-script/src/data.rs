// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and writing a settings group Script keeps in the document's extension data (core
//! decision 0006), under its `wordsmith.` keys. A value that does not parse gives the defaults, so
//! a hostile file never fails to open; each group's own `repaired` step bounds what it read.
//!
//! The shared helpers (positions, clean lines, colors) are the core's (`jcj_features::data`).

use jcj_doc::Document;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use jcj_features::data::{body_pos, clean_line, color_hex, parse_color, pos_at};

/// The group stored under `key`, or its defaults.
pub fn load<T: DeserializeOwned + Default>(doc: &Document, key: &str) -> T {
    doc.ext.get(key).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

/// The raw value stored under `key`.
pub fn raw<'a>(doc: &'a Document, key: &str) -> Option<&'a serde_json::Value> {
    doc.ext.get(key)
}

/// Store `value` under `key` (Script's keys are constants in its own namespace, so this cannot
/// fail; a value that does not serialize is stored as `null`, which removes the key).
pub fn store<T: Serialize>(doc: &mut Document, key: &str, value: &T) {
    let _ = doc.ext.set(key, serde_json::to_value(value).unwrap_or_default());
}

/// Store a raw value under `key`.
pub fn store_raw(doc: &mut Document, key: &str, value: serde_json::Value) {
    let _ = doc.ext.set(key, value);
}
