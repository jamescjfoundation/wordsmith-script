// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Story board view (decision 0028): the script's beats as cards on the core's canvas
//! (`jcj_board::Canvas`): a dot grid, flow lines between beats, drag to move (snapping to the
//! grid when it is on), double-click the canvas to add a beat, drag from a beat's handle to
//! another beat to link them, and the card window where Tab moves between the title and the
//! body. The beats live in the script (`wordsmith_script::storyboard`), so every change is a
//! command, and a beat can be sent to the script as outline paragraphs.

use egui::{Color32, Sense, Ui, vec2};
use jcj_board::{Board, BoardEvent, CanvasState, Card, CardEdit, CardId, Link, LinkStyle, Selection};
use serde_json::json;
use wordsmith_script::storyboard::{BEAT_H, BEAT_W, PALETTE, Storyboard};

use crate::editor::Editor;

/// The board's own state in the editor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardState {
    /// The board (canvas) shown.
    pub current: u32,
    /// A board name being typed (new, or a rename of the current board).
    pub naming: Option<(bool, String)>,
    /// The beats selected.
    pub selection: Selection,
    /// The beat open in the card window.
    pub edit: Option<CardEdit>,
    /// A drag or a link under way.
    pub canvas: CanvasState,
}

impl BoardState {
    /// The main selected beat.
    pub fn selected(&self) -> Option<u32> {
        self.selection.primary().map(|c| c.0)
    }

    /// Every selected beat, the main one first.
    pub fn ids(&self) -> Vec<u32> {
        self.selection.ids().into_iter().map(|c| c.0).collect()
    }
}

fn color(hex: &str) -> Color32 {
    jcj_board::card_color(hex)
}

fn style_of(s: &str) -> LinkStyle {
    match s {
        "forward" => LinkStyle::Forward,
        "back" => LinkStyle::Back,
        "both" => LinkStyle::Both,
        _ => LinkStyle::Line,
    }
}

/// Board `current` of the script's story board, as the canvas draws it.
pub fn board_of(sb: &Storyboard, current: u32) -> Board {
    let cards = sb
        .beats
        .iter()
        .filter(|b| b.board == current)
        .map(|b| Card {
            id: CardId(b.id),
            title: b.title.clone(),
            body: b.body.clone(),
            color: b.color.clone(),
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
            ..Card::default()
        })
        .collect::<Vec<_>>();
    let links = sb
        .links
        .iter()
        .filter(|l| cards.iter().any(|c| c.id.0 == l.from) && cards.iter().any(|c| c.id.0 == l.to))
        .map(|l| Link { from: CardId(l.from), to: CardId(l.to), style: style_of(&l.style), color: l.color.clone() })
        .collect();
    Board { cards, links, grid: sb.grid, snap: sb.snap }
}

/// What to do after drawing.
enum Act {
    Run(&'static str, serde_json::Value),
    Select(Option<u32>),
    /// Add a beat to the selection, or take it out.
    Toggle(u32),
    Edit(u32),
}

/// The board tabs: pick a board, add, rename or delete one, and export it as PDF.
fn board_bar(ed: &mut Editor, ui: &mut Ui, board: &Storyboard, act: &mut Vec<Act>) {
    if !board.boards.iter().any(|b| b.id == ed.board.current) {
        ed.board.current = board.boards.first().map_or(0, |b| b.id);
    }
    ui.horizontal_wrapped(|ui| {
        for b in &board.boards {
            if ui.selectable_label(ed.board.current == b.id, b.name.as_str()).clicked() {
                ed.board.current = b.id;
                ed.board.selection.select(None);
            }
        }
        if ui.small_button("+").on_hover_text("New board").clicked() {
            ed.board.naming = Some((true, String::new()));
        }
        if ui.small_button("Rename").clicked() {
            let name = board.boards.iter().find(|b| b.id == ed.board.current).map(|b| b.name.clone()).unwrap_or_default();
            ed.board.naming = Some((false, name));
        }
        if board.boards.len() > 1 && ui.small_button("Delete Board").clicked() {
            act.push(Act::Run("storyboard.deleteBoard", json!({"id": ed.board.current})));
        }
        ui.separator();
        if ui.button("Export PDF…").clicked() {
            ed.dialog_params = Some(json!({"board": ed.board.current}));
            act.push(Act::Run("storyboard.exportPdf", json!({"board": ed.board.current})));
        }
    });
    let mut done = None;
    if let Some((new, name)) = ed.board.naming.as_mut() {
        ui.horizontal(|ui| {
            ui.label(if *new { "New board" } else { "Rename board" });
            let r = ui.text_edit_singleline(name);
            if ui.button("OK").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                done = Some(true);
            }
            if ui.button("Cancel").clicked() {
                done = Some(false);
            }
        });
    }
    if let Some(ok) = done
        && let Some((new, name)) = ed.board.naming.take()
        && ok
    {
        if new {
            act.push(Act::Run("storyboard.addBoard", json!({"name": name})));
        } else {
            act.push(Act::Run("storyboard.renameBoard", json!({"id": ed.board.current, "name": name})));
        }
    }
}

pub fn view(ed: &mut Editor, ui: &mut Ui) {
    let all = Storyboard::of_doc(&ed.session.doc);
    let mut act: Vec<Act> = Vec::new();
    board_bar(ed, ui, &all, &mut act);
    // Only the current board's beats are drawn.
    let board = board_of(&all, ed.board.current);
    ui.horizontal(|ui| {
        if ui.button("Add Beat").clicked() {
            act.push(Act::Run("beat.add", json!({"board": ed.board.current})));
        }
        let mut snap = all.snap;
        if ui.checkbox(&mut snap, "Snap to grid").changed() {
            act.push(Act::Run("storyboard.configure", json!({"snap": snap})));
        }
        if let Some(id) = ed.board.selected() {
            ui.separator();
            if ui.button("Edit").clicked() {
                act.push(Act::Edit(id));
            }
            if ui.button("Send to Script").on_hover_text("Its title as Outline 1 and its body as Summary, at the caret").clicked() {
                act.push(Act::Run("beat.toScript", json!({"id": id})));
            }
            if ui.button("Delete").clicked() {
                act.push(Act::Run("beat.delete", json!({"id": id})));
                act.push(Act::Select(None));
            }
            for hex in PALETTE {
                let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                ui.painter().rect_filled(r, 3.0, color(hex));
                if resp.clicked() {
                    for b in ed.board.ids() {
                        act.push(Act::Run("beat.update", json!({"id": b, "color": hex})));
                    }
                }
            }
            ui.menu_button("Arrange", |ui| {
                for (how, label) in [("grid", "On the Grid"), ("column", "In a Column"), ("row", "In a Row"), ("cascade", "Cascade")] {
                    if ui.button(label).clicked() {
                        act.push(Act::Run("beats.arrange", json!({"ids": ed.board.ids(), "how": how})));
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Shift-click beats to arrange several");
            // This beat's links: how each is drawn, or remove it.
            let links: Vec<wordsmith_script::storyboard::Link> = all.links.iter().filter(|l| l.from == id || l.to == id).cloned().collect();
            if !links.is_empty() {
                ui.menu_button("Links", |ui| {
                    for l in links {
                        let other = if l.from == id { l.to } else { l.from };
                        let name = all.beat(other).map_or(String::new(), |b| b.title.clone());
                        ui.horizontal(|ui| {
                            ui.label(format!("{} {name}", if l.from == id { "→" } else { "←" }));
                            for (st, label) in [("line", "—"), ("forward", "→"), ("back", "←"), ("both", "↔")] {
                                if ui.selectable_label(l.style == st, label).clicked() {
                                    act.push(Act::Run("beat.styleLink", json!({"from": l.from, "to": l.to, "style": st})));
                                }
                            }
                            for hex in ["", "#C0392B", "#1F5FA8", "#2E7D32"] {
                                let c = if hex.is_empty() { Color32::GRAY } else { color(hex) };
                                let (r, resp) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
                                ui.painter().rect_filled(r, 2.0, c);
                                if resp.clicked() {
                                    act.push(Act::Run("beat.styleLink", json!({"from": l.from, "to": l.to, "color": hex})));
                                }
                            }
                            if ui.small_button("Remove").clicked() {
                                act.push(Act::Run("beat.unlink", json!({"from": l.from, "to": l.to})));
                            }
                        });
                    }
                });
            }
        }
        ui.label("Double-click to add a beat; drag the dot on a beat's edge to another beat to link them.");
    });
    // The card window has the keyboard while it is open: Delete there edits its text.
    let canvas = jcj_board::Canvas::new("story-board")
        .delete_key(ed.board.edit.is_none())
        .empty_hint("Double-click to add a beat; drag the dot on a beat's edge to another beat to link them.");
    let events = canvas.show(ui, &board, &ed.board.selection, &mut ed.board.canvas);
    for e in events {
        match e {
            BoardEvent::Select(id) => act.push(Act::Select(id.map(|c| c.0))),
            BoardEvent::Toggle(id) => act.push(Act::Toggle(id.0)),
            BoardEvent::Open(id) => act.push(Act::Edit(id.0)),
            BoardEvent::Moved { id, x, y } => act.push(Act::Run("beat.update", json!({"id": id.0, "x": x, "y": y}))),
            BoardEvent::AddAt { x, y } => {
                act.push(Act::Run("beat.add", json!({"board": ed.board.current, "x": (x - BEAT_W / 2.0).max(0.0), "y": (y - BEAT_H / 2.0).max(0.0)})))
            }
            BoardEvent::Link { from, to } => act.push(Act::Run("beat.link", json!({"from": from.0, "to": to.0}))),
            BoardEvent::Delete(ids) => {
                for id in ids {
                    act.push(Act::Run("beat.delete", json!({"id": id.0})));
                }
                act.push(Act::Select(None));
            }
            BoardEvent::Reorder { .. } => {}
        }
    }
    for a in act {
        match a {
            Act::Run(cmd, params) => {
                let added = cmd == "beat.add";
                if cmd == "storyboard.addBoard"
                    && let Ok(v) = ed.session.run(cmd, &params)
                {
                    ed.board.current = v["id"].as_u64().and_then(|x| u32::try_from(x).ok()).unwrap_or(0);
                    continue;
                }
                let before = ed.session.rev();
                ed.run(cmd, params.clone());
                // A new beat is selected and opened for its title.
                if added
                    && ed.session.rev() != before
                    && let Some(b) = Storyboard::of_doc(&ed.session.doc).beats.last()
                {
                    ed.board.selection.select(Some(CardId(b.id)));
                    ed.board.edit = Some(edit_of(b));
                }
            }
            Act::Select(id) => ed.board.selection.select(id.map(CardId)),
            Act::Toggle(id) => ed.board.selection.toggle(CardId(id)),
            Act::Edit(id) => {
                if let Some(b) = Storyboard::of_doc(&ed.session.doc).beat(id) {
                    ed.board.selection.select(Some(CardId(id)));
                    ed.board.edit = Some(edit_of(b));
                }
            }
        }
    }
    beat_window(ed, ui.ctx());
}

/// Beat `b` opened in the card window, in its title.
pub(crate) fn edit_of(b: &wordsmith_script::storyboard::Beat) -> CardEdit {
    CardEdit::of(&Card { id: CardId(b.id), title: b.title.clone(), body: b.body.clone(), ..Card::default() })
}

/// The beat window (the core's card window): title and body, Tab between them, Save.
pub(crate) fn beat_window(ed: &mut Editor, ctx: &egui::Context) {
    if let Some(e) = jcj_board::card_window(ctx, &mut ed.board.edit) {
        ed.run("beat.update", json!({"id": e.id.0, "title": e.title, "body": e.body}));
    }
}
