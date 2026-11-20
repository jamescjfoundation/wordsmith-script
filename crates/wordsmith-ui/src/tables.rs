// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Tables panel (decision 0022): the scenes and the characters of the script as tables beside
//! it. Editing a cell edits the script (a scene's heading or summary; a character's name, renamed
//! everywhere), dragging a scene row reorders the scenes, and each table exports as CSV. All
//! through commands, so undo and agents work the same.

use egui::{Id, RichText, Ui};
use serde_json::{Value, json};

use crate::editor::Editor;

/// Which table the panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Scenes,
    Characters,
}

/// A cell being edited: the table, its row, its column, and the text so far.
#[derive(Clone, Debug, PartialEq)]
pub struct CellEdit {
    pub table: Table,
    pub row: usize,
    pub column: &'static str,
    pub text: String,
    /// The text before editing (a rename needs the old name).
    pub was: String,
    focused: bool,
}

impl CellEdit {
    pub fn new(table: Table, row: usize, column: &'static str, text: &str) -> Self {
        CellEdit { table, row, column, text: text.to_string(), was: text.to_string(), focused: false }
    }
}

/// The command a finished cell edit runs, with its parameters (none when nothing changed).
pub fn commit(edit: &CellEdit) -> Option<(&'static str, Value)> {
    if edit.text == edit.was {
        return None;
    }
    match (edit.table, edit.column) {
        (Table::Scenes, "heading") => Some(("scene.setHeading", json!({"scene": edit.row, "text": edit.text}))),
        (Table::Scenes, "summary") => Some(("scene.setSummary", json!({"scene": edit.row, "text": edit.text}))),
        (Table::Characters, "name") => Some(("character.rename", json!({"from": edit.was, "to": edit.text}))),
        _ => None,
    }
}

enum Act {
    Commit(CellEdit),
    Cancel,
    Edit(CellEdit),
    Move { from: usize, onto: usize },
    Open(usize),
}

/// One cell: its text, or the editor when it is the cell being edited. Clicking starts editing.
fn cell(ui: &mut Ui, ed_cell: &mut Option<CellEdit>, here: (Table, usize, &'static str), text: &str, act: &mut Option<Act>) {
    let (table, row, column) = here;
    if let Some(e) = ed_cell.as_mut().filter(|e| e.table == table && e.row == row && e.column == column) {
        let r = ui.add(egui::TextEdit::singleline(&mut e.text).desired_width(180.0));
        if !e.focused {
            r.request_focus();
            e.focused = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            *act = Some(Act::Cancel);
        } else if r.lost_focus() {
            *act = Some(Act::Commit(e.clone()));
        }
        return;
    }
    let shown = if text.is_empty() { RichText::new("—").weak() } else { RichText::new(text) };
    if ui.add(egui::Label::new(shown).sense(egui::Sense::click()).truncate()).on_hover_text("Click to edit").clicked() {
        *act = Some(Act::Edit(CellEdit::new(table, row, column, text)));
    }
}

fn scenes_table(ed: &mut Editor, ui: &mut Ui, act: &mut Option<Act>) {
    let scenes = ed.scene_list();
    let pag = ed.paginated();
    let mut editing = ed.table_edit.clone();
    egui::Grid::new("scenes_table").striped(true).num_columns(5).show(ui, |ui| {
        for h in ["#", "Heading", "Page", "Characters", "Summary"] {
            ui.label(RichText::new(h).strong());
        }
        ui.end_row();
        for (i, sc) in scenes.iter().enumerate() {
            let drag = ui.dnd_drag_source(Id::new(("scene-row", i)), i, |ui| ui.label(format!("{}", i + 1)));
            if let Some(from) = drag.response.dnd_release_payload::<usize>() {
                *act = Some(Act::Move { from: *from, onto: i });
            }
            cell(ui, &mut editing, (crate::tables::Table::Scenes, i, "heading"), sc["heading"].as_str().unwrap_or(""), act);
            let start = sc["start"].as_u64().unwrap_or(0) as usize;
            let page = pag
                .map
                .iter()
                .position(|m| m.is_some_and(|m| m.block == start))
                .and_then(|out| wordsmith_script::commands::character::page_of_output(&pag, out));
            if ui.link(page.map(|p| p.to_string()).unwrap_or_default()).on_hover_text("Go to the scene").clicked() {
                *act = Some(Act::Open(i));
            }
            let who: Vec<&str> = sc["characters"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            ui.add(egui::Label::new(who.join(", ")).truncate());
            cell(ui, &mut editing, (Table::Scenes, i, "summary"), sc["summary"].as_str().unwrap_or(""), act);
            ui.end_row();
        }
    });
    if act.is_none() {
        ed.table_edit = editing;
    }
}

fn characters_table(ed: &mut Editor, ui: &mut Ui, act: &mut Option<Act>) {
    let rows = ed.session.run("character.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let mut editing = ed.table_edit.clone();
    egui::Grid::new("characters_table").striped(true).num_columns(4).show(ui, |ui| {
        for h in ["Character", "Speeches", "Scenes", "First scene"] {
            ui.label(RichText::new(h).strong());
        }
        ui.end_row();
        for (i, r) in rows.iter().enumerate() {
            cell(ui, &mut editing, (Table::Characters, i, "name"), r["name"].as_str().unwrap_or(""), act);
            ui.label(r["speeches"].to_string());
            ui.label(r["scenes"].to_string());
            match r["firstScene"].as_u64() {
                Some(n) => {
                    if ui.link((n + 1).to_string()).clicked() {
                        *act = Some(Act::Open(n as usize));
                    }
                }
                None => {
                    ui.label("");
                }
            }
            ui.end_row();
        }
    });
    if act.is_none() {
        ed.table_edit = editing;
    }
}

/// The panel, at the right of the window, while it is open.
pub fn panel(ed: &mut Editor, ui: &mut Ui) {
    let Some(table) = ed.tables else { return };
    egui::Panel::right("tables").resizable(true).default_size(460.0).show(ui, |ui| {
        let mut act = None;
        ui.horizontal(|ui| {
            for (t, label) in [(Table::Scenes, "Scenes"), (Table::Characters, "Characters")] {
                if ui.selectable_label(table == t, label).clicked() {
                    ed.tables = Some(t);
                    ed.table_edit = None;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Export CSV…").clicked() {
                    let id = if table == Table::Scenes { "tables.exportScenesCsv" } else { "tables.exportCharactersCsv" };
                    ed.run(id, json!({}));
                }
            });
        });
        ui.separator();
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| match table {
            Table::Scenes => scenes_table(ed, ui, &mut act),
            Table::Characters => characters_table(ed, ui, &mut act),
        });
        match act {
            Some(Act::Commit(e)) => {
                ed.table_edit = None;
                if let Some((id, params)) = commit(&e) {
                    ed.run(id, params);
                }
            }
            Some(Act::Cancel) => ed.table_edit = None,
            Some(Act::Edit(e)) => ed.table_edit = Some(e),
            Some(Act::Move { from, onto }) if from != onto => {
                let count = ed.scene_list().len();
                ed.run("scene.move", json!({"scene": from, "before": crate::structure::drop_target(from, onto, count)}));
            }
            Some(Act::Open(i)) => {
                ed.structure = None;
                ed.run("scene.goto", json!({"scene": i}));
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_edits_become_commands() {
        let mut e = CellEdit::new(Table::Characters, 0, "name", "ANNA");
        assert_eq!(commit(&e), None, "unchanged");
        e.text = "ZOE".into();
        assert_eq!(commit(&e), Some(("character.rename", json!({"from": "ANNA", "to": "ZOE"}))));
        let mut h = CellEdit::new(Table::Scenes, 2, "summary", "");
        h.text = "They meet.".into();
        assert_eq!(commit(&h), Some(("scene.setSummary", json!({"scene": 2, "text": "They meet."}))));
    }
}
