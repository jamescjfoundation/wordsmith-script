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
    Edit(u32),
}

pub fn view(ed: &mut Editor, ui: &mut Ui) {
    let board = Storyboard::of_doc(&ed.session.doc);
    let mut act: Vec<Act> = Vec::new();
    ui.horizontal(|ui| {
        if ui.button("Add Beat").clicked() {
            act.push(Act::Run("beat.add", json!({})));
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
                    act.push(Act::Run("beat.update", json!({"id": id, "color": hex})));
                }
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
            painter.line_segment([p, q], line);
            let dir = (q - p).normalized();
            let side = vec2(-dir.y, dir.x);
            painter.add(egui::Shape::convex_polygon(vec![q, q - dir * 10.0 + side * 5.0, q - dir * 10.0 - side * 5.0], line.color, Stroke::NONE));
        }
        let pointer = ui.ctx().pointer_interact_pos();
        for b in &board.beats {
            let r = shown(b);
            let resp = ui.interact(r, Id::new(("beat", b.id)), Sense::click_and_drag());
            let selected = ed.board.selected == Some(b.id);
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
                act.push(Act::Select(Some(b.id)));
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
            act.push(Act::Run("beat.add", json!({"x": (at.x - BEAT_W / 2.0).max(0.0), "y": (at.y - BEAT_H / 2.0).max(0.0)})));
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
            Act::Select(id) => ed.board.selected = id,
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

/// The beat window: title and body, Tab between them, Save or Cancel.
fn beat_window(ed: &mut Editor, ctx: &egui::Context) {
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
