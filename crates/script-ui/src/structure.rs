// SPDX-License-Identifier: MIT OR Apache-2.0
//! The structure views (SPEC: index cards, one per scene, reordered to reorder the script; the
//! scene view, scenes as movable blocks of script text). Both show the one script and change it
//! only through the scene commands, so undo puts a move back (decision 0022). The index cards
//! are the core's corkboard (`jcj_board::Corkboard`).
//!
//! Dragging a card or a scene onto another moves it there: before the one dropped on when it
//! came from later in the script, after it when it came from earlier. "Open" (or a double-click
//! on a card) goes to the scene in the script view.

use egui::{Id, RichText, Ui, vec2};
use jcj_board::{Board, BoardEvent, Card, CardId, drop_target};
use serde_json::{Value, json};
use wordsmith_script_model::{ElementKind, Elements};

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

/// What to do after drawing: move a scene (dropped on another, or to a place in the order), or
/// open one.
enum Act {
    Move { from: usize, onto: usize },
    Reorder { from: usize, before: Option<usize> },
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
        Some(Act::Reorder { from, before }) => {
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

/// A pale tint of `hex` for a card (cards are pale, so their dark ink reads), or white.
fn card_tint(hex: Option<&str>) -> String {
    let Some(c) = hex.and_then(wordsmith_script_model::revisions::parse_color) else { return "#FFFFFF".into() };
    let mix = |v: u8| (f32::from(v) * 0.45 + 255.0 * 0.55).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", mix(c.0), mix(c.1), mix(c.2))
}

/// The scenes as the corkboard's cards: the number and heading, the title, the summary (or the
/// start of the scene's text) and who speaks in it.
pub fn scene_cards(scenes: &[Value], script_text: bool) -> Board {
    let cards = scenes
        .iter()
        .enumerate()
        .map(|(i, sc)| {
            let number = sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string);
            let summary = if script_text { String::new() } else { text_of(sc, "summary") };
            let text = if summary.is_empty() { text_of(sc, "firstAction") } else { summary };
            let who: Vec<&str> =
                sc.get("characters").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            let body = [text_of(sc, "title"), jcj_board::text::preview(&text, 110), who.join(", ")]
                .into_iter()
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            let color = wordsmith_script_model::scene_data::SCENE_COLORS.iter().find(|(n, _)| *n == text_of(sc, "color")).map(|c| c.1);
            Card {
                id: CardId(u32::try_from(i).unwrap_or(u32::MAX)),
                title: format!("{number}  {}", text_of(sc, "heading")),
                body,
                color: card_tint(color),
                ..Card::default()
            }
        })
        .collect();
    Board { cards, ..Board::default() }
}

pub fn cards(ed: &mut Editor, ui: &mut Ui) {
    let scenes = ed.scene_list();
    let count = scenes.len();
    let mut action = None;
    cards_bar(ed, ui);
    let selected = ed.cards_selection.primary().map(|c| c.0 as usize).filter(|i| *i < count);
    ui.horizontal(|ui| {
        if ui.add_enabled(selected.is_some(), egui::Button::new("Open")).on_hover_text("Go to the scene (or double-click its card)").clicked() {
            action = selected.map(Act::Open);
        }
        if ui.add_enabled(selected.is_some(), egui::Button::new("Properties")).on_hover_text("The scene's properties").clicked()
            && let Some(i) = selected
        {
            ed.session.push_ui_request(json!({"open": "tool.sceneProperties", "scene": i}));
        }
    });
    let board = scene_cards(&scenes, ed.tools.cards.script_text);
    let events = jcj_board::Corkboard::new("index-cards")
        .per_row(ed.tools.cards.per_row)
        .empty_hint("The script has no scenes yet. Each Scene Heading starts one.")
        .show(ui, &board, &ed.cards_selection);
    for e in events {
        match e {
            BoardEvent::Select(id) => ed.cards_selection.select(id),
            BoardEvent::Toggle(id) => ed.cards_selection.toggle(id),
            BoardEvent::Open(id) => action = Some(Act::Open(id.0 as usize)),
            BoardEvent::Reorder { from, before } => action = Some(Act::Reorder { from, before }),
            _ => {}
        }
    }
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
                let drag = ui.dnd_drag_source(Id::unique(("scene-block", i)), i, |ui| {
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
