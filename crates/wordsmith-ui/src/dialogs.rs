// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dialogs: the command palette, element settings, crash recovery and About. Each one changes
//! the script only by running commands.

use egui::Context;
use serde_json::{Map, Value, json};
use wordsmith_engine::{Registry, Session};
use wordsmith_script::Elements;

use crate::editor::Editor;

/// Most commands the palette lists at once.
const PALETTE_ROWS: usize = 40;

/// The command palette: type to filter every command, Enter or click to run one.
#[derive(Default)]
pub struct Palette {
    pub query: String,
}

/// Commands matching `query` (every word in the id, label or menu location), as (id, label,
/// shortcut). Commands that need parameters are left out: the palette runs commands as they are.
pub fn palette_matches(registry: &Registry, query: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    registry
        .all()
        .iter()
        .filter(|s| s.params == "{}")
        .filter(|s| {
            let hay = format!("{} {} {}", s.id, s.label, s.location).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|s| (s.id, s.label, s.shortcut))
        .take(PALETTE_ROWS)
        .collect()
}

/// The element settings being edited, as `element.settings` reports them.
pub struct SettingsDialog {
    pub element: String,
    pub original: Value,
    pub form: Value,
    pub new_name: String,
}

impl SettingsDialog {
    pub fn new(session: &mut Session, element: &str) -> Self {
        let original = session.run("element.settings", &json!({"element": element})).unwrap_or(Value::Null);
        SettingsDialog { element: element.to_string(), form: original.clone(), original, new_name: String::new() }
    }

    /// The fields that changed, for `element.configure`.
    pub fn changes(&self) -> Value {
        let mut out = Map::new();
        out.insert("element".into(), json!(self.element));
        if let (Some(form), Some(orig)) = (self.form.as_object(), self.original.as_object()) {
            for (k, v) in form {
                if !matches!(k.as_str(), "id" | "kind" | "builtin") && orig.get(k) != Some(v) {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        Value::Object(out)
    }
}

/// Copies left by an earlier run, offered for recovery.
#[derive(Default)]
pub struct RecoveryPrompt;

fn text_field(ui: &mut egui::Ui, form: &mut Value, key: &str) {
    let mut s = form.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    if ui.text_edit_singleline(&mut s).changed() {
        form[key] = json!(s);
    }
}

fn number_field(ui: &mut egui::Ui, form: &mut Value, key: &str, range: std::ops::RangeInclusive<f64>, step: f64) {
    let mut x = form.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    if ui.add(egui::DragValue::new(&mut x).range(range).speed(step)).changed() {
        form[key] = json!(x);
    }
}

fn flag_field(ui: &mut egui::Ui, form: &mut Value, key: &str, label: &str) {
    let mut b = form.get(key).and_then(Value::as_bool).unwrap_or(false);
    if ui.checkbox(&mut b, label).changed() {
        form[key] = json!(b);
    }
}

fn choice_field(ui: &mut egui::Ui, form: &mut Value, key: &str, choices: &[(Value, String)]) {
    let current = form.get(key).cloned().unwrap_or(Value::Null);
    let shown = choices.iter().find(|(v, _)| *v == current).map_or("—".to_string(), |(_, l)| l.clone());
    egui::ComboBox::from_id_salt(key).selected_text(shown).show_ui(ui, |ui| {
        for (v, label) in choices {
            if ui.selectable_label(*v == current, label.as_str()).clicked() {
                form[key] = v.clone();
            }
        }
    });
}

fn settings_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut dialog) = ed.settings.take() else { return };
    let elements = Elements::of_doc(&ed.session.doc);
    let element_choices: Vec<(Value, String)> = elements.all().iter().map(|d| (json!(d.id), d.name.clone())).collect();
    let mut open = true;
    let mut action: Option<(&str, Value)> = None;
    let mut switch_to: Option<String> = None;
    egui::Window::new("Element Settings").open(&mut open).resizable(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Element");
            let mut wrapper = json!({"e": dialog.element});
            choice_field(ui, &mut wrapper, "e", &element_choices);
            if wrapper["e"] != json!(dialog.element) {
                switch_to = wrapper["e"].as_str().map(str::to_string);
            }
        });
        ui.separator();
        let form = &mut dialog.form;
        egui::Grid::new("element-settings").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            text_field(ui, form, "name");
            ui.end_row();
            ui.label("Font");
            text_field(ui, form, "font");
            ui.end_row();
            ui.label("Size (pt)");
            number_field(ui, form, "size", 4.0..=72.0, 0.5);
            ui.end_row();
            ui.label("Style");
            ui.horizontal(|ui| {
                flag_field(ui, form, "bold", "Bold");
                flag_field(ui, form, "italic", "Italic");
                flag_field(ui, form, "underline", "Underline");
                flag_field(ui, form, "uppercase", "Capitals");
            });
            ui.end_row();
            ui.label("Alignment");
            choice_field(ui, form, "align", &["left", "center", "right", "justify"].map(|a| (json!(a), a.to_string())));
            ui.end_row();
            ui.label("Left (in from page edge)");
            number_field(ui, form, "left", 0.0..=8.5, 0.05);
            ui.end_row();
            ui.label("Right (in from page edge)");
            number_field(ui, form, "right", 0.0..=8.5, 0.05);
            ui.end_row();
            ui.label("Line spacing");
            choice_field(ui, form, "lineSpacing", &[1.0, 1.5, 2.0, 3.0].map(|x| (json!(x), format!("{x}"))));
            ui.end_row();
            ui.label("Space before (lines)");
            number_field(ui, form, "spaceBefore", 0.0..=30.0, 0.5);
            ui.end_row();
            ui.label("Return creates");
            choice_field(ui, form, "next", &element_choices);
            ui.end_row();
            ui.label("Tab switches to");
            let mut tab_choices = vec![(Value::Null, "Nothing".to_string())];
            tab_choices.extend(element_choices.iter().cloned());
            choice_field(ui, form, "tab", &tab_choices);
            ui.end_row();
            ui.label("Pages");
            ui.vertical(|ui| {
                flag_field(ui, form, "startNewPage", "Start a new page");
                flag_field(ui, form, "keepWithNext", "Keep with next");
                flag_field(ui, form, "keepTogether", "Keep lines together");
            });
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                action = Some(("element.configure", dialog.changes()));
            }
            if dialog.original.get("builtin") == Some(&json!(false)) && ui.button("Delete Element").clicked() {
                action = Some(("element.delete", json!({"element": dialog.element})));
            }
        });
        ui.horizontal(|ui| {
            ui.label("New element");
            ui.text_edit_singleline(&mut dialog.new_name);
            if ui.add_enabled(!dialog.new_name.trim().is_empty(), egui::Button::new("Create")).clicked() {
                action = Some(("element.create", json!({"name": dialog.new_name.trim()})));
            }
        });
    });
    if let Some((id, params)) = action {
        let made = (id == "element.create").then(|| ed.session.run(id, &params));
        match made {
            Some(Ok(v)) => switch_to = v.get("id").and_then(Value::as_str).map(str::to_string),
            Some(Err(e)) => ed.message = e.to_string(),
            None => ed.run(id, params),
        }
        if id == "element.delete" {
            switch_to = Some(Elements::of_doc(&ed.session.doc).id_of(wordsmith_script::ElementKind::Action).to_string());
        }
        let element = switch_to.clone().unwrap_or(dialog.element.clone());
        dialog = SettingsDialog::new(&mut ed.session, &element);
    } else if let Some(element) = switch_to {
        dialog = SettingsDialog::new(&mut ed.session, &element);
    }
    if open {
        ed.settings = Some(dialog);
    }
}

fn palette_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut palette) = ed.palette.take() else { return };
    let mut open = true;
    let mut chosen = None;
    egui::Window::new("Commands").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        let edit = ui.text_edit_singleline(&mut palette.query);
        edit.request_focus();
        let matches = palette_matches(&ed.session.registry, &palette.query);
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            chosen = matches.first().map(|m| m.0);
        }
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (id, label, shortcut) in &matches {
                let text = if shortcut.is_empty() { (*label).to_string() } else { format!("{label}    {shortcut}") };
                if ui.selectable_label(false, text).clicked() {
                    chosen = Some(*id);
                }
            }
        });
    });
    if let Some(id) = chosen {
        ed.run(id, json!({}));
        return;
    }
    if open {
        ed.palette = Some(palette);
    }
}

fn recovery_window(ed: &mut Editor, ctx: &Context) {
    if ed.recovery.is_none() {
        return;
    }
    let copies = ed.session.recovery_documents();
    let blank = ed.session.is_untouched_blank();
    let mut open = true;
    let mut action = None;
    egui::Window::new("Recover Unsaved Scripts").open(&mut open).show(ctx, |ui| {
        if copies.is_empty() {
            ui.label("There are no unsaved scripts to recover.");
        }
        if !blank && !copies.is_empty() {
            ui.label("Start a new script (File › New Script) to restore one here.");
        }
        for c in &copies {
            ui.horizontal(|ui| {
                ui.label(format!("{} — {}", c.name, c.saved_at));
                if ui.add_enabled(blank, egui::Button::new("Restore")).clicked() {
                    action = Some(json!({"restore": c.id}));
                }
                if ui.button("Discard").clicked() {
                    action = Some(json!({"discard": c.id}));
                }
            });
        }
    });
    if let Some(params) = action {
        ed.run("file.recover", params);
    }
    if !open {
        ed.recovery = None;
    }
}

fn about_window(ed: &mut Editor, ctx: &Context) {
    let mut open = ed.show_about;
    egui::Window::new("About").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        for line in crate::about_text().lines() {
            ui.label(line);
        }
    });
    ed.show_about = open;
}

/// Show whichever dialogs are open.
pub fn show(ed: &mut Editor, ctx: &Context) {
    palette_window(ed, ctx);
    settings_window(ed, ctx);
    recovery_window(ed, ctx);
    about_window(ed, ctx);
}
