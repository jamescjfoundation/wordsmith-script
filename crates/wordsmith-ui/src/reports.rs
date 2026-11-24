// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Reports window, the scene properties window and the character window (SPEC-2 9.2, 12,
//! 13). Each reads with pure commands and changes the script only through commands.

use egui::{Context, RichText};
use serde_json::{Value, json};
use wordsmith_script::Elements;

use crate::editor::Editor;

/// The report being asked for, and what came back.
#[derive(Clone, Debug)]
pub struct ReportState {
    pub kind: &'static str,
    pub sort: &'static str,
    pub name: String,
    pub elements: Vec<String>,
    pub dialogue: bool,
    pub a: String,
    pub b: String,
    pub result: Option<Value>,
}

impl Default for ReportState {
    fn default() -> Self {
        ReportState {
            kind: "scenes",
            sort: "script",
            name: String::new(),
            elements: Vec::new(),
            dialogue: false,
            a: "Gender".into(),
            b: "Role".into(),
            result: None,
        }
    }
}

impl ReportState {
    pub fn params(&self) -> Value {
        json!({"kind": self.kind, "sort": self.sort, "name": self.name, "elements": self.elements, "dialogue": self.dialogue, "a": self.a, "b": self.b})
    }
}

const KINDS: [(&str, &str); 8] = [
    ("scenes", "Scenes"),
    ("locations", "Locations"),
    ("cast", "Cast"),
    ("character", "One character"),
    ("filtered", "Filtered script"),
    ("notes", "Notes"),
    ("statistics", "Statistics"),
    ("representation", "Representation"),
];

/// The report kind named `name`, if there is one.
pub fn kind(name: &str) -> Option<&'static str> {
    KINDS.iter().map(|(k, _)| *k).find(|k| *k == name)
}

fn sorts(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "scenes" => &[
            ("script", "Script order"),
            ("alphabetical", "A to Z"),
            ("shortest", "Shortest"),
            ("longest", "Longest"),
            ("production", "Production order"),
        ],
        "locations" => &[("script", "Script order"), ("alphabetical", "A to Z"), ("most", "Most scenes"), ("least", "Fewest scenes")],
        "cast" => &[("script", "Script order"), ("alphabetical", "A to Z"), ("scenes", "Most scenes"), ("speaking", "Most speaking scenes")],
        "notes" => &[("script", "Script order"), ("date", "Newest"), ("type", "Type")],
        _ => &[],
    }
}

fn reports_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut st) = ed.tools.reports.take() else { return };
    let mut open = true;
    let mut run = false;
    let mut action: Option<(&'static str, Value)> = None;
    let characters: Vec<String> = ed
        .session
        .run("character.list", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    let traits: Vec<String> = ed
        .session
        .run("character.traits", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    let elements: Vec<(String, String)> = Elements::of_doc(&ed.session.doc).all().iter().map(|d| (d.id.clone(), d.name.clone())).collect();
    egui::Window::new("Reports").open(&mut open).default_size([720.0, 520.0]).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (k, label) in KINDS {
                if ui.selectable_label(st.kind == k, label).clicked() {
                    st.kind = k;
                    st.sort = "script";
                    run = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let list = sorts(st.kind);
            if !list.is_empty() {
                ui.label("Sort:");
                for (k, label) in list {
                    if ui.selectable_label(st.sort == *k, *label).clicked() {
                        st.sort = k;
                        run = true;
                    }
                }
            }
            match st.kind {
                "character" => {
                    egui::ComboBox::from_id_salt("report-character")
                        .selected_text(if st.name.is_empty() { "Choose…" } else { st.name.as_str() })
                        .show_ui(ui, |ui| {
                            for c in &characters {
                                if ui.selectable_label(st.name == *c, c.as_str()).clicked() {
                                    st.name = c.clone();
                                    run = true;
                                }
                            }
                        });
                    if ui.checkbox(&mut st.dialogue, "All dialogue").changed() {
                        run = true;
                    }
                }
                "filtered" => {
                    ui.label("Keep:");
                    for (id, name) in &elements {
                        let mut on = st.elements.contains(id);
                        if ui.checkbox(&mut on, name.as_str()).changed() {
                            if on {
                                st.elements.push(id.clone());
                            } else {
                                st.elements.retain(|x| x != id);
                            }
                            run = true;
                        }
                    }
                }
                "representation" => {
                    for (salt, which) in [("rep-a", 0), ("rep-b", 1)] {
                        let current = if which == 0 { st.a.clone() } else { st.b.clone() };
                        egui::ComboBox::from_id_salt(salt).selected_text(current.as_str()).show_ui(ui, |ui| {
                            for t in &traits {
                                if ui.selectable_label(current == *t, t.as_str()).clicked() {
                                    if which == 0 {
                                        st.a = t.clone();
                                    } else {
                                        st.b = t.clone();
                                    }
                                    run = true;
                                }
                            }
                        });
                    }
                }
                _ => {}
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                run = true;
            }
            if ui.button("Save as Text…").clicked() {
                action = Some(("report.export", st.params()));
            }
            if ui.button("Save as CSV…").clicked() {
                let mut p = st.params();
                p["csv"] = json!(true);
                action = Some(("report.export", p));
            }
            if st.kind == "filtered" && ui.button("Filtered Script as PDF…").clicked() {
                action = Some(("report.exportFilteredPdf", json!({"elements": st.elements})));
            }
        });
        ui.separator();
        match &st.result {
            Some(r) => show_report(ui, r),
            None => {
                ui.label("Choose a report.");
            }
        }
    });
    if run || st.result.is_none() {
        st.result = match ed.session.run("report.run", &st.params()) {
            Ok(v) => Some(v),
            Err(e) => Some(json!({"title": "", "sections": [], "lines": [e.to_string()]})),
        };
    }
    if let Some((cmd, params)) = action {
        // The dialog's answer runs the command with these options and the chosen path.
        ed.dialog_params = Some(params.clone());
        ed.run(cmd, params);
    }
    if open {
        ed.tools.reports = Some(st);
    }
}

/// A report's sections as tables.
pub fn show_report(ui: &mut egui::Ui, r: &Value) {
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        ui.heading(r["title"].as_str().unwrap_or(""));
        for (n, s) in r["sections"].as_array().cloned().unwrap_or_default().iter().enumerate() {
            if let Some(h) = s["heading"].as_str().filter(|h| !h.is_empty()) {
                ui.add_space(6.0);
                ui.label(RichText::new(h).strong());
            }
            let cols = s["columns"].as_array().cloned().unwrap_or_default();
            egui::Grid::new(("report-section", n)).striped(true).num_columns(cols.len()).show(ui, |ui| {
                for c in &cols {
                    ui.label(RichText::new(c.as_str().unwrap_or("")).strong());
                }
                ui.end_row();
                for row in s["rows"].as_array().cloned().unwrap_or_default() {
                    for c in row.as_array().cloned().unwrap_or_default() {
                        ui.add(egui::Label::new(c.as_str().unwrap_or("")).truncate());
                    }
                    ui.end_row();
                }
            });
        }
        for l in r["lines"].as_array().cloned().unwrap_or_default() {
            ui.label(l.as_str().unwrap_or(""));
        }
    });
}

/// Text being typed into a window's fields, by field, until it is committed.
pub type Drafts = std::collections::HashMap<String, String>;

/// A field that shows `current` and keeps what is typed in `drafts` until focus leaves; then
/// returns the new text if it changed.
pub(crate) fn draft_field(ui: &mut egui::Ui, drafts: &mut Drafts, key: &str, current: &str, multiline: bool, width: f32) -> Option<String> {
    let mut text = drafts.get(key).cloned().unwrap_or_else(|| current.to_string());
    let edit = if multiline { egui::TextEdit::multiline(&mut text).desired_rows(2) } else { egui::TextEdit::singleline(&mut text) };
    let r = ui.add(edit.desired_width(width).id_salt(key));
    if r.changed() {
        drafts.insert(key.to_string(), text.clone());
    }
    if r.lost_focus() {
        drafts.remove(key);
        if text != current {
            return Some(text);
        }
    }
    None
}

fn scene_properties_window(ed: &mut Editor, ctx: &Context) {
    let Some((scene, mut new_name, mut drafts)) = ed.tools.scene_props.take() else { return };
    let Ok(props) = ed.session.run("scene.properties", &json!({"scene": scene})) else { return };
    let mut open = true;
    let mut action: Option<(&'static str, Value)> = None;
    egui::Window::new("Scene Properties").open(&mut open).default_width(420.0).show(ctx, |ui| {
        ui.label(RichText::new(props["heading"].as_str().unwrap_or("")).strong());
        egui::Grid::new("scene-props").num_columns(2).show(ui, |ui| {
            ui.label("Title");
            if let Some(title) = draft_field(ui, &mut drafts, "title", props["title"].as_str().unwrap_or(""), false, 260.0) {
                action = Some(("scene.setProperties", json!({"scene": scene, "title": title})));
            }
            ui.end_row();
            ui.label("Color");
            ui.horizontal_wrapped(|ui| {
                let current = props["color"].as_str().unwrap_or("");
                if ui.selectable_label(current.is_empty(), "None").clicked() {
                    action = Some(("scene.setProperties", json!({"scene": scene, "color": ""})));
                }
                for (name, hex) in wordsmith_script::scene_data::SCENE_COLORS {
                    let c = wordsmith_script::revisions::parse_color(hex).map_or(egui::Color32::GRAY, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
                    if ui.selectable_label(current == name, RichText::new(name).background_color(c)).clicked() {
                        action = Some(("scene.setProperties", json!({"scene": scene, "color": name})));
                    }
                }
            });
            ui.end_row();
            ui.label("Summary");
            if let Some(summary) = draft_field(ui, &mut drafts, "summary", props["summary"].as_str().unwrap_or(""), true, 260.0) {
                action = Some(("scene.setSummary", json!({"scene": scene, "text": summary})));
            }
            ui.end_row();
        });
        ui.separator();
        ui.label(RichText::new("Characters in the scene").strong());
        let speaking: Vec<String> =
            props["speaking"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
        for who in props["cast"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)) {
            ui.horizontal(|ui| {
                let tag = if speaking.contains(&who) { "" } else { " (does not speak)" };
                ui.label(format!("{who}{tag}"));
                let label = if speaking.contains(&who) { "Hide" } else { "Remove" };
                if ui.small_button(label).clicked() {
                    action = Some(("scene.removeCharacter", json!({"scene": scene, "name": who})));
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label("Arc beat");
                if let Some(arc) = draft_field(ui, &mut drafts, &format!("arc:{who}"), props["arcs"][&who].as_str().unwrap_or(""), false, 260.0) {
                    action = Some(("scene.setProperties", json!({"scene": scene, "arc": {"character": who, "text": arc}})));
                }
            });
        }
        for who in props["hidden"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{who} (hidden)")).weak());
                if ui.small_button("Show").clicked() {
                    action = Some(("scene.addCharacter", json!({"scene": scene, "name": who})));
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut new_name).hint_text("Name").desired_width(160.0));
            if ui.add_enabled(!new_name.trim().is_empty(), egui::Button::new("Add a character who does not speak")).clicked() {
                action = Some(("scene.addCharacter", json!({"scene": scene, "name": new_name.trim()})));
                new_name.clear();
            }
        });
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    if open {
        ed.tools.scene_props = Some((scene, new_name, drafts));
    }
}

fn character_window(ed: &mut Editor, ctx: &Context) {
    let Some((mut name, mut drafts)) = ed.tools.character.take() else { return };
    let list: Vec<String> = ed
        .session
        .run("character.list", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    if name.is_empty() {
        name = list.first().cloned().unwrap_or_default();
    }
    let info = ed.session.run("character.get", &json!({"name": name})).unwrap_or(Value::Null);
    let mut open = true;
    let mut action: Option<(&'static str, Value)> = None;
    egui::Window::new("Character Details").open(&mut open).default_width(380.0).show(ctx, |ui| {
        egui::ComboBox::from_id_salt("character-pick").selected_text(name.as_str()).show_ui(ui, |ui| {
            for c in &list {
                if ui.selectable_label(name == *c, c.as_str()).clicked() {
                    name = c.clone();
                }
            }
        });
        if name.is_empty() {
            ui.label("No one speaks yet.");
            return;
        }
        egui::Grid::new("traits").num_columns(2).show(ui, |ui| {
            for t in info["traits"].as_array().cloned().unwrap_or_default().iter().filter_map(|t| t.as_str().map(str::to_string)) {
                ui.label(&t);
                if let Some(v) =
                    draft_field(ui, &mut drafts, &format!("{name}:{t}"), info["person"]["traits"][&t].as_str().unwrap_or(""), false, 200.0)
                {
                    action = Some(("character.set", json!({"name": name, "traits": {t: v}})));
                }
                ui.end_row();
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            let mut on = info["person"]["highlightOn"].as_bool().unwrap_or(false);
            if ui.checkbox(&mut on, "Highlight speeches").changed() {
                action = Some(("character.set", json!({"name": name, "highlightOn": on})));
            }
            let hex = info["person"]["highlight"].as_str().unwrap_or("#FFF176");
            let c = wordsmith_script::revisions::parse_color(hex).unwrap_or(jcj_doc::props::Rgb(255, 241, 118));
            let mut color = egui::Color32::from_rgb(c.0, c.1, c.2);
            if ui.color_edit_button_srgba(&mut color).changed() {
                action = Some(("character.set", json!({"name": name, "highlight": format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())})));
            }
        });
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    if open {
        ed.tools.character = Some((name, drafts));
    }
}

pub fn show(ed: &mut Editor, ctx: &Context) {
    reports_window(ed, ctx);
    scene_properties_window(ed, ctx);
    character_window(ed, ctx);
}
