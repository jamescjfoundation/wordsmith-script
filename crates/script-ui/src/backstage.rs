// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's part of the File page (decision 0037). The shell draws the page and what every app
//! has (new, open and recent, save, export and print headings, preferences, keys, about, and the
//! templates' tiles from [`templates`]); Script adds a new tab and first steps to New, Import to
//! Open, templates, revert and tabs to Save, its formats to Export, its print choices to Print,
//! and a Compare page. Every button runs a command.

use egui::{RichText, Ui, vec2};
use jcj_shell::backstage::{action, heading, sub};
use jcj_shell::widgets::{self, Control};
use jcj_shell::{Cmd, Page, Shell, TemplatePreview, TemplateSpec};
use serde_json::json;

use crate::editor::Editor;
use crate::theme::{BODY, medium};

/// Draw Script's part of File page `page`.
pub fn page_ui(ed: &mut Editor, page: &Page, ui: &mut Ui, shell: &mut Shell) {
    match page {
        Page::New => new(ed, ui, shell),
        Page::Open => open(ed, ui, shell),
        Page::Save => save(ed, ui, shell),
        Page::Export => export(ed, ui, shell),
        Page::Print => print(ed, ui, shell),
        Page::App(id) if id == "compare" => compare(ed, ui, shell),
        _ => {}
    }
}

/// A wide button on the page that runs `id` with `params` (closing the File page).
fn act(ed: &Editor, ui: &mut Ui, shell: &mut Shell, id: &str, params: serde_json::Value, title: &str, about: &str) -> bool {
    let c = Control::of(Cmd::with(id, params)).icon(crate::script_icons::generic(crate::script_icons::for_command(id))).label(title).about(about);
    action(ui, shell, ed, &c)
}

/// Script's templates as the shell's chooser draws them on the New page (`App::templates`): each
/// under its category, with a tile showing its first lines in a fixed-width font, as a script
/// is set (built once; templates never change).
pub fn templates() -> Vec<TemplateSpec> {
    static ALL: std::sync::OnceLock<Vec<TemplateSpec>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        wordsmith_script_model::templates::TEMPLATES
            .iter()
            .map(|t| {
                let lines: Vec<String> = wordsmith_script_model::templates::build(t.id).map_or_else(Vec::new, |doc| {
                    doc.body.iter().filter_map(|b| b.as_para()).map(|p| p.text.clone()).filter(|x| !x.trim().is_empty()).take(3).collect()
                });
                let preview =
                    if lines.is_empty() { TemplatePreview::lines(["INT. —", "", "(an empty page, ready)"]) } else { TemplatePreview::lines(lines) };
                TemplateSpec::new(t.id, t.name, t.about).in_group(t.category).with_preview(preview.monospace())
            })
            .collect()
    })
    .clone()
}

fn new(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    let t = crate::theme::get(ui.ctx());
    // The templates are the shell's chooser, above (`App::templates`). A new script in a tab of
    // its own; unsaved work stays.
    let tab = ui.add(egui::Button::new(RichText::new("New tab").font(medium(BODY))).min_size(vec2(120.0, 30.0)));
    widgets::record(shell, Some("file.newTab"), tab.rect);
    if tab.on_hover_text(widgets::tip(shell, &*ed, "file.newTab", "")).clicked() {
        shell.chrome.file_page = None;
        ed.run("file.newTab", json!({}));
    }
    sub(ui, "First steps");
    for tip in [
        "Return starts the next element and Tab changes this one: a heading leads to action, a name to dialogue.",
        "Type INT, press Tab, type the place, press Tab, type the time of day.",
        "The Write tab's element tiles show each element's key and what Return and Tab make.",
        "The Script panel on the left lists scenes, notes and characters; the Inspector on the right shows the scene and element at the caret.",
        "F10 shows a letter on every ribbon tab and control: type the letters to use them from the keyboard.",
    ] {
        ui.label(RichText::new(format!("•  {tip}")).color(t.text_dim));
    }
    let mut show = ed.tools.show_start_on_launch;
    if ui.checkbox(&mut show, "Show this page when the app starts").changed() {
        ed.tools.show_start_on_launch = show;
    }
}

fn open(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    sub(ui, "Import");
    act(ed, ui, shell, "file.import", json!({}), "Import…", "Plain text, RTF or a PDF made from text.");
    if shell.dialogs.recovery.is_empty() {
        sub(ui, "Unsaved work");
        act(ed, ui, shell, "file.recoverPrompt", json!({}), "Recover Unsaved Scripts…", "Copies kept when the app closed without saving.");
    }
}

fn save(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    act(ed, ui, shell, "file.saveAsTemplate", json!({}), "Save as Template…", "Start new scripts from this one.");
    act(ed, ui, shell, "file.revert", json!({}), "Revert", "Put back the script as last saved.");
    act(ed, ui, shell, "file.closeTab", json!({}), "Close Tab", "Close this script's tab.");
}

fn export(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    act(ed, ui, shell, "file.exportPdfAs", json!({}), "PDF…", "The pages as they print.");
    act(ed, ui, shell, "file.exportRevisedPdfAs", json!({}), "Revised Pages as PDF…", "Only the pages with revised text.");
    act(ed, ui, shell, "file.exportFountain", json!({}), "Fountain…", "Plain-text screenplay format other apps read.");
    act(ed, ui, shell, "file.exportRtf", json!({}), "Rich Text (RTF)…", "For word processors.");
    act(ed, ui, shell, "file.exportText", json!({}), "Plain Text…", "The words alone.");
    act(ed, ui, shell, "file.exportHtml", json!({}), "Web Page (HTML)…", "To read in a browser.");
    sub(ui, "Planning");
    let board = ed.board.current;
    act(ed, ui, shell, "storyboard.exportPdf", json!({"board": board}), "Story Board as PDF…", "The board showing, on one page.");
    act(ed, ui, shell, "cards.exportPdf", json!({}), "Index Cards as PDF…", "A grid of cards on Letter pages.");
}

/// The print choices; the shell's Print button under them prints with them.
fn print(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    // The print choices, as the Print window has them.
    let mut c = ed.print.clone();
    let sets: Vec<String> = wordsmith_script_model::revisions::Revisions::of_doc(&ed.session.doc).sets.iter().map(|s| s.name.clone()).collect();
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
    ed.print = c;
    act(ed, ui, shell, "file.printPreview", params.clone(), "Preview", "Open the pages in your PDF viewer.");
    // The PDF's file dialog runs the export with these choices.
    act(ed, ui, shell, "file.exportPdfAs", params, "Save as PDF…", "Keep the pages as a PDF file.");
}

fn compare(ed: &mut Editor, ui: &mut Ui, shell: &mut Shell) {
    let t = crate::theme::get(ui.ctx());
    heading(ui, "Compare");
    ui.label(RichText::new("Pick an earlier version of this script. The comparison opens as a new script: added text marked as inserted, removed text as deleted, and moved scenes with a note.").color(t.text_dim));
    ui.add_space(8.0);
    act(ed, ui, shell, "compare.withFile", json!({}), "Compare with Another Version…", "Scenes are matched first, then their paragraphs.");
}
