// SPDX-License-Identifier: MIT OR Apache-2.0
//! The element registry: every paragraph in a script has exactly one element, and each element
//! is a paragraph style plus the screenplay behavior a style cannot express (casing, the Tab
//! target, paginate-as, a shortcut). See `docs/SPEC.md`, "Document model and elements".
//!
//! Formatting lives in the element's paragraph style and nowhere else: font, size, bold,
//! italic, underline, indents, alignment, line spacing, space before, the next element on
//! Return (`Style::next`), start new page and keep rules. The registry is stored in the
//! document's `app_data` under [`ELEMENTS_KEY`], so undo covers it.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wordsmith_doc::{Document, Paragraph};

/// `app_data` key of the element registry.
pub const ELEMENTS_KEY: &str = "wordsmith.elements";

/// Most elements a script may define, built-in ones included. Bounds hostile files.
pub const MAX_ELEMENTS: usize = 128;

/// Longest element id or name, in bytes. Bounds hostile files.
pub const MAX_NAME_LEN: usize = 64;

/// What an element is for. Built-in elements have their own kind; elements the writer makes are
/// [`ElementKind::Custom`]. Code that needs "the scene heading element" asks for a kind, never a
/// name, so renaming an element changes nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ElementKind {
    General,
    SceneHeading,
    Action,
    Character,
    Dialogue,
    Parenthetical,
    Transition,
    Shot,
    CastList,
    NewAct,
    EndOfAct,
    /// A run of scenes inside an act (SPEC-2 3.1).
    Sequence,
    Outline1,
    Outline2,
    Outline3,
    Note,
    Summary,
    Custom,
}

impl ElementKind {
    /// Every built-in kind, in the order menus list them.
    pub const BUILTIN: [ElementKind; 17] = [
        ElementKind::General,
        ElementKind::SceneHeading,
        ElementKind::Action,
        ElementKind::Character,
        ElementKind::Parenthetical,
        ElementKind::Dialogue,
        ElementKind::Transition,
        ElementKind::Shot,
        ElementKind::CastList,
        ElementKind::NewAct,
        ElementKind::EndOfAct,
        ElementKind::Sequence,
        ElementKind::Outline1,
        ElementKind::Outline2,
        ElementKind::Outline3,
        ElementKind::Note,
        ElementKind::Summary,
    ];

    /// The built-in element's id, which is also its paragraph style id.
    pub fn id(self) -> &'static str {
        match self {
            ElementKind::General => "General",
            ElementKind::SceneHeading => "SceneHeading",
            ElementKind::Action => "Action",
            ElementKind::Character => "Character",
            ElementKind::Dialogue => "Dialogue",
            ElementKind::Parenthetical => "Parenthetical",
            ElementKind::Transition => "Transition",
            ElementKind::Shot => "Shot",
            ElementKind::CastList => "CastList",
            ElementKind::NewAct => "NewAct",
            ElementKind::EndOfAct => "EndOfAct",
            ElementKind::Sequence => "Sequence",
            ElementKind::Outline1 => "Outline1",
            ElementKind::Outline2 => "Outline2",
            ElementKind::Outline3 => "Outline3",
            ElementKind::Note => "Note",
            ElementKind::Summary => "Summary",
            ElementKind::Custom => "Custom",
        }
    }

    /// The built-in element's default display name.
    pub fn default_name(self) -> &'static str {
        match self {
            ElementKind::General => "General",
            ElementKind::SceneHeading => "Scene Heading",
            ElementKind::Action => "Action",
            ElementKind::Character => "Character",
            ElementKind::Dialogue => "Dialogue",
            ElementKind::Parenthetical => "Parenthetical",
            ElementKind::Transition => "Transition",
            ElementKind::Shot => "Shot",
            ElementKind::CastList => "Cast List",
            ElementKind::NewAct => "New Act",
            ElementKind::EndOfAct => "End of Act",
            ElementKind::Sequence => "Sequence",
            ElementKind::Outline1 => "Outline 1",
            ElementKind::Outline2 => "Outline 2",
            ElementKind::Outline3 => "Outline 3",
            ElementKind::Note => "Note",
            ElementKind::Summary => "Summary",
            ElementKind::Custom => "Custom",
        }
    }

    /// The fixed shortcut of the ten core elements (`Mod+0` to `Mod+9`, SPEC "Typing behavior").
    pub fn core_shortcut(self) -> Option<&'static str> {
        Some(match self {
            ElementKind::General => "Mod+0",
            ElementKind::SceneHeading => "Mod+1",
            ElementKind::Action => "Mod+2",
            ElementKind::Character => "Mod+3",
            ElementKind::Parenthetical => "Mod+4",
            ElementKind::Dialogue => "Mod+5",
            ElementKind::Transition => "Mod+6",
            ElementKind::Shot => "Mod+7",
            ElementKind::CastList => "Mod+8",
            ElementKind::NewAct => "Mod+9",
            _ => return None,
        })
    }

    /// The kind of a built-in element id.
    pub fn from_builtin_id(id: &str) -> Option<ElementKind> {
        ElementKind::BUILTIN.into_iter().find(|k| k.id() == id)
    }

    /// The element's place in the story's structure, broadest first (SPEC-2 3.1): Outline 1,
    /// acts, Outline 2, Outline 3, Sequence, Scene Heading. Outline 3's place was open in the
    /// spec; it sits under Outline 2 (decision 0031). Other elements have no rank.
    pub fn structure_rank(self) -> Option<u8> {
        Some(match self {
            ElementKind::Outline1 => 0,
            ElementKind::NewAct | ElementKind::EndOfAct => 1,
            ElementKind::Outline2 => 2,
            ElementKind::Outline3 => 3,
            ElementKind::Sequence => 4,
            ElementKind::SceneHeading => 5,
            _ => return None,
        })
    }

    /// Outline elements, sent from the outliner and shown or hidden as a group (M7).
    pub fn is_outline(self) -> bool {
        matches!(self, ElementKind::Outline1 | ElementKind::Outline2 | ElementKind::Outline3 | ElementKind::Note | ElementKind::Summary)
    }
}

/// How an element's text is cased.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Casing {
    /// Text is kept as typed.
    #[default]
    AsTyped,
    /// Text is stored in capitals: typing converts it, and so does changing a paragraph into the
    /// element (`docs/decisions/0009-element-model.md`).
    Upper,
    /// Text is stored in small letters (SPEC-2 3.2 element settings).
    Lower,
}

impl Casing {
    /// `text` cased for this element.
    pub fn apply(self, text: &str) -> String {
        match self {
            Casing::AsTyped => text.to_string(),
            Casing::Upper => text.to_uppercase(),
            Casing::Lower => text.to_lowercase(),
        }
    }
}

/// One element: its identity and the behavior a paragraph style cannot hold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementDef {
    /// Stable id, also the id of the element's paragraph style.
    pub id: String,
    /// Display name.
    pub name: String,
    pub kind: ElementKind,
    #[serde(default)]
    pub casing: Casing,
    /// The element Tab switches the paragraph to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<String>,
    /// Tab switches only when the paragraph is empty (Character: SPEC Tab table).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tab_only_when_blank: bool,
    /// The element whose page-break rules this one follows (M2); `None` follows its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paginate_as: Option<String>,
    /// The shortcut that switches a paragraph to this element. Core elements have fixed ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}

impl ElementDef {
    pub fn is_builtin(&self) -> bool {
        self.kind != ElementKind::Custom
    }
}

/// The default behavior of a built-in element. Formatting defaults are in [`crate::template`].
pub fn builtin_def(kind: ElementKind) -> ElementDef {
    use ElementKind as K;
    let (casing, tab, blank) = match kind {
        K::SceneHeading => (Casing::Upper, Some(K::Action), false),
        K::Action => (Casing::AsTyped, Some(K::Character), false),
        K::Character => (Casing::Upper, Some(K::Transition), true),
        K::Dialogue => (Casing::AsTyped, Some(K::Parenthetical), false),
        K::Parenthetical => (Casing::AsTyped, Some(K::Dialogue), false),
        K::Transition => (Casing::Upper, Some(K::SceneHeading), false),
        K::Shot => (Casing::Upper, None, false),
        K::NewAct | K::EndOfAct => (Casing::Upper, None, false),
        K::Sequence => (Casing::Upper, Some(K::SceneHeading), false),
        K::Outline1 => (Casing::AsTyped, Some(K::Outline2), false),
        K::Outline2 => (Casing::AsTyped, Some(K::Outline3), false),
        K::Outline3 | K::Note => (Casing::AsTyped, Some(K::Note), false),
        K::General | K::CastList | K::Summary | K::Custom => (Casing::AsTyped, None, false),
    };
    ElementDef {
        id: kind.id().to_string(),
        name: kind.default_name().to_string(),
        kind,
        casing,
        tab: tab.map(|k| k.id().to_string()),
        tab_only_when_blank: blank,
        paginate_as: None,
        shortcut: kind.core_shortcut().map(str::to_string),
    }
}

/// A script's elements, in menu order.
#[derive(Clone, Debug, PartialEq)]
pub struct Elements {
    defs: Vec<ElementDef>,
}

impl Default for Elements {
    fn default() -> Self {
        Elements::defaults()
    }
}

impl Elements {
    /// The built-in elements with their default behavior.
    pub fn defaults() -> Self {
        Elements { defs: ElementKind::BUILTIN.into_iter().map(builtin_def).collect() }
    }

    pub fn all(&self) -> &[ElementDef] {
        &self.defs
    }

    pub fn get(&self, id: &str) -> Option<&ElementDef> {
        self.defs.iter().find(|d| d.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ElementDef> {
        self.defs.iter_mut().find(|d| d.id == id)
    }

    /// The element of a built-in kind. Every registry has one of each (see [`Elements::from_value`]).
    pub fn of_kind(&self, kind: ElementKind) -> Option<&ElementDef> {
        self.defs.iter().find(|d| d.kind == kind)
    }

    /// The id of the element of `kind`, or the kind's default id.
    pub fn id_of(&self, kind: ElementKind) -> &str {
        self.of_kind(kind).map_or(kind.id(), |d| d.id.as_str())
    }

    /// Add a custom element. Fails when the id is taken, empty, too long or the registry is full.
    pub fn push(&mut self, def: ElementDef) -> Result<(), String> {
        if def.id.is_empty() || def.id.len() > MAX_NAME_LEN || def.name.is_empty() || def.name.len() > MAX_NAME_LEN {
            return Err(format!("element ids and names are 1 to {MAX_NAME_LEN} bytes"));
        }
        if self.get(&def.id).is_some() {
            return Err(format!("there is already an element `{}`", def.id));
        }
        if self.defs.len() >= MAX_ELEMENTS {
            return Err(format!("a script has at most {MAX_ELEMENTS} elements"));
        }
        self.defs.push(def);
        Ok(())
    }

    /// Remove a custom element. Built-in elements cannot be removed.
    pub fn remove(&mut self, id: &str) -> Result<ElementDef, String> {
        let i = self.defs.iter().position(|d| d.id == id).ok_or_else(|| format!("no element `{id}`"))?;
        if self.defs.get(i).is_some_and(ElementDef::is_builtin) {
            return Err(format!("`{id}` is built in and cannot be removed"));
        }
        let removed = self.defs.remove(i);
        // Nothing may point at an element that is gone.
        for d in &mut self.defs {
            if d.tab.as_deref() == Some(id) {
                d.tab = None;
            }
            if d.paginate_as.as_deref() == Some(id) {
                d.paginate_as = None;
            }
        }
        Ok(removed)
    }

    /// A fresh id for a custom element (`Custom1`, `Custom2`…).
    pub fn new_custom_id(&self) -> String {
        let mut n = 1usize;
        loop {
            let id = format!("Custom{n}");
            if self.get(&id).is_none() {
                return id;
            }
            n += 1;
        }
    }

    /// The registry a document holds, or the defaults when it holds none. A stored value that
    /// does not parse also gives the defaults: a hostile or damaged file never fails to open.
    pub fn of_doc(doc: &Document) -> Elements {
        match doc.app_data.get(ELEMENTS_KEY) {
            Some(v) => Elements::from_value(v),
            None => Elements::defaults(),
        }
    }

    /// Parse and repair a stored registry: entries over the limits are dropped, duplicates keep
    /// the first, every built-in kind is present (missing ones come back with defaults), core
    /// shortcuts are fixed, and references to unknown elements are cleared.
    pub fn from_value(v: &serde_json::Value) -> Elements {
        let parsed: Vec<ElementDef> = match serde_json::from_value(v.get("elements").cloned().unwrap_or(serde_json::Value::Null)) {
            Ok(list) => list,
            Err(e) => {
                log::warn!("element registry unreadable, using defaults: {e}");
                return Elements::defaults();
            }
        };
        let mut defs: Vec<ElementDef> = Vec::new();
        for mut d in parsed.into_iter().take(MAX_ELEMENTS) {
            let ok_len = |s: &str| !s.is_empty() && s.len() <= MAX_NAME_LEN;
            if !ok_len(&d.id) || !ok_len(&d.name) || defs.iter().any(|e| e.id == d.id) {
                continue;
            }
            if d.kind != ElementKind::Custom {
                // One element per built-in kind, always with its own id and fixed shortcut.
                if defs.iter().any(|e| e.kind == d.kind) || d.id != d.kind.id() {
                    continue;
                }
                d.shortcut = d.kind.core_shortcut().map(str::to_string);
            } else if d.shortcut.as_deref().is_some_and(|s| s.len() > MAX_NAME_LEN) {
                d.shortcut = None;
            }
            defs.push(d);
        }
        for kind in ElementKind::BUILTIN {
            if !defs.iter().any(|d| d.kind == kind) {
                defs.push(builtin_def(kind));
            }
        }
        // Over the limit, custom elements give way, newest first: built-in ones always stay.
        while defs.len() > MAX_ELEMENTS {
            match defs.iter().rposition(|d| d.kind == ElementKind::Custom) {
                Some(i) => {
                    defs.remove(i);
                }
                None => break,
            }
        }
        let ids: Vec<String> = defs.iter().map(|d| d.id.clone()).collect();
        for d in &mut defs {
            if d.tab.as_ref().is_some_and(|t| !ids.contains(t)) {
                d.tab = None;
            }
            if d.paginate_as.as_ref().is_some_and(|t| !ids.contains(t)) {
                d.paginate_as = None;
            }
        }
        Elements { defs }
    }

    /// The registry as stored in `app_data`.
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({"version": 1, "elements": self.defs})
    }

    /// Store the registry in the document.
    pub fn store(&self, doc: &mut Document) {
        doc.app_data.insert(ELEMENTS_KEY.to_string(), Arc::new(self.to_value()));
    }

    /// The element of a paragraph: its style's element, else General. An unusual or unknown
    /// style never blocks anything; it simply reads as General.
    pub fn of_para(&self, p: &Paragraph) -> &ElementDef {
        p.props.style.as_deref().and_then(|s| self.get(s)).or_else(|| self.of_kind(ElementKind::General)).unwrap_or(&FALLBACK_GENERAL)
    }
}

/// Used only if a registry somehow has no General element (never, after [`Elements::from_value`]).
static FALLBACK_GENERAL: std::sync::LazyLock<ElementDef> = std::sync::LazyLock::new(|| builtin_def(ElementKind::General));

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_spec_element_is_built_in() {
        let e = Elements::defaults();
        for name in [
            "General",
            "Scene Heading",
            "Action",
            "Character",
            "Dialogue",
            "Parenthetical",
            "Transition",
            "Shot",
            "Cast List",
            "New Act",
            "End of Act",
            "Sequence",
            "Outline 1",
            "Outline 2",
            "Outline 3",
            "Note",
            "Summary",
        ] {
            assert!(e.all().iter().any(|d| d.name == name), "{name}");
        }
    }

    #[test]
    fn tab_table_matches_the_spec() {
        let e = Elements::defaults();
        let tab = |k: ElementKind| e.of_kind(k).and_then(|d| d.tab.clone());
        use ElementKind as K;
        assert_eq!(tab(K::SceneHeading).as_deref(), Some("Action"));
        assert_eq!(tab(K::Action).as_deref(), Some("Character"));
        assert_eq!(tab(K::Character).as_deref(), Some("Transition"));
        assert!(e.of_kind(K::Character).is_some_and(|d| d.tab_only_when_blank));
        assert_eq!(tab(K::Dialogue).as_deref(), Some("Parenthetical"));
        assert_eq!(tab(K::Parenthetical).as_deref(), Some("Dialogue"));
        assert_eq!(tab(K::Transition).as_deref(), Some("SceneHeading"));
        assert_eq!(tab(K::Outline1).as_deref(), Some("Outline2"));
        assert_eq!(tab(K::Outline2).as_deref(), Some("Outline3"));
        assert_eq!(tab(K::Outline3).as_deref(), Some("Note"));
        assert_eq!(tab(K::Note).as_deref(), Some("Note"));
        assert_eq!(tab(K::Sequence).as_deref(), Some("SceneHeading"));
        for k in [K::General, K::Shot, K::CastList, K::NewAct, K::EndOfAct, K::Summary] {
            assert_eq!(tab(k), None, "{k:?}");
        }
    }

    #[test]
    fn casing_matches_the_spec() {
        let e = Elements::defaults();
        use ElementKind as K;
        for k in [K::SceneHeading, K::Character, K::Transition, K::Shot] {
            assert_eq!(e.of_kind(k).map(|d| d.casing), Some(Casing::Upper), "{k:?}");
        }
        for k in [K::Action, K::Dialogue, K::Parenthetical, K::General] {
            assert_eq!(e.of_kind(k).map(|d| d.casing), Some(Casing::AsTyped), "{k:?}");
        }
        assert_eq!(Casing::Upper.apply("int. café - night"), "INT. CAFÉ - NIGHT");
    }

    #[test]
    fn core_shortcuts_are_mod_0_to_9() {
        let e = Elements::defaults();
        let order = ["General", "SceneHeading", "Action", "Character", "Parenthetical", "Dialogue", "Transition", "Shot", "CastList", "NewAct"];
        for (n, id) in order.iter().enumerate() {
            assert_eq!(e.get(id).and_then(|d| d.shortcut.clone()), Some(format!("Mod+{n}")), "{id}");
        }
    }

    #[test]
    fn registry_round_trips_through_the_document() {
        let mut doc = Document::new();
        assert_eq!(Elements::of_doc(&doc), Elements::defaults());
        let mut e = Elements::defaults();
        let mut custom = builtin_def(ElementKind::Action);
        custom.id = e.new_custom_id();
        custom.name = "Lyrics".into();
        custom.kind = ElementKind::Custom;
        custom.shortcut = Some("Mod+Alt+L".into());
        e.push(custom).unwrap();
        e.store(&mut doc);
        assert_eq!(Elements::of_doc(&doc), e);
        assert_eq!(e.new_custom_id(), "Custom2");
    }

    #[test]
    fn hostile_registries_are_repaired() {
        // Not even a list: defaults.
        assert_eq!(Elements::from_value(&json!({"elements": 3})), Elements::defaults());
        assert_eq!(Elements::from_value(&json!(null)), Elements::defaults());
        // Duplicates, empty and oversized names, a built-in kind under another id, a core shortcut
        // changed, a dangling Tab target, and far too many entries.
        let mut list = vec![
            json!({"id": "Action", "name": "Action", "kind": "action", "shortcut": "Mod+9", "tab": "Nowhere"}),
            json!({"id": "Action", "name": "Again", "kind": "action"}),
            json!({"id": "", "name": "x", "kind": "custom"}),
            json!({"id": "x".repeat(500), "name": "x", "kind": "custom"}),
            json!({"id": "Fake", "name": "Fake", "kind": "sceneHeading"}),
        ];
        for i in 0..1000 {
            list.push(json!({"id": format!("C{i}"), "name": "c", "kind": "custom"}));
        }
        let e = Elements::from_value(&json!({"elements": list}));
        assert!(e.all().len() <= MAX_ELEMENTS);
        let action = e.get("Action").unwrap();
        assert_eq!(action.name, "Action");
        assert_eq!(action.shortcut.as_deref(), Some("Mod+2"));
        assert_eq!(action.tab, None);
        assert!(e.get("Fake").is_none());
        for kind in ElementKind::BUILTIN {
            assert!(e.of_kind(kind).is_some(), "{kind:?} restored");
        }
    }

    #[test]
    fn builtin_elements_cannot_be_removed_and_references_are_cleared() {
        let mut e = Elements::defaults();
        assert!(e.remove("Action").is_err());
        let mut c = builtin_def(ElementKind::Action);
        c.id = "Custom1".into();
        c.kind = ElementKind::Custom;
        e.push(c).unwrap();
        if let Some(d) = e.get_mut("Shot") {
            d.tab = Some("Custom1".into());
        }
        e.remove("Custom1").unwrap();
        assert_eq!(e.get("Shot").and_then(|d| d.tab.clone()), None);
        assert!(e.push(builtin_def(ElementKind::Action)).is_err(), "ids are unique");
    }

    #[test]
    fn unknown_styles_read_as_general() {
        let e = Elements::defaults();
        let mut p = Paragraph::new();
        assert_eq!(e.of_para(&p).kind, ElementKind::General);
        p.props.style = Some("Heading1".into());
        assert_eq!(e.of_para(&p).kind, ElementKind::General);
        p.props.style = Some("Dialogue".into());
        assert_eq!(e.of_para(&p).kind, ElementKind::Dialogue);
    }
}
