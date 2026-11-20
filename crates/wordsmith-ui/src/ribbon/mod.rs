// SPDX-License-Identifier: MIT OR Apache-2.0
//! The window's chrome (v0.3 plan): a quick-access row (save, undo, redo, the script's name, a
//! command search box), the ribbon's tab strip and the current tab's groups, and key tips (Alt
//! shows a letter on every tab, then on every control). Tabs are named for screenplay work; every
//! control runs a command from the registry. The File tab opens the backstage
//! ([`crate::backstage`]).

use egui::{Align2, Event, Key, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::editor::Editor;
use crate::theme::{BODY, Tokens, medium, regular, semibold};

pub mod tabs;
pub mod widgets;

/// A ribbon tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Write,
    Insert,
    Plan,
    Production,
    Review,
    View,
    /// Contextual: shown with the story board.
    Board,
    /// Contextual: shown with the index cards.
    Cards,
}

impl Tab {
    pub fn name(self) -> &'static str {
        match self {
            Tab::Write => "Write",
            Tab::Insert => "Insert",
            Tab::Plan => "Plan",
            Tab::Production => "Production",
            Tab::Review => "Review",
            Tab::View => "View",
            Tab::Board => "Story Board",
            Tab::Cards => "Index Cards",
        }
    }

    /// The key tip letter that opens the tab.
    pub fn letter(self) -> &'static str {
        match self {
            Tab::Write => "W",
            Tab::Insert => "I",
            Tab::Plan => "P",
            Tab::Production => "O",
            Tab::Review => "R",
            Tab::View => "V",
            Tab::Board => "B",
            Tab::Cards => "C",
        }
    }
}

/// The left side panel (the Navigator) and the right one (the Inspector).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    Scenes,
    Outline,
    Bookmarks,
    Notes,
    Characters,
    Find,
    Element,
    Scene,
    Revisions,
    Page,
    Changes,
    Checker,
    Goals,
}

/// The chrome's state.
pub struct Chrome {
    pub tab: Tab,
    /// The ribbon shows its tabs only (a click opens a tab's groups for one use).
    pub collapsed: bool,
    /// The ribbon's groups show for one use while collapsed.
    pub peek: bool,
    /// The old menu bar, under the quick-access row (off by default).
    pub menu_bar: bool,
    /// The File page in place of the script.
    pub backstage: Option<crate::backstage::Page>,
    /// The Navigator's and the Inspector's open panel, if any, and their widths.
    pub left: Option<Panel>,
    pub right: Option<Panel>,
    pub left_w: f32,
    pub right_w: f32,
    /// Key tips: off, showing tab letters (stage 0) or control letters (stage 1), and what has
    /// been typed so far.
    pub keytips: bool,
    pub keytip_stage: u8,
    pub keytip_typed: String,
    /// This frame's controls with key tips, and the next letters to give out.
    pub keytip_targets: Vec<(String, String)>,
    keytip_next: usize,
    /// The first-run tour's step, while it shows.
    pub tour: Option<usize>,
    /// The ribbon's groups are being drawn (only their controls take key tips).
    pub in_ribbon: bool,
    /// The File page's print choices.
    pub print: crate::production::PrintChoices,
}

impl Default for Chrome {
    fn default() -> Self {
        Chrome {
            tab: Tab::Write,
            collapsed: false,
            peek: false,
            menu_bar: false,
            backstage: None,
            left: Some(Panel::Scenes),
            right: None,
            left_w: 260.0,
            right_w: 290.0,
            keytips: false,
            keytip_stage: 0,
            keytip_typed: String::new(),
            keytip_targets: Vec::new(),
            keytip_next: 0,
            tour: None,
            in_ribbon: false,
            print: Default::default(),
        }
    }
}

/// Letters for the `n`th control: A to Z, then two letters.
pub fn letters(n: usize) -> String {
    const L: &[u8] = b"ABCDEFGHJKLMNQSTUXYZ";
    if n < L.len() {
        return (L[n] as char).to_string();
    }
    let n = n - L.len();
    format!("{}{}", L[(n / L.len()) % L.len()] as char, L[n % L.len()] as char)
}

impl Chrome {
    /// The key tip letters for the next control drawn this frame.
    pub fn keytip_for(&mut self, _id: &str) -> String {
        let l = letters(self.keytip_next);
        self.keytip_next += 1;
        l
    }

    fn end_keytips(&mut self) {
        self.keytips = false;
        self.keytip_stage = 0;
        self.keytip_typed.clear();
    }
}

/// The tabs showing now: the main ones, and a contextual one with its view.
pub fn tabs_now(ed: &Editor) -> Vec<Tab> {
    let mut v = vec![Tab::Write, Tab::Insert, Tab::Plan, Tab::Production, Tab::Review, Tab::View];
    match ed.structure {
        Some(crate::structure::Structure::Board) => v.push(Tab::Board),
        Some(crate::structure::Structure::Cards) => v.push(Tab::Cards),
        _ => {}
    }
    v
}

/// Key tips: Alt alone shows the tab letters; a letter opens that tab and shows its controls'
/// letters; typing those runs the control; Escape ends.
fn handle_keytips(ed: &mut Editor, ui: &mut Ui) {
    let events = ui.input(|i| i.events.clone());
    let alt_alone = ui.input(|i| {
        i.events.iter().any(|e| matches!(e, Event::Key { key: Key::F10, pressed: true, .. }))
            || (i.modifiers.alt
                && !i.modifiers.command
                && !i.modifiers.shift
                && i.events.iter().any(|e| matches!(e, Event::Key { key: Key::Space, pressed: true, .. })))
    });
    if alt_alone {
        ed.chrome.keytips = !ed.chrome.keytips;
        ed.chrome.keytip_stage = 0;
        ed.chrome.keytip_typed.clear();
        ui.input_mut(|i| i.events.clear());
        return;
    }
    if !ed.chrome.keytips {
        return;
    }
    let mut consumed = false;
    for e in events {
        match e {
            Event::Key { key: Key::Escape, pressed: true, .. } => {
                ed.chrome.end_keytips();
                consumed = true;
            }
            Event::Text(t) => {
                consumed = true;
                let up = t.to_uppercase();
                if ed.chrome.keytip_stage == 0 {
                    if up == "F" {
                        ed.chrome.backstage = Some(crate::backstage::Page::New);
                        ed.chrome.end_keytips();
                    } else if let Some(tab) = tabs_now(ed).into_iter().find(|t| t.letter() == up) {
                        ed.chrome.tab = tab;
                        ed.chrome.keytip_stage = 1;
                    }
                } else {
                    ed.chrome.keytip_typed.push_str(&up);
                    let typed = ed.chrome.keytip_typed.clone();
                    let targets = ed.chrome.keytip_targets.clone();
                    if let Some((_, id)) = targets.iter().find(|(l, _)| *l == typed) {
                        let id = id.clone();
                        ed.chrome.end_keytips();
                        if let Some(spec) = ed.session.registry.get(&id) {
                            let id = spec.id;
                            ed.run(id, json!({}));
                        }
                    } else if !targets.iter().any(|(l, _)| l.starts_with(&typed)) {
                        ed.chrome.keytip_typed.clear();
                    }
                }
            }
            _ => {}
        }
    }
    if consumed {
        ui.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Text(_) | Event::Key { .. })));
    }
}

/// The quick-access row, the tab strip and the ribbon.
pub fn show(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    handle_keytips(ed, ui);
    ed.chrome.keytip_targets.clear();
    ed.chrome.keytip_next = 0;
    // The contextual tab goes when its view does.
    let now = tabs_now(ed);
    if !now.contains(&ed.chrome.tab) {
        ed.chrome.tab = Tab::Write;
    }
    egui::Panel::top("quick")
        .frame(egui::Frame::NONE.fill(t.surface_high).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| quick_row(ed, ui));
    if ed.chrome.menu_bar {
        crate::chrome::menu_bar(ed, ui);
    }
    let show_groups = !ed.chrome.collapsed || ed.chrome.peek;
    egui::Panel::top("ribbon")
        .frame(egui::Frame::NONE.fill(t.surface).stroke(Stroke::new(1.0, t.border)).inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 0,
            bottom: if show_groups { 4 } else { 0 },
        }))
        .show(ui, |ui| {
            tab_strip(ed, ui, &now);
            if show_groups {
                ui.add_space(2.0);
                let avail = ui.available_width();
                crate::ribbon_fit::begin(ui.ctx(), ed.chrome.tab.name(), avail);
                egui::ScrollArea::horizontal()
                    .id_salt("ribbon-scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ed.chrome.in_ribbon = true;
                            tabs::draw(ed, ui);
                            ed.chrome.in_ribbon = false;
                        });
                    });
                crate::ribbon_fit::end(ui.ctx());
            }
        });
}

fn quick_row(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (id, icon) in [("file.save", "save"), ("edit.undo", "undo"), ("edit.redo", "redo")] {
            widgets::small(ui, ed, widgets::cmd(id), icon, "", "", false);
        }
        ui.add_space(10.0);
        let state = if ed.session.dirty {
            "Not saved"
        } else if ed.session.path.is_some() {
            "Saved"
        } else {
            "Not saved yet"
        };
        ui.label(egui::RichText::new(crate::editor::script_name(&ed.session)).font(semibold(BODY)).color(t.text));
        ui.label(egui::RichText::new(format!("— {state}")).font(regular(BODY)).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Show and hide the side panels.
            let right = ed.chrome.right.is_some();
            if ui
                .add(egui::Button::new(if right { "Inspector ▸" } else { "◂ Inspector" }).frame(false))
                .on_hover_text("Show or hide the Inspector")
                .clicked()
            {
                ed.chrome.right = if right { None } else { Some(Panel::Element) };
            }
            let left = ed.chrome.left.is_some();
            if ui
                .add(egui::Button::new(if left { "◂ Navigator" } else { "Navigator ▸" }).frame(false))
                .on_hover_text("Show or hide the Navigator")
                .clicked()
            {
                ed.chrome.left = if left { None } else { Some(Panel::Scenes) };
            }
            // The command search box: the palette.
            let key = if ed.mac { "⌘⇧P" } else { "Ctrl+Shift+P" };
            let (r, resp) = ui.allocate_exact_size(vec2(300.0, 24.0), Sense::click());
            ui.painter().rect_filled(r, 5.0, t.input);
            ui.painter().rect_stroke(r, 5.0, Stroke::new(1.0, if resp.hovered() { t.accent } else { t.border }), egui::StrokeKind::Inside);
            crate::script_icons::paint(
                ui.painter(),
                egui::Rect::from_center_size(pos2(r.min.x + 13.0, r.center().y), vec2(14.0, 14.0)),
                "find",
                t.text_dim,
                t.accent,
            );
            ui.painter().text(pos2(r.min.x + 26.0, r.center().y), Align2::LEFT_CENTER, "Search commands and help", regular(12.0), t.text_dim);
            ui.painter().text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, key, regular(11.0), t.text_dim);
            if resp.on_hover_text("Find any command by name").clicked() {
                ed.run("view.commandPalette", json!({}));
            }
        });
    });
}

fn tab_strip(ed: &mut Editor, ui: &mut Ui, tabs: &[Tab]) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        // File: the backstage.
        let file = ui.add(egui::Button::new(egui::RichText::new("File").font(medium(13.0)).color(t.surface_high)).fill(t.accent).corner_radius(4));
        if file.on_hover_text("New, open, save, export, print, preferences").clicked() {
            ed.chrome.backstage = Some(crate::backstage::Page::New);
        }
        if ed.chrome.keytips && ed.chrome.keytip_stage == 0 {
            mark(ui, "F");
        }
        for tab in tabs {
            let current = ed.chrome.tab == *tab;
            let contextual = matches!(tab, Tab::Board | Tab::Cards);
            let color = if current {
                t.accent
            } else if contextual {
                t.accent.gamma_multiply(0.8)
            } else {
                t.text
            };
            let g = ui.painter().layout_no_wrap(tab.name().to_string(), medium(13.0), color);
            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 20.0, 28.0), Sense::click());
            if resp.hovered() && !current {
                ui.painter().rect_filled(r.shrink2(vec2(0.0, 3.0)), 4.0, t.hover);
            }
            ui.painter().galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, color);
            if current {
                ui.painter().line_segment([pos2(r.min.x + 8.0, r.max.y - 2.0), pos2(r.max.x - 8.0, r.max.y - 2.0)], Stroke::new(2.5, t.accent));
            }
            if ed.chrome.keytips && ed.chrome.keytip_stage == 0 {
                mark(ui, tab.letter());
            }
            if resp.clicked() {
                if ed.chrome.collapsed {
                    ed.chrome.peek = !(current && ed.chrome.peek);
                }
                ed.chrome.tab = *tab;
            }
            if resp.double_clicked() {
                ed.chrome.collapsed = !ed.chrome.collapsed;
                ed.chrome.peek = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if ed.chrome.collapsed { "▾ Show ribbon" } else { "▴ Hide ribbon" };
            if ui
                .add(egui::Button::new(egui::RichText::new(label).font(regular(11.5)).color(t.text_dim)).frame(false))
                .on_hover_text("Ribbon: show the groups, or the tabs only (double-click a tab too)")
                .clicked()
            {
                ed.chrome.collapsed = !ed.chrome.collapsed;
                ed.chrome.peek = false;
            }
        });
    });
}

/// A key tip letter beside the control just drawn.
fn mark(ui: &mut Ui, letter: &str) {
    let t = Tokens::get(ui.ctx());
    let r = ui.min_rect();
    let at = pos2(ui.cursor().min.x - 6.0, r.max.y - 4.0);
    let g = ui.painter().layout_no_wrap(letter.to_string(), semibold(10.0), t.surface_high);
    let bx = egui::Rect::from_center_size(at, g.size() + vec2(6.0, 2.0));
    ui.painter().rect_filled(bx, 3.0, t.text);
    ui.painter().galley(bx.min + vec2(3.0, 1.0), g, t.surface_high);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_count_on() {
        assert_eq!(letters(0), "A");
        assert_eq!(letters(19), "Z");
        assert_eq!(letters(20), "AA");
        assert_eq!(letters(21), "AB");
    }
}
