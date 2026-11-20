// SPDX-License-Identifier: MIT OR Apache-2.0
//! The File page (v0.3 plan, M18): a full-window page in place of the script for new, open,
//! save, export, print, compare, preferences, keys and about. Every button runs a command.

use egui::{Align2, Color32, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::editor::Editor;
use crate::theme::{BODY, CAPTION, Tokens, medium, regular, semibold};

/// A page of the backstage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    New,
    Open,
    Save,
    Export,
    Print,
    Compare,
    Preferences,
    Keys,
    About,
}

const PAGES: [(Page, &str, &str); 9] = [
    (Page::New, "New", "plus"),
    (Page::Open, "Open", "open"),
    (Page::Save, "Save", "save"),
    (Page::Export, "Export", "export"),
    (Page::Print, "Print", "print"),
    (Page::Compare, "Compare", "compare"),
    (Page::Preferences, "Preferences", "options"),
    (Page::Keys, "Keys", "macro"),
    (Page::About, "About", "help"),
];

/// Every command the File page runs (the coverage test checks menus against this and the ribbon).
pub const BACKSTAGE_COMMANDS: &[&str] = &[
    "file.new",
    "view.templates",
    "file.newTab",
    "file.open",
    "file.import",
    "file.openPath",
    "file.save",
    "file.saveAs",
    "file.saveAsTemplate",
    "file.revert",
    "file.closeTab",
    "file.exportPdfAs",
    "file.exportRevisedPdfAs",
    "file.exportFountain",
    "file.exportText",
    "file.exportRtf",
    "file.exportHtml",
    "storyboard.exportPdf",
    "cards.exportPdf",
    "view.print",
    "file.print",
    "file.printPreview",
    "file.printRevised",
    "compare.withFile",
    "app.preferences",
    "view.keys",
    "help.about",
    "help.checkForUpdates",
    "file.recoverPrompt",
    "view.start",
];

/// Draw the backstage in `ui` (the central panel).
pub fn show(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let Some(page) = ed.chrome.backstage else { return };
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        ed.chrome.backstage = None;
        return;
    }
    let full = ui.max_rect();
    // The column of pages, in the accent color.
    let col = egui::Rect::from_min_size(full.min, vec2(200.0, full.height()));
    ui.painter().rect_filled(col, 0.0, t.accent);
    let mut picked = None;
    let mut back = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(col.shrink2(vec2(10.0, 14.0))), |ui| {
        let ink = Color32::WHITE;
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, 6.0, Color32::from_white_alpha(30));
        }
        ui.painter().text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, "←  Back to the script", medium(BODY), ink);
        back = resp.on_hover_text("Escape").clicked();
        ui.add_space(10.0);
        for (p, label, icon) in PAGES {
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if p == page {
                ui.painter().rect_filled(r, 6.0, Color32::from_white_alpha(45));
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 6.0, Color32::from_white_alpha(25));
            }
            crate::script_icons::paint(
                ui.painter(),
                egui::Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(18.0, 18.0)),
                icon,
                ink,
                ink,
            );
            ui.painter().text(pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, label, medium(13.5), ink);
            if resp.clicked() {
                picked = Some(p);
            }
        }
    });
    if back {
        ed.chrome.backstage = None;
        return;
    }
    if let Some(p) = picked {
        ed.chrome.backstage = Some(p);
    }
    let content = egui::Rect::from_min_max(pos2(col.max.x + 32.0, full.min.y + 20.0), pos2(full.max.x - 24.0, full.max.y - 10.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        egui::ScrollArea::vertical().id_salt("backstage").auto_shrink(false).show(ui, |ui| match page {
            Page::New => new(ed, ui),
            Page::Open => open(ed, ui),
            Page::Save => save(ed, ui),
            Page::Export => export(ed, ui),
            Page::Print => print(ed, ui),
            Page::Compare => compare(ed, ui),
            Page::Preferences => {
                heading(ui, "Preferences");
                crate::dialogs::prefs_body(ed, ui);
            }
            Page::Keys => {
                heading(ui, "Keys");
                if ed.tools.keys.is_none() {
                    ed.tools.keys = Some(Default::default());
                }
                crate::workspace::keys_body(ed, ui);
            }
            Page::About => about(ed, ui),
        });
    });
}

fn heading(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(RichText::new(text).font(semibold(26.0)).color(t.text));
    ui.add_space(12.0);
}

fn sub(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(10.0);
    ui.label(RichText::new(text).font(semibold(15.0)).color(t.text));
    ui.add_space(4.0);
}

/// A wide button with an icon, a title and a line under it; runs `id` (closing the page).
fn action(ed: &mut Editor, ui: &mut Ui, id: &'static str, params: serde_json::Value, title: &str, about: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let ok = ed.session.why_disabled(id).is_none();
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(560.0), 50.0), if ok { Sense::click() } else { Sense::hover() });
    ui.painter().rect_filled(r, 8.0, if resp.hovered() && ok { t.hover } else { t.surface_high });
    ui.painter().rect_stroke(r, 8.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let ink = if ok { t.text } else { t.text_dim };
    crate::script_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(pos2(r.min.x + 26.0, r.center().y), vec2(24.0, 24.0)),
        crate::script_icons::for_command(id),
        ink,
        t.accent,
    );
    ui.painter().text(pos2(r.min.x + 50.0, r.min.y + 17.0), Align2::LEFT_CENTER, title, medium(13.5), ink);
    ui.painter().text(pos2(r.min.x + 50.0, r.min.y + 35.0), Align2::LEFT_CENTER, about, regular(CAPTION), t.text_dim);
    ui.add_space(6.0);
    let resp = resp.on_hover_text(crate::ribbon::widgets::tip(ed, id, ""));
    if resp.clicked() && ok {
        ed.chrome.backstage = None;
        ed.run(id, params);
        return true;
    }
    false
}

/// The first lines of a template, as its preview (built once; templates never change).
fn preview(id: &str) -> String {
    static CACHE: std::sync::OnceLock<std::collections::HashMap<&'static str, String>> = std::sync::OnceLock::new();
    let all = CACHE.get_or_init(|| {
        wordsmith_script::templates::TEMPLATES
            .iter()
            .map(|t| {
                let text = wordsmith_script::templates::build(t.id).map_or(String::new(), |doc| {
                    doc.body
                        .iter()
                        .filter_map(|b| b.as_para())
                        .map(|p| p.text.clone())
                        .filter(|x| !x.trim().is_empty())
                        .take(3)
                        .collect::<Vec<_>>()
                        .join("\n")
                });
                (t.id, if text.is_empty() { "INT. —\n\n(an empty page, ready)".to_string() } else { text })
            })
            .collect()
    });
    all.get(id).cloned().unwrap_or_default()
}

fn new(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, "New");
    let mut picked: Option<&str> = None;
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(RichText::new("New empty script").font(medium(BODY))).min_size(vec2(160.0, 30.0))).clicked() {
            picked = Some("screenplay");
        }
        if ui.add(egui::Button::new(RichText::new("New tab").font(medium(BODY))).min_size(vec2(120.0, 30.0))).clicked() {
            ed.chrome.backstage = None;
            ed.run("file.newTab", json!({}));
        }
    });
    if !ed.recent.is_empty() {
        sub(ui, "Recent");
        ui.horizontal_wrapped(|ui| {
            for r in ed.recent.clone().into_iter().take(6) {
                let name = std::path::Path::new(&r).file_name().and_then(|n| n.to_str()).unwrap_or(&r).to_string();
                if ui.link(name).on_hover_text(&r).clicked() {
                    ed.chrome.backstage = None;
                    ed.run("file.openPath", json!({"path": r}));
                }
            }
        });
    }
    let mut cats: Vec<&str> = wordsmith_script::templates::TEMPLATES.iter().map(|x| x.category).collect();
    cats.dedup();
    for cat in cats {
        sub(ui, cat);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
            for tpl in wordsmith_script::templates::TEMPLATES.iter().filter(|x| x.category == cat) {
                if template_card(ui, &t, tpl.id, tpl.name, tpl.about) {
                    picked = Some(tpl.id);
                }
            }
        });
    }
    sub(ui, "First steps");
    for tip in [
        "Return starts the next element and Tab changes this one: a heading leads to action, a name to dialogue.",
        "Type INT, press Tab, type the place, press Tab, type the time of day.",
        "The Write tab's element tiles show each element's key and what Return and Tab make.",
        "The Navigator on the left lists scenes, notes and characters; the Inspector on the right shows the scene and element at the caret.",
        "F10 shows a letter on every ribbon tab and control: type the letters to use them from the keyboard.",
    ] {
        ui.label(RichText::new(format!("•  {tip}")).color(t.text_dim));
    }
    let mut show = ed.tools.show_start_on_launch;
    if ui.checkbox(&mut show, "Show this page when the app starts").changed() {
        ed.tools.show_start_on_launch = show;
    }
    if let Some(id) = picked {
        ed.chrome.backstage = None;
        // Unsaved work stays: the new script opens in a tab of its own.
        if ed.session.dirty {
            ed.new_tab();
        }
        ed.run("file.newFromTemplate", json!({"template": id}));
    }
}

/// A template's card: a little page with its first lines, its name and what it is for.
fn template_card(ui: &mut Ui, t: &Tokens, id: &str, name: &str, about: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(210.0, 140.0), Sense::click());
    ui.painter().rect_filled(r, 8.0, if resp.hovered() { t.hover } else { t.surface_high });
    ui.painter().rect_stroke(r, 8.0, Stroke::new(1.0, if resp.hovered() { t.accent } else { t.border }), egui::StrokeKind::Inside);
    let page = egui::Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(r.width() - 24.0, 78.0));
    ui.painter().rect_filled(page, 3.0, Color32::WHITE);
    ui.painter().rect_stroke(page, 3.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::simple(preview(id), egui::FontId::monospace(8.5), Color32::from_gray(60), page.width() - 12.0);
    job.wrap.max_rows = 6;
    ui.painter().galley(page.min + vec2(6.0, 6.0), ui.painter().layout_job(job), Color32::from_gray(60));
    ui.painter().text(pos2(r.min.x + 12.0, r.min.y + 104.0), Align2::LEFT_CENTER, name, medium(13.0), t.text);
    let mut job = egui::text::LayoutJob::simple(about.to_string(), regular(10.5), t.text_dim, r.width() - 24.0);
    job.wrap.max_rows = 1;
    ui.painter().galley(pos2(r.min.x + 12.0, r.min.y + 116.0), ui.painter().layout_job(job), t.text_dim);
    resp.on_hover_text(about).clicked()
}

fn open(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, "Open");
    action(ed, ui, "file.open", json!({}), "Open…", "A Wordsmith Script file, a template or a Fountain script.");
    action(ed, ui, "file.import", json!({}), "Import…", "Plain text, RTF or a PDF made from text.");
    sub(ui, "Recent");
    if ed.recent.is_empty() {
        ui.label(RichText::new("Scripts you open or save show here.").color(t.text_dim));
    }
    let mut pick = None;
    for r in ed.recent.clone() {
        let name = std::path::Path::new(&r).file_name().and_then(|n| n.to_str()).unwrap_or(&r).to_string();
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(560.0), 38.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 6.0, t.hover);
        }
        ui.painter().text(pos2(rect.min.x + 10.0, rect.min.y + 12.0), Align2::LEFT_CENTER, &name, medium(13.0), t.text);
        ui.painter().text(pos2(rect.min.x + 10.0, rect.min.y + 28.0), Align2::LEFT_CENTER, &r, regular(10.5), t.text_dim);
        if resp.clicked() {
            pick = Some(r);
        }
    }
    sub(ui, "Unsaved work");
    action(ed, ui, "file.recoverPrompt", json!({}), "Recover Unsaved Scripts…", "Copies kept when the app closed without saving.");
    if let Some(path) = pick {
        ed.chrome.backstage = None;
        ed.run("file.openPath", json!({"path": path}));
    }
}

fn save(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, "Save");
    let where_ = ed.session.path.as_ref().map_or("Not saved to a file yet.".to_string(), |p| p.display().to_string());
    ui.label(RichText::new(where_).color(t.text_dim));
    ui.add_space(8.0);
    action(ed, ui, "file.save", json!({}), "Save", "Keep the changes in the script's file.");
    action(ed, ui, "file.saveAs", json!({}), "Save As…", "Keep the script in a new file.");
    action(ed, ui, "file.saveAsTemplate", json!({}), "Save as Template…", "Start new scripts from this one.");
    action(ed, ui, "file.revert", json!({}), "Revert", "Put back the script as last saved.");
    action(ed, ui, "file.closeTab", json!({}), "Close Tab", "Close this script's tab.");
}

fn export(ed: &mut Editor, ui: &mut Ui) {
    heading(ui, "Export");
    action(ed, ui, "file.exportPdfAs", json!({}), "PDF…", "The pages as they print.");
    action(ed, ui, "file.exportRevisedPdfAs", json!({}), "Revised Pages as PDF…", "Only the pages with revised text.");
    action(ed, ui, "file.exportFountain", json!({}), "Fountain…", "Plain-text screenplay format other apps read.");
    action(ed, ui, "file.exportRtf", json!({}), "Rich Text (RTF)…", "For word processors.");
    action(ed, ui, "file.exportText", json!({}), "Plain Text…", "The words alone.");
    action(ed, ui, "file.exportHtml", json!({}), "Web Page (HTML)…", "To read in a browser.");
    sub(ui, "Planning");
    let board = ed.board.current;
    action(ed, ui, "storyboard.exportPdf", json!({"board": board}), "Story Board as PDF…", "The board showing, on one page.");
    action(ed, ui, "cards.exportPdf", json!({}), "Index Cards as PDF…", "A grid of cards on Letter pages.");
}

fn print(ed: &mut Editor, ui: &mut Ui) {
    heading(ui, "Print");
    // The print choices, as the Print window has them.
    let mut c = ed.chrome.print.clone();
    let sets: Vec<String> = wordsmith_script::revisions::Revisions::of_doc(&ed.session.doc).sets.iter().map(|s| s.name.clone()).collect();
    let characters: Vec<String> = ed
        .session
        .run("character.list", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    egui::Grid::new("bs-print").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
        ui.label("Print");
        ui.horizontal(|ui| {
            for (k, l) in [("all", "Everything"), ("pages", "Pages"), ("scenes", "Scenes")] {
                ui.selectable_value(&mut c.range, k, l);
            }
            if c.range != "all" {
                ui.add(egui::TextEdit::singleline(&mut c.from).desired_width(50.0).hint_text("from"));
                ui.label("to");
                ui.add(egui::TextEdit::singleline(&mut c.to).desired_width(50.0).hint_text("to"));
            }
        });
        ui.end_row();
        ui.label("Pages");
        ui.horizontal(|ui| {
            for (k, l) in [("all", "All"), ("revised", "Revised only"), ("unrevised", "Unrevised only"), ("set", "One set")] {
                ui.selectable_value(&mut c.which, k, l);
            }
        });
        ui.end_row();
        if c.which == "set" {
            ui.label("Set");
            egui::ComboBox::from_id_salt("bs-print-set").selected_text(sets.get(c.set).cloned().unwrap_or_else(|| "—".into())).show_ui(ui, |ui| {
                for (i, name) in sets.iter().enumerate() {
                    ui.selectable_value(&mut c.set, i, name.as_str());
                }
            });
            ui.end_row();
        }
        ui.label("Sides for");
        egui::ComboBox::from_id_salt("bs-print-sides").selected_text(if c.sides.is_empty() { "The whole script" } else { c.sides.as_str() }).show_ui(
            ui,
            |ui| {
                ui.selectable_value(&mut c.sides, String::new(), "The whole script");
                for name in &characters {
                    ui.selectable_value(&mut c.sides, name.clone(), name.as_str());
                }
            },
        );
        ui.end_row();
        ui.label("Copies");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut c.copies).range(1..=999));
            ui.checkbox(&mut c.collate, "Collate");
            ui.checkbox(&mut c.reverse, "Last page first");
        });
        ui.end_row();
        ui.label("Include");
        ui.horizontal(|ui| {
            ui.checkbox(&mut c.title_page, "Title page");
            ui.checkbox(&mut c.marks, "Revision marks");
        });
        ui.end_row();
    });
    ui.add_space(10.0);
    let params = c.params();
    ed.chrome.print = c;
    action(ed, ui, "file.print", params.clone(), "Print", "Send the pages to the printer.");
    action(ed, ui, "file.printPreview", params.clone(), "Preview", "Open the pages in your PDF viewer.");
    // The PDF's file dialog runs the export with these choices.
    if action(ed, ui, "file.exportPdfAs", params.clone(), "Save as PDF…", "Keep the pages as a PDF file.") {
        ed.dialog_params = Some(params);
    }
}

fn compare(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, "Compare");
    ui.label(RichText::new("Pick an earlier version of this script. The comparison opens as a new script: added text marked as inserted, removed text as deleted, and moved scenes with a note.").color(t.text_dim));
    ui.add_space(8.0);
    action(ed, ui, "compare.withFile", json!({}), "Compare with Another Version…", "Scenes are matched first, then their paragraphs.");
}

fn about(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, "About");
    ui.label(RichText::new(crate::about_text()).font(regular(BODY)).color(t.text));
    sub(ui, "Updates");
    crate::updates::actions(ed, ui);
    ui.checkbox(&mut ed.updates.check_at_start, "Check for updates when the app starts");
}
