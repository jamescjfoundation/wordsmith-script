// SPDX-License-Identifier: MIT OR Apache-2.0
//! The native Wordsmith Script file (`.wss`): plain UTF-8 text, made to read and diff well.
//! The format is documented in `docs/format.md`; decision 0017 explains the choices.
//!
//! ```text
//! Wordsmith Script 5
//! === settings ===
//! { ...sorted, pretty-printed JSON: page, styles, core, ext... }
//! === script ===
//! @SceneHeading
//! INT. KITCHEN - NIGHT
//!
//! @Action
//! Rain *hammers* the window.
//! ```
//!
//! Reading is defensive: the size, the paragraph count and the settings are bounded, a file
//! from a newer version is refused with a clear message, older versions are migrated, and
//! anything the settings lack is filled from the screenplay template.

use std::collections::BTreeMap;
use std::sync::Arc;

use jcj_doc::{CoreProps, Document, SectionProps, StyleSheet, para_block};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wordsmith_script::keys::{ScriptPara, ScriptRun};
use wordsmith_script::template::new_script;
use wordsmith_script::{ElementKind, Elements, scene_numbers};

/// The format version this build writes.
pub const VERSION: u32 = 5;
/// Most tracked-change records a file may hold: as many as tracking makes. A script with more
/// is refused when saving rather than written without some of them.
pub const MAX_REVISIONS: usize = wordsmith_script::tracking::MAX_RECORDS;
/// Most attributes of a newer version kept on one paragraph, and the longest one kept.
pub const MAX_EXTRA_ATTRS: usize = 32;
pub const MAX_EXTRA_ATTR_LEN: usize = 256;
/// The first line's words before the version number.
pub const MAGIC: &str = "Wordsmith Script";
/// Largest native file read, bytes.
pub const MAX_BYTES: usize = 64 << 20;
/// Largest settings part, bytes.
pub const MAX_SETTINGS_BYTES: usize = 16 << 20;
/// Most paragraphs a script may have.
pub const MAX_PARAGRAPHS: usize = 500_000;

const SETTINGS_MARK: &str = "=== settings ===";
const SCRIPT_MARK: &str = "=== script ===";

/// Everything but the paragraphs, as the settings part holds it.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    /// Page size and margins.
    page: Option<SectionProps>,
    /// The element styles (and document defaults).
    styles: Option<StyleSheet>,
    /// Title and the other document properties.
    core: Option<CoreProps>,
    /// The document's extension data, by key (format 5; `appData` before): Script's own data
    /// (`wordsmith.` keys: elements, autocomplete, scene numbers...) and the shared features'
    /// (`core.` keys: notes, revisions, page locks, bookmarks, watermark, header and footer,
    /// the script's own correct words).
    ext: BTreeMap<String, Value>,
    /// Changes are tracked (format 5; before, `appData` `wordsmith.tracking`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    track_changes: bool,
    /// Tracked changes' authors and dates (format 4; marked in the text as `{sI3}`, `{sX3}`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    revisions: Vec<jcj_doc::Revision>,
}

/// Why a file could not be read.
#[derive(Debug, PartialEq)]
pub enum ReadError {
    NotNative,
    TooLarge,
    NewerVersion(u32),
    Damaged(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::NotNative => write!(f, "this is not a Wordsmith Script file"),
            ReadError::TooLarge => write!(f, "the file is larger than {} MB", MAX_BYTES >> 20),
            ReadError::NewerVersion(v) => {
                write!(f, "the file was made by a newer Wordsmith Script (format {v}); this version reads up to format {VERSION}")
            }
            ReadError::Damaged(why) => write!(f, "the file is damaged: {why}"),
        }
    }
}

/// Why a script could not be written.
#[derive(Debug, PartialEq)]
pub enum WriteError {
    /// The settings could not be turned into JSON.
    Settings(String),
    /// More tracked-change records than a file holds.
    TooManyRevisions(usize),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Settings(why) => write!(f, "the script's settings could not be saved: {why}"),
            WriteError::TooManyRevisions(n) => {
                write!(f, "the script has {n} tracked-change records and a file holds at most {MAX_REVISIONS}; accept or reject some changes first")
            }
        }
    }
}

impl std::error::Error for WriteError {}

/// Whether an `@` line attribute is one this version reads.
fn known_attr(a: &str) -> bool {
    a == "page-break"
        || a == "dual"
        || a.starts_with('#')
        || ["omitted=", "beat=", "bookmark=", "scene=", "revised="].iter().any(|p| a.starts_with(p))
}

/// The file's text for `doc`.
pub fn write(doc: &Document) -> Result<String, WriteError> {
    if doc.revisions.len() > MAX_REVISIONS {
        return Err(WriteError::TooManyRevisions(doc.revisions.len()));
    }
    let mut page = doc.last_section.clone();
    // Headers and footers belong to the output, not the script: their fields are kept in
    // the extension data (decision 0021).
    page.headers = Default::default();
    page.footers = Default::default();
    let settings = Settings {
        page: Some(page),
        styles: Some((*doc.styles).clone()),
        core: Some(doc.core.clone()),
        ext: doc.ext.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        track_changes: doc.settings.track_changes,
        revisions: doc.revisions.clone(),
    };
    let json = sorted(serde_json::to_value(&settings).map_err(|e| WriteError::Settings(e.to_string()))?);
    let mut out = format!("{MAGIC} {VERSION}\n{SETTINGS_MARK}\n");
    out.push_str(&serde_json::to_string_pretty(&json).map_err(|e| WriteError::Settings(e.to_string()))?);
    out.push('\n');
    out.push_str(SCRIPT_MARK);
    out.push('\n');
    let elements = Elements::of_doc(doc);
    for p in doc.body.iter().filter_map(|b| b.as_para()) {
        out.push('@');
        out.push_str(&elements.of_para(p).id);
        if p.props.page_break_before == Some(true) {
            out.push_str(" page-break");
        }
        // Only a number the reader keeps is written: one rule for both (`scene_numbers::clean`).
        if let Some(n) = p.props.scene_number().as_deref().and_then(scene_numbers::clean) {
            out.push_str(&format!(" #{n}"));
        }
        if let Some(id) = p.props.omitted() {
            out.push_str(&format!(" omitted={id}"));
        }
        if let Some(id) = p.props.beat() {
            out.push_str(&format!(" beat={id}"));
        }
        if let Some(id) = p.props.bookmark() {
            out.push_str(&format!(" bookmark={id}"));
        }
        if p.props.dual() == Some(true) {
            out.push_str(" dual");
        }
        if let Some(id) = p.props.scene_id() {
            out.push_str(&format!(" scene={id}"));
        }
        if let Some(set) = p.mark.revised() {
            out.push_str(&format!(" revised={set}"));
        }
        // A newer version's attributes, as they were read.
        for a in p.props.extra_attrs().iter().filter(|a| extra_attr_ok(a)) {
            out.push(' ');
            out.push_str(a);
        }
        out.push('\n');
        let encoded = crate::emphasis::encode_with(p, true, &|_| None);
        // An empty paragraph has no text lines at all.
        let text_lines: Vec<&str> = if encoded.is_empty() { Vec::new() } else { encoded.split('\n').collect() };
        for line in text_lines {
            if line.is_empty() {
                // An empty line inside a paragraph (two line breaks in a row).
                out.push_str("\\\n");
            } else {
                if line.starts_with('@') || line.starts_with('=') {
                    out.push('\\');
                }
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push('\n');
    }
    Ok(out)
}

/// Whether `a` can be kept as an attribute of a newer version: one word, not one this version
/// reads, within the bounds.
fn extra_attr_ok(a: &str) -> bool {
    !a.is_empty() && a.len() <= MAX_EXTRA_ATTR_LEN && !a.contains(char::is_whitespace) && !known_attr(a)
}

/// JSON with every object's keys in order, so the file is stable and diffs are small.
fn sorted(v: Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, sorted(v))).collect::<BTreeMap<_, _>>().into_iter().collect()),
        Value::Array(a) => Value::Array(a.into_iter().map(sorted).collect()),
        other => other,
    }
}

/// The version on a file's first line, if it is a native file.
pub fn version_of(text: &str) -> Option<u32> {
    let first = text.trim_start_matches('\u{feff}').lines().next()?;
    first.strip_prefix(MAGIC)?.trim().parse().ok()
}

/// Bring a file's parts from `from` up to [`VERSION`]. Each step is one function, so a future
/// format change adds a step and keeps every older file readable.
fn migrate(
    from: u32,
    settings: Value,
    paragraphs: Vec<(String, Vec<String>, String)>,
) -> Result<(Value, Vec<(String, Vec<String>, String)>), ReadError> {
    match from {
        // 1 → 2: notes' anchors are marked in the text (`{n12}…{/n}`), so a `{` in older text is
        // escaped to stay a plain brace.
        1 => {
            let paragraphs = paragraphs.into_iter().map(|(id, attrs, text)| (id, attrs, text.replace('{', "\\{"))).collect();
            migrate(2, settings, paragraphs)
        }
        // 2 → 3: revision and locked-page marks join the note marks; `{` was already escaped.
        2 => migrate(3, settings, paragraphs),
        // 3 → 4: more character styles (`{sD,K}…{/s}`) join the marks; `{` was already escaped.
        3 => migrate(4, settings, paragraphs),
        // 4 → 5: the script's data becomes the document's extension data (`ext`); the shared
        // features' groups move to the core's keys, and change tracking to its own setting.
        4 => migrate(5, to_ext(settings), paragraphs),
        VERSION => Ok((settings, paragraphs)),
        v if v > VERSION => Err(ReadError::NewerVersion(v)),
        v => Err(ReadError::Damaged(format!("unknown format {v}"))),
    }
}

/// Script's `appData` keys whose data the shared features now keep under the core's keys
/// (format 5). The values keep their shape.
pub const MOVED_KEYS: [(&str, &str); 8] = [
    ("wordsmith.notes", "core.notes"),
    ("wordsmith.noteTypes", "core.note_types"),
    ("wordsmith.revisions", "core.revisions"),
    ("wordsmith.locks", "core.page_locks"),
    ("wordsmith.bookmarks", "core.bookmarks"),
    ("wordsmith.watermark", "core.watermark"),
    ("wordsmith.headerFooter", "core.header_footer"),
    ("wordsmith.spelling", "core.spelling"),
];

/// Format 4 settings as format 5: `appData` becomes `ext` with the moved keys renamed, and
/// `wordsmith.tracking` becomes `trackChanges`.
fn to_ext(settings: Value) -> Value {
    let Value::Object(mut s) = settings else { return settings };
    let mut ext = match s.remove("appData") {
        Some(Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    };
    for (old, new) in MOVED_KEYS {
        if let Some(v) = ext.remove(old) {
            ext.insert(new.to_string(), v);
        }
    }
    if let Some(t) = ext.remove("wordsmith.tracking")
        && t.get("on").and_then(Value::as_bool) == Some(true)
    {
        s.insert("trackChanges".into(), Value::Bool(true));
    }
    s.insert("ext".into(), Value::Object(ext));
    Value::Object(s)
}

/// Read a native file.
pub fn read(text: &str) -> Result<Document, ReadError> {
    if text.len() > MAX_BYTES {
        return Err(ReadError::TooLarge);
    }
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let version = version_of(&text).ok_or(ReadError::NotNative)?;
    if version > VERSION {
        return Err(ReadError::NewerVersion(version));
    }
    let after_magic = text.split_once('\n').map_or("", |(_, rest)| rest);
    let rest =
        after_magic.strip_prefix(SETTINGS_MARK).and_then(|r| r.strip_prefix('\n')).ok_or_else(|| ReadError::Damaged("no settings part".into()))?;
    let marker = format!("\n{SCRIPT_MARK}\n");
    let (json_text, script) = match rest.split_once(&marker) {
        Some(parts) => parts,
        None => rest.strip_suffix(&format!("\n{SCRIPT_MARK}")).map(|j| (j, "")).ok_or_else(|| ReadError::Damaged("no script part".into()))?,
    };
    if json_text.len() > MAX_SETTINGS_BYTES {
        return Err(ReadError::Damaged("the settings part is too large".into()));
    }
    let settings: Value = serde_json::from_str(json_text).map_err(|e| ReadError::Damaged(format!("settings: {e}")))?;

    // Paragraphs: (element id, attributes, encoded text).
    let mut paragraphs: Vec<(String, Vec<String>, String)> = Vec::new();
    let mut current: Option<(String, Vec<String>, Vec<String>)> = None;
    for line in script.split('\n') {
        if let Some((id, attrs, lines)) = current.as_mut() {
            if line.is_empty() {
                paragraphs.push((std::mem::take(id), std::mem::take(attrs), lines.join("\n")));
                current = None;
                continue;
            }
            let line = if line == "\\" { "" } else { line.strip_prefix('\\').filter(|r| r.starts_with('@') || r.starts_with('=')).unwrap_or(line) };
            lines.push(line.to_string());
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let head = line
            .strip_prefix('@')
            .ok_or_else(|| ReadError::Damaged(format!("expected `@Element`, found {:?}", line.chars().take(40).collect::<String>())))?;
        let mut words = head.split_whitespace();
        let id = words.next().unwrap_or("").to_string();
        if id.is_empty() || id.len() > wordsmith_script::element::MAX_NAME_LEN {
            return Err(ReadError::Damaged("a paragraph without an element".into()));
        }
        current = Some((id, words.map(str::to_string).collect(), Vec::new()));
        if paragraphs.len() >= MAX_PARAGRAPHS {
            return Err(ReadError::Damaged(format!("more than {MAX_PARAGRAPHS} paragraphs")));
        }
    }
    if let Some((id, attrs, lines)) = current {
        paragraphs.push((id, attrs, lines.join("\n")));
    }
    let (settings, paragraphs) = migrate(version, settings, paragraphs)?;
    let settings: Settings = serde_json::from_value(settings).map_err(|e| ReadError::Damaged(format!("settings: {e}")))?;

    // Start from the template and take what the file has.
    let mut doc = new_script();
    if let Some(page) = settings.page {
        doc.last_section = page;
        doc.last_section.headers = Default::default();
        doc.last_section.footers = Default::default();
    }
    if let Some(styles) = settings.styles {
        doc.styles = Arc::new(styles);
    }
    if let Some(core) = settings.core {
        doc.core = core;
    }
    doc.ext = jcj_doc::Ext::new();
    for (k, v) in settings.ext {
        // A key that is not `namespace.name` cannot be kept; it is left out.
        let _ = doc.ext.set(&k, v);
    }
    doc.settings.track_changes = settings.track_changes;
    if settings.revisions.len() > MAX_REVISIONS {
        return Err(ReadError::Damaged(format!("more than {MAX_REVISIONS} tracked-change records")));
    }
    doc.revisions = settings.revisions;
    if !doc.ext.contains(wordsmith_script::element::ELEMENTS_KEY) {
        Elements::defaults().store(&mut doc);
    }
    repair_styles(&mut doc);
    let elements = Elements::of_doc(&doc);
    let general = elements.id_of(ElementKind::General).to_string();
    doc.body = paragraphs
        .into_iter()
        .map(|(id, attrs, text)| {
            // A paragraph of an element the file does not define reads as General.
            let style = if elements.get(&id).is_some() { id } else { general.clone() };
            let mut p = crate::emphasis::decode_with(&style, &text, false, true);
            // A locked scene number, and an omitted scene's placeholder.
            p.props.set_scene_number(attrs.iter().find_map(|a| a.strip_prefix('#')).and_then(scene_numbers::clean));
            p.props.set_omitted(attrs.iter().find_map(|a| a.strip_prefix("omitted=")).and_then(|n| n.parse().ok()));
            p.props.set_beat(attrs.iter().find_map(|a| a.strip_prefix("beat=")).and_then(|n| n.parse().ok()));
            p.props.set_bookmark(attrs.iter().find_map(|a| a.strip_prefix("bookmark=")).and_then(|n| n.parse().ok()));
            p.props.set_dual(attrs.iter().any(|a| a == "dual").then_some(true));
            p.props.set_scene_id(attrs.iter().find_map(|a| a.strip_prefix("scene=")).and_then(|n| n.parse().ok()));
            // A deletion in the paragraph, revised in a set.
            p.mark.set_revised(attrs.iter().find_map(|a| a.strip_prefix("revised=")).and_then(|n| n.parse().ok()));
            if attrs.iter().any(|a| a == "page-break") {
                p.props.page_break_before = Some(true);
            }
            // Attributes of a newer version are kept, in order, and written back on save.
            p.props.set_extra_attrs(attrs.iter().filter(|a| extra_attr_ok(a)).take(MAX_EXTRA_ATTRS).cloned().collect());
            para_block(p)
        })
        .collect();
    doc.ensure_nonempty();
    Ok(doc)
}

/// Every element has a paragraph style; one the file lacks comes from the template.
fn repair_styles(doc: &mut Document) {
    let elements = Elements::of_doc(doc);
    let template = wordsmith_script::template::style_sheet(&elements);
    let sheet = Arc::make_mut(&mut doc.styles);
    for def in elements.all() {
        if sheet.get(&def.id).is_none()
            && let Some(st) = template.get(&def.id)
        {
            sheet.upsert(st.clone());
        }
    }
}

#[cfg(test)]
mod tests;
