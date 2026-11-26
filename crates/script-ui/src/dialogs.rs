// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's dialogs: element settings, "is this a script?", the title page, header and footer,
//! notes, revision sets, and Script's own preferences. The shell has the general ones (the
//! command palette, crash recovery, "save changes first?", About). Each one changes the script
//! only by running commands.

use egui::Context;
use jcj_engine::Session;
use jcj_shell::Shell;
use serde_json::{Map, Value, json};
use wordsmith_script_model::Elements;

use crate::editor::Editor;

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

/// `#RRGGBB` as a color.
fn parse_hex(s: &str) -> Option<egui::Color32> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(egui::Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
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
                flag_field(ui, form, "lowercase", "Small letters");
                flag_field(ui, form, "strike", "Strike through");
            });
            ui.end_row();
            ui.label("Text color");
            ui.horizontal(|ui| {
                let mut has = !form["color"].is_null();
                if ui.checkbox(&mut has, "Own color").changed() {
                    form["color"] = if has { json!("#000000") } else { Value::Null };
                }
                if let Some(c) = form["color"].as_str().and_then(parse_hex) {
                    let mut c = c;
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        form["color"] = json!(format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b()));
                    }
                }
            });
            ui.end_row();
            ui.label("Highlight");
            let mut choices: Vec<(Value, String)> = vec![(Value::Null, "None".into())];
            choices.extend(wordsmith_script_model::commands::format::HIGHLIGHTS.iter().map(|(n, _)| (json!(n), (*n).to_string())));
            choice_field(ui, form, "highlight", &choices);
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
            if ui.button("Template Formatting for All").on_hover_text("Every element's formatting back to the screenplay template's").clicked() {
                action = Some(("element.applyTemplate", json!({})));
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
            switch_to = Some(Elements::of_doc(&ed.session.doc).id_of(wordsmith_script_model::ElementKind::Action).to_string());
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

/// Script's own preferences, after the shell's on the Preferences page: backups, the File page
/// at start, writing statistics.
pub fn prefs_body(ed: &mut Editor, ui: &mut egui::Ui, _shell: &mut Shell) {
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
    ui.separator();
    ui.heading("Starting a script");
    ui.checkbox(&mut ed.tools.show_start_on_launch, "Show the File page's New page when no script opens");
    ui.separator();
    ui.heading("Writing statistics");
    if ui.checkbox(&mut ed.stats.data.enabled, "Keep writing statistics").changed() {
        ed.stats.dirty = true;
    }
    ui.horizontal(|ui| {
        ui.label("Sprints last");
        if ui.add(egui::DragValue::new(&mut ed.stats.data.sprint_minutes).range(1..=240).suffix(" minutes")).changed() {
            ed.stats.dirty = true;
        }
    });
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
        flag_field(ui, &mut form, "firstPage", "Show the header on page 1");
        let mut footer_first = form["firstPageFooter"].as_bool().unwrap_or(form["firstPage"].as_bool().unwrap_or(false));
        if ui.checkbox(&mut footer_first, "Show the footer on page 1").changed() {
            form["firstPageFooter"] = json!(footer_first);
        }
        ui.horizontal(|ui| {
            ui.label("First page number");
            number_field(ui, &mut form, "startPage", 1.0..=100_000.0, 1.0);
        });
        ui.horizontal(|ui| {
            ui.label("Date");
            text_field(ui, &mut form, "date");
        });
        ui.label("Fields: {page}, {scene}, {date}, {title}, {revision}, {collated} (every revision set used). An empty date is today's.");
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
    egui::Area::new(egui::Id::unique("note_popover")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.label(if id.is_some() { "Note" } else { "New note" });
            let r = ui.add(egui::TextEdit::multiline(&mut text).desired_rows(4).desired_width(260.0));
            if id.is_none() && !r.has_focus() && text.is_empty() {
                r.request_focus();
            }
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    action = Some(match id {
                        Some(i) => ("note.edit", json!({"id": i, "text": text, "now": crate::now_secs()})),
                        None => ("note.add", json!({"text": text, "now": crate::now_secs()})),
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

/// The revision sets: the current set, each set's name, color and text styles, the mark and its
/// place, colored pages, and rolling back. Every change is a command.
fn revisions_window(ed: &mut Editor, ctx: &Context) {
    let mut open = ed.show_revisions;
    let rev = wordsmith_script_model::revisions::Revisions::of_doc(&ed.session.doc);
    let mut action: Option<(&str, Value)> = None;
    egui::Window::new("Revision Sets").open(&mut open).collapsible(false).show(ctx, |ui| {
        let mut mode = rev.mode;
        if ui.checkbox(&mut mode, "Revision mode: mark what I type and delete").changed() {
            action = Some(("revision.mode", json!({"on": mode})));
        }
        if rev.sets.is_empty() {
            ui.label("No revision sets yet. Next Revision starts Blue (the original draft is White).");
        }
        egui::Grid::new("revision_sets").num_columns(7).show(ui, |ui| {
            for h in ["Current", "Name", "Color", "Bold", "Underline", "Strike", "Mark"] {
                ui.label(egui::RichText::new(h).strong());
            }
            ui.end_row();
            for (i, set) in rev.sets.iter().enumerate() {
                if ui.radio(set.name == rev.current, "").clicked() {
                    action = Some(("revision.setCurrent", json!({"name": set.name})));
                }
                let mut name = set.name.clone();
                if ui.add(egui::TextEdit::singleline(&mut name).desired_width(140.0)).lost_focus() && name != set.name {
                    action = Some(("revision.configure", json!({"set": {"index": i, "name": name}})));
                }
                let mut color = wordsmith_script_model::revisions::parse_color(&set.color).map_or([255, 255, 255], |c| [c.0, c.1, c.2]);
                if ui.color_edit_button_srgb(&mut color).changed() {
                    let hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
                    action = Some(("revision.configure", json!({"set": {"index": i, "color": hex}})));
                }
                for (key, on) in [("bold", set.bold), ("underline", set.underline), ("strike", set.strike)] {
                    let mut b = on;
                    if ui.checkbox(&mut b, "").changed() {
                        action = Some(("revision.configure", json!({"set": {"index": i, key: b}})));
                    }
                }
                // The set's own margin mark (empty: the script's).
                let mut mark = set.mark.clone();
                if ui.add(egui::TextEdit::singleline(&mut mark).desired_width(24.0).char_limit(1)).changed() {
                    action = Some(("revision.configure", json!({"set": {"index": i, "mark": mark}})));
                }
                ui.end_row();
            }
        });
        if ui.button("Next Revision").clicked() {
            action = Some(("revision.next", json!({})));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Mark");
            let mut mark = rev.mark.clone();
            if ui.add(egui::TextEdit::singleline(&mut mark).desired_width(24.0)).changed() && !mark.trim().is_empty() {
                action = Some(("revision.configure", json!({"mark": mark.trim()})));
            }
            ui.label("at");
            let mut at = f64::from(rev.mark_at);
            if ui.add(egui::DragValue::new(&mut at).range(0.1..=8.0).speed(0.05).suffix(" in")).changed() {
                action = Some(("revision.configure", json!({"markAt": at})));
            }
            ui.label("from the page's left edge");
        });
        let mut active_only = rev.active_only;
        if ui.checkbox(&mut active_only, "Show only the current set (otherwise every set, collated)").changed() {
            action = Some(("revision.configure", json!({"activeOnly": active_only})));
        }
        let mut show_marks = rev.show_marks;
        if ui.checkbox(&mut show_marks, "Show revision marks").changed() {
            action = Some(("revision.configure", json!({"showMarks": show_marks})));
        }
        let mut color_pages = rev.color_pages;
        if ui.checkbox(&mut color_pages, "Color revised pages in PDF and print").changed() {
            action = Some(("revision.configure", json!({"colorPages": color_pages})));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Roll back to");
            if ui.button("the original (White)").clicked() {
                action = Some(("revision.rollback", json!({"to": ""})));
            }
            for set in rev.sets.iter().rev().skip(1) {
                if ui.button(set.name.as_str()).clicked() {
                    action = Some(("revision.rollback", json!({"to": set.name})));
                }
            }
        });
        ui.label("Rolling back clears the later sets' marks; the text stays as it is.");
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    ed.show_revisions = open;
}

/// Show whichever dialogs are open.
pub fn show(ed: &mut Editor, ctx: &Context) {
    crate::tools::show(ed, ctx);
    revisions_window(ed, ctx);
    note_popover(ed, ctx);
    header_footer_window(ed, ctx);
    title_page_window(ed, ctx);
    settings_window(ed, ctx);
    import_window(ed, ctx);
}
