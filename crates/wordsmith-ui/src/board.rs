// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Story board view (decision 0028): beats as colored cards on a canvas with a dot grid, flow
//! lines between them, drag to move (snapping to the grid when it is on), double-click the canvas
//! to add a beat, drag from a beat's handle to another beat to link them, and a beat window where
//! Tab moves between the title and the body. Every change is a command.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};
use serde_json::json;
use wordsmith_script::storyboard::{BEAT_H, BEAT_W, Beat, PALETTE, Storyboard};

use crate::editor::Editor;

/// A beat being edited in the beat window.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatEdit {
    pub id: u32,
    pub title: String,
    pub body: String,
    /// Which field has the keyboard: the body, else the title.
    pub on_body: bool,
    focus_set: bool,
}

impl BeatEdit {
    pub fn of(b: &Beat) -> Self {
        BeatEdit { id: b.id, title: b.title.clone(), body: b.body.clone(), on_body: false, focus_set: false }
    }

    /// Tab: the other field.
    pub fn toggle(&mut self) {
        self.on_body = !self.on_body;
        self.focus_set = false;
    }
}

/// The board's own state in the editor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardState {
    /// The board (canvas) shown.
    pub current: u32,
    /// Beats added to the selection with Shift (besides `selected`).
    pub also: Vec<u32>,
    /// A board name being typed (new, or a rename of the current board).
    pub naming: Option<(bool, String)>,
    pub selected: Option<u32>,
    pub edit: Option<BeatEdit>,
    /// A beat being dragged: its id and how far it has moved.
    dragging: Option<(u32, Vec2)>,
    /// A link being drawn from this beat.
    linking: Option<u32>,
}

fn color(hex: &str) -> Color32 {
    wordsmith_script::revisions::parse_color(hex).map_or(Color32::from_rgb(255, 242, 168), |c| Color32::from_rgb(c.0, c.1, c.2))
}

fn beat_rect(origin: Pos2, b: &Beat) -> Rect {
    Rect::from_min_size(origin + vec2(b.x, b.y), vec2(b.w, b.h))
}

/// Where a line from `from` towards `to` leaves `r`'s edge.
fn edge_point(r: Rect, to: Pos2) -> Pos2 {
    let c = r.center();
    let d = to - c;
    if d.length() < 1.0 {
        return c;
    }
    let tx = if d.x.abs() > 1e-3 { (r.width() / 2.0) / d.x.abs() } else { f32::INFINITY };
    let ty = if d.y.abs() > 1e-3 { (r.height() / 2.0) / d.y.abs() } else { f32::INFINITY };
    c + d * tx.min(ty)
}

/// What to do after drawing.
enum Act {
    Run(&'static str, serde_json::Value),
    Select(Option<u32>),
    /// Add a beat to the selection, or take it out.
    Toggle(u32),
    Edit(u32),
}

/// Every selected beat: the main one and those added with Shift.
fn selection(ed: &Editor) -> Vec<u32> {
    let mut v: Vec<u32> = ed.board.selected.into_iter().collect();
    v.extend(ed.board.also.iter().copied().filter(|x| Some(*x) != ed.board.selected));
    v
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
                ed.board.selected = None;
                ed.board.also.clear();
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
    let mut board = all.clone();
    board.beats.retain(|b| b.board == ed.board.current);
    ui.horizontal(|ui| {
        if ui.button("Add Beat").clicked() {
            act.push(Act::Run("beat.add", json!({"board": ed.board.current})));
        }
        let mut snap = board.snap;
        if ui.checkbox(&mut snap, "Snap to grid").changed() {
            act.push(Act::Run("storyboard.configure", json!({"snap": snap})));
        }
        if let Some(id) = ed.board.selected {
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
                    for b in selection(ed) {
                        act.push(Act::Run("beat.update", json!({"id": b, "color": hex})));
                    }
                }
            }
            ui.menu_button("Arrange", |ui| {
                for (how, label) in [("grid", "On the Grid"), ("column", "In a Column"), ("row", "In a Row"), ("cascade", "Cascade")] {
                    if ui.button(label).clicked() {
                        act.push(Act::Run("beats.arrange", json!({"ids": selection(ed), "how": how})));
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Shift-click beats to arrange several");
            // This beat's links: how each is drawn, or remove it.
            let links: Vec<wordsmith_script::storyboard::Link> = board.links.iter().filter(|l| l.from == id || l.to == id).cloned().collect();
            if !links.is_empty() {
                ui.menu_button("Links", |ui| {
                    for l in links {
                        let other = if l.from == id { l.to } else { l.from };
                        let name = board.beat(other).map_or(String::new(), |b| b.title.clone());
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
    let extent = board.beats.iter().fold(vec2(1600.0, 1000.0), |e, b| vec2(e.x.max(b.x + b.w + 400.0), e.y.max(b.y + b.h + 400.0)));
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let (canvas, bg) = ui.allocate_exact_size(extent, Sense::click());
        let origin = canvas.min;
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 0.0, ui.visuals().extreme_bg_color);
        // The grid: a dot at every crossing in view.
        let clip = ui.clip_rect().intersect(canvas);
        let g = board.grid.max(5.0);
        let dot = ui.visuals().weak_text_color().gamma_multiply(0.5);
        let mut y = ((clip.min.y - origin.y) / g).floor() * g;
        while origin.y + y <= clip.max.y {
            let mut x = ((clip.min.x - origin.x) / g).floor() * g;
            while origin.x + x <= clip.max.x {
                painter.circle_filled(origin + vec2(x, y), 1.0, dot);
                x += g;
            }
            y += g;
        }
        // Where each beat is drawn now (a dragged one follows the pointer).
        let dragging = ed.board.dragging;
        let shown = |b: &Beat| {
            let r = beat_rect(origin, b);
            match dragging {
                Some((id, d)) if id == b.id => r.translate(d),
                _ => r,
            }
        };
        // Flow lines, under the beats.
        let line = Stroke::new(2.0, ui.visuals().text_color().gamma_multiply(0.6));
        for l in &board.links {
            let (Some(a), Some(b)) = (board.beat(l.from), board.beat(l.to)) else { continue };
            let (ra, rb) = (shown(a), shown(b));
            let (p, q) = (edge_point(ra, rb.center()), edge_point(rb, ra.center()));
            let stroke = if l.color.is_empty() { line } else { Stroke::new(2.0, color(&l.color)) };
            painter.line_segment([p, q], stroke);
            let head = |tip: Pos2, from: Pos2| {
                let dir = (tip - from).normalized();
                let side = vec2(-dir.y, dir.x);
                egui::Shape::convex_polygon(vec![tip, tip - dir * 10.0 + side * 5.0, tip - dir * 10.0 - side * 5.0], stroke.color, Stroke::NONE)
            };
            if matches!(l.style.as_str(), "forward" | "both") {
                painter.add(head(q, p));
            }
            if matches!(l.style.as_str(), "back" | "both") {
                painter.add(head(p, q));
            }
        }
        let pointer = ui.ctx().pointer_interact_pos();
        for b in &board.beats {
            let r = shown(b);
            let resp = ui.interact(r, Id::new(("beat", b.id)), Sense::click_and_drag());
            let selected = ed.board.selected == Some(b.id) || ed.board.also.contains(&b.id);
            painter.rect_filled(r, 6.0, color(&b.color));
            painter.rect_stroke(
                r,
                6.0,
                Stroke::new(
                    if selected { 2.5 } else { 1.0 },
                    if selected { ui.visuals().selection.stroke.color } else { Color32::from_black_alpha(60) },
                ),
                egui::StrokeKind::Inside,
            );
            let ink = Color32::from_gray(20);
            let title = if b.title.is_empty() { "Untitled beat" } else { b.title.as_str() };
            let tg = painter.layout(title.to_string(), FontId::proportional(14.0), ink, r.width() - 16.0);
            let title_h = tg.size().y;
            painter.galley(r.min + vec2(8.0, 6.0), tg, ink);
            let body = painter.layout(b.body.clone(), FontId::proportional(12.0), ink, r.width() - 16.0);
            let body_rect = Rect::from_min_max(r.min + vec2(8.0, 10.0 + title_h), r.max - vec2(8.0, 6.0));
            ui.painter_at(body_rect).galley(body_rect.min, body, ink);
            // The link handle.
            let handle = pos2(r.max.x, r.center().y);
            painter.circle_filled(handle, 5.0, ui.visuals().selection.stroke.color);
            let hresp = ui.interact(Rect::from_center_size(handle, vec2(14.0, 14.0)), Id::new(("beat-link", b.id)), Sense::drag());
            if hresp.drag_started() {
                ed.board.linking = Some(b.id);
            }
            if resp.clicked() {
                if ui.input(|i| i.modifiers.shift) {
                    act.push(Act::Toggle(b.id));
                } else {
                    act.push(Act::Select(Some(b.id)));
                }
            }
            if resp.double_clicked() {
                act.push(Act::Edit(b.id));
            }
            if resp.drag_started() && ed.board.linking.is_none() {
                ed.board.dragging = Some((b.id, Vec2::ZERO));
                act.push(Act::Select(Some(b.id)));
            }
            if resp.dragged()
                && let Some((id, d)) = ed.board.dragging.as_mut()
                && *id == b.id
            {
                *d += resp.drag_delta();
            }
            if resp.drag_stopped()
                && let Some((id, d)) = ed.board.dragging.take()
                && id == b.id
            {
                act.push(Act::Run("beat.update", json!({"id": id, "x": (b.x + d.x).max(0.0), "y": (b.y + d.y).max(0.0)})));
            }
        }
        // A link being drawn, and where it lands.
        if let (Some(from), Some(p)) = (ed.board.linking, pointer) {
            if let Some(a) = board.beat(from) {
                painter.line_segment([shown(a).center(), p], line);
            }
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(target) = board.beats.iter().find(|b| b.id != from && shown(b).contains(p)) {
                    act.push(Act::Run("beat.link", json!({"from": from, "to": target.id})));
                }
                ed.board.linking = None;
            }
        }
        if bg.double_clicked()
            && let Some(p) = pointer
        {
            let at = p - origin;
            act.push(Act::Run(
                "beat.add",
                json!({"board": ed.board.current, "x": (at.x - BEAT_W / 2.0).max(0.0), "y": (at.y - BEAT_H / 2.0).max(0.0)}),
            ));
        } else if bg.clicked() {
            act.push(Act::Select(None));
        }
    });
    // Delete removes the selected beat when no field has the keyboard.
    if let Some(id) = ed.board.selected
        && ed.board.edit.is_none()
        && !ui.ctx().text_edit_focused()
        && ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
    {
        act.push(Act::Run("beat.delete", json!({"id": id})));
        act.push(Act::Select(None));
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
                    ed.board.selected = Some(b.id);
                    ed.board.edit = Some(BeatEdit::of(b));
                }
            }
            Act::Select(id) => {
                ed.board.selected = id;
                ed.board.also.clear();
            }
            Act::Toggle(id) => {
                if ed.board.selected == Some(id) {
                    ed.board.selected = ed.board.also.pop();
                } else if let Some(i) = ed.board.also.iter().position(|x| *x == id) {
                    ed.board.also.remove(i);
                } else if ed.board.selected.is_none() {
                    ed.board.selected = Some(id);
                } else {
                    ed.board.also.push(id);
                }
            }
            Act::Edit(id) => {
                if let Some(b) = Storyboard::of_doc(&ed.session.doc).beat(id) {
                    ed.board.selected = Some(id);
                    ed.board.edit = Some(BeatEdit::of(b));
                }
            }
        }
    }
    beat_window(ed, ui.ctx());
}

/// `body` with a new list line: a bullet, or number `n`.
pub fn with_list_line(body: &str, n: Option<usize>) -> String {
    let mut out = body.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    match n {
        Some(n) => out.push_str(&format!("{n}. ")),
        None => out.push_str("• "),
    }
    out
}

/// The number after the last numbered line of `body` (1 when there is none).
pub fn next_number(body: &str) -> usize {
    body.lines().rev().find_map(|l| l.split_once(". ").and_then(|(n, _)| n.trim().parse::<usize>().ok())).map_or(1, |n| n + 1)
}

/// The beat window: title and body, Tab between them, Save or Cancel.
pub(crate) fn beat_window(ed: &mut Editor, ctx: &egui::Context) {
    let Some(mut e) = ed.board.edit.take() else { return };
    let mut open = true;
    let mut save = false;
    egui::Window::new("Beat").open(&mut open).collapsible(false).resizable(false).anchor(Align2::RIGHT_TOP, [-16.0, 60.0]).show(ctx, |ui| {
        // Tab moves between the title and the body (taken before the fields see it).
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
            e.toggle();
        }
        ui.label("Title");
        let t = ui.add(egui::TextEdit::singleline(&mut e.title).desired_width(300.0));
        ui.label("Body");
        let b = ui.add(egui::TextEdit::multiline(&mut e.body).desired_rows(6).desired_width(300.0).lock_focus(true));
        if !e.focus_set {
            if e.on_body {
                b.request_focus()
            } else {
                t.request_focus()
            }
            e.focus_set = true;
        }
        ui.horizontal(|ui| {
            // Simple lists: each button starts a new list line in the body.
            if ui.small_button("• List").clicked() {
                e.body = with_list_line(&e.body, None);
            }
            if ui.small_button("1. List").clicked() {
                let n = next_number(&e.body);
                e.body = with_list_line(&e.body, Some(n));
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                save = true;
            }
            ui.label("Tab: title / body");
        });
    });
    if save {
        ed.run("beat.update", json!({"id": e.id, "title": e.title, "body": e.body}));
        return;
    }
    if open {
        ed.board.edit = Some(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_lines_number_on() {
        assert_eq!(with_list_line("", None), "• ");
        assert_eq!(with_list_line("1. Rain\n", Some(next_number("1. Rain\n"))), "1. Rain\n2. ");
        assert_eq!(next_number("Notes"), 1);
    }

    #[test]
    fn tab_moves_between_title_and_body() {
        let mut e = BeatEdit::of(&Beat { id: 3, title: "T".into(), ..Beat::default() });
        assert!(!e.on_body);
        e.toggle();
        assert!(e.on_body);
        e.toggle();
        assert!(!e.on_body);
    }

    #[test]
    fn link_lines_leave_from_the_edges() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 50.0));
        assert_eq!(edge_point(r, pos2(500.0, 25.0)), pos2(100.0, 25.0));
        assert_eq!(edge_point(r, pos2(50.0, -500.0)), pos2(50.0, 0.0));
    }
}
