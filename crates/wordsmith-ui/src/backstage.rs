// SPDX-License-Identifier: MIT OR Apache-2.0
//! The File page (v0.3 plan, M18): a full-window page in place of the script for new, open,
//! save, export, print, compare, preferences, keys and about.

use egui::Ui;

use crate::editor::Editor;

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
    "file.recoverPrompt",
    "view.start",
];

/// Draw the backstage in `ui` (the central panel).
pub fn show(ed: &mut Editor, ui: &mut Ui) {
    if ui.button("← Back to the script").clicked() {
        ed.chrome.backstage = None;
    }
}
