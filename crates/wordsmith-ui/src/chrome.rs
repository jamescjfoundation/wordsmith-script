// SPDX-License-Identifier: MIT OR Apache-2.0
//! Menus, toolbar and status bar. Every item runs a command by id; labels and shortcuts come from
//! the registry, so menus, keys, the palette, the CLI and MCP never disagree.

use egui::Ui;
use serde_json::json;
use wordsmith_script::{ElementKind, Elements};

use crate::editor::{Editor, ZOOM_RANGE};

/// A menu item for command `id`, labelled and with its shortcut from the registry; greyed out
/// when the command says it cannot run.
fn item(ed: &mut Editor, ui: &mut Ui, id: &str) {
    let Some(spec) = ed.session.registry.get(id) else { return };
    let (label, shortcut) = (spec.label, spec.shortcut);
    let enabled = ed.session.why_disabled(id).is_none();
    let button = egui::Button::new(label).shortcut_text(shortcut.split(" / ").next().unwrap_or(""));
    if ui.add_enabled(enabled, button).clicked() {
        ed.run(id, json!({}));
        ui.close();
    }
}

pub fn menu_bar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                item(ed, ui, "file.new");
                ui.separator();
                item(ed, ui, "file.exportPdfAs");
                item(ed, ui, "file.print");
                ui.separator();
                item(ed, ui, "file.recoverPrompt");
            });
            ui.menu_button("Edit", |ui| {
                for id in ["edit.undo", "edit.redo", "edit.cut", "edit.copy", "select.all"] {
                    item(ed, ui, id);
                }
            });
            ui.menu_button("Format", |ui| {
                ui.label("Element");
                for id in [
                    "element.general",
                    "element.sceneHeading",
                    "element.action",
                    "element.character",
                    "element.parenthetical",
                    "element.dialogue",
                    "element.transition",
                    "element.shot",
                    "element.castList",
                    "element.newAct",
                    "element.endOfAct",
                ] {
                    item(ed, ui, id);
                }
                ui.separator();
                item(ed, ui, "view.elementSettings");
            });
            ui.menu_button("View", |ui| {
                item(ed, ui, "view.commandPalette");
                if ui.button("Zoom In").clicked() {
                    ed.zoom = (ed.zoom * 1.25).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                }
                if ui.button("Zoom Out").clicked() {
                    ed.zoom = (ed.zoom / 1.25).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                }
            });
            ui.menu_button("Help", |ui| item(ed, ui, "help.about"));
        });
    });
}

pub fn toolbar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.add_enabled(ed.session.can_undo(), egui::Button::new("Undo")).clicked() {
                ed.run("edit.undo", json!({}));
            }
            if ui.add_enabled(ed.session.can_redo(), egui::Button::new("Redo")).clicked() {
                ed.run("edit.redo", json!({}));
            }
            ui.separator();
            let elements = Elements::of_doc(&ed.session.doc);
            let current = match ed.session.doc.para_at(&ed.session.sel.focus) {
                Some(p) => elements.of_para(p).clone(),
                None => {
                    elements.of_kind(ElementKind::General).cloned().unwrap_or_else(|| wordsmith_script::element::builtin_def(ElementKind::General))
                }
            };
            let mut chosen = None;
            egui::ComboBox::from_id_salt("element").selected_text(current.name.as_str()).width(160.0).show_ui(ui, |ui| {
                for def in elements.all().iter().filter(|d| !d.kind.is_outline() || d.id == current.id) {
                    if ui.selectable_label(def.id == current.id, def.name.as_str()).clicked() {
                        chosen = Some(def.id.clone());
                    }
                }
            });
            if let Some(id) = chosen {
                ed.run("element.set", json!({"element": id}));
            }
            ui.separator();
            ui.label(format!("{:.0}%", ed.zoom * 100.0));
        });
    });
}

pub fn status_bar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::bottom("status").show(ui, |ui| {
        ui.horizontal(|ui| {
            let scenes = wordsmith_script::scenes(&ed.session.doc).len();
            let pag = ed.paginated();
            let page = pag.to_output(&ed.session.sel.focus).and_then(|at| pag.layout.caret(&at)).map_or(1, |c| c.page + 1);
            ui.label(format!("Page {page} of {}", pag.layout.pages.len()));
            ui.separator();
            ui.label(format!("{scenes} scene{}", if scenes == 1 { "" } else { "s" }));
            if !ed.message.is_empty() {
                ui.separator();
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), ed.message.as_str());
            }
        });
    });
}
