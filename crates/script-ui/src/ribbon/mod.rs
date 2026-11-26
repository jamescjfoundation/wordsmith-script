// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's ribbon (decision 0037): tabs named for screenplay work (Write, Insert, Plan,
//! Production, Review, View) and the contextual Story Board and Index Cards tabs, drawn by the
//! shell (`jcj_shell`) with the groups in [`tabs`]; and the first-run tour's steps. The shell
//! draws the quick-access row, the tab strip, key tips and the File page.

use jcj_shell::TabSpec;

pub mod tabs;
pub mod widgets;

/// A ribbon tab's id.
pub mod tab {
    pub const WRITE: &str = "write";
    pub const INSERT: &str = "insert";
    pub const PLAN: &str = "plan";
    pub const PRODUCTION: &str = "production";
    pub const REVIEW: &str = "review";
    pub const VIEW: &str = "view";
    /// Contextual: shown with the story board.
    pub const BOARD: &str = "board";
    /// Contextual: shown with the index cards.
    pub const CARDS: &str = "cards";
}

/// The tabs showing now: the main ones, and a contextual one with its view.
pub fn tabs_now(ed: &crate::Editor) -> Vec<TabSpec> {
    let mut v = vec![
        TabSpec::new(tab::WRITE, "Write", "W"),
        TabSpec::new(tab::INSERT, "Insert", "I"),
        TabSpec::new(tab::PLAN, "Plan", "P"),
        TabSpec::new(tab::PRODUCTION, "Production", "O"),
        TabSpec::new(tab::REVIEW, "Review", "R"),
        TabSpec::new(tab::VIEW, "View", "V"),
    ];
    match ed.structure {
        Some(crate::structure::Structure::Board) => v.push(TabSpec::new(tab::BOARD, "Story Board", "B").contextual()),
        Some(crate::structure::Structure::Cards) => v.push(TabSpec::new(tab::CARDS, "Index Cards", "C").contextual()),
        _ => {}
    }
    v
}

/// The first-run tour: what each part of the window is for, one step at a time.
pub const TOUR: [(&str, &str); 5] = [
    (
        "The ribbon",
        "Tabs for writing, inserting, planning, production, review and the view. Each control shows its key when you point at it; F10 shows letters to use them from the keyboard.",
    ),
    ("Elements", "Write › Elements sets the paragraph's element. Each tile shows its key and what Return and Tab make from it."),
    ("The Script panel", "The rail on the left: scenes (drag to reorder), the outline, bookmarks, notes, characters and find."),
    (
        "The Inspector",
        "The rail on the right: the element and scene at the caret, revisions, the page, tracked changes, the format checker and your goals.",
    ),
    (
        "The File page",
        "File holds new scripts from templates, recent scripts, export, print and preferences. The status bar below shows the page, words and what Return and Tab make. At start the app asks GitHub for a newer version; Preferences › Starting turns that off.",
    ),
];
