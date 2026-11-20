// SPDX-License-Identifier: MIT OR Apache-2.0
//! The structure views (SPEC: index cards, one per scene, reordered to reorder the script; the
//! scene view, scenes as movable blocks of script text). Both show the one script and change it
//! only through the scene commands, so undo puts a move back (decision 0022).
//!
//! Dragging a card or a scene onto another moves it there: before the one dropped on when it
//! came from later in the script, after it when it came from earlier. "Open" goes to the scene in
//! the script view.

use egui::{Id, RichText, Ui, vec2};
use serde_json::{Value, json};
use wordsmith_script::{ElementKind, Elements};

use crate::editor::Editor;

/// Which structure view shows instead of the script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Structure {
    Cards,
    Scenes,
    /// The story board (decision 0028).
    Board,
    /// The outline editor's lanes (decision 0028).
    Outline,
}

/// A card's text: the first few lines' worth, cut at a word.
fn preview(text: &str) -> String {
    const MAX: usize = 110;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let cut: String = text.chars().take(MAX).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end())
}

/// A card's heading: two lines' worth at most.
fn heading_preview(text: &str) -> String {
    if text.chars().count() <= 52 {
        return text.to_string();
    }
    format!("{}…", text.chars().take(50).collect::<String>().trim_end())
}

/// A card's size, points.
pub const CARD: egui::Vec2 = vec2(230.0, 150.0);

/// The scene a drop on scene `onto` moves scene `from` before (`None`: to the end).
pub fn drop_target(from: usize, onto: usize, count: usize) -> Option<usize> {
    if from < onto {
        // Dragged later: it goes after the scene dropped on.
        (onto + 1 < count).then_some(onto + 1)
    } else {
        Some(onto)
    }
}

/// What to do after drawing: move a scene, or open one.
enum Act {
    Move { from: usize, onto: usize },
    Open(usize),
}

fn text_of(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn act(ed: &mut Editor, action: Option<Act>, count: usize) {
    match action {
        Some(Act::Move { from, onto }) if from != onto => {
            let before = drop_target(from, onto, count);
            ed.run("scene.move", json!({"scene": from, "before": before}));
        }
        Some(Act::Open(i)) => {
            ed.structure = None;
            ed.run("scene.goto", json!({"scene": i}));
        }
        _ => {}
    }
}

/// The index cards: a card per scene with its heading, its summary (or, in grey, the start of its
/// first action) and who speaks in it.
/// How the index cards show and print.
#[derive(Clone, Debug, PartialEq)]
pub struct CardPrefs {
    /// Cards per row, 1 to 9; 0 fits as many as the window takes.
    pub per_row: usize,
    /// Show the scene's text instead of its summary.
    pub script_text: bool,
    /// Printing: one 3 × 5 in card per page, or a grid on Letter.
    pub stock: bool,
    pub columns: usize,
    pub rows: usize,
    pub frames: bool,
    pub grey: bool,
    /// The print options are showing.
    pub printing: bool,
}

impl Default for CardPrefs {
    fn default() -> Self {
        CardPrefs { per_row: 0, script_text: false, stock: false, columns: 2, rows: 3, frames: true, grey: false, printing: false }
    }
}

fn cards_bar(ed: &mut Editor, ui: &mut Ui) {
    let p = &mut ed.tools.cards;
    ui.horizontal_wrapped(|ui| {
        ui.label("Cards per row");
        egui::ComboBox::from_id_salt("per-row")
            .selected_text(if p.per_row == 0 { "Fit".to_string() } else { p.per_row.to_string() })
            .width(60.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut p.per_row, 0, "Fit");
                for n in 1..=9 {
                    ui.selectable_value(&mut p.per_row, n, n.to_string());
                }
            });
        ui.selectable_value(&mut p.script_text, false, "Summaries");
        ui.selectable_value(&mut p.script_text, true, "Script text");
        ui.separator();
        if ui.button("Print Cards…").clicked() {
            p.printing = !p.printing;
        }
    });
    if !p.printing {
        return;
    }
    let mut go = false;
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut p.stock, true, "3 × 5 cards");
        ui.selectable_value(&mut p.stock, false, "Grid on Letter");
        if !p.stock {
            ui.add(egui::DragValue::new(&mut p.columns).range(1..=6).prefix("Columns "));
            ui.add(egui::DragValue::new(&mut p.rows).range(1..=6).prefix("Rows "));
        }
        ui.checkbox(&mut p.frames, "Frames");
        ui.checkbox(&mut p.grey, "Grey background");
        if ui.button("Save PDF…").clicked() {
            go = true;
        }
    });
    if go {
        let params =
            json!({"stock": if p.stock { "card" } else { "grid" }, "columns": p.columns, "rows": p.rows, "frames": p.frames, "grey": p.grey});
        ed.dialog_params = Some(params.clone());
        ed.run("cards.exportPdf", params);
    }
}

pub fn cards(ed: &mut Editor, ui: &mut Ui) {
    let scenes = ed.scene_list();
    let count = scenes.len();
    let mut action = None;
    cards_bar(ed, ui);
    let per_row = ed.tools.cards.per_row;
    let script_text = ed.tools.cards.script_text;
    let card = if per_row == 0 { CARD } else { vec2(((ui.available_width() - 24.0) / per_row as f32 - 12.0).max(120.0), CARD.y) };
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        if scenes.is_empty() {
            ui.label("The script has no scenes yet. Each Scene Heading starts one.");
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
            for (i, sc) in scenes.iter().enumerate() {
                // A fixed allocation per card, so the row knows when to wrap.
                ui.allocate_ui_with_layout(vec2(card.x, card.y + 32.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(card.x);
                    let drag = ui.dnd_drag_source(Id::new(("scene-card", i)), i, |ui| {
                        egui::Frame::group(ui.style()).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
                            // Every card the same size: a long heading or summary is cut short.
                            ui.set_min_size(card - vec2(12.0, 12.0));
                            ui.set_max_size(card - vec2(12.0, 12.0));
                            // The scene's color as a band along the top.
                            if let Some(hex) =
                                wordsmith_script::scene_data::SCENE_COLORS.iter().find(|(n, _)| *n == text_of(sc, "color")).map(|c| c.1)
                                && let Some(c) = wordsmith_script::revisions::parse_color(hex)
                            {
                                let r = ui.max_rect();
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(r.min - vec2(6.0, 6.0), vec2(r.width() + 12.0, 4.0)),
                                    0.0,
                                    egui::Color32::from_rgb(c.0, c.1, c.2),
                                );
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{}  {}",
                                    sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string),
                                    heading_preview(&text_of(sc, "heading"))
                                ))
                                .strong(),
                            );
                            let title = text_of(sc, "title");
                            if !title.is_empty() {
                                ui.label(RichText::new(title).italics());
                            }
                            let summary = if script_text { String::new() } else { text_of(sc, "summary") };
                            if summary.is_empty() {
                                ui.label(RichText::new(preview(&text_of(sc, "firstAction"))).weak());
                            } else {
                                ui.label(preview(&summary));
                            }
                            let who: Vec<&str> = sc
                                .get("characters")
                                .and_then(Value::as_array)
                                .map(|a| a.iter().filter_map(Value::as_str).collect())
                                .unwrap_or_default();
                            if !who.is_empty() {
                                ui.label(RichText::new(who.join(", ")).small());
                            }
                        });
                    });
                    let r = &drag.response;
                    if let Some(from) = r.dnd_release_payload::<usize>() {
                        action = Some(Act::Move { from: *from, onto: i });
                    } else if r.dnd_hover_payload::<usize>().is_some_and(|f| *f != i) {
                        ui.painter().rect_stroke(r.rect, 4.0, egui::Stroke::new(2.0, ui.visuals().selection.stroke.color), egui::StrokeKind::Outside);
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("Open").clicked() {
                            action = Some(Act::Open(i));
                        }
                        if ui.small_button("Properties").clicked() {
                            ed.session.push_ui_request(json!({"open": "tool.sceneProperties", "scene": i}));
                        }
                    });
                });
            }
        });
    });
    act(ed, action, count);
}

/// The scene view: each scene as a block of its text, roughly as the page lays it out. Only the
/// blocks in sight are laid out, so a long script stays quick.
pub fn scene_view(ed: &mut Editor, ui: &mut Ui) {
    let scenes = ed.scene_list();
    let count = scenes.len();
    let doc = ed.session.doc.clone();
    let elements = Elements::of_doc(&doc);
    let row = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
    // Each block: a heading row, its paragraphs' lines and a gap.
    let heights: Vec<f32> = scenes
        .iter()
        .map(|sc| {
            let (a, b) = (sc["start"].as_u64().unwrap_or(0) as usize, sc["end"].as_u64().unwrap_or(0) as usize);
            let lines: usize = (a + 1..b).filter_map(|i| doc.body.get(i)?.as_para()).map(|p| p.text.split('\n').count() + 1).sum();
            (lines as f32 + 3.0) * row
        })
        .collect();
    let total: f32 = heights.iter().sum();
    let mut action = None;
    egui::ScrollArea::vertical().auto_shrink(false).show_viewport(ui, |ui, viewport| {
        ui.set_height(total);
        let mut top = 0.0;
        for (i, (sc, h)) in scenes.iter().zip(&heights).enumerate() {
            if top + h < viewport.min.y || top > viewport.max.y {
                top += h;
                continue;
            }
            let rect = egui::Rect::from_min_size(ui.min_rect().min + vec2(0.0, top), vec2(ui.available_width().max(400.0), *h));
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                let drag = ui.dnd_drag_source(Id::new(("scene-block", i)), i, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}  {}", i + 1, text_of(sc, "heading"))).monospace().strong());
                    });
                });
                if let Some(from) = drag.response.dnd_release_payload::<usize>() {
                    action = Some(Act::Move { from: *from, onto: i });
                } else if drag.response.dnd_hover_payload::<usize>().is_some_and(|f| *f != i) {
                    ui.painter().hline(rect.x_range(), rect.top(), egui::Stroke::new(2.0, ui.visuals().selection.stroke.color));
                }
                if ui.small_button("Open").clicked() {
                    action = Some(Act::Open(i));
                }
                let (a, b) = (sc["start"].as_u64().unwrap_or(0) as usize, sc["end"].as_u64().unwrap_or(0) as usize);
                for p in (a + 1..b).filter_map(|j| doc.body.get(j)?.as_para()) {
                    let kind = elements.of_para(p).kind;
                    let indent = match kind {
                        ElementKind::Character => 22,
                        ElementKind::Parenthetical => 16,
                        ElementKind::Dialogue => 10,
                        ElementKind::Transition => 44,
                        _ => 0,
                    };
                    let colour = if kind.is_outline() { ui.visuals().weak_text_color() } else { ui.visuals().text_color() };
                    for line in p.text.split('\n') {
                        ui.label(RichText::new(format!("{}{line}", " ".repeat(indent))).monospace().color(colour));
                    }
                }
            });
            top += h;
        }
    });
    act(ed, action, count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_land_where_the_writer_expects() {
        // Dragged later: after the card dropped on; earlier: before it.
        assert_eq!(drop_target(0, 2, 4), Some(3));
        assert_eq!(drop_target(0, 3, 4), None, "after the last: to the end");
        assert_eq!(drop_target(3, 1, 4), Some(1));
    }
}
