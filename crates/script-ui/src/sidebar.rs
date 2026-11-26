// SPDX-License-Identifier: MIT OR Apache-2.0
//! The side panels (decision 0037): an icon rail at each side of the window, the Script panel on
//! the left (where things are: scenes, outline, bookmarks, notes, characters, find) and the
//! Inspector on the right (what is at the caret: element, scene, revisions, page, changes, the
//! format checker, goals). A click on a rail icon opens its panel; a second click closes it.
//! Every change runs a command, so undo and agents work the same.

//! The side panels' contents (decision 0037): the Script panel on the left (where things are:
//! scenes, outline, bookmarks, notes, characters, find) and the Inspector on the right (what is
//! at the caret: element, scene, revisions, page, changes, the format checker, goals). The shell
//! draws the rails, the panels' frames and title bars, and keeps which are open where. Every
//! change runs a command, so undo and agents work the same.

use egui::{Align2, Color32, RichText, Sense, Stroke, Ui, pos2, vec2};
use jcj_shell::Shell;
use jcj_shell::workspace::{PanelSpec, Side};
use serde_json::{Value, json};

use crate::editor::{Editor, ToShell};
use crate::reports::{Drafts, draft_field};
use crate::theme::{CAPTION, medium, regular, semibold};

/// The panels: id (kept in the preferences), title, rail icon, side.
const PANELS: [(&str, &str, &str, Side); 13] = [
    ("scenes", "Scenes", "navPane", Side::Left),
    ("outline", "Outline", "outline", Side::Left),
    ("bookmarks", "Bookmarks", "bookmark", Side::Left),
    ("notes", "Notes", "comment", Side::Left),
    ("characters", "Characters", "user", Side::Left),
    ("find", "Find", "find", Side::Left),
    ("element", "Element", "styles", Side::Right),
    ("scene", "Scene", "properties", Side::Right),
    ("revisions", "Revisions", "pageColor", Side::Right),
    ("page", "Page", "protect", Side::Right),
    ("changes", "Changes", "trackChanges", Side::Right),
    ("checker", "Check Format", "checkErrors", Side::Right),
    ("goals", "Goals", "wordCount", Side::Right),
];

/// The side panels Script offers.
pub fn panels() -> Vec<PanelSpec> {
    PANELS.iter().map(|(id, title, icon, side)| PanelSpec::new(*id, *title, *icon, *side)).collect()
}

/// Whether panel `id` is one of the Script panel's (the left side) rather than the Inspector's.
pub fn is_left(id: &str) -> bool {
    PANELS.iter().any(|(p, _, _, side)| *p == id && *side == Side::Left)
}

/// Whether `id` is one of Script's panels.
pub fn is_panel(id: &str) -> bool {
    PANELS.iter().any(|(p, _, _, _)| *p == id)
}

/// Field drafts the panels keep while text is typed.
#[derive(Default)]
pub struct PanelState {
    pub drafts: Drafts,
    pub filter: String,
    pub new_name: String,
    /// Characters whose details are open.
    pub open: Vec<String>,
    /// Format issues skipped with Ignore, as (block, rule).
    pub ignored: Vec<(u64, String)>,
}

fn hint(ui: &mut Ui, text: &str) {
    let t = crate::theme::get(ui.ctx());
    ui.label(RichText::new(text).font(regular(CAPTION)).color(t.text_dim));
}

/// Draw panel `id`'s contents (the shell gives it its frame and title).
pub fn panel(ed: &mut Editor, ui: &mut Ui, _shell: &mut Shell, id: &str) {
    match id {
        "scenes" => scenes(ed, ui),
        "outline" => outline(ed, ui),
        "bookmarks" => bookmarks(ed, ui),
        "notes" => notes(ed, ui),
        "characters" => characters(ed, ui),
        "find" => find(ed, ui),
        "element" => element(ed, ui),
        "scene" => scene(ed, ui),
        "revisions" => revisions(ed, ui),
        "page" => page(ed, ui),
        "changes" => changes(ed, ui),
        "checker" => checker(ed, ui),
        "goals" => goals(ed, ui),
        _ => {}
    }
}

fn color_of(name: &str) -> Option<Color32> {
    wordsmith_script_model::scene_data::SCENE_COLORS
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, hex)| wordsmith_script_model::revisions::parse_color(hex))
        .map(|c| Color32::from_rgb(c.0, c.1, c.2))
}

/// Go to body block `block` in the script view.
fn go(ed: &mut Editor, block: usize, off: usize) {
    ed.session.sel = jcj_engine::Selection::caret(wordsmith_script_model::data::body_pos(block, off));
    ed.request_scroll_to_caret();
}

/// Scene rows' height.
const SCENE_ROW: f32 = 40.0;

fn scenes(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    ui.add(egui::TextEdit::singleline(&mut ed.panels.filter).hint_text("Filter scenes").desired_width(f32::INFINITY));
    let (list, eighths) = ed.panel_scenes();
    let here =
        wordsmith_script_model::scenes::scene_at(&wordsmith_script_model::scenes(&ed.session.doc), ed.session.sel.focus.path.last()).map(|s| s.index);
    let f = ed.panels.filter.trim().to_lowercase();
    let shown: Vec<usize> = (0..list.len())
        .filter(|i| {
            f.is_empty()
                || list.get(*i).is_some_and(|sc| {
                    format!("{} {}", sc["heading"].as_str().unwrap_or(""), sc["title"].as_str().unwrap_or("")).to_lowercase().contains(&f)
                })
        })
        .collect();
    let mut act: Option<(usize, &str)> = None;
    let mut moved: Option<(usize, usize)> = None;
    if list.is_empty() {
        hint(ui, "No scenes yet. Each Scene Heading starts one.");
    }
    hint(ui, "Drag a scene to move it; right-click for more.");
    egui::ScrollArea::vertical().id_salt("nav-scenes").auto_shrink(false).show_rows(ui, SCENE_ROW, shown.len(), |ui, rows| {
        for &i in &shown[rows] {
            let Some(sc) = list.get(i) else { continue };
            let heading = sc["heading"].as_str().unwrap_or("");
            let title_text = sc["title"].as_str().unwrap_or("");
            let number = sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string);
            let drag = ui.dnd_drag_source(egui::Id::unique(("nav-scene", i)), i, |ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), SCENE_ROW - 2.0), Sense::click());
                let current = here == Some(i);
                ui.painter().rect_filled(
                    r,
                    5.0,
                    if current {
                        t.accent_soft
                    } else if resp.hovered() {
                        t.hover
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                if let Some(c) = color_of(sc["color"].as_str().unwrap_or("")) {
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(4.0, r.height())), 2.0, c);
                }
                ui.painter().text(pos2(r.min.x + 10.0, r.min.y + 11.0), Align2::LEFT_CENTER, &number, semibold(11.5), t.text_dim);
                let mut job = egui::text::LayoutJob::simple_singleline(heading.to_string(), medium(12.0), t.text);
                job.wrap.max_width = r.width() - 76.0;
                job.wrap.max_rows = 1;
                ui.painter().galley(pos2(r.min.x + 34.0, r.min.y + 3.0), ui.painter().layout_job(job), t.text);
                let len = eighths.get(i).map(|e| wordsmith_script_model::reports::format_eighths(*e)).unwrap_or_default();
                ui.painter().text(pos2(r.max.x - 6.0, r.min.y + 11.0), Align2::RIGHT_CENTER, len, regular(CAPTION), t.text_dim);
                let second = if title_text.is_empty() {
                    sc["cast"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default()
                } else {
                    title_text.to_string()
                };
                let mut job = egui::text::LayoutJob::simple_singleline(second, regular(CAPTION), t.text_dim);
                job.wrap.max_width = r.width() - 40.0;
                job.wrap.max_rows = 1;
                ui.painter().galley(pos2(r.min.x + 34.0, r.min.y + 20.0), ui.painter().layout_job(job), t.text_dim);
                if resp.clicked() {
                    act = Some((i, "go"));
                }
                resp.context_menu(|ui| {
                    for (what, label) in
                        [("props", "Scene Properties"), ("omit", "Omit Scene"), ("recall", "Recall Scene"), ("select", "Select Scene")]
                    {
                        if ui.button(label).clicked() {
                            act = Some((i, what));
                            ui.close();
                        }
                    }
                });
            });
            if let Some(from) = drag.response.dnd_release_payload::<usize>() {
                moved = Some((*from, i));
            }
        }
    });
    if let Some((from, onto)) = moved.filter(|(a, b)| a != b) {
        let count = list.len();
        ed.run("scene.move", json!({"scene": from, "before": jcj_board::drop_target(from, onto, count)}));
    }
    if let Some((i, what)) = act {
        // Where the scene is now (the list may be a moment old while typing).
        let start = wordsmith_script_model::scenes(&ed.session.doc).get(i).map_or(0, |s| s.start);
        match what {
            "go" => go(ed, start, 0),
            "props" => {
                go(ed, start, 0);
                ed.to_shell.push(ToShell::ShowPanel("scene".into()));
            }
            "omit" => ed.run("scene.omit", json!({"scene": i})),
            "recall" => ed.run("scene.recall", json!({"scene": i})),
            _ => {
                go(ed, start, 0);
                ed.run("select.scene", json!({}));
            }
        }
    }
}

fn outline(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let elements = wordsmith_script_model::Elements::of_doc(&ed.session.doc);
    let mut go_to = None;
    let mut any = false;
    for (i, b) in ed.session.doc.body.iter().enumerate() {
        let Some(p) = b.as_para() else { continue };
        let def = elements.of_para(p);
        let Some(rank) = def.kind.structure_rank() else { continue };
        any = true;
        let indent = f32::from(rank) * 10.0;
        let text = if p.text.trim().is_empty() { def.name.clone() } else { p.text.clone() };
        let font = if rank <= 1 {
            semibold(12.5)
        } else if rank == 5 {
            regular(12.0)
        } else {
            medium(12.0)
        };
        let color = if rank == 5 { t.text_dim } else { t.text };
        ui.horizontal(|ui| {
            ui.add_space(indent);
            if ui
                .add(egui::Label::new(RichText::new(text).font(font).color(color)).truncate().sense(Sense::click()))
                .on_hover_text(def.name.as_str())
                .clicked()
            {
                go_to = Some(i);
            }
        });
    }
    if !any {
        hint(ui, "Acts, sequences and outline paragraphs show here as a tree, with the scenes under them.");
    }
    if let Some(b) = go_to {
        go(ed, b, 0);
    }
}

fn bookmarks(ed: &mut Editor, ui: &mut Ui) {
    let mut add = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut ed.panels.new_name).hint_text("Name").desired_width(ui.available_width() - 60.0));
        add = ui.button("Add").on_hover_text("Bookmark the caret's paragraph").clicked();
    });
    if add {
        let name = std::mem::take(&mut ed.panels.new_name);
        ed.run("bookmark.add", json!({"name": name.trim()}));
    }
    let list = ed.session.run("bookmark.list", &json!({})).ok().and_then(|v| v["bookmarks"].as_array().cloned()).unwrap_or_default();
    if list.is_empty() {
        hint(ui, "No bookmarks yet (Mod+M bookmarks the caret's paragraph).");
    }
    let mut act: Option<(&str, Value)> = None;
    for b in list {
        ui.horizontal(|ui| {
            let kind = if b["label"].as_bool() == Some(true) { "Label" } else { "" };
            if ui.link(format!("{}  ", b["name"].as_str().unwrap_or(""))).on_hover_text(b["text"].as_str().unwrap_or("")).clicked() {
                act = Some(("bookmark.goto", json!({"id": b["id"]})));
            }
            hint(ui, &format!("p. {} {kind}", b["page"].as_str().unwrap_or("?")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    act = Some(("bookmark.delete", json!({"id": b["id"]})));
                }
            });
        });
    }
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn notes(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    ui.horizontal(|ui| {
        if ui.button("New Note").on_hover_text("On the selected text (Mod+Alt+N)").clicked() {
            ed.run("view.note", json!({}));
        }
        ui.add(egui::TextEdit::singleline(&mut ed.panels.filter).hint_text("Filter").desired_width(f32::INFINITY));
    });
    let list = ed.session.run("note.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    if list.is_empty() {
        hint(ui, "No notes yet. Select text and add one.");
    }
    let f = ed.panels.filter.trim().to_lowercase();
    let mut act: Option<(&str, Value)> = None;
    for n in list {
        let text = n["text"].as_str().unwrap_or("");
        if !f.is_empty() && !format!("{text} {} {}", n["type"].as_str().unwrap_or(""), n["title"].as_str().unwrap_or("")).to_lowercase().contains(&f)
        {
            continue;
        }
        let id = n["id"].as_u64().unwrap_or(0);
        egui::Frame::NONE.fill(t.input).stroke(Stroke::new(1.0, t.border)).corner_radius(6).inner_margin(8).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let kind = n["type"].as_str().unwrap_or("");
                ui.label(RichText::new(kind).font(medium(11.0)).color(t.accent));
                let who = n["author"].as_str().unwrap_or("");
                let when = wordsmith_script_model::reports_more::date(n["modified"].as_u64().unwrap_or(0));
                hint(ui, &format!("{who} {when}"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("×").on_hover_text("Delete the note").clicked() {
                        act = Some(("note.delete", json!({"id": id})));
                    }
                    if n["attached"].as_bool() == Some(true) && ui.small_button("Go").clicked() {
                        act = Some(("note.goto", json!({"id": id})));
                    }
                });
            });
            hint(ui, &format!("“{}”", n["quote"].as_str().unwrap_or("")));
            if let Some(new) = draft_field(ui, &mut ed.panels.drafts, &format!("note:{id}"), text, true, ui.available_width()) {
                act = Some(("note.edit", json!({"id": id, "text": new, "now": crate::now_secs()})));
            }
        });
        ui.add_space(4.0);
    }
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn characters(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let rows = ed.session.run("character.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let traits: Vec<String> = ed
        .session
        .run("character.traits", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|x| x.as_str().map(str::to_string))
        .collect();
    let cast = wordsmith_script_model::cast::Cast::of_doc(&ed.session.doc);
    if rows.is_empty() {
        hint(ui, "Speakers show here once they speak.");
    }
    let mut act: Option<(&str, Value)> = None;
    for r in rows {
        let name = r["name"].as_str().unwrap_or("").to_string();
        let person = cast.people.get(&name).cloned().unwrap_or_default();
        let open = ed.panels.open.contains(&name);
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Label::new(RichText::new(format!("{} {name}", if open { "▾" } else { "▸" })).font(medium(12.5)).color(t.text))
                        .sense(Sense::click()),
                )
                .clicked()
            {
                if open {
                    ed.panels.open.retain(|x| *x != name);
                } else {
                    ed.panels.open.push(name.clone());
                }
            }
            hint(ui, &format!("{} speeches · {} scenes", r["speeches"], r["scenes"]));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut on = person.highlight_on;
                if ui.checkbox(&mut on, "").on_hover_text("Highlight this character's speeches").changed() {
                    act = Some(("character.set", json!({"name": name, "highlightOn": on})));
                }
            });
        });
        if open {
            ui.indent(("char", &name), |ui| {
                ui.horizontal(|ui| {
                    hint(ui, "Highlight");
                    let c = person
                        .highlight
                        .as_deref()
                        .and_then(wordsmith_script_model::revisions::parse_color)
                        .unwrap_or(jcj_doc::props::Rgb(255, 241, 118));
                    let mut color = Color32::from_rgb(c.0, c.1, c.2);
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        act = Some((
                            "character.set",
                            json!({"name": name, "highlight": format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())}),
                        ));
                    }
                });
                for tr in &traits {
                    ui.horizontal(|ui| {
                        ui.add_sized([90.0, 18.0], egui::Label::new(RichText::new(tr).font(regular(CAPTION)).color(t.text_dim)));
                        let now = person.traits.get(tr).cloned().unwrap_or_default();
                        if let Some(v) = draft_field(ui, &mut ed.panels.drafts, &format!("{name}:{tr}"), &now, false, ui.available_width()) {
                            act = Some(("character.set", json!({"name": name, "traits": {tr: v}})));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    hint(ui, "Rename to");
                    if let Some(v) = draft_field(ui, &mut ed.panels.drafts, &format!("rename:{name}"), &name, false, ui.available_width()) {
                        act = Some(("character.rename", json!({"from": name, "to": v})));
                    }
                });
            });
        }
    }
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn find(ed: &mut Editor, ui: &mut Ui) {
    let before = ed.find.clone().unwrap_or_default();
    let mut bar = before.clone();
    ui.add(egui::TextEdit::singleline(&mut bar.query).hint_text("Find").desired_width(f32::INFINITY));
    ui.add(egui::TextEdit::singleline(&mut bar.with).hint_text("Replace with").desired_width(f32::INFINITY));
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut bar.match_case, "Match case");
        ui.checkbox(&mut bar.whole_word, "Whole word");
    });
    let mut action = None;
    ui.horizontal(|ui| {
        for (label, id) in [("Previous", "find.previous"), ("Next", "find.next"), ("Replace", "find.replace"), ("All", "find.replaceAll")] {
            if ui.button(label).clicked() {
                action = Some(id);
            }
        }
    });
    ui.separator();
    let mut go_to = None;
    if !bar.query.is_empty() {
        let found = ed.session.run("find.all", &bar.params()).ok().and_then(|v| v["matches"].as_array().cloned()).unwrap_or_default();
        hint(ui, &format!("{} found", found.len()));
        for m in found.iter().take(300) {
            let block = m["block"].as_u64().unwrap_or(0) as usize;
            let (a, b) = (m["start"].as_u64().unwrap_or(0) as usize, m["end"].as_u64().unwrap_or(0) as usize);
            let text = ed.session.doc.body.get(block).and_then(|x| x.as_para()).map(|p| p.text.clone()).unwrap_or_default();
            let from = text.floor_char_boundary(a.saturating_sub(24));
            let to = text.ceil_char_boundary((b + 32).min(text.len()));
            let shown = text.get(from..to).unwrap_or("").to_string();
            if ui.add(egui::Label::new(RichText::new(format!("…{shown}…")).font(regular(12.0))).truncate().sense(Sense::click())).clicked() {
                go_to = Some((block, a, b));
            }
        }
    }
    if let Some(id) = action.filter(|_| !bar.query.is_empty()) {
        ed.run(id, bar.params());
    }
    if let Some((block, a, b)) = go_to {
        ed.session.sel = jcj_engine::Selection {
            anchor: wordsmith_script_model::data::body_pos(block, a),
            focus: wordsmith_script_model::data::body_pos(block, b),
        };
        ed.request_scroll_to_caret();
    }
    // The query, shared with the find bar, also marks the matches on the pages. It is written
    // only when the panel's own fields change, so the bar can close while the panel is open.
    if bar.params() != before.params() {
        ed.find = Some(bar);
    }
}

fn element(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let h = ed.session.run("element.hint", &json!({})).unwrap_or(Value::Null);
    ui.label(RichText::new(h["element"].as_str().unwrap_or("")).font(semibold(16.0)).color(t.accent));
    hint(ui, &format!("Return makes {}. Tab makes {}.", h["enter"].as_str().unwrap_or(""), h["tab"].as_str().unwrap_or("nothing")));
    let elements = wordsmith_script_model::Elements::of_doc(&ed.session.doc);
    let id = ed.session.doc.para_at(&ed.session.sel.focus).map(|p| elements.of_para(p).id.clone()).unwrap_or_default();
    let s = ed.session.run("element.settings", &json!({"element": id})).unwrap_or(Value::Null);
    ui.add_space(6.0);
    egui::Grid::new("element-summary").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
        let row = |ui: &mut Ui, k: &str, v: String| {
            hint(ui, k);
            ui.label(RichText::new(v).font(regular(12.0)));
            ui.end_row();
        };
        row(ui, "Font", format!("{} {} pt", s["font"].as_str().unwrap_or(""), s["size"].as_f64().unwrap_or(12.0)));
        row(ui, "Indents", format!("{:.2} in to {:.2} in", s["left"].as_f64().unwrap_or(0.0), s["right"].as_f64().unwrap_or(0.0)));
        row(ui, "Space before", format!("{} lines", s["spaceBefore"].as_f64().unwrap_or(0.0)));
        row(ui, "Alignment", s["align"].as_str().unwrap_or("").to_string());
        let mut style = Vec::new();
        for (k, l) in [("uppercase", "capitals"), ("bold", "bold"), ("italic", "italic"), ("underline", "underlined")] {
            if s[k].as_bool() == Some(true) {
                style.push(l);
            }
        }
        row(ui, "Style", if style.is_empty() { "plain".into() } else { style.join(", ") });
        row(ui, "Key", s["shortcut"].as_str().unwrap_or("—").to_string());
    });
    ui.add_space(6.0);
    if ui.button("Edit Element…").clicked() {
        ed.run("view.elementSettings", json!({}));
    }
    ui.separator();
    hint(ui, "Change this paragraph to");
    let mut pick = None;
    ui.horizontal_wrapped(|ui| {
        for def in elements.all().iter().filter(|d| !d.kind.is_outline()) {
            if ui.selectable_label(def.id == id, def.name.as_str()).clicked() {
                pick = Some(def.id.clone());
            }
        }
    });
    if let Some(p) = pick {
        ed.run("element.set", json!({"element": p}));
    }
}

fn scene(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let all = wordsmith_script_model::scenes(&ed.session.doc);
    let Some(sc) = wordsmith_script_model::scenes::scene_at(&all, ed.session.sel.focus.path.last()).cloned() else {
        hint(ui, "Put the caret in a scene to see it here.");
        return;
    };
    let Ok(props) = ed.session.run("scene.properties", &json!({"scene": sc.index})) else { return };
    ui.label(RichText::new(props["heading"].as_str().unwrap_or("")).font(medium(12.5)).color(t.text));
    let mut act: Option<(&str, Value)> = None;
    let drafts = &mut ed.panels.drafts;
    let key = |k: &str| format!("scene{}:{k}", sc.start);
    hint(ui, "Title");
    if let Some(v) = draft_field(ui, drafts, &key("title"), props["title"].as_str().unwrap_or(""), false, ui.available_width()) {
        act = Some(("scene.setProperties", json!({"scene": sc.index, "title": v})));
    }
    hint(ui, "Color");
    ui.horizontal_wrapped(|ui| {
        let current = props["color"].as_str().unwrap_or("");
        if ui.selectable_label(current.is_empty(), "None").clicked() {
            act = Some(("scene.setProperties", json!({"scene": sc.index, "color": ""})));
        }
        for (name, _) in wordsmith_script_model::scene_data::SCENE_COLORS {
            let c = color_of(name).unwrap_or(Color32::GRAY);
            let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
            ui.painter().rect_filled(r, 4.0, c);
            if current == name {
                ui.painter().rect_stroke(r, 4.0, Stroke::new(2.0, t.text), egui::StrokeKind::Outside);
            }
            if resp.on_hover_text(name).clicked() {
                act = Some(("scene.setProperties", json!({"scene": sc.index, "color": name})));
            }
        }
    });
    hint(ui, "Summary");
    if let Some(v) = draft_field(ui, drafts, &key("summary"), props["summary"].as_str().unwrap_or(""), true, ui.available_width()) {
        act = Some(("scene.setSummary", json!({"scene": sc.index, "text": v})));
    }
    ui.separator();
    ui.label(RichText::new("In this scene").font(medium(12.5)));
    let speaking: Vec<String> =
        props["speaking"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
    for who in props["cast"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&who).font(medium(12.0)));
            if !speaking.contains(&who) {
                hint(ui, "does not speak");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(if speaking.contains(&who) { "Hide" } else { "×" }).clicked() {
                    act = Some(("scene.removeCharacter", json!({"scene": sc.index, "name": who})));
                }
            });
        });
        if let Some(v) = draft_field(ui, drafts, &key(&format!("arc:{who}")), props["arcs"][&who].as_str().unwrap_or(""), false, ui.available_width())
        {
            act = Some(("scene.setProperties", json!({"scene": sc.index, "arc": {"character": who, "text": v}})));
        }
    }
    for who in props["hidden"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(str::to_string)) {
        ui.horizontal(|ui| {
            hint(ui, &format!("{who} (hidden)"));
            if ui.small_button("Show").clicked() {
                act = Some(("scene.addCharacter", json!({"scene": sc.index, "name": who})));
            }
        });
    }
    hint(ui, "The line under each name is their arc beat in this scene.");
    let mut add = false;
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut ed.panels.new_name).hint_text("Someone who does not speak").desired_width(ui.available_width() - 50.0),
        );
        add = ui.button("Add").clicked();
    });
    if add && !ed.panels.new_name.trim().is_empty() {
        let name = std::mem::take(&mut ed.panels.new_name);
        act = Some(("scene.addCharacter", json!({"scene": sc.index, "name": name.trim()})));
    }
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn revisions(ed: &mut Editor, ui: &mut Ui) {
    let rev = wordsmith_script_model::revisions::Revisions::of_doc(&ed.session.doc);
    let mut act: Option<(&str, Value)> = None;
    let mut mode = rev.mode;
    if ui.checkbox(&mut mode, "Revision mode").on_hover_text("Mark what is typed and deleted in the current set").changed() {
        act = Some(("revision.mode", json!({"on": mode})));
    }
    if rev.sets.is_empty() {
        hint(ui, "No sets yet. Next Set starts Blue (the original draft is White).");
    }
    for (i, set) in rev.sets.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.radio(set.name == rev.current, "").on_hover_text("Make it the current set").clicked() {
                act = Some(("revision.setCurrent", json!({"name": set.name})));
            }
            let mut color = wordsmith_script_model::revisions::parse_color(&set.color).map_or([255, 255, 255], |c| [c.0, c.1, c.2]);
            if ui.color_edit_button_srgb(&mut color).changed() {
                act =
                    Some(("revision.configure", json!({"set": {"index": i, "color": format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])}})));
            }
            ui.label(RichText::new(&set.name).font(medium(12.5)));
            for (k, l, on) in [("bold", "B", set.bold), ("underline", "U", set.underline), ("strike", "S", set.strike)] {
                if ui.selectable_label(on, l).on_hover_text(format!("Revised text in {k}")).clicked() {
                    act = Some(("revision.configure", json!({"set": {"index": i, k: !on}})));
                }
            }
        });
    }
    ui.horizontal(|ui| {
        if ui.button("Next Set").clicked() {
            act = Some(("revision.next", json!({})));
        }
        if ui.button("All sets…").on_hover_text("Names, marks and rolling back").clicked() {
            ed.show_revisions = true;
        }
    });
    ui.separator();
    for (k, label, on) in [
        ("activeOnly", "Show only the current set", rev.active_only),
        ("showMarks", "Show revision marks", rev.show_marks),
        ("colorPages", "Color revised pages in PDF", rev.color_pages),
    ] {
        let mut b = on;
        if ui.checkbox(&mut b, label).changed() {
            act = Some(("revision.configure", json!({k: b})));
        }
    }
    ui.horizontal(|ui| {
        if ui.button("Previous revised").clicked() {
            act = Some(("revision.gotoPrevious", json!({})));
        }
        if ui.button("Next revised").clicked() {
            act = Some(("revision.gotoNext", json!({})));
        }
    });
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn page(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let pag = ed.paginated();
    let page = pag.to_output(&ed.session.sel.focus).and_then(|at| pag.layout.caret(&at)).map_or(1, |c| c.page + 1);
    let label = pag.labels.as_ref().and_then(|l| l.get(page - 1).cloned());
    let locks = wordsmith_script_model::locks::Locks::of_doc(&ed.session.doc);
    ui.label(RichText::new(format!("Page {}", label.clone().unwrap_or_else(|| page.to_string()))).font(semibold(16.0)).color(t.accent));
    hint(
        ui,
        &if locks.is_locked() {
            format!("Pages are locked (level {}).", locks.level())
        } else {
            format!("{} pages; not locked.", pag.layout.pages.len())
        },
    );
    let mut act: Option<(&str, Value)> = None;
    ui.horizontal_wrapped(|ui| {
        if !locks.is_locked() {
            if ui.button("Lock Pages").clicked() {
                act = Some(("lock.pages", json!({})));
            }
        } else {
            for (id, l) in [
                ("lock.relock", "Relock"),
                ("lock.lockPage", "Lock Page Here"),
                ("lock.unlockPage", "Unlock This Page"),
                ("lock.unlockLevel", "Unlock a Level"),
                ("lock.unlockAll", "Unlock All"),
            ] {
                if ui.button(l).clicked() {
                    act = Some((id, json!({})));
                }
            }
            if ui.button("Start ▲").on_hover_text("Move this page's start up a paragraph").clicked() {
                act = Some(("lock.adjustBreak", json!({"by": -1})));
            }
            if ui.button("Start ▼").on_hover_text("Move this page's start down a paragraph").clicked() {
                act = Some(("lock.adjustBreak", json!({"by": 1})));
            }
        }
    });
    if locks.is_locked() {
        ui.horizontal(|ui| {
            hint(ui, "This page's number");
            if let Some(v) = draft_field(ui, &mut ed.panels.drafts, "page-label", label.as_deref().unwrap_or(""), false, 70.0) {
                act = Some(("lock.setLabel", json!({"label": v})));
            }
        });
    }
    ui.separator();
    ui.label(RichText::new("Scene numbers").font(medium(12.5)));
    let n = wordsmith_script_model::scene_numbers::SceneNumbers::of_doc(&ed.session.doc);
    ui.horizontal_wrapped(|ui| {
        let mut show = n.show;
        if ui.checkbox(&mut show, "Show").changed() {
            act = Some(("scene.showNumbers", json!({"show": show})));
        }
        let mut locked = n.locked;
        if ui.checkbox(&mut locked, "Keep (lock)").changed() {
            act = Some(("scene.lockNumbers", json!({"lock": locked})));
        }
        if ui.button("Renumber").clicked() {
            act = Some(("scene.renumber", json!({})));
        }
        if ui.button("Options…").clicked() {
            act = Some(("view.sceneNumbers", json!({})));
        }
    });
    ui.horizontal(|ui| {
        hint(ui, "This scene's number");
        if let Some(v) = draft_field(ui, &mut ed.panels.drafts, "scene-number", "", false, 70.0) {
            act = Some(("scene.setNumber", json!({"number": v})));
        }
    });
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn changes(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let list = ed.session.run("track.list", &json!({})).unwrap_or(Value::Null);
    let mut act: Option<(&str, Value)> = None;
    let mut on = list["on"].as_bool().unwrap_or(false);
    if ui.checkbox(&mut on, "Track changes").changed() {
        act = Some(("track.toggle", json!({"on": on})));
    }
    let items = list["changes"].as_array().cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        if ui.add_enabled(!items.is_empty(), egui::Button::new("Accept All")).clicked() {
            act = Some(("track.acceptAll", json!({})));
        }
        if ui.add_enabled(!items.is_empty(), egui::Button::new("Reject All")).clicked() {
            act = Some(("track.rejectAll", json!({})));
        }
    });
    if items.is_empty() {
        hint(ui, "No tracked changes.");
    }
    let mut pick: Option<(usize, usize, usize, bool)> = None;
    for c in items.iter().take(500) {
        let inserted = c["kind"] == "inserted";
        let (block, a, b) =
            (c["block"].as_u64().unwrap_or(0) as usize, c["start"].as_u64().unwrap_or(0) as usize, c["end"].as_u64().unwrap_or(0) as usize);
        egui::Frame::NONE.fill(t.input).stroke(Stroke::new(1.0, t.border)).corner_radius(6).inner_margin(6).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(if inserted { "Added" } else { "Deleted" }).font(medium(11.0)).color(if inserted {
                    t.accent
                } else {
                    t.danger
                }));
                hint(ui, &format!("{} {}", c["author"].as_str().unwrap_or(""), c["date"].as_str().unwrap_or("")));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✗").on_hover_text("Reject").clicked() {
                        pick = Some((block, a, b, false));
                    }
                    if ui.small_button("✓").on_hover_text("Accept").clicked() {
                        pick = Some((block, a, b, true));
                    }
                });
            });
            let text = c["text"].as_str().unwrap_or("");
            let rt = RichText::new(text).font(regular(12.0));
            if ui.add(egui::Label::new(if inserted { rt.underline() } else { rt.strikethrough() }).truncate().sense(Sense::click())).clicked() {
                ed.session.sel = jcj_engine::Selection {
                    anchor: wordsmith_script_model::data::body_pos(block, a),
                    focus: wordsmith_script_model::data::body_pos(block, b),
                };
                ed.request_scroll_to_caret();
            }
        });
    }
    if let Some((block, a, b, accept)) = pick {
        ed.session.sel = jcj_engine::Selection {
            anchor: wordsmith_script_model::data::body_pos(block, a),
            focus: wordsmith_script_model::data::body_pos(block, b),
        };
        act = Some((if accept { "track.accept" } else { "track.reject" }, json!({})));
    }
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn checker(ed: &mut Editor, ui: &mut Ui) {
    let found = ed.session.run("check.run", &json!({})).unwrap_or(Value::Null);
    let ignored = ed.panels.ignored.clone();
    let issues: Vec<Value> = found["issues"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !ignored.contains(&(i["block"].as_u64().unwrap_or(0), i["rule"].as_str().unwrap_or("").to_string())))
        .collect();
    hint(ui, &if issues.is_empty() { "No format issues.".to_string() } else { format!("{} issues", issues.len()) });
    let mut act: Option<(&str, Value)> = None;
    for i in issues.iter().take(300) {
        ui.horizontal(|ui| {
            let text = i["text"].as_str().unwrap_or("");
            if ui
                .add(egui::Label::new(RichText::new(i["message"].as_str().unwrap_or("")).font(regular(12.0))).truncate().sense(Sense::click()))
                .on_hover_text(text)
                .clicked()
            {
                act = Some(("check.goto", json!({"block": i["block"]})));
            }
        });
        ui.horizontal(|ui| {
            if i["fixable"].as_bool() == Some(true) && ui.small_button("Fix").clicked() {
                act = Some(("check.fix", json!({"block": i["block"], "rule": i["rule"]})));
            }
            if ui.small_button("Go").clicked() {
                act = Some(("check.goto", json!({"block": i["block"]})));
            }
            if ui.small_button("Ignore").clicked() {
                ed.panels.ignored.push((i["block"].as_u64().unwrap_or(0), i["rule"].as_str().unwrap_or("").to_string()));
            }
        });
        ui.separator();
    }
    let settings = ed.session.run("check.settings", &json!({})).unwrap_or(Value::Null);
    let current = settings["beforePrint"].as_str().unwrap_or("off").to_string();
    hint(ui, "Before printing:");
    ui.horizontal(|ui| {
        for (v, l) in [("off", "Don't check"), ("ask", "Ask"), ("scan", "Check")] {
            if ui.selectable_label(current == v, l).clicked() {
                act = Some(("check.configure", json!({"beforePrint": v})));
            }
        }
    });
    if let Some((c, p)) = act {
        ed.run(c, p);
    }
}

fn goals(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::get(ui.ctx());
    let today = wordsmith_script_model::reports_more::date(crate::now_secs());
    let day = ed.stats.data.days.get(&today).cloned().unwrap_or_default();
    let g = ed.stats.data.goals.clone();
    let bar = |ui: &mut Ui, label: &str, have: f32, goal: f32, unit: &str| {
        ui.label(RichText::new(format!("{label}: {have:.0} of {goal:.0} {unit}")).font(regular(12.0)));
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
        ui.painter().rect_filled(r, 4.0, t.hover);
        let f = if goal > 0.0 { (have / goal).clamp(0.0, 1.0) } else { 0.0 };
        ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(r.width() * f, r.height())), 4.0, t.accent);
        ui.add_space(4.0);
    };
    bar(ui, "Pages today", day.pages as f32, g.pages_per_day, "pages");
    bar(ui, "Time today", day.seconds as f32 / 60.0, g.minutes_per_day as f32, "minutes");
    ui.label(RichText::new(format!("{} words today", day.words)).font(regular(12.0)));
    ui.add_space(6.0);
    let sprint = crate::workspace::sprint_left(ed);
    ui.horizontal(|ui| {
        let label = match sprint {
            Some(left) => format!("Stop sprint ({}:{:02})", left.as_secs() / 60, left.as_secs() % 60),
            None => format!("Start a {}-minute sprint", ed.stats.data.sprint_minutes),
        };
        if ui.button(label).clicked() {
            crate::workspace::toggle_sprint(ed);
        }
    });
    if ui.button("Statistics…").clicked() {
        ed.run("view.stats", json!({}));
    }
}
