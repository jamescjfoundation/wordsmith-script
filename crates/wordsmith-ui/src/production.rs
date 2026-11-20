// SPDX-License-Identifier: MIT OR Apache-2.0
//! Production windows (SPEC-2 7, 11, 18): print choices, the watermark and scene-number
//! options. Each changes the script, or prints, only through commands.

use egui::Context;
use serde_json::{Value, json};

use crate::editor::Editor;

/// The print window's choices.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintChoices {
    /// "all", "pages" or "scenes".
    pub range: &'static str,
    pub from: String,
    pub to: String,
    pub copies: u32,
    pub collate: bool,
    pub reverse: bool,
    /// "all", "revised", "unrevised" or "set".
    pub which: &'static str,
    pub set: usize,
    pub title_page: bool,
    pub marks: bool,
    /// Sides for this character (empty: the whole script).
    pub sides: String,
}

impl Default for PrintChoices {
    fn default() -> Self {
        PrintChoices {
            range: "all",
            from: String::new(),
            to: String::new(),
            copies: 1,
            collate: true,
            reverse: false,
            which: "all",
            set: 0,
            title_page: true,
            marks: true,
            sides: String::new(),
        }
    }
}

impl PrintChoices {
    /// The print options as the export and print commands take them.
    pub fn params(&self) -> Value {
        let mut v =
            json!({"copies": self.copies, "collate": self.collate, "reverse": self.reverse, "titlePage": self.title_page, "marks": self.marks});
        let n = |t: &str| t.trim().parse::<u64>().ok();
        if self.range != "all"
            && let Some(a) = n(&self.from)
        {
            let b = n(&self.to).unwrap_or(a);
            // Scenes are counted from 1 in the window, from 0 by the commands.
            if self.range == "scenes" {
                v["scenes"] = json!([a.saturating_sub(1), b.saturating_sub(1)]);
            } else {
                v["pages"] = json!([a, b]);
            }
        }
        match self.which {
            "revised" => v["revisedOnly"] = json!(true),
            "unrevised" => v["unrevisedOnly"] = json!(true),
            "set" => v["set"] = json!(self.set),
            _ => {}
        }
        if !self.sides.is_empty() {
            v["sides"] = json!(self.sides);
        }
        v
    }
}

fn print_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut c) = ed.tools.print.take() else { return };
    let mut open = true;
    let mut action: Option<&'static str> = None;
    let sets: Vec<String> = wordsmith_script::revisions::Revisions::of_doc(&ed.session.doc).sets.iter().map(|s| s.name.clone()).collect();
    let characters: Vec<String> = ed
        .session
        .run("character.list", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    egui::Window::new("Print").open(&mut open).resizable(false).show(ctx, |ui| {
        egui::Grid::new("print").num_columns(2).show(ui, |ui| {
            ui.label("Print");
            ui.horizontal(|ui| {
                for (k, l) in [("all", "Everything"), ("pages", "Pages"), ("scenes", "Scenes")] {
                    ui.selectable_value(&mut c.range, k, l);
                }
                if c.range != "all" {
                    ui.add(egui::TextEdit::singleline(&mut c.from).desired_width(40.0).hint_text("from"));
                    ui.label("to");
                    ui.add(egui::TextEdit::singleline(&mut c.to).desired_width(40.0).hint_text("to"));
                }
            });
            ui.end_row();
            ui.label("Pages");
            ui.horizontal(|ui| {
                for (k, l) in [("all", "All"), ("revised", "Revised only"), ("unrevised", "Unrevised only"), ("set", "One set")] {
                    ui.selectable_value(&mut c.which, k, l);
                }
            });
            ui.end_row();
            if c.which == "set" {
                ui.label("Set");
                egui::ComboBox::from_id_salt("print-set").selected_text(sets.get(c.set).cloned().unwrap_or_else(|| "—".into())).show_ui(ui, |ui| {
                    for (i, name) in sets.iter().enumerate() {
                        ui.selectable_value(&mut c.set, i, name.as_str());
                    }
                });
                ui.end_row();
            }
            ui.label("Sides for");
            egui::ComboBox::from_id_salt("print-sides")
                .selected_text(if c.sides.is_empty() { "The whole script" } else { c.sides.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut c.sides, String::new(), "The whole script");
                    for name in &characters {
                        ui.selectable_value(&mut c.sides, name.clone(), name.as_str());
                    }
                });
            ui.end_row();
            ui.label("Copies");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut c.copies).range(1..=999));
                ui.checkbox(&mut c.collate, "Collate");
                ui.checkbox(&mut c.reverse, "Last page first");
            });
            ui.end_row();
            ui.label("Include");
            ui.horizontal(|ui| {
                ui.checkbox(&mut c.title_page, "Title page");
                ui.checkbox(&mut c.marks, "Revision marks");
            });
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Print").clicked() {
                action = Some("file.print");
            }
            if ui.button("Save as PDF…").clicked() {
                action = Some("file.exportPdfAs");
            }
            if ui.button("Preview").clicked() {
                action = Some("file.printPreview");
            }
        });
    });
    if let Some(cmd) = action {
        let params = c.params();
        // The format checker first, when the script asks for it.
        let check = ed.session.run("check.settings", &json!({})).ok().and_then(|v| v["beforePrint"].as_str().map(str::to_string)).unwrap_or_default();
        let issues = ed.session.run("check.run", &json!({})).ok().and_then(|v| v["count"].as_u64()).unwrap_or(0);
        if check == "scan" && issues > 0 && ed.tools.checker.is_none() {
            ed.message = format!("{issues} format issues: fix or ignore them, then print again.");
            crate::tools::open(ed, "tool.checker", &Value::Null);
        } else {
            ed.dialog_params = Some(params.clone());
            ed.run(cmd, params);
        }
    }
    if open {
        ed.tools.print = Some(c);
    }
}

fn watermark_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut form) = ed.tools.watermark.take() else { return };
    let mut open = true;
    let mut apply = false;
    egui::Window::new("Watermark").open(&mut open).resizable(false).show(ctx, |ui| {
        let mut on = form["on"].as_bool().unwrap_or(false);
        if ui.checkbox(&mut on, "Print a watermark on every page").changed() {
            form["on"] = json!(on);
        }
        egui::Grid::new("watermark").num_columns(2).show(ui, |ui| {
            for (key, label) in [("text", "Text"), ("recipient", "For (recipient)")] {
                ui.label(label);
                let mut t = form[key].as_str().unwrap_or("").to_string();
                if ui.text_edit_singleline(&mut t).changed() {
                    form[key] = json!(t);
                }
                ui.end_row();
            }
            ui.label("Strength");
            let mut o = form["opacity"].as_u64().unwrap_or(70) as u32;
            if ui.add(egui::Slider::new(&mut o, 0..=100).suffix("%")).changed() {
                form["opacity"] = json!(o);
            }
            ui.end_row();
        });
        if ui.button("Apply").clicked() {
            apply = true;
        }
    });
    if apply {
        ed.run("watermark.set", json!({"on": form["on"], "text": form["text"], "recipient": form["recipient"], "opacity": form["opacity"]}));
    }
    if open {
        ed.tools.watermark = Some(form);
    }
}

fn scene_numbers_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut number) = ed.tools.scene_numbers.take() else { return };
    let mut open = true;
    let mut action: Option<(&'static str, Value)> = None;
    let settings = ed.session.run("scene.numbers", &json!({})).unwrap_or(Value::Null);
    let o = wordsmith_script::scene_numbers::SceneNumbers::of_doc(&ed.session.doc);
    egui::Window::new("Scene Numbers").open(&mut open).resizable(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            let mut show = settings["show"].as_bool().unwrap_or(false);
            if ui.checkbox(&mut show, "Show numbers").changed() {
                action = Some(("scene.showNumbers", json!({"show": show})));
            }
            let mut locked = settings["locked"].as_bool().unwrap_or(false);
            if ui.checkbox(&mut locked, "Keep numbers (locked)").changed() {
                action = Some(("scene.lockNumbers", json!({"lock": locked})));
            }
        });
        egui::Grid::new("scene-numbers").num_columns(2).show(ui, |ui| {
            ui.label("Start at");
            let mut start = o.start;
            if ui.add(egui::DragValue::new(&mut start).range(1..=100_000)).changed() {
                action = Some(("scene.configureNumbers", json!({"start": start})));
            }
            ui.end_row();
            ui.label("Start again every");
            let mut every = o.restart_every;
            if ui.add(egui::DragValue::new(&mut every).range(0..=10_000).suffix(" scenes (0: never)")).changed() {
                action = Some(("scene.configureNumbers", json!({"restartEvery": every})));
            }
            ui.end_row();
            ui.label("Added scenes");
            ui.horizontal(|ui| {
                if ui.selectable_label(!o.prefix, "1, 1A, 2").clicked() {
                    action = Some(("scene.configureNumbers", json!({"prefix": false})));
                }
                if ui.selectable_label(o.prefix, "1, A2, 2").clicked() {
                    action = Some(("scene.configureNumbers", json!({"prefix": true})));
                }
            });
            ui.end_row();
            ui.label("Show at");
            ui.horizontal(|ui| {
                for (k, l) in [("both", "Both sides"), ("left", "Left"), ("right", "Right")] {
                    if ui.selectable_label(o.sides == k, l).clicked() {
                        action = Some(("scene.configureNumbers", json!({"sides": k})));
                    }
                }
            });
            ui.end_row();
            ui.label("Font");
            let mut font = o.font.clone();
            if ui.add(egui::TextEdit::singleline(&mut font).hint_text("The script's")).lost_focus() && font != o.font {
                action = Some(("scene.configureNumbers", json!({"font": font})));
            }
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Renumber all").clicked() {
                action = Some(("scene.renumber", json!({})));
            }
            if ui.button("Renumber from here").clicked() {
                action = Some(("scene.renumber", json!({"fromCaret": true})));
            }
            if ui.button("Number new scenes only").clicked() {
                action = Some(("scene.renumber", json!({"keepExisting": true})));
            }
        });
        ui.horizontal(|ui| {
            ui.label("This scene's number");
            ui.add(egui::TextEdit::singleline(&mut number).desired_width(60.0));
            if ui.add_enabled(!number.trim().is_empty(), egui::Button::new("Set")).clicked() {
                action = Some(("scene.setNumber", json!({"number": number.trim()})));
            }
        });
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    if open {
        ed.tools.scene_numbers = Some(number);
    }
}

pub fn show(ed: &mut Editor, ctx: &Context) {
    print_window(ed, ctx);
    watermark_window(ed, ctx);
    scene_numbers_window(ed, ctx);
}
