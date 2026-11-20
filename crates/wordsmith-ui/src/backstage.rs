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

/// Draw the backstage in `ui` (the central panel).
pub fn show(ed: &mut Editor, ui: &mut Ui) {
    if ui.button("← Back to the script").clicked() {
        ed.chrome.backstage = None;
    }
}
