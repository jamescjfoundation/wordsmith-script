// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dialogs: the command palette, element settings, crash recovery, About, unsaved changes,
//! "is this a script?" and preferences. Each one changes the script only by running commands.

use egui::Context;
use serde_json::{Map, Value, json};
use wordsmith_engine::{Registry, Session};
use wordsmith_script::Elements;

use crate::editor::{Editor, Then, UnsavedAnswer};

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

/// "Save changes first?" before New, Open, Import or closing the window.
fn unsaved_window(ed: &mut Editor, ctx: &Context) {
    let Some(then) = ed.unsaved else { return };
    let name = crate::editor::script_name(&ed.session);
    let doing = match then {
        Then::New => "starting a new script",
        Then::Open => "opening another script",
        Then::Import => "importing",
        Then::Close => "closing",
    };
    let mut answer = None;
    egui::Window::new("Unsaved Changes").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.label(format!("Save the changes to “{name}” before {doing}?"));
        ui.label("If you don't save, your changes since the last save are lost.");
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                answer = Some(UnsavedAnswer::Save);
            }
            if ui.button("Don't Save").clicked() {
                answer = Some(UnsavedAnswer::DontSave);
            }
            if ui.button("Cancel").clicked() {
                answer = Some(UnsavedAnswer::Cancel);
            }
        });
    });
    if let Some(a) = answer {
        ed.answer_unsaved(a);
    }
}

/// "Is this a script?" when importing plain text or RTF (SPEC: import asks and auto-formats).
fn import_window(ed: &mut Editor, ctx: &Context) {
    let Some(q) = ed.import_question.clone() else { return };
    let file = std::path::Path::new(&q.path).file_name().and_then(|n| n.to_str()).unwrap_or("the file").to_string();
    let mut answer: Option<Option<bool>> = None;
    egui::Window::new("Import").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.label(format!("Is “{file}” a script?"));
        ui.label("A script is formatted into scene headings, action, characters and dialogue. Other text comes in as action, a paragraph at a time.");
        ui.horizontal(|ui| {
            let (yes, no) = (ui.button("It's a Script"), ui.button("Plain Text"));
            // The guess is the default: it has the keyboard focus.
            if q.guess {
                yes.request_focus()
            } else {
                no.request_focus()
            }
            if yes.clicked() {
                answer = Some(Some(true));
            }
            if no.clicked() {
                answer = Some(Some(false));
            }
            if ui.button("Cancel").clicked() {
                answer = Some(None);
            }
        });
    });
    if let Some(a) = answer {
        ed.answer_import(a);
    }
}

/// Preferences: auto-save and backups (the view settings are kept as they are left).
fn prefs_window(ed: &mut Editor, ctx: &Context) {
    let mut open = ed.show_prefs;
    egui::Window::new("Preferences").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.heading("Auto-save");
        ui.checkbox(&mut ed.autosave.enabled, "Save scripts automatically");
        let mut minutes = ed.autosave.every.as_secs() / 60;
        ui.horizontal(|ui| {
            ui.label("Every");
            if ui.add(egui::DragValue::new(&mut minutes).range(1..=u64::from(crate::prefs::MAX_AUTOSAVE_MINUTES))).changed() {
                ed.autosave.every = std::time::Duration::from_secs(minutes * 60);
            }
            ui.label("minutes, once typing pauses for");
            let mut idle = ed.autosave.idle.as_secs();
            if ui.add(egui::DragValue::new(&mut idle).range(1..=u64::from(crate::prefs::MAX_IDLE_SECONDS))).changed() {
                ed.autosave.idle = std::time::Duration::from_secs(idle);
            }
            ui.label("seconds");
        });
        ui.label("Only scripts saved to a file are saved automatically; crash recovery keeps the others.");
        ui.separator();
        ui.heading("Backups");
        ui.checkbox(&mut ed.backups.enabled, "Keep a backup copy of every save");
        ui.horizontal(|ui| {
            ui.label("Keep up to");
            let mut keep = ed.backups.keep;
            if ui.add(egui::DragValue::new(&mut keep).range(1..=crate::prefs::MAX_BACKUPS as usize)).changed() {
                ed.backups.keep = keep;
            }
            ui.label("copies of each script; the oldest are removed");
        });
        if let Some(dir) = &ed.backups.dir {
            ui.label(format!("Backups folder: {}", dir.display()));
        }
    });
    ed.show_prefs = open;
}

/// The title page fields, edited as text; Apply sets them (one undo step).
fn title_page_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut form) = ed.title_page.take() else { return };
    let mut open = true;
    let mut apply = false;
    egui::Window::new("Title Page").open(&mut open).collapsible(false).show(ctx, |ui| {
        egui::Grid::new("title_page").num_columns(2).show(ui, |ui| {
            for (key, label, rows) in [
                ("title", "Title", 2),
                ("credit", "Credit", 1),
                ("authors", "Authors", 2),
                ("source", "Source", 2),
                ("notes", "Notes", 2),
                ("contact", "Contact", 4),
                ("draftDate", "Draft date", 1),
                ("copyright", "Copyright", 1),
            ] {
                ui.label(label);
                let mut text = form.get(key).and_then(Value::as_str).unwrap_or("").to_string();
                if ui.add(egui::TextEdit::multiline(&mut text).desired_rows(rows).desired_width(360.0)).changed() {
                    form[key] = json!(text);
                }
                ui.end_row();
            }
        });
        ui.label("Printed before the script when any field has text. The script's pages are numbered from 1 after it.");
        if ui.button("Apply").clicked() {
            apply = true;
        }
    });
    if apply {
        ed.run("titlePage.set", form.clone());
    }
    if open {
        ed.title_page = Some(form);
    }
}

/// Header and footer slots with their fields; Apply sets them (one undo step).
fn header_footer_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut form) = ed.header_footer.take() else { return };
    let mut open = true;
    let mut apply = false;
    egui::Window::new("Header and Footer").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        egui::Grid::new("header_footer").num_columns(4).show(ui, |ui| {
            ui.label("");
            for slot in ["Left", "Center", "Right"] {
                ui.label(slot);
            }
            ui.end_row();
            for (band, label) in [("header", "Header"), ("footer", "Footer")] {
                ui.label(label);
                for slot in ["left", "center", "right"] {
                    let mut text = form.get(band).and_then(|b| b.get(slot)).and_then(Value::as_str).unwrap_or("").to_string();
                    if ui.add(egui::TextEdit::singleline(&mut text).desired_width(150.0)).changed() {
                        form[band][slot] = json!(text);
                    }
                }
                ui.end_row();
            }
        });
        flag_field(ui, &mut form, "firstPage", "Show on page 1");
        ui.horizontal(|ui| {
            ui.label("Date");
            text_field(ui, &mut form, "date");
        });
        ui.label("Fields: {page}, {scene}, {date}, {title}, {revision}. An empty date is today's.");
        if ui.button("Apply").clicked() {
            apply = true;
        }
    });
    if apply {
        ed.run("headerFooter.set", form.clone());
    }
    if open {
        ed.header_footer = Some(form);
    }
}

/// The note popover, beside the caret: a note's text to edit, or a new note for the selection.
fn note_popover(ed: &mut Editor, ctx: &Context) {
    let Some((id, mut text)) = ed.note_popover.take() else { return };
    let at = ed.caret_rect().map_or(egui::pos2(200.0, 200.0), |r| r.right_bottom() + egui::vec2(8.0, 8.0));
    let mut keep = true;
    let mut action: Option<(&str, Value)> = None;
    egui::Area::new(egui::Id::new("note_popover")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.label(if id.is_some() { "Note" } else { "New note" });
            let r = ui.add(egui::TextEdit::multiline(&mut text).desired_rows(4).desired_width(260.0));
            if id.is_none() && !r.has_focus() && text.is_empty() {
                r.request_focus();
            }
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    action = Some(match id {
                        Some(i) => ("note.edit", json!({"id": i, "text": text})),
                        None => ("note.add", json!({"text": text})),
                    });
                    keep = false;
                }
                if let Some(i) = id
                    && ui.button("Delete").clicked()
                {
                    action = Some(("note.delete", json!({"id": i})));
                    keep = false;
                }
                if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    keep = false;
                }
            });
        });
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    if keep {
        ed.note_popover = Some((id, text));
    }
}

/// Show whichever dialogs are open.
pub fn show(ed: &mut Editor, ctx: &Context) {
    note_popover(ed, ctx);
    header_footer_window(ed, ctx);
    title_page_window(ed, ctx);
    palette_window(ed, ctx);
    settings_window(ed, ctx);
    recovery_window(ed, ctx);
    about_window(ed, ctx);
    prefs_window(ed, ctx);
    import_window(ed, ctx);
    unsaved_window(ed, ctx);
}
