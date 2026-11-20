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
pub fn cards(ed: &mut Editor, ui: &mut Ui) {
    let scenes = ed.scene_list();
    let count = scenes.len();
    let mut action = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        if scenes.is_empty() {
            ui.label("The script has no scenes yet. Each Scene Heading starts one.");
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
            for (i, sc) in scenes.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(CARD.x);
                    let drag = ui.dnd_drag_source(Id::new(("scene-card", i)), i, |ui| {
                        egui::Frame::group(ui.style()).fill(ui.visuals().extreme_bg_color).show(ui, |ui| {
                            ui.set_min_size(CARD - vec2(12.0, 12.0));
                            ui.set_max_width(CARD.x - 12.0);
                            ui.label(
                                RichText::new(format!(
                                    "{}  {}",
                                    sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string),
                                    text_of(sc, "heading")
                                ))
                                .strong(),
                            );
                            let summary = text_of(sc, "summary");
                            if summary.is_empty() {
                                ui.label(RichText::new(text_of(sc, "firstAction")).weak());
                            } else {
                                ui.label(summary);
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
                    if ui.small_button("Open").clicked() {
                        action = Some(Act::Open(i));
                    }
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
