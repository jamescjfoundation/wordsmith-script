// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Tables panel (decision 0022, SPEC-2 10): the scenes, the characters and the notes of the
//! script as tables beside it. Editing a cell edits the script (a scene's heading or summary; a
//! character's name, renamed everywhere, or a trait; a note), dragging a scene row reorders the
//! scenes, columns sort by a click on their name, a filter keeps matching rows, columns can be
//! hidden, and the writer can add, rename and remove tabs of their own (saved with the script).
//! All through commands, so undo and agents work the same.

use egui::{Id, RichText, Ui};
use serde_json::{Value, json};

use crate::editor::Editor;

/// Which table the panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Scenes,
    Characters,
    Notes,
}

impl Table {
    pub fn key(self) -> &'static str {
        match self {
            Table::Scenes => "scenes",
            Table::Characters => "characters",
            Table::Notes => "notes",
        }
    }

    pub fn from_key(k: &str) -> Table {
        match k {
            "characters" => Table::Characters,
            "notes" => Table::Notes,
            _ => Table::Scenes,
        }
    }
}

/// How the shown table is sorted, filtered and which columns are hidden, and which of the
/// writer's own tabs is open (if one is).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    /// The column sorted by, and whether descending.
    pub sort: Option<(String, bool)>,
    pub filter: String,
    pub hidden: Vec<String>,
    /// The writer's tab shown (index into `tables.tabs`).
    pub tab: Option<usize>,
    /// A tab name being typed (for a new tab or a rename).
    pub naming: Option<String>,
}

/// A cell being edited: the table, its row, its column, and the text so far.
#[derive(Clone, Debug, PartialEq)]
pub struct CellEdit {
    pub table: Table,
    pub row: usize,
    pub column: String,
    pub text: String,
    /// The text before editing (a rename needs the old name).
    pub was: String,
    /// The row's key: a character's name, a note's id.
    pub key: String,
    focused: bool,
}

impl CellEdit {
    pub fn new(table: Table, row: usize, column: &str, text: &str) -> Self {
        CellEdit { table, row, column: column.to_string(), text: text.to_string(), was: text.to_string(), key: String::new(), focused: false }
    }
}

/// The command a finished cell edit runs, with its parameters (none when nothing changed).
pub fn commit(edit: &CellEdit) -> Option<(&'static str, Value)> {
    if edit.text == edit.was {
        return None;
    }
    match (edit.table, edit.column.as_str()) {
        (Table::Scenes, "heading") => Some(("scene.setHeading", json!({"scene": edit.row, "text": edit.text}))),
        (Table::Scenes, "summary") => Some(("scene.setSummary", json!({"scene": edit.row, "text": edit.text}))),
        (Table::Scenes, "title") => Some(("scene.setProperties", json!({"scene": edit.row, "title": edit.text}))),
        (Table::Characters, "name") => Some(("character.rename", json!({"from": edit.was, "to": edit.text}))),
        (Table::Characters, col) if col.starts_with("trait:") => {
            let t = col.trim_start_matches("trait:");
            Some(("character.set", json!({"name": edit.key, "traits": {t: edit.text}})))
        }
        // A note's row is its id.
        (Table::Notes, "text") => Some(("note.edit", json!({"id": edit.row, "text": edit.text}))),
        (Table::Notes, "title") => Some(("note.edit", json!({"id": edit.row, "title": edit.text}))),
        (Table::Notes, "type") => Some(("note.edit", json!({"id": edit.row, "type": edit.text}))),
        _ => None,
    }
}

enum Act {
    Run(&'static str, Value),
    Commit(CellEdit),
    Cancel,
    Edit(CellEdit),
    Move { from: usize, onto: usize },
    Open(usize),
    Sort(String),
}

/// One column: its key in the rows and its heading; editable columns open an editor on click.
struct Column {
    key: String,
    label: String,
    editable: bool,
}

fn col(key: &str, label: &str, editable: bool) -> Column {
    Column { key: key.into(), label: label.into(), editable }
}

/// A cell's text in a row.
fn text_of(row: &Value, key: &str) -> String {
    match &row[key] {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
        Value::Bool(b) => {
            if *b {
                "yes".into()
            } else {
                String::new()
            }
        }
        other => other.to_string(),
    }
}

/// Rows in the order and with the filter of `view`, as (row index, row).
fn arranged(rows: &[Value], columns: &[Column], view: &View) -> Vec<(usize, Value)> {
    let f = view.filter.trim().to_lowercase();
    let mut out: Vec<(usize, Value)> = rows
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, r)| f.is_empty() || columns.iter().any(|c| text_of(r, &c.key).to_lowercase().contains(&f)))
        .collect();
    if let Some((key, desc)) = &view.sort {
        out.sort_by(|(_, a), (_, b)| {
            let (x, y) = (text_of(a, key), text_of(b, key));
            let ord = match (x.parse::<f64>(), y.parse::<f64>()) {
                (Ok(p), Ok(q)) => p.partial_cmp(&q).unwrap_or(std::cmp::Ordering::Equal),
                _ => x.to_lowercase().cmp(&y.to_lowercase()),
            };
            if *desc { ord.reverse() } else { ord }
        });
    }
    out
}

/// One cell: its text, or the editor when it is the cell being edited. Clicking an editable
/// cell starts editing.
fn cell(ui: &mut Ui, editing: &mut Option<CellEdit>, here: (Table, usize, &str, &str), text: &str, act: &mut Option<Act>) {
    let (table, row, column, key) = here;
    if let Some(e) = editing.as_mut().filter(|e| e.table == table && e.row == row && e.column == column) {
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
        let mut e = CellEdit::new(table, row, column, text);
        e.key = key.to_string();
        *act = Some(Act::Edit(e));
    }
}

fn header(ui: &mut Ui, columns: &[&Column], view: &View, act: &mut Option<Act>) {
    for c in columns {
        let arrow = match &view.sort {
            Some((k, desc)) if *k == c.key => {
                if *desc {
                    " ▼"
                } else {
                    " ▲"
                }
            }
            _ => "",
        };
        if ui.add(egui::Label::new(RichText::new(format!("{}{arrow}", c.label)).strong()).sense(egui::Sense::click())).on_hover_text("Sort").clicked()
        {
            *act = Some(Act::Sort(c.key.clone()));
        }
    }
    ui.end_row();
}

fn scene_rows(ed: &mut Editor) -> Vec<Value> {
    let scenes = ed.scene_list();
    let pag = ed.paginated();
    let eighths = wordsmith_script::reports::scene_eighths(&ed.session.doc, &pag);
    scenes
        .iter()
        .enumerate()
        .map(|(i, sc)| {
            let start = sc["start"].as_u64().unwrap_or(0) as usize;
            let page = pag
                .map
                .iter()
                .position(|m| m.is_some_and(|m| m.block == start))
                .and_then(|out| wordsmith_script::commands::character::page_of_output(&pag, out));
            json!({
                "number": sc["number"].as_str().map_or_else(|| (i + 1).to_string(), str::to_string),
                "heading": sc["heading"],
                "title": sc["title"],
                "page": page,
                "length": eighths.get(i).map(|e| wordsmith_script::reports::format_eighths(*e)).unwrap_or_default(),
                "cast": sc["cast"],
                "summary": sc["summary"],
                "color": sc["color"],
            })
        })
        .collect()
}

fn scenes_table(ed: &mut Editor, ui: &mut Ui, view: &View, act: &mut Option<Act>) {
    let rows = scene_rows(ed);
    let columns = [
        col("number", "#", false),
        col("heading", "Heading", true),
        col("title", "Title", true),
        col("page", "Page", false),
        col("length", "Length", false),
        col("cast", "Characters", false),
        col("summary", "Summary", true),
        col("color", "Color", false),
    ];
    let shown: Vec<&Column> = columns.iter().filter(|c| !view.hidden.contains(&c.key)).collect();
    // Dragging rows reorders the scenes only in script order with every row showing.
    let can_drag = view.sort.is_none() && view.filter.trim().is_empty();
    let mut editing = ed.table_edit.clone();
    egui::Grid::new("scenes_table").striped(true).num_columns(shown.len()).show(ui, |ui| {
        header(ui, &shown, view, act);
        for (i, r) in arranged(&rows, &columns, view) {
            for c in &shown {
                let text = text_of(&r, &c.key);
                match c.key.as_str() {
                    "number" if can_drag => {
                        let drag = ui.dnd_drag_source(Id::unique(("scene-row", i)), i, |ui| ui.label(text));
                        if let Some(from) = drag.response.dnd_release_payload::<usize>() {
                            *act = Some(Act::Move { from: *from, onto: i });
                        }
                    }
                    "page" => {
                        if ui.link(text).on_hover_text("Go to the scene").clicked() {
                            *act = Some(Act::Open(i));
                        }
                    }
                    k if c.editable => cell(ui, &mut editing, (Table::Scenes, i, k, ""), &text, act),
                    _ => {
                        ui.add(egui::Label::new(text).truncate());
                    }
                }
            }
            ui.end_row();
        }
    });
    if act.is_none() {
        ed.table_edit = editing;
    }
}

fn characters_table(ed: &mut Editor, ui: &mut Ui, view: &View, act: &mut Option<Act>) {
    let list = ed.session.run("character.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let traits: Vec<String> = ed
        .session
        .run("character.traits", &json!({}))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    let cast = wordsmith_script::cast::Cast::of_doc(&ed.session.doc);
    let rows: Vec<Value> = list
        .iter()
        .map(|r| {
            let name = r["name"].as_str().unwrap_or("").to_string();
            let person = cast.people.get(&name).cloned().unwrap_or_default();
            let mut row = json!({
                "name": name,
                "speeches": r["speeches"],
                "scenes": r["scenes"],
                "firstScene": r["firstScene"].as_u64().map(|n| n + 1),
                "highlight": person.highlight_on,
            });
            for t in &traits {
                row[format!("trait:{t}")] = json!(person.traits.get(t).cloned().unwrap_or_default());
            }
            row
        })
        .collect();
    let mut columns = vec![
        col("name", "Character", true),
        col("speeches", "Speeches", false),
        col("scenes", "Scenes", false),
        col("firstScene", "First scene", false),
        col("highlight", "Highlight", false),
    ];
    columns.extend(traits.iter().map(|t| Column { key: format!("trait:{t}"), label: t.clone(), editable: true }));
    let shown: Vec<&Column> = columns.iter().filter(|c| !view.hidden.contains(&c.key)).collect();
    let mut editing = ed.table_edit.clone();
    egui::Grid::new("characters_table").striped(true).num_columns(shown.len()).show(ui, |ui| {
        header(ui, &shown, view, act);
        for (i, r) in arranged(&rows, &columns, view) {
            let name = text_of(&r, "name");
            for c in &shown {
                let text = text_of(&r, &c.key);
                match c.key.as_str() {
                    "firstScene" => {
                        if let Some(n) = r["firstScene"].as_u64() {
                            if ui.link(n.to_string()).clicked() {
                                *act = Some(Act::Open(n as usize - 1));
                            }
                        } else {
                            ui.label("");
                        }
                    }
                    "highlight" => {
                        let mut on = r["highlight"].as_bool().unwrap_or(false);
                        if ui.checkbox(&mut on, "").on_hover_text("Highlight this character's speeches").changed() {
                            *act = Some(Act::Run("character.set", json!({"name": name, "highlightOn": on})));
                        }
                    }
                    k if c.editable => cell(ui, &mut editing, (Table::Characters, i, k, &name), &text, act),
                    _ => {
                        ui.label(text);
                    }
                }
            }
            ui.end_row();
        }
    });
    if act.is_none() {
        ed.table_edit = editing;
    }
}

fn notes_table(ed: &mut Editor, ui: &mut Ui, view: &View, act: &mut Option<Act>) {
    let list = ed.session.run("note.list", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    if list.is_empty() {
        ui.label("No notes yet. Select some text and choose Edit › Note… to add one.");
    }
    let rows: Vec<Value> = list
        .iter()
        .map(|r| {
            json!({
                "id": r["id"],
                "scene": r["scene"].as_u64().map(|n| n + 1),
                "quote": if r["attached"].as_bool() == Some(true) { r["quote"].clone() } else { json!("(its text was deleted)") },
                "attached": r["attached"],
                "title": r["title"],
                "type": r["type"],
                "text": r["text"],
                "author": r["author"],
                "modified": wordsmith_script::reports_more::date(r["modified"].as_u64().unwrap_or(0)),
            })
        })
        .collect();
    let columns = [
        col("scene", "Scene", false),
        col("quote", "Attached to", false),
        col("title", "Title", true),
        col("type", "Type", true),
        col("text", "Note", true),
        col("author", "Writer", false),
        col("modified", "Changed", false),
    ];
    let shown: Vec<&Column> = columns.iter().filter(|c| !view.hidden.contains(&c.key)).collect();
    let mut editing = ed.table_edit.clone();
    egui::Grid::new("notes_table").striped(true).num_columns(shown.len() + 1).show(ui, |ui| {
        header(ui, &shown, view, act);
        for (_, r) in arranged(&rows, &columns, view) {
            let id = r["id"].as_u64().unwrap_or(0) as usize;
            for c in &shown {
                let text = text_of(&r, &c.key);
                match c.key.as_str() {
                    "quote" if r["attached"].as_bool() == Some(true) => {
                        if ui.link(text).on_hover_text("Go to the text").clicked() {
                            *act = Some(Act::Run("note.goto", json!({"id": id})));
                        }
                    }
                    k if c.editable => cell(ui, &mut editing, (Table::Notes, id, k, ""), &text, act),
                    _ => {
                        ui.add(egui::Label::new(text).truncate());
                    }
                }
            }
            if ui.small_button("Delete").clicked() {
                *act = Some(Act::Run("note.delete", json!({"id": id})));
            }
            ui.end_row();
        }
    });
    if act.is_none() {
        ed.table_edit = editing;
    }
}

/// The column keys of a table, for the column chooser.
fn column_keys(ed: &mut Editor, table: Table) -> Vec<(String, String)> {
    match table {
        Table::Scenes => [
            ("heading", "Heading"),
            ("title", "Title"),
            ("page", "Page"),
            ("length", "Length"),
            ("cast", "Characters"),
            ("summary", "Summary"),
            ("color", "Color"),
        ]
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .to_vec(),
        Table::Characters => {
            let mut v: Vec<(String, String)> =
                [("speeches", "Speeches"), ("scenes", "Scenes"), ("firstScene", "First scene"), ("highlight", "Highlight")]
                    .map(|(k, l)| (k.to_string(), l.to_string()))
                    .to_vec();
            let traits = ed.session.run("character.traits", &json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
            v.extend(traits.iter().filter_map(|t| t.as_str()).map(|t| (format!("trait:{t}"), t.to_string())));
            v
        }
        Table::Notes => {
            [("scene", "Scene"), ("quote", "Attached to"), ("title", "Title"), ("type", "Type"), ("author", "Writer"), ("modified", "Changed")]
                .map(|(k, l)| (k.to_string(), l.to_string()))
                .to_vec()
        }
    }
}

/// The panel, at the right of the window, while it is open.
pub fn panel(ed: &mut Editor, ui: &mut Ui) {
    let Some(table) = ed.tables else { return };
    let tabs = wordsmith_script::commands::character::table_tabs(&ed.session.doc);
    let mut view = ed.table_view.clone();
    egui::Panel::right("tables").resizable(true).default_size(520.0).show(ui, |ui| {
        let mut act = None;
        let mut save_tabs: Option<Vec<wordsmith_script::commands::character::TableTab>> = None;
        ui.horizontal_wrapped(|ui| {
            for (t, label) in [(Table::Scenes, "Scenes"), (Table::Characters, "Characters"), (Table::Notes, "Notes")] {
                if ui.selectable_label(table == t && view.tab.is_none(), label).clicked() {
                    ed.tables = Some(t);
                    ed.table_edit = None;
                    view = View::default();
                }
            }
            for (i, tab) in tabs.iter().enumerate() {
                if ui.selectable_label(view.tab == Some(i), tab.name.as_str()).clicked() {
                    ed.tables = Some(Table::from_key(&tab.table));
                    ed.table_edit = None;
                    view = View { tab: Some(i), filter: tab.filter.clone(), hidden: tab.hidden.clone(), ..View::default() };
                }
            }
            if ui.small_button("+").on_hover_text("A new tab showing this table as it is now").clicked() {
                view.naming = Some(String::new());
            }
        });
        if let Some(name) = view.naming.as_mut() {
            let mut done = None;
            ui.horizontal(|ui| {
                ui.label(if view.tab.is_some() { "Rename tab" } else { "New tab" });
                let r = ui.text_edit_singleline(name);
                if ui.button("OK").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    done = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    done = Some(false);
                }
            });
            if let Some(ok) = done {
                let name = name.trim().to_string();
                if ok && !name.is_empty() {
                    let mut all = tabs.clone();
                    let tab = wordsmith_script::commands::character::TableTab {
                        name,
                        table: table.key().into(),
                        filter: view.filter.clone(),
                        hidden: view.hidden.clone(),
                    };
                    match view.tab.and_then(|i| all.get_mut(i)) {
                        Some(t) => *t = tab,
                        None => {
                            all.push(tab);
                            view.tab = Some(all.len() - 1);
                        }
                    }
                    save_tabs = Some(all);
                }
                view.naming = None;
            }
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut view.filter).hint_text("Filter").desired_width(140.0));
            ui.menu_button("Columns", |ui| {
                for (k, l) in column_keys(ed, table) {
                    let mut on = !view.hidden.contains(&k);
                    if ui.checkbox(&mut on, l).changed() {
                        if on {
                            view.hidden.retain(|x| *x != k);
                        } else {
                            view.hidden.push(k);
                        }
                    }
                }
            });
            if let Some(i) = view.tab {
                if ui.small_button("Save").on_hover_text("Keep this tab's filter and columns").clicked() {
                    let mut all = tabs.clone();
                    if let Some(t) = all.get_mut(i) {
                        t.filter = view.filter.clone();
                        t.hidden = view.hidden.clone();
                    }
                    save_tabs = Some(all);
                }
                if ui.small_button("Rename").clicked() {
                    view.naming = tabs.get(i).map(|t| t.name.clone());
                }
                if ui.small_button("Remove").clicked() {
                    let mut all = tabs.clone();
                    if i < all.len() {
                        all.remove(i);
                    }
                    save_tabs = Some(all);
                    view = View::default();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if table != Table::Notes && ui.button("Export CSV…").clicked() {
                    let id = if table == Table::Scenes { "tables.exportScenesCsv" } else { "tables.exportCharactersCsv" };
                    ed.run(id, json!({}));
                }
            });
        });
        ui.separator();
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| match table {
            Table::Scenes => scenes_table(ed, ui, &view, &mut act),
            Table::Characters => characters_table(ed, ui, &view, &mut act),
            Table::Notes => notes_table(ed, ui, &view, &mut act),
        });
        if let Some(tabs) = save_tabs {
            ed.run("tables.setTabs", json!({"tabs": tabs}));
        }
        match act {
            Some(Act::Commit(e)) => {
                ed.table_edit = None;
                if let Some((id, params)) = commit(&e) {
                    ed.run(id, params);
                }
            }
            Some(Act::Cancel) => ed.table_edit = None,
            Some(Act::Run(id, params)) => {
                ed.structure = None;
                ed.run(id, params);
            }
            Some(Act::Edit(e)) => ed.table_edit = Some(e),
            Some(Act::Move { from, onto }) if from != onto => {
                let count = ed.scene_list().len();
                ed.run("scene.move", json!({"scene": from, "before": jcj_board::drop_target(from, onto, count)}));
            }
            Some(Act::Open(i)) => {
                ed.structure = None;
                ed.run("scene.goto", json!({"scene": i}));
            }
            Some(Act::Sort(key)) => {
                view.sort = match view.sort.take() {
                    Some((k, false)) if k == key => Some((k, true)),
                    Some((k, true)) if k == key => None,
                    _ => Some((key, false)),
                };
            }
            _ => {}
        }
    });
    ed.table_view = view;
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
        let mut t = CellEdit::new(Table::Characters, 0, "trait:Age", "");
        t.key = "ANNA".into();
        t.text = "40".into();
        assert_eq!(commit(&t), Some(("character.set", json!({"name": "ANNA", "traits": {"Age": "40"}}))));
    }

    #[test]
    fn rows_sort_by_number_or_text_and_filter() {
        let rows = vec![json!({"a": "10", "b": "zed"}), json!({"a": "9", "b": "Amy"}), json!({"a": "100", "b": "bob"})];
        let cols = [col("a", "A", false), col("b", "B", false)];
        let order = |v: &View| arranged(&rows, &cols, v).into_iter().map(|(i, _)| i).collect::<Vec<_>>();
        assert_eq!(order(&View::default()), [0, 1, 2]);
        assert_eq!(order(&View { sort: Some(("a".into(), false)), ..View::default() }), [1, 0, 2], "numbers by value");
        assert_eq!(order(&View { sort: Some(("b".into(), true)), ..View::default() }), [0, 2, 1]);
        assert_eq!(order(&View { filter: "BO".into(), ..View::default() }), [2]);
    }
}
