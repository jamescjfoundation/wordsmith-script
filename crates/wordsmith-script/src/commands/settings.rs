// SPDX-License-Identifier: MIT OR Apache-2.0
//! Element settings (SPEC "Formatting, page geometry and pagination", per-element settings):
//! font, size and style; alignment; line spacing 1, 1.5, 2 or 3; space before 0 to 30 lines;
//! left and right indents in inches from the page's left edge; next element; start new page;
//! keep rules; paginate-as; shortcut. Each change is one undoable edit of the document (the
//! element's paragraph style and its registry entry), and applies to every paragraph of that
//! element at once.

use std::sync::Arc;

use jcj_doc::props::{Align, LineSpacing, Underline};
use jcj_doc::{Document, Style};
use jcj_engine::{CmdError, CmdResult, CommandSpec, Session, params as p};
use serde_json::{Value, json};

use crate::element::{Casing, ElementDef, ElementKind, Elements, MAX_NAME_LEN};
use crate::scene_heading::{MAX_INTROS, MAX_PIECE_LEN, SceneHeadingSettings};
use crate::template::{INCH, LINE, MARGIN_LEFT, MARGIN_RIGHT, PAGE_W};

/// Line spacing choices (SPEC).
pub const LINE_SPACINGS: [f32; 4] = [1.0, 1.5, 2.0, 3.0];
/// Most blank lines before a paragraph (SPEC: "any value up to 30 lines").
pub const MAX_SPACE_BEFORE_LINES: f32 = 30.0;
/// Font sizes accepted, points.
pub const FONT_SIZES: std::ops::RangeInclusive<f32> = 4.0..=72.0;
/// Longest font family name, bytes.
pub const MAX_FONT_LEN: usize = 128;

fn align_name(a: Align) -> &'static str {
    match a {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
        Align::Justify | Align::Distribute => "justify",
    }
}

fn parse_align(s: &str) -> Result<Align, CmdError> {
    Ok(match s {
        "left" => Align::Left,
        "center" => Align::Center,
        "right" => Align::Right,
        "justify" => Align::Justify,
        _ => return Err(CmdError::Params("`align` is left, center, right or justify".into())),
    })
}

fn style_of<'a>(doc: &'a Document, id: &str) -> Result<&'a Style, CmdError> {
    doc.styles.get(id).ok_or_else(|| CmdError::Failed(format!("element `{id}` has no paragraph style")))
}

/// Inches from the page's left edge.
fn left_in(st: &Style) -> f32 {
    (st.para.indent_left.unwrap_or(0.0) + MARGIN_LEFT) / INCH
}
fn right_in(st: &Style) -> f32 {
    (PAGE_W - MARGIN_RIGHT - st.para.indent_right.unwrap_or(0.0)) / INCH
}
fn spacing_lines(st: &Style) -> f32 {
    match st.para.line_spacing {
        Some(LineSpacing::Exactly(pt)) => pt / LINE,
        Some(LineSpacing::Multiple(m)) => m,
        Some(LineSpacing::AtLeast(pt)) => pt / LINE,
        None => 1.0,
    }
}

/// An element's settings as agents and the settings dialog see them.
pub fn settings_json(doc: &Document, def: &ElementDef) -> Result<Value, CmdError> {
    let st = style_of(doc, &def.id)?;
    Ok(json!({
        "id": def.id,
        "name": def.name,
        "kind": def.kind,
        "builtin": def.is_builtin(),
        "font": st.chr.font,
        "size": st.chr.size,
        "bold": st.chr.bold.unwrap_or(false),
        "italic": st.chr.italic.unwrap_or(false),
        "underline": st.chr.underline.is_some_and(|u| u != Underline::None),
        "uppercase": def.casing == Casing::Upper,
        "lowercase": def.casing == Casing::Lower,
        "strike": st.chr.strike.unwrap_or(false),
        "color": match st.chr.color { Some(jcj_doc::props::TextColor::Rgb(jcj_doc::props::Rgb(r, g, b))) => json!(format!("#{r:02X}{g:02X}{b:02X}")), _ => Value::Null },
        "highlight": highlight_name(&st.chr),
        "align": align_name(st.para.align.unwrap_or_default()),
        "left": left_in(st),
        "right": right_in(st),
        "lineSpacing": spacing_lines(st),
        "spaceBefore": st.para.space_before.unwrap_or(0.0) / LINE,
        "next": st.next,
        "tab": def.tab,
        "tabOnlyWhenBlank": def.tab_only_when_blank,
        "startNewPage": st.para.page_break_before.unwrap_or(false),
        "keepWithNext": st.para.keep_next.unwrap_or(false),
        "keepTogether": st.para.keep_lines.unwrap_or(false),
        "paginateAs": def.paginate_as,
        "shortcut": def.shortcut,
    }))
}

fn element_param(v: &Value, elements: &Elements) -> Result<ElementDef, CmdError> {
    let id = p::req_str(v, "element")?;
    elements.get(id).cloned().ok_or_else(|| CmdError::Params(format!("no element `{id}`")))
}

/// `element.settings {element?}`: one element's settings, or every element's.
fn get(s: &mut Session, v: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    if v.get("element").is_some() {
        return settings_json(&s.doc, &element_param(v, &elements)?);
    }
    let all: Result<Vec<Value>, CmdError> = elements.all().iter().map(|d| settings_json(&s.doc, d)).collect();
    Ok(Value::Array(all?))
}

/// A reference to another element (`next`, `tab`, `paginateAs`): its id, or null to clear.
fn element_ref(v: &Value, key: &str, elements: &Elements) -> Result<Option<Option<String>>, CmdError> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(id)) if elements.get(id).is_some() => Ok(Some(Some(id.clone()))),
        Some(_) => Err(CmdError::Params(format!("`{key}` must be an element id or null"))),
    }
}

fn number(v: &Value, key: &str) -> Result<Option<f32>, CmdError> {
    match v.get(key) {
        None => Ok(None),
        Some(x) => {
            x.as_f64().map(|f| f as f32).filter(|f| f.is_finite()).map(Some).ok_or_else(|| CmdError::Params(format!("`{key}` must be a number")))
        }
    }
}

fn flag(v: &Value, key: &str) -> Result<Option<bool>, CmdError> {
    match v.get(key) {
        None => Ok(None),
        Some(x) => x.as_bool().map(Some).ok_or_else(|| CmdError::Params(format!("`{key}` must be true or false"))),
    }
}

/// `element.configure {element, …settings}`: change an element. Every value is checked before
/// anything changes, so a refused call changes nothing.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut elements = Elements::of_doc(&s.doc);
    let def = element_param(v, &elements)?;
    let st = style_of(&s.doc, &def.id)?;
    let mut style = st.clone();
    let mut new_def = def.clone();

    if let Some(name) = p::str(v, "name") {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(CmdError::Params(format!("`name` is 1 to {MAX_NAME_LEN} bytes")));
        }
        if elements.all().iter().any(|d| d.id != def.id && d.name.eq_ignore_ascii_case(name)) {
            return Err(CmdError::Params(format!("another element is already called `{name}`")));
        }
        new_def.name = name.to_string();
        style.name = name.to_string();
    }
    if let Some(font) = p::str(v, "font") {
        if font.trim().is_empty() || font.len() > MAX_FONT_LEN {
            return Err(CmdError::Params(format!("`font` is 1 to {MAX_FONT_LEN} bytes")));
        }
        style.chr.font = Some(font.trim().to_string());
    }
    if let Some(size) = number(v, "size")? {
        if !FONT_SIZES.contains(&size) {
            return Err(CmdError::Params("`size` is 4 to 72 points".into()));
        }
        style.chr.size = Some(size);
    }
    if let Some(b) = flag(v, "bold")? {
        style.chr.bold = Some(b);
    }
    if let Some(i) = flag(v, "italic")? {
        style.chr.italic = Some(i);
    }
    if let Some(u) = flag(v, "underline")? {
        style.chr.underline = u.then_some(Underline::Single);
    }
    if let Some(u) = flag(v, "uppercase")? {
        new_def.casing = if u { Casing::Upper } else { Casing::AsTyped };
    }
    if let Some(l) = flag(v, "lowercase")? {
        if l {
            new_def.casing = Casing::Lower;
        } else if new_def.casing == Casing::Lower {
            new_def.casing = Casing::AsTyped;
        }
    }
    if let Some(k) = flag(v, "strike")? {
        style.chr.strike = Some(k);
    }
    match v.get("color") {
        None => {}
        Some(Value::Null) => style.chr.color = None,
        Some(Value::String(c)) => {
            let rgb = jcj_doc::props::Rgb::parse(c.trim()).ok_or_else(|| CmdError::Params("`color` is #RRGGBB or null".into()))?;
            style.chr.color = Some(jcj_doc::props::TextColor::Rgb(rgb));
        }
        Some(_) => return Err(CmdError::Params("`color` is #RRGGBB or null".into())),
    }
    match v.get("highlight") {
        None => {}
        Some(Value::Null) => {
            style.chr.highlight = None;
            style.chr.shading = None;
        }
        Some(Value::String(h)) => {
            if let Some((_, named)) = super::format::HIGHLIGHTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(h.trim())) {
                style.chr.highlight = Some(*named);
                style.chr.shading = None;
            } else {
                let rgb = jcj_doc::props::Rgb::parse(h.trim())
                    .ok_or_else(|| CmdError::Params("`highlight` is a highlighter name, #RRGGBB or null".into()))?;
                style.chr.highlight = None;
                style.chr.shading = Some(rgb);
            }
        }
        Some(_) => return Err(CmdError::Params("`highlight` is a highlighter name, #RRGGBB or null".into())),
    }
    if let Some(a) = p::str(v, "align") {
        style.para.align = Some(parse_align(a)?);
    }
    let left = number(v, "left")?.unwrap_or_else(|| left_in(st));
    let right = number(v, "right")?.unwrap_or_else(|| right_in(st));
    if v.get("left").is_some() || v.get("right").is_some() {
        let page_in = PAGE_W / INCH;
        if !(0.0..=page_in).contains(&left) || !(0.0..=page_in).contains(&right) || right - left < 0.5 {
            return Err(CmdError::Params(format!(
                "`left` and `right` are inches from the page's left edge (0 to {page_in}), at least half an inch apart"
            )));
        }
        style.para.indent_left = Some(left * INCH - MARGIN_LEFT);
        style.para.indent_right = Some(PAGE_W - MARGIN_RIGHT - right * INCH);
    }
    if let Some(sp) = number(v, "lineSpacing")? {
        if !LINE_SPACINGS.contains(&sp) {
            return Err(CmdError::Params("`lineSpacing` is 1, 1.5, 2 or 3".into()));
        }
        style.para.line_spacing = Some(LineSpacing::Exactly(sp * LINE));
    }
    if let Some(lines) = number(v, "spaceBefore")? {
        if !(0.0..=MAX_SPACE_BEFORE_LINES).contains(&lines) {
            return Err(CmdError::Params("`spaceBefore` is 0 to 30 lines".into()));
        }
        style.para.space_before = Some(lines * LINE);
    }
    if let Some(next) = element_ref(v, "next", &elements)? {
        style.next = Some(next.unwrap_or_else(|| def.id.clone()));
    }
    if let Some(tab) = element_ref(v, "tab", &elements)? {
        new_def.tab = tab;
    }
    if let Some(b) = flag(v, "tabOnlyWhenBlank")? {
        new_def.tab_only_when_blank = b;
    }
    if let Some(pa) = element_ref(v, "paginateAs", &elements)? {
        new_def.paginate_as = pa.filter(|id| *id != def.id);
    }
    if let Some(b) = flag(v, "startNewPage")? {
        style.para.page_break_before = Some(b);
    }
    if let Some(b) = flag(v, "keepWithNext")? {
        style.para.keep_next = Some(b);
    }
    if let Some(b) = flag(v, "keepTogether")? {
        style.para.keep_lines = Some(b);
    }
    if v.get("shortcut").is_some() {
        if def.kind.core_shortcut().is_some() {
            return Err(CmdError::Params("core elements keep their fixed shortcuts".into()));
        }
        new_def.shortcut = match v.get("shortcut") {
            Some(Value::Null) => None,
            Some(Value::String(k)) if !k.is_empty() && k.len() <= MAX_NAME_LEN => {
                if s.registry.by_shortcut(k).is_some()
                    || elements.all().iter().any(|d| d.id != def.id && d.shortcut.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(k)))
                {
                    return Err(CmdError::Params(format!("`{k}` is already in use")));
                }
                Some(k.clone())
            }
            _ => return Err(CmdError::Params("`shortcut` is a key such as `Mod+Alt+L`, or null".into())),
        };
    }

    // Everything checked: apply.
    let casing_now_upper = new_def.casing != Casing::AsTyped && def.casing != new_def.casing;
    if let Some(d) = elements.get_mut(&def.id) {
        *d = new_def.clone();
    }
    elements.store(&mut s.doc);
    Arc::make_mut(&mut s.doc.styles).upsert(style);
    if casing_now_upper {
        recase_all(s, &elements, &def.id)?;
    }
    touch_all(&mut s.doc);
    settings_json(&s.doc, &new_def)
}

/// Every paragraph of element `id` re-cased (after the element became uppercase).
fn recase_all(s: &mut Session, elements: &Elements, id: &str) -> Result<(), CmdError> {
    let paths: Vec<jcj_doc::Path> = paragraphs_of(&s.doc, id);
    for path in paths {
        super::element::apply_element(s, elements, &path, id, false)?;
    }
    Ok(())
}

fn paragraphs_of(doc: &Document, id: &str) -> Vec<jcj_doc::Path> {
    doc.body
        .iter()
        .enumerate()
        .filter(|(_, b)| b.as_para().is_some_and(|p| p.props.style.as_deref() == Some(id)))
        .map(|(i, _)| jcj_doc::Path::top(i))
        .collect()
}

/// A style change reaches every paragraph of the element: mark them all for layout again.
fn touch_all(doc: &mut Document) {
    for block in &mut doc.body {
        if let jcj_doc::Block::Para(p) = Arc::make_mut(block) {
            p.touch();
        }
    }
}

/// `element.create {name, shortcut?}`: a custom element, starting from Action's current settings
/// (SPEC: "New ones start from Action's settings").
fn create(s: &mut Session, v: &Value) -> CmdResult {
    let mut elements = Elements::of_doc(&s.doc);
    let name = p::req_str(v, "name")?.trim().to_string();
    if elements.all().iter().any(|d| d.name.eq_ignore_ascii_case(&name)) {
        return Err(CmdError::Params(format!("an element is already called `{name}`")));
    }
    let action_id = elements.id_of(ElementKind::Action).to_string();
    let action = elements.get(&action_id).cloned().ok_or_else(|| CmdError::Failed("the script has no Action element".into()))?;
    let id = elements.new_custom_id();
    let def = ElementDef { id: id.clone(), name: name.clone(), kind: ElementKind::Custom, shortcut: None, ..action };
    elements.push(def).map_err(CmdError::Params)?;
    let mut style = style_of(&s.doc, &action_id)?.clone();
    style.id = id.clone();
    style.name = name;
    style.builtin = false;
    elements.store(&mut s.doc);
    Arc::make_mut(&mut s.doc.styles).upsert(style);
    if v.get("shortcut").is_some() {
        configure(s, &json!({"element": id, "shortcut": v["shortcut"]}))?;
    }
    let elements = Elements::of_doc(&s.doc);
    let def = elements.get(&id).cloned().ok_or_else(|| CmdError::Failed("the new element was not kept".into()))?;
    settings_json(&s.doc, &def)
}

/// `element.delete {element}`: remove a custom element. Its paragraphs become Action, and
/// elements whose Return target it was return to themselves.
fn delete(s: &mut Session, v: &Value) -> CmdResult {
    let mut elements = Elements::of_doc(&s.doc);
    let def = element_param(v, &elements)?;
    elements.remove(&def.id).map_err(CmdError::Failed)?;
    elements.store(&mut s.doc);
    let action = elements.id_of(ElementKind::Action).to_string();
    for path in paragraphs_of(&s.doc, &def.id) {
        super::element::apply_element(s, &elements, &path, &action, false)?;
    }
    let sheet = Arc::make_mut(&mut s.doc.styles);
    sheet.remove(&def.id);
    for st in &mut sheet.styles {
        if st.next.as_deref() == Some(def.id.as_str()) {
            st.next = Some(st.id.clone());
        }
    }
    touch_all(&mut s.doc);
    Ok(json!({"deleted": def.id}))
}

/// An element's highlight as `element.settings` shows it: a highlighter name or `#RRGGBB`.
fn highlight_name(c: &jcj_doc::props::CharProps) -> Value {
    if let Some(h) = c.highlight
        && let Some((n, _)) = super::format::HIGHLIGHTS.iter().find(|(_, x)| *x == h)
    {
        return json!(n);
    }
    match c.shading {
        Some(jcj_doc::props::Rgb(r, g, b)) => json!(format!("#{r:02X}{g:02X}{b:02X}")),
        None => Value::Null,
    }
}

/// `element.applyTemplate`: every element's formatting and Return target back to the
/// screenplay template's (names, casing and Tab stay). Custom elements take Action's.
fn apply_template(s: &mut Session, _: &Value) -> CmdResult {
    let elements = Elements::of_doc(&s.doc);
    let fresh = crate::template::style_sheet(&elements);
    let sheet = Arc::make_mut(&mut s.doc.styles);
    for def in elements.all() {
        if let Some(st) = fresh.get(&def.id) {
            sheet.upsert(st.clone());
        }
    }
    sheet.default_chr = fresh.default_chr.clone();
    touch_all(&mut s.doc);
    Ok(json!({"elements": elements.all().len()}))
}

/// `element.applyFontToAll {font?, size?}`: one font and size for every element, leaving bold and
/// italic alone (SPEC, Verified).
fn apply_font_to_all(s: &mut Session, v: &Value) -> CmdResult {
    let font = p::str(v, "font").map(str::trim);
    let size = number(v, "size")?;
    if font.is_none() && size.is_none() {
        return Err(CmdError::Params("give `font`, `size` or both".into()));
    }
    if font.is_some_and(|f| f.is_empty() || f.len() > MAX_FONT_LEN) {
        return Err(CmdError::Params(format!("`font` is 1 to {MAX_FONT_LEN} bytes")));
    }
    if size.is_some_and(|z| !FONT_SIZES.contains(&z)) {
        return Err(CmdError::Params("`size` is 4 to 72 points".into()));
    }
    let ids: Vec<String> = Elements::of_doc(&s.doc).all().iter().map(|d| d.id.clone()).collect();
    let sheet = Arc::make_mut(&mut s.doc.styles);
    for st in sheet.styles.iter_mut().filter(|st| ids.contains(&st.id)) {
        if let Some(f) = font {
            st.chr.font = Some(f.to_string());
        }
        if let Some(z) = size {
            st.chr.size = Some(z);
        }
    }
    if let Some(f) = font {
        sheet.default_chr.font = Some(f.to_string());
    }
    if let Some(z) = size {
        sheet.default_chr.size = Some(z);
    }
    touch_all(&mut s.doc);
    Ok(json!({"elements": ids.len()}))
}

/// `element.dialogueThenCharacter {value}`: the option that makes the element after Dialogue
/// Character instead of Action (SPEC, Sourced), for rapid back-and-forth.
fn dialogue_then_character(s: &mut Session, v: &Value) -> CmdResult {
    let on = p::bool(v, "value").ok_or_else(|| CmdError::Params("`value` (true or false) is required".into()))?;
    let elements = Elements::of_doc(&s.doc);
    let target = elements.id_of(if on { ElementKind::Character } else { ElementKind::Action }).to_string();
    configure(s, &json!({"element": elements.id_of(ElementKind::Dialogue), "next": target}))?;
    Ok(json!({"value": on}))
}

/// `sceneHeading.settings` / `sceneHeading.configure {intros?, introSeparator?, timeSeparator?}`.
fn scene_heading_configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut settings = SceneHeadingSettings::of_doc(&s.doc);
    if let Some(list) = v.get("intros") {
        let list = list.as_array().ok_or_else(|| CmdError::Params("`intros` is a list of strings".into()))?;
        if list.len() > MAX_INTROS {
            return Err(CmdError::Params(format!("at most {MAX_INTROS} intros")));
        }
        let mut intros = Vec::new();
        for i in list {
            let i = i
                .as_str()
                .map(str::trim)
                .filter(|x| !x.is_empty() && x.len() <= MAX_PIECE_LEN)
                .ok_or_else(|| CmdError::Params(format!("each intro is 1 to {MAX_PIECE_LEN} bytes")))?;
            intros.push(i.to_uppercase());
        }
        settings.intros = intros;
    }
    for (key, slot) in [("introSeparator", &mut settings.intro_separator), ("timeSeparator", &mut settings.time_separator)] {
        if let Some(x) = v.get(key) {
            let x = x
                .as_str()
                .filter(|x| !x.is_empty() && x.len() <= MAX_PIECE_LEN)
                .ok_or_else(|| CmdError::Params(format!("`{key}` is 1 to {MAX_PIECE_LEN} bytes")))?;
            *slot = x.to_string();
        }
    }
    settings.store(&mut s.doc);
    Ok(serde_json::to_value(&settings).unwrap_or_default())
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Format › Elements";
    vec![
        CommandSpec::new("element.settings", "Element Settings", LOC, get).params(r#"{"element"?: elementId}"#).pure(),
        CommandSpec::new("element.configure", "Change Element Settings", LOC, configure).params(
            r#"{"element": id, "name"?, "font"?, "size"? (4-72), "bold"?, "italic"?, "underline"?, "uppercase"?, "lowercase"?, "strike"?, "color"? (#RRGGBB|null), "highlight"? (name|#RRGGBB|null), "align"? (left|center|right|justify), "left"?, "right"? (inches from the page's left edge), "lineSpacing"? (1|1.5|2|3), "spaceBefore"? (0-30 lines), "next"?, "tab"?, "tabOnlyWhenBlank"?, "paginateAs"? (element id or null), "startNewPage"?, "keepWithNext"?, "keepTogether"?, "shortcut"? (custom elements only)}"#,
        ),
        CommandSpec::new("element.create", "New Element", LOC, create).params(r#"{"name": string, "shortcut"?: string}"#),
        CommandSpec::new("element.delete", "Delete Element", LOC, delete).params(r#"{"element": customElementId}"#),
        CommandSpec::new("element.applyFontToAll", "Font for All Elements", LOC, apply_font_to_all).params(r#"{"font"?: string, "size"?: number}"#),
        CommandSpec::new("element.applyTemplate", "Template Formatting for All Elements", LOC, apply_template),
        CommandSpec::new("element.dialogueThenCharacter", "Character After Dialogue", LOC, dialogue_then_character).params(r#"{"value": bool}"#),
        CommandSpec::new("sceneHeading.settings", "Scene Heading Settings", LOC, |s, _| Ok(serde_json::to_value(SceneHeadingSettings::of_doc(&s.doc)).unwrap_or_default())).pure(),
        CommandSpec::new("sceneHeading.configure", "Change Scene Heading Settings", LOC, scene_heading_configure)
            .params(r#"{"intros"?: [string], "introSeparator"?: string, "timeSeparator"?: string}"#),
    ]
}

#[cfg(test)]
mod tests {
    use jcj_engine::{CmdError, Session};
    use serde_json::json;

    use crate::commands::{new_session, text::body_lines};

    fn get(s: &mut Session, id: &str) -> serde_json::Value {
        s.run("element.settings", &json!({"element": id})).unwrap()
    }

    #[test]
    fn every_setting_round_trips_and_undoes_in_one_step() {
        let mut s = new_session();
        let before = get(&mut s, "Dialogue");
        let change = json!({
            "element": "Dialogue", "name": "Speech", "font": "JetBrains Mono", "size": 11.0, "bold": true, "italic": true,
            "underline": true, "uppercase": true, "align": "center", "left": 2.0, "right": 6.5, "lineSpacing": 1.5,
            "spaceBefore": 2.0, "next": "Character", "tab": "Action", "startNewPage": true, "keepWithNext": true,
            "keepTogether": true, "paginateAs": "Action"
        });
        s.run("element.configure", &change).unwrap();
        let after = get(&mut s, "Dialogue");
        for key in [
            "name",
            "font",
            "size",
            "bold",
            "italic",
            "underline",
            "uppercase",
            "align",
            "lineSpacing",
            "spaceBefore",
            "next",
            "tab",
            "startNewPage",
            "keepWithNext",
            "keepTogether",
            "paginateAs",
        ] {
            assert_eq!(after[key], change[key], "{key}");
        }
        assert!((after["left"].as_f64().unwrap() - 2.0).abs() < 1e-4);
        assert!((after["right"].as_f64().unwrap() - 6.5).abs() < 1e-4);
        assert_eq!(s.undo_depth(), 1);
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(get(&mut s, "Dialogue"), before);
    }

    #[test]
    fn a_change_reaches_every_paragraph_of_the_element() {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "one\ntwo"})).unwrap();
        s.run("select.all", &json!({})).unwrap();
        s.run("element.dialogue", &json!({})).unwrap();
        s.run("element.configure", &json!({"element": "Dialogue", "uppercase": true})).unwrap();
        assert_eq!(body_lines(&s), vec![("Dialogue".into(), "ONE".into()), ("Dialogue".into(), "TWO".into())]);
    }

    #[test]
    fn out_of_range_values_are_refused_and_change_nothing() {
        let mut s = new_session();
        let before = get(&mut s, "Action");
        for bad in [
            json!({"element": "Action", "lineSpacing": 1.25}),
            json!({"element": "Action", "spaceBefore": 31}),
            json!({"element": "Action", "spaceBefore": -1}),
            json!({"element": "Action", "size": 200}),
            json!({"element": "Action", "left": 7.0, "right": 7.2}),
            json!({"element": "Action", "left": -1}),
            json!({"element": "Action", "align": "sideways"}),
            json!({"element": "Action", "next": "Nope"}),
            json!({"element": "Action", "shortcut": "Mod+K"}),
            json!({"element": "Action", "font": ""}),
            json!({"element": "Action", "name": "Dialogue"}),
            json!({"element": "Action", "bold": "yes"}),
            json!({"element": "Missing"}),
        ] {
            assert!(matches!(s.run("element.configure", &bad), Err(CmdError::Params(_))), "{bad}");
            assert_eq!(get(&mut s, "Action"), before, "{bad}");
        }
        assert!(!s.can_undo());
    }

    #[test]
    fn custom_elements_start_from_action_and_can_be_deleted() {
        let mut s = new_session();
        s.run("element.configure", &json!({"element": "Action", "left": 1.75, "spaceBefore": 2})).unwrap();
        let made = s.run("element.create", &json!({"name": "Lyrics", "shortcut": "Mod+Alt+L"})).unwrap();
        let id = made["id"].as_str().unwrap().to_string();
        assert_eq!(id, "Custom1");
        let action = get(&mut s, "Action");
        for key in ["font", "size", "left", "right", "spaceBefore", "lineSpacing", "align"] {
            assert_eq!(made[key], action[key], "{key} starts from Action");
        }
        assert_eq!(made["shortcut"], "Mod+Alt+L");
        assert!(matches!(s.run("element.create", &json!({"name": "lyrics"})), Err(CmdError::Params(_))));
        // A paragraph of the custom element turns into Action when it is deleted.
        s.run("element.set", &json!({"element": id})).unwrap();
        s.run("text.insert", &json!({"text": "la la"})).unwrap();
        s.run("element.delete", &json!({"element": id})).unwrap();
        assert_eq!(body_lines(&s), vec![("Action".into(), "la la".into())]);
        assert!(s.run("element.settings", &json!({"element": id})).is_err());
        assert!(matches!(s.run("element.delete", &json!({"element": "Action"})), Err(CmdError::Failed(_))));
        // Undo brings it all back.
        s.run("edit.undo", &json!({})).unwrap();
        assert_eq!(body_lines(&s)[0].0, id);
    }

    #[test]
    fn font_for_all_leaves_bold_and_italic_alone() {
        let mut s = new_session();
        s.run("element.configure", &json!({"element": "Note", "bold": true})).unwrap();
        s.run("element.applyFontToAll", &json!({"font": "JetBrains Mono", "size": 11})).unwrap();
        let all = s.run("element.settings", &json!({})).unwrap();
        for e in all.as_array().unwrap() {
            assert_eq!(e["font"], "JetBrains Mono", "{}", e["id"]);
            assert_eq!(e["size"], 11.0, "{}", e["id"]);
        }
        assert_eq!(get(&mut s, "Note")["bold"], true);
        assert_eq!(get(&mut s, "Note")["italic"], true);
        assert_eq!(get(&mut s, "Outline1")["bold"], true);
        assert_eq!(get(&mut s, "Action")["bold"], false);
        assert!(matches!(s.run("element.applyFontToAll", &json!({})), Err(CmdError::Params(_))));
    }

    #[test]
    fn dialogue_then_character_option() {
        let mut s = new_session();
        s.run("element.dialogueThenCharacter", &json!({"value": true})).unwrap();
        s.run("element.dialogue", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "Hi.\n"})).unwrap();
        assert_eq!(body_lines(&s).last().map(|l| l.0.clone()).as_deref(), Some("Character"));
        s.run("element.dialogueThenCharacter", &json!({"value": false})).unwrap();
        assert_eq!(get(&mut s, "Dialogue")["next"], "Action");
    }

    #[test]
    fn scene_heading_separators_can_be_changed() {
        let mut s = new_session();
        s.run("sceneHeading.configure", &json!({"intros": ["int", "ext", "est"], "timeSeparator": " -- "})).unwrap();
        s.run("text.insert", &json!({"text": "est"})).unwrap();
        s.run("text.tab", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "city"})).unwrap();
        s.run("text.tab", &json!({})).unwrap();
        s.run("text.insert", &json!({"text": "dawn"})).unwrap();
        assert_eq!(body_lines(&s)[0].1, "EST. CITY -- DAWN");
        assert!(s.run("sceneHeading.configure", &json!({"introSeparator": ""})).is_err());
        assert!(s.run("sceneHeading.configure", &json!({"intros": "INT"})).is_err());
    }

    #[test]
    fn a_custom_element_with_its_own_shortcut_cannot_take_a_used_one() {
        let mut s = new_session();
        assert!(matches!(s.run("element.create", &json!({"name": "Song", "shortcut": "Mod+1"})), Err(CmdError::Params(_))));
        assert!(Elements::of_doc(&s.doc).get("Custom1").is_none(), "a refused create leaves nothing");
    }

    use crate::element::Elements;
}

#[cfg(test)]
mod spec2_tests {
    use super::*;
    use crate::commands::new_session;

    #[test]
    fn color_highlight_strike_and_small_letters() {
        let mut s = new_session();
        s.run("text.insert", &json!({"text": "int. hall\nRain FALLS."})).unwrap();
        let after = s
            .run("element.configure", &json!({"element": "Action", "strike": true, "color": "#336699", "highlight": "Lime", "lowercase": true}))
            .unwrap();
        assert_eq!((after["strike"].as_bool(), after["color"].as_str(), after["highlight"].as_str()), (Some(true), Some("#336699"), Some("Lime")));
        assert_eq!(crate::commands::text::body_lines(&s)[1].1, "rain falls.");
        let after = s.run("element.configure", &json!({"element": "Action", "highlight": "#ABCDEF", "color": null, "lowercase": false})).unwrap();
        assert_eq!((after["highlight"].as_str(), after["color"].is_null(), after["lowercase"].as_bool()), (Some("#ABCDEF"), true, Some(false)));
        assert!(s.run("element.configure", &json!({"element": "Action", "color": "teal-ish"})).is_err());
    }

    #[test]
    fn template_formatting_comes_back() {
        let mut s = new_session();
        s.run("element.configure", &json!({"element": "Dialogue", "left": 1.5, "bold": true, "next": "Character"})).unwrap();
        s.run("element.applyTemplate", &json!({})).unwrap();
        let d = s.run("element.settings", &json!({"element": "Dialogue"})).unwrap();
        assert!((d["left"].as_f64().unwrap_or(0.0) - 2.5).abs() < 1e-4);
        assert_eq!((d["bold"].as_bool(), d["next"].as_str()), (Some(false), Some("Action")));
    }
}
