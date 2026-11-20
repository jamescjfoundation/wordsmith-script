// SPDX-License-Identifier: MIT OR Apache-2.0
//! The native Wordsmith Script file (`.wss`): plain UTF-8 text, made to read and diff well.
//! The format is documented in `docs/format.md`; decision 0017 explains the choices.
//!
//! ```text
//! Wordsmith Script 3
//! === settings ===
//! { ...sorted, pretty-printed JSON: page, styles, core, appData... }
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

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wordsmith_doc::{CoreProps, Document, SectionProps, StyleSheet, para_block};
use wordsmith_script::template::new_script;
use wordsmith_script::{ElementKind, Elements};

/// The format version this build writes.
pub const VERSION: u32 = 3;
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
    /// The script's own data: elements, autocomplete, settings of every feature, by key.
    app_data: BTreeMap<String, Value>,
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

/// The file's text for `doc`.
pub fn write(doc: &Document) -> String {
    let mut page = doc.last_section.clone();
    // Headers and footers belong to the output, not the script (M5 keeps their fields in
    // app_data).
    page.headers = Default::default();
    page.footers = Default::default();
    let settings = Settings {
        page: Some(page),
        styles: Some((*doc.styles).clone()),
        core: Some(doc.core.clone()),
        app_data: doc.app_data.iter().map(|(k, v)| (k.clone(), (**v).clone())).collect(),
    };
    let json = sorted(serde_json::to_value(&settings).unwrap_or(Value::Null));
    let mut out = format!("{MAGIC} {VERSION}\n{SETTINGS_MARK}\n");
    out.push_str(&serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into()));
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
        if let Some(n) = p.props.scene_number.as_deref().filter(|n| !n.is_empty() && !n.contains(char::is_whitespace)) {
            out.push_str(&format!(" #{n}"));
        }
        if let Some(id) = p.props.omitted {
            out.push_str(&format!(" omitted={id}"));
        }
        if let Some(id) = p.props.beat {
            out.push_str(&format!(" beat={id}"));
        }
        if let Some(id) = p.props.bookmark {
            out.push_str(&format!(" bookmark={id}"));
        }
        if p.props.dual == Some(true) {
            out.push_str(" dual");
        }
        if let Some(set) = p.mark.revised {
            out.push_str(&format!(" revised={set}"));
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
    out
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
        VERSION => Ok((settings, paragraphs)),
        v if v > VERSION => Err(ReadError::NewerVersion(v)),
        v => Err(ReadError::Damaged(format!("unknown format {v}"))),
    }
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
    doc.app_data = settings.app_data.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();
    if !doc.app_data.contains_key(wordsmith_script::element::ELEMENTS_KEY) {
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
            p.props.scene_number = attrs.iter().find_map(|a| a.strip_prefix('#')).filter(|n| !n.is_empty() && n.len() <= 12).map(str::to_string);
            p.props.omitted = attrs.iter().find_map(|a| a.strip_prefix("omitted=")).and_then(|n| n.parse().ok());
            p.props.beat = attrs.iter().find_map(|a| a.strip_prefix("beat=")).and_then(|n| n.parse().ok());
            p.props.bookmark = attrs.iter().find_map(|a| a.strip_prefix("bookmark=")).and_then(|n| n.parse().ok());
            p.props.dual = attrs.iter().any(|a| a == "dual").then_some(true);
            // A deletion in the paragraph, revised in a set.
            p.mark.revised = attrs.iter().find_map(|a| a.strip_prefix("revised=")).and_then(|n| n.parse().ok());
            if attrs.iter().any(|a| a == "page-break") {
                p.props.page_break_before = Some(true);
            }
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
