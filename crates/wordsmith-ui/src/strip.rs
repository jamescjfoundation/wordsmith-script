// SPDX-License-Identifier: MIT OR Apache-2.0
//! The structure strip (SPEC-2 9.3): a band above the script with a lane per structure element
//! and one for the scenes, blocks as long as the pages they span, a page ruler along the top,
//! and its own zoom. Clicking a block goes there in the script.

use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::editor::Editor;

/// Zoom limits, as a share of the default (25 % to 400 %).
pub const ZOOM: std::ops::RangeInclusive<f32> = 0.25..=4.0;
/// Points one page takes at 100 %.
const PAGE_W: f32 = 48.0;
const LANE_H: f32 = 22.0;
const RULER_H: f32 = 16.0;
const LABEL_W: f32 = 90.0;

fn color_of(hex: &str, fallback: Color32) -> Color32 {
    wordsmith_script::revisions::parse_color(hex).map_or(fallback, |c| Color32::from_rgb(c.0, c.1, c.2))
}

pub fn panel(ed: &mut Editor, ui: &mut Ui) {
    if !ed.tools.strip || ed.structure.is_some() {
        return;
    }
    if ed.tools.strip_zoom <= 0.0 {
        ed.tools.strip_zoom = 1.0;
    }
    let data = ed.session.run("strip.get", &json!({})).unwrap_or(Value::Null);
    let lanes = data["lanes"].as_array().cloned().unwrap_or_default();
    let pages = data["pages"].as_u64().unwrap_or(1).max(1) as f32;
    let mut go: Option<usize> = None;
    let height = RULER_H + LANE_H * lanes.len() as f32 + 34.0;
    egui::Panel::top("strip").resizable(false).exact_size(height).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Structure");
            let mut pct = ed.tools.strip_zoom * 100.0;
            if ui.add(egui::Slider::new(&mut pct, 25.0..=400.0).suffix("%").show_value(true)).changed() {
                ed.tools.strip_zoom = (pct / 100.0).clamp(*ZOOM.start(), *ZOOM.end());
            }
        });
        let per_page = PAGE_W * ed.tools.strip_zoom;
        egui::ScrollArea::horizontal().id_salt("strip-scroll").show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(LABEL_W + pages * per_page + 8.0, RULER_H + LANE_H * lanes.len() as f32), Sense::hover());
            let painter = ui.painter_at(rect);
            let x0 = rect.min.x + LABEL_W;
            let weak = ui.visuals().weak_text_color();
            let ink = ui.visuals().text_color();
            // The page ruler: a tick every page, a number every few.
            let step = if per_page >= 30.0 {
                1
            } else if per_page >= 12.0 {
                5
            } else {
                10
            };
            for n in 0..=(pages as usize) {
                let x = x0 + n as f32 * per_page;
                painter.line_segment([pos2(x, rect.min.y + RULER_H - 4.0), pos2(x, rect.min.y + RULER_H)], Stroke::new(1.0, weak));
                if n % step == 0 && (n as f32) < pages {
                    painter.text(pos2(x + 2.0, rect.min.y), egui::Align2::LEFT_TOP, (n + 1).to_string(), FontId::proportional(10.0), weak);
                }
            }
            for (i, lane) in lanes.iter().enumerate() {
                let y = rect.min.y + RULER_H + i as f32 * LANE_H;
                painter.text(
                    pos2(rect.min.x + 2.0, y + LANE_H / 2.0),
                    egui::Align2::LEFT_CENTER,
                    lane["name"].as_str().unwrap_or(""),
                    FontId::proportional(11.0),
                    ink,
                );
                for (k, b) in lane["blocks"].as_array().cloned().unwrap_or_default().iter().enumerate() {
                    let start = b["start"].as_f64().unwrap_or(0.0) as f32;
                    let end = (b["end"].as_f64().unwrap_or(0.0) as f32).max(start + 0.05);
                    let r = Rect::from_min_max(Pos2::new(x0 + start * per_page, y + 2.0), Pos2::new(x0 + end * per_page - 1.0, y + LANE_H - 2.0));
                    let shade = if k % 2 == 0 { Color32::from_rgb(205, 218, 235) } else { Color32::from_rgb(222, 230, 242) };
                    let fill = color_of(b["color"].as_str().unwrap_or(""), shade);
                    let resp = ui.interact(r, ui.id().with(("strip", i, k)), Sense::click());
                    painter.rect_filled(r, 3.0, fill);
                    if resp.hovered() {
                        painter.rect_stroke(r, 3.0, Stroke::new(1.5, ui.visuals().selection.stroke.color), egui::StrokeKind::Inside);
                    }
                    let label = b["label"].as_str().unwrap_or("");
                    let g = painter.layout_no_wrap(label.to_string(), FontId::proportional(10.0), Color32::from_gray(30));
                    ui.painter_at(r.shrink(2.0)).galley(pos2(r.min.x + 3.0, r.center().y - g.size().y / 2.0), g, Color32::from_gray(30));
                    if resp.on_hover_text(label).clicked() {
                        go = b["block"].as_u64().map(|x| x as usize);
                    }
                }
            }
        });
    });
    if let Some(block) = go {
        ed.session.sel = wordsmith_engine::Selection::caret(wordsmith_script::data::body_pos(block, 0));
        ed.request_scroll_to_caret();
    }
}
