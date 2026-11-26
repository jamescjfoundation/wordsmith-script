// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Outline view (decision 0028): one row per lane with its beats as cards in order, beats in no
//! lane below, and the Script lane: the script's scenes, with the pages each scene starts on.
//! Dragging a beat onto a card puts it there; onto a lane's empty end, at the end. Beats sent to
//! the script show the pages they cover. The beats are the story board's, so edits show in both.

use egui::{Id, RichText, Ui, vec2};
use serde_json::{Value, json};
use wordsmith_script_model::outline::TEMPLATES;
use wordsmith_script_model::storyboard::{Beat, Storyboard};

use crate::editor::Editor;

/// A beat card's size, points.
const CARD: egui::Vec2 = vec2(170.0, 84.0);

enum Act {
    Run(&'static str, Value),
    Edit(u32),
}

fn beat_card(ui: &mut Ui, b: &Beat, pages: Option<&(usize, usize)>, act: &mut Option<Act>, lane: Option<u32>, index: usize) {
    let fill = wordsmith_script_model::revisions::parse_color(&b.color)
        .map_or(egui::Color32::from_rgb(255, 242, 168), |c| egui::Color32::from_rgb(c.0, c.1, c.2));
    let drag = ui.dnd_drag_source(Id::unique(("outline-beat", b.id)), b.id, |ui| {
        egui::Frame::new().fill(fill).corner_radius(6.0).inner_margin(6.0).show(ui, |ui| {
            ui.set_min_size(CARD - vec2(12.0, 12.0));
            ui.set_max_width(CARD.x - 12.0);
            let ink = egui::Color32::from_gray(20);
            ui.label(RichText::new(if b.title.is_empty() { "Untitled beat" } else { b.title.as_str() }).strong().color(ink));
            if let Some((a, z)) = pages {
                ui.label(RichText::new(if a == z { format!("page {a}") } else { format!("pages {a}–{z}") }).small().color(ink));
            }
        });
    });
    let r = &drag.response;
    if let Some(dropped) = r.dnd_release_payload::<u32>()
        && *dropped != b.id
    {
        *act = Some(Act::Run("beat.place", json!({"id": *dropped, "lane": lane, "index": index})));
    }
    if r.dnd_hover_payload::<u32>().is_some_and(|d| *d != b.id) {
        ui.painter().vline(r.rect.left() - 4.0, r.rect.y_range(), egui::Stroke::new(2.0, ui.visuals().selection.stroke.color));
    }
    if r.double_clicked() || (r.clicked() && ui.input(|i| i.modifiers.command)) {
        *act = Some(Act::Edit(b.id));
    }
}

/// A lane's row of beats, ending in a drop area that puts a beat at the end.
fn row(ui: &mut Ui, board: &Storyboard, lane: Option<u32>, pages: &std::collections::HashMap<u32, (usize, usize)>, act: &mut Option<Act>) {
    ui.horizontal_wrapped(|ui| {
        let beats = board.lane_beats(lane);
        for (i, b) in beats.iter().enumerate() {
            beat_card(ui, b, pages.get(&b.id), act, lane, i);
        }
        let (rect, resp) = ui.allocate_exact_size(vec2(80.0, CARD.y), egui::Sense::hover());
        let hovering = resp.dnd_hover_payload::<u32>().is_some();
        ui.painter().rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, if hovering { ui.visuals().selection.stroke.color } else { ui.visuals().weak_text_color() }),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "drop here", egui::FontId::proportional(11.0), ui.visuals().weak_text_color());
        if let Some(id) = resp.dnd_release_payload::<u32>() {
            *act = Some(Act::Run("beat.place", json!({"id": *id, "lane": lane})));
        }
    });
}

pub fn view(ed: &mut Editor, ui: &mut Ui) {
    let board = Storyboard::of_doc(&ed.session.doc);
    let pag = ed.paginated();
    let pages = wordsmith_script_model::outline::beat_pages(&ed.session.doc, &pag);
    let mut act: Option<Act> = None;
    ui.horizontal(|ui| {
        if ui.button("Add Lane").clicked() {
            act = Some(Act::Run("lane.add", json!({"name": format!("Lane {}", board.lanes.len() + 1)})));
        }
        egui::ComboBox::from_id_salt("outline_template").selected_text("Add a structure…").show_ui(ui, |ui| {
            for (name, label) in TEMPLATES {
                if ui.selectable_label(false, label).clicked() {
                    act = Some(Act::Run("outline.template", json!({"name": name})));
                }
            }
        });
        if ui.button("Add Beat").clicked() {
            act = Some(Act::Run("beat.add", json!({"lane": board.lanes.first().map(|l| l.id)})));
        }
        ui.label("Drag beats between lanes; double-click one to edit it.");
    });
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let n = board.lanes.len();
        for (i, lane) in board.lanes.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(f32::from(lane.indent) * 24.0);
                let mut name = lane.name.clone();
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(140.0).font(egui::TextStyle::Heading));
                if r.lost_focus() && name.trim() != lane.name && !name.trim().is_empty() {
                    act = Some(Act::Run("lane.rename", json!({"id": lane.id, "name": name})));
                }
                if ui.add_enabled(i > 0, egui::Button::new("↑")).on_hover_text("Move up").clicked() {
                    act = Some(Act::Run("lane.move", json!({"id": lane.id, "to": i - 1})));
                }
                if ui.add_enabled(i + 1 < n, egui::Button::new("↓")).on_hover_text("Move down").clicked() {
                    act = Some(Act::Run("lane.move", json!({"id": lane.id, "to": i + 1})));
                }
                if ui.add_enabled(lane.indent > 0, egui::Button::new("⇤")).on_hover_text("Outdent").clicked() {
                    act = Some(Act::Run("lane.indent", json!({"id": lane.id, "indent": lane.indent - 1})));
                }
                if ui.add_enabled(lane.indent < 2, egui::Button::new("⇥")).on_hover_text("Indent").clicked() {
                    act = Some(Act::Run("lane.indent", json!({"id": lane.id, "indent": lane.indent + 1})));
                }
                if ui.button("✕").on_hover_text("Delete the lane (its beats stay, in no lane)").clicked() {
                    act = Some(Act::Run("lane.delete", json!({"id": lane.id})));
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(f32::from(lane.indent) * 24.0);
                row(ui, &board, Some(lane.id), &pages, &mut act);
            });
            ui.separator();
        }
        ui.label(RichText::new("Beats in no lane").strong());
        row(ui, &board, None, &pages, &mut act);
        ui.separator();
        ui.label(RichText::new("Script").strong());
        let scenes = ed.scene_list();
        ui.horizontal_wrapped(|ui| {
            for (i, sc) in scenes.iter().enumerate() {
                let start = sc["start"].as_u64().unwrap_or(0) as usize;
                let page = pag
                    .map
                    .iter()
                    .position(|m| m.is_some_and(|m| m.block == start))
                    .and_then(|out| wordsmith_script_model::commands::character::page_of_output(&pag, out));
                let number = sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string);
                let text = format!("{number}  {}\n{}", sc["heading"].as_str().unwrap_or(""), page.map(|p| format!("page {p}")).unwrap_or_default());
                if ui.add(egui::Button::new(RichText::new(text).small()).min_size(vec2(150.0, 40.0))).on_hover_text("Go to the scene").clicked() {
                    ed.structure = None;
                    ed.run("scene.goto", json!({"scene": i}));
                    return;
                }
            }
        });
    });
    match act {
        Some(Act::Run(cmd, params)) => ed.run(cmd, params),
        Some(Act::Edit(id)) => {
            if let Some(b) = Storyboard::of_doc(&ed.session.doc).beat(id) {
                ed.board.selection.select(Some(jcj_board::CardId(id)));
                ed.board.edit = Some(crate::board::edit_of(b));
            }
        }
        None => {}
    }
    crate::board::beat_window(ed, ui.ctx());
}
