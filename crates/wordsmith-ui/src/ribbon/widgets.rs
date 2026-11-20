// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ribbon controls: large and small buttons, toggles, galleries and groups. Every button runs a
//! command by id; it is greyed out when the command says it cannot run, and its tooltip shows the
//! command's name, key and a line on what it does. Groups shrink to fit the window through
//! [`crate::ribbon_fit`] (copied from WordCraft).

use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::editor::Editor;
use crate::ribbon_fit;
use crate::theme::{CAPTION, Tokens, medium, regular};

/// Height of a group's controls, and of its title row under them.
pub const CONTENT_H: f32 = 72.0;
pub const TITLE_H: f32 = 16.0;
/// A large button's width.
pub const BIG_W: f32 = 58.0;
/// A small button's height.
pub const SMALL_H: f32 = 21.0;

/// The tooltip for command `id`: its name and key, then `about`.
pub fn tip(ed: &Editor, id: &str, about: &str) -> String {
    let spec = ed.session.registry.get(id);
    let name = spec.map_or(id, |s| s.label);
    let key = spec.map_or("", |s| s.shortcut).split(" / ").next().unwrap_or("");
    let mut out = if key.is_empty() { name.to_string() } else { format!("{name}   {key}") };
    if !about.is_empty() {
        out.push('\n');
        out.push_str(about);
    }
    if let Some(why) = ed.session.why_disabled(id) {
        out.push_str(&format!("\n(Not now: {why})"));
    }
    out
}

/// A command and the parameters a control runs it with.
#[derive(Clone, Debug)]
pub struct Cmd {
    pub id: &'static str,
    pub params: Value,
}

pub fn cmd(id: &'static str) -> Cmd {
    Cmd { id, params: json!({}) }
}

pub fn cmd_with(id: &'static str, params: Value) -> Cmd {
    Cmd { id, params }
}

fn enabled(ed: &Editor, c: &Cmd) -> bool {
    ed.session.registry.get(c.id).is_some() && ed.session.why_disabled(c.id).is_none()
}

fn icon_name(c: &Cmd, icon: &str) -> String {
    if icon.is_empty() { crate::script_icons::for_command(c.id).to_string() } else { icon.to_string() }
}

/// The background of a control: hovered, on, or plain.
fn bg(ui: &Ui, r: Rect, hovered: bool, on: bool, t: &Tokens) {
    if on {
        ui.painter().rect_filled(r, 4.0, t.accent_soft);
        ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, t.accent.gamma_multiply(0.5)), egui::StrokeKind::Inside);
    } else if hovered {
        ui.painter().rect_filled(r, 4.0, t.hover);
    }
}

/// A large button: icon over a label of up to two lines. `on` draws it pressed (a toggle).
pub fn big(ui: &mut Ui, ed: &mut Editor, c: Cmd, icon: &str, label: &str, about: &str, on: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let ok = enabled(ed, &c);
    let (r, resp) = ui.allocate_exact_size(vec2(BIG_W, CONTENT_H), if ok { Sense::click() } else { Sense::hover() });
    bg(ui, r, resp.hovered() && ok, on, &t);
    let ink = if ok { t.text } else { t.text_dim.gamma_multiply(0.6) };
    let accent = if ok { t.accent } else { ink };
    crate::script_icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(r.center().x, r.min.y + 19.0), vec2(26.0, 26.0)),
        &icon_name(&c, icon),
        ink,
        accent,
    );
    let mut job = egui::text::LayoutJob::simple(label.to_string(), regular(CAPTION), ink, BIG_W - 4.0);
    job.halign = egui::Align::Center;
    job.wrap.max_rows = 2;
    let g = ui.painter().layout_job(job);
    ui.painter().galley(pos2(r.center().x, r.min.y + 37.0), g, ink);
    keytip_mark(ui, ed, r, c.id);
    let resp = resp.on_hover_text(tip(ed, c.id, about));
    if resp.clicked() && ok {
        ed.run(c.id, c.params);
        return true;
    }
    false
}

/// A small button: icon and a label (the label goes in a compact group; the tooltip keeps it).
pub fn small(ui: &mut Ui, ed: &mut Editor, c: Cmd, icon: &str, label: &str, about: &str, on: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let ok = enabled(ed, &c);
    let compact = ribbon_fit::compact(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_string(), regular(BODY_SMALL), t.text);
    let label_w = galley.size().x + 8.0;
    if !compact {
        ribbon_fit::add_saving(ui.ctx(), label_w);
    }
    let w = 24.0 + if compact || label.is_empty() { 0.0 } else { label_w };
    let (r, resp) = ui.allocate_exact_size(vec2(w, SMALL_H), if ok { Sense::click() } else { Sense::hover() });
    bg(ui, r, resp.hovered() && ok, on, &t);
    let ink = if ok { t.text } else { t.text_dim.gamma_multiply(0.6) };
    let accent = if ok { t.accent } else { ink };
    crate::script_icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(r.min.x + 12.0, r.center().y), vec2(16.0, 16.0)),
        &icon_name(&c, icon),
        ink,
        accent,
    );
    if !compact && !label.is_empty() {
        ui.painter().text(pos2(r.min.x + 24.0, r.center().y), Align2::LEFT_CENTER, label, regular(BODY_SMALL), ink);
    }
    keytip_mark(ui, ed, r, c.id);
    let resp = resp.on_hover_text(tip(ed, c.id, about));
    if resp.clicked() && ok {
        ed.run(c.id, c.params);
        return true;
    }
    false
}

/// Label size of small buttons.
const BODY_SMALL: f32 = 12.0;

/// Small buttons stacked three to a column.
pub fn stack(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(0.0, CONTENT_H), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 1.5;
        ui.spacing_mut().button_padding = vec2(4.0, 1.0);
        ui.set_max_height(CONTENT_H);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        add(ui);
    });
}

/// A group: its controls in a row with its title under them. Shrinks (compact: small buttons
/// lose their labels) or folds into one drop-down when the tab is wider than the window.
pub fn group(ui: &mut Ui, ed: &mut Editor, title: &str, add: impl FnOnce(&mut Ui, &mut Editor)) {
    let t = Tokens::get(ui.ctx());
    let level = ribbon_fit::level(ui.ctx(), title);
    let start = ui.cursor().min.x;
    if level == ribbon_fit::COLLAPSED {
        let (r, resp) = ui.allocate_exact_size(vec2(62.0, CONTENT_H + TITLE_H), Sense::click());
        bg(ui, r, resp.hovered(), false, &t);
        ui.painter().text(pos2(r.center().x, r.min.y + 26.0), Align2::CENTER_CENTER, "▾", regular(16.0), t.text_dim);
        ui.painter().text(pos2(r.center().x, r.max.y - 8.0), Align2::CENTER_CENTER, title, regular(CAPTION), t.text_dim);
        let popup = ui.make_persistent_id(("ribbon-group", title));
        if resp.clicked() {
            egui::Popup::toggle_id(ui.ctx(), popup);
        }
        egui::Popup::new(popup, ui.ctx().clone(), egui::PopupAnchor::from(&resp), ui.layer_id()).open_memory(None).show(|ui| {
            ui.horizontal(|ui| add(ui, ed));
        });
        divider(ui, &t);
        ribbon_fit::record(ui.ctx(), title, level, ui.cursor().min.x - start);
        return;
    }
    ribbon_fit::set_compact(ui.ctx(), level == ribbon_fit::COMPACT);
    // The controls in a band of fixed height, the title in the row under it, so every group's
    // title lines up.
    let top = ui.cursor().min;
    let inner = ui.allocate_ui_with_layout(vec2(ui.available_width(), CONTENT_H), egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.set_height(CONTENT_H);
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        add(ui, ed);
    });
    let w = inner.response.rect.width().max(40.0);
    let title_rect = Rect::from_min_size(pos2(top.x, top.y + CONTENT_H), vec2(w, TITLE_H));
    ui.painter().text(title_rect.center(), Align2::CENTER_CENTER, title, regular(CAPTION), t.text_dim);
    // Reserve the title row's height beside the band.
    ui.allocate_space(vec2(0.0, CONTENT_H + TITLE_H - inner.response.rect.height()));
    ribbon_fit::end_compact(ui.ctx());
    divider(ui, &t);
    ribbon_fit::record(ui.ctx(), title, level, ui.cursor().min.x - start);
}

fn divider(ui: &mut Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, CONTENT_H + TITLE_H), Sense::hover());
    ui.painter().line_segment([pos2(r.center().x, r.min.y + 6.0), pos2(r.center().x, r.max.y - 4.0)], Stroke::new(1.0, t.border));
}

/// A tile in a gallery: icon, name and a second line (a key, a hint). `on` marks the current one.
/// `text` is the tile's name and its second line.
pub fn tile(ui: &mut Ui, ed: &mut Editor, c: Cmd, icon: &str, text: (&str, &str), about: &str, on: bool) -> bool {
    let (name, second) = text;
    let t = Tokens::get(ui.ctx());
    let ok = enabled(ed, &c);
    let w = 112.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), if ok { Sense::click() } else { Sense::hover() });
    bg(ui, r, resp.hovered() && ok, on, &t);
    ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, if on { t.accent } else { t.border }), egui::StrokeKind::Inside);
    let ink = if ok { t.text } else { t.text_dim };
    crate::script_icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(r.min.x + 13.0, r.center().y), vec2(18.0, 18.0)),
        &icon_name(&c, icon),
        ink,
        t.accent,
    );
    ui.painter().text(pos2(r.min.x + 25.0, r.min.y + 9.5), Align2::LEFT_CENTER, name, medium(11.5), ink);
    ui.painter().text(pos2(r.min.x + 25.0, r.min.y + 21.5), Align2::LEFT_CENTER, second, regular(9.5), t.text_dim);
    keytip_mark(ui, ed, r, c.id);
    let resp = resp.on_hover_text(tip(ed, c.id, about));
    if resp.clicked() && ok {
        ed.run(c.id, c.params);
        return true;
    }
    false
}

/// A color chip (a revision set, a highlighter) that runs a command.
pub fn chip(ui: &mut Ui, ed: &mut Editor, c: Cmd, color: Color32, label: &str, about: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let g = ui.painter().layout_no_wrap(label.to_string(), regular(12.0), t.text);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 30.0, SMALL_H), Sense::click());
    bg(ui, r, resp.hovered(), false, &t);
    ui.painter().rect_filled(Rect::from_center_size(pos2(r.min.x + 11.0, r.center().y), vec2(12.0, 12.0)), 3.0, color);
    ui.painter().rect_stroke(
        Rect::from_center_size(pos2(r.min.x + 11.0, r.center().y), vec2(12.0, 12.0)),
        3.0,
        Stroke::new(1.0, t.border),
        egui::StrokeKind::Middle,
    );
    ui.painter().galley(pos2(r.min.x + 22.0, r.center().y - g.size().y / 2.0), g, t.text);
    let resp = resp.on_hover_text(tip(ed, c.id, about));
    if resp.clicked() {
        ed.run(c.id, c.params);
        return true;
    }
    false
}

/// When key tips show (Alt), the letters that press each control of this tab (M19).
fn keytip_mark(ui: &Ui, ed: &mut Editor, r: Rect, id: &str) {
    if !ed.chrome.keytips || ed.chrome.keytip_stage != 1 || !ed.chrome.in_ribbon {
        return;
    }
    let letters = ed.chrome.keytip_for(id);
    let t = Tokens::get(ui.ctx());
    let at = pos2(r.center().x, r.max.y - 4.0);
    let g = ui.painter().layout_no_wrap(letters.clone(), crate::theme::semibold(10.0), t.surface_high);
    let bx = Rect::from_center_size(at, g.size() + vec2(6.0, 2.0));
    ui.painter().rect_filled(bx, 3.0, t.text);
    ui.painter().galley(bx.min + vec2(3.0, 1.0), g, t.surface_high);
    ed.chrome.keytip_targets.push((letters, id.to_string()));
}
