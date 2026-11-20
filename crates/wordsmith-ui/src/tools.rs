// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tool windows added after v0.1 (SPEC-2): Go To, bookmarks, the format checker and the
//! reformat tool. Each changes the script only by running commands. A window opens through a
//! UI request (`{"open": "tool.<name>"}`) from its `view.*` command.

use egui::Context;
use serde_json::{Value, json};
use wordsmith_script::Elements;

use crate::editor::Editor;

/// The format checker's window: the issues found and the one shown.
#[derive(Clone, Debug, Default)]
pub struct Checker {
    pub issues: Vec<Value>,
    pub at: usize,
    /// Issues skipped with Ignore (block, rule) until the next scan.
    pub ignored: Vec<(u64, String)>,
}

/// Open tool windows.
#[derive(Default)]
pub struct Tools {
    /// Go To: the page and scene typed.
    pub goto: Option<(String, String)>,
    pub bookmarks: bool,
    /// The name typed for a new bookmark, and whether it is a label.
    pub bookmark_name: (String, bool),
    pub checker: Option<Checker>,
    pub reformat: bool,
}

/// Act on a UI request to open a tool window. Returns whether it was one.
pub fn open(ed: &mut Editor, what: &str) -> bool {
    match what {
        "tool.goto" => ed.tools.goto = Some(Default::default()),
        "tool.bookmarks" => ed.tools.bookmarks = !ed.tools.bookmarks,
        "tool.checker" => {
            let mut c = Checker::default();
            rescan(ed, &mut c);
            ed.tools.checker = Some(c);
        }
        "tool.reformat" => {
            ed.tools.reformat = true;
            ed.run("paragraph.next", json!({}));
            ed.run("paragraph.previous", json!({}));
        }
        _ => return false,
    }
    true
}

fn rescan(ed: &mut Editor, c: &mut Checker) {
    let found = ed.session.run("check.run", &json!({})).unwrap_or(Value::Null);
    c.issues = found["issues"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !c.ignored.contains(&(i["block"].as_u64().unwrap_or(0), i["rule"].as_str().unwrap_or("").to_string())))
        .collect();
    c.at = c.at.min(c.issues.len().saturating_sub(1));
    if let Some(i) = c.issues.get(c.at) {
        let block = i["block"].clone();
        let _ = ed.session.run("check.goto", &json!({"block": block}));
    }
}

fn goto_window(ed: &mut Editor, ctx: &Context) {
    let Some((mut page, mut scene)) = ed.tools.goto.take() else { return };
    let mut open = true;
    let mut action: Option<(&str, Value)> = None;
    egui::Window::new("Go To").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
        egui::Grid::new("goto").num_columns(3).show(ui, |ui| {
            ui.label("Page");
            let r = ui.add(egui::TextEdit::singleline(&mut page).desired_width(80.0));
            if ui.button("Go").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                action = Some(("goto.page", json!({"page": page.trim()})));
            }
            ui.end_row();
            ui.label("Scene");
            let r = ui.add(egui::TextEdit::singleline(&mut scene).desired_width(80.0));
            if ui.button("Go").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                action = Some(("goto.scene", json!({"scene": scene.trim()})));
            }
            ui.end_row();
        });
        let marks = ed.session.run("bookmark.list", &json!({})).unwrap_or(Value::Null);
        let list = marks["bookmarks"].as_array().cloned().unwrap_or_default();
        if !list.is_empty() {
            ui.separator();
            ui.label("Bookmarks and labels");
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for b in list {
                    let kind = if b["label"].as_bool() == Some(true) { "Label" } else { "Bookmark" };
                    let text = format!("{kind}: {}  (page {})", b["name"].as_str().unwrap_or(""), b["page"].as_str().unwrap_or("?"));
                    if ui.selectable_label(false, text).clicked() {
                        action = Some(("bookmark.goto", json!({"id": b["id"]})));
                    }
                }
            });
        }
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
        if ed.message.is_empty() {
            open = false;
        }
    }
    if open {
        ed.tools.goto = Some((page, scene));
    }
}

fn bookmarks_window(ed: &mut Editor, ctx: &Context) {
    if !ed.tools.bookmarks {
        return;
    }
    let mut open = true;
    let mut action: Option<(&str, Value)> = None;
    let (mut name, mut label) = std::mem::take(&mut ed.tools.bookmark_name);
    egui::Window::new("Bookmarks").open(&mut open).default_width(320.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut name).hint_text("Name").desired_width(160.0));
            ui.checkbox(&mut label, "Label");
            if ui.button("Add here").clicked() {
                action = Some(("bookmark.add", json!({"name": name.trim(), "label": label})));
                name.clear();
            }
        });
        ui.separator();
        let marks = ed.session.run("bookmark.list", &json!({})).unwrap_or(Value::Null);
        let list = marks["bookmarks"].as_array().cloned().unwrap_or_default();
        if list.is_empty() {
            ui.label("No bookmarks yet. Put the caret on a paragraph and add one.");
        }
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for b in list {
                ui.horizontal(|ui| {
                    let kind = if b["label"].as_bool() == Some(true) { " (label)" } else { "" };
                    let text = format!("{}{kind} — page {}", b["name"].as_str().unwrap_or(""), b["page"].as_str().unwrap_or("?"));
                    if ui.link(text).clicked() {
                        action = Some(("bookmark.goto", json!({"id": b["id"]})));
                    }
                    if ui.small_button("Delete").clicked() {
                        action = Some(("bookmark.delete", json!({"id": b["id"]})));
                    }
                });
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Previous").clicked() {
                action = Some(("bookmark.previous", json!({})));
            }
            if ui.button("Next").clicked() {
                action = Some(("bookmark.next", json!({})));
            }
        });
    });
    ed.tools.bookmark_name = (name, label);
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
    }
    ed.tools.bookmarks = open;
}

fn checker_window(ed: &mut Editor, ctx: &Context) {
    let Some(mut c) = ed.tools.checker.take() else { return };
    let mut open = true;
    let mut fix = false;
    let mut step: i64 = 0;
    let mut ignore = false;
    egui::Window::new("Check Format").open(&mut open).resizable(false).default_width(360.0).show(ctx, |ui| {
        match c.issues.get(c.at) {
            None => {
                ui.label("No format issues found.");
            }
            Some(issue) => {
                ui.label(format!("Issue {} of {}", c.at + 1, c.issues.len()));
                ui.strong(issue["message"].as_str().unwrap_or(""));
                let text = issue["text"].as_str().unwrap_or("");
                ui.label(if text.trim().is_empty() { "(empty paragraph)".to_string() } else { format!("“{text}”") });
                if issue["fixable"].as_bool() != Some(true) {
                    ui.label("Fix this one in the script, then check again.");
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(issue["fixable"].as_bool() == Some(true), egui::Button::new("Fix")).clicked() {
                        fix = true;
                    }
                    if ui.button("Ignore").clicked() {
                        ignore = true;
                    }
                    if ui.button("Previous").clicked() {
                        step = -1;
                    }
                    if ui.button("Next").clicked() {
                        step = 1;
                    }
                });
            }
        }
        if ui.button("Check again").clicked() {
            c.ignored.clear();
            step = 0;
            fix = false;
            ignore = false;
            c.at = 0;
            rescan(ed, &mut c);
        }
        let settings = ed.session.run("check.settings", &json!({})).unwrap_or(Value::Null);
        let current = settings["beforePrint"].as_str().unwrap_or("off").to_string();
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Before printing:");
            for (v, l) in [("off", "Don't check"), ("ask", "Ask"), ("scan", "Check")] {
                if ui.selectable_label(current == v, l).clicked() {
                    let _ = ed.session.run("check.configure", &json!({"beforePrint": v}));
                }
            }
        });
    });
    if let Some(issue) = c.issues.get(c.at).cloned() {
        if fix {
            ed.run("check.fix", json!({"block": issue["block"], "rule": issue["rule"]}));
            rescan(ed, &mut c);
        } else if ignore {
            c.ignored.push((issue["block"].as_u64().unwrap_or(0), issue["rule"].as_str().unwrap_or("").to_string()));
            rescan(ed, &mut c);
        } else if step != 0 {
            let n = c.issues.len() as i64;
            c.at = (c.at as i64 + step).rem_euclid(n.max(1)) as usize;
            let block = c.issues.get(c.at).map(|i| i["block"].clone()).unwrap_or(Value::Null);
            ed.run("check.goto", json!({"block": block}));
        }
    }
    if open {
        ed.tools.checker = Some(c);
    }
}

/// Keys the reformat tool takes: a digit for an element in menu order, D to delete.
fn reformat_window(ed: &mut Editor, ctx: &Context) {
    if !ed.tools.reformat {
        return;
    }
    let mut open = true;
    let elements: Vec<(String, String)> = Elements::of_doc(&ed.session.doc).all().iter().take(10).map(|d| (d.id.clone(), d.name.clone())).collect();
    let mut action: Option<(&str, Value)> = None;
    let mut advance = false;
    egui::Window::new("Reformat").open(&mut open).resizable(false).show(ctx, |ui| {
        let hint = ed.session.run("element.hint", &json!({})).unwrap_or(Value::Null);
        ui.label(format!("This paragraph is {}.", hint["element"].as_str().unwrap_or("?")));
        ui.label("Press a number to make it that element, D to delete it, or the arrows to move.");
        egui::Grid::new("reformat").num_columns(2).show(ui, |ui| {
            for (n, (id, name)) in elements.iter().enumerate() {
                if ui.button(format!("{n}")).clicked() {
                    action = Some(("element.set", json!({"element": id})));
                    advance = true;
                }
                ui.label(name);
                ui.end_row();
            }
        });
        let keys = [
            egui::Key::Num0,
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ];
        ui.input(|i| {
            for (n, k) in keys.iter().enumerate() {
                if i.key_pressed(*k)
                    && let Some((id, _)) = elements.get(n)
                {
                    action = Some(("element.set", json!({"element": id})));
                    advance = true;
                }
            }
            if i.key_pressed(egui::Key::D) {
                action = Some(("paragraph.delete", json!({})));
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                action = Some(("paragraph.next", json!({})));
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                action = Some(("paragraph.previous", json!({})));
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Previous").clicked() {
                action = Some(("paragraph.previous", json!({})));
            }
            if ui.button("Next").clicked() {
                action = Some(("paragraph.next", json!({})));
            }
            if ui.button("Delete").clicked() {
                action = Some(("paragraph.delete", json!({})));
            }
        });
    });
    if let Some((cmd, params)) = action {
        ed.run(cmd, params);
        if advance {
            ed.run("paragraph.next", json!({}));
        } else if cmd == "paragraph.delete" {
            // Select the paragraph that took the deleted one's place.
            ed.run("paragraph.next", json!({}));
            ed.run("paragraph.previous", json!({}));
        }
    }
    ed.tools.reformat = open;
}

/// Whether a tool window wants the keyboard (the reformat tool takes digits and letters).
pub fn has_keyboard(ed: &Editor) -> bool {
    ed.tools.reformat
}

pub fn show(ed: &mut Editor, ctx: &Context) {
    goto_window(ed, ctx);
    bookmarks_window(ed, ctx);
    checker_window(ed, ctx);
    reformat_window(ed, ctx);
}
