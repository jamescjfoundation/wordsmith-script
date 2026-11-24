// SPDX-License-Identifier: MIT OR Apache-2.0
//! Script's ribbon tabs, status bar and menu bar draw with the shell's controls
//! (`jcj_shell::widgets`): every control runs a command by id, greyed out when it cannot run, with
//! its name, key and a line on what it does as its tooltip, and key tips. This module gives the
//! tabs the short calls they are written with ([`big`], [`small`], [`tile`], [`group`], ...) over
//! [`Rb`]: the shell to draw in and queue commands on, and the editor the controls describe.

use std::ops::Deref;
use std::sync::Arc;

use egui::{Color32, Rect, Ui};
use jcj_shell::widgets::{self as w, Control};
use jcj_shell::{Cmd, Shell};
use serde_json::Value;

pub use jcj_shell::widgets::{CONTENT_H, SMALL_H, TITLE_H, stack};

use crate::editor::Editor;

/// What a ribbon control is drawn with: the shell (to draw in and run commands through), the
/// editor (what the controls show), and the ribbon's facts about the script for this revision.
pub struct Rb<'a> {
    pub shell: &'a mut Shell,
    pub ed: &'a Editor,
    pub data: Arc<crate::ribbon::tabs::RibbonData>,
}

impl Deref for Rb<'_> {
    type Target = Editor;
    fn deref(&self) -> &Editor {
        self.ed
    }
}

/// Command `id` with no parameters.
pub fn cmd(id: impl Into<String>) -> Cmd {
    Cmd::new(id)
}

/// Command `id` with `params`.
pub fn cmd_with(id: impl Into<String>, params: Value) -> Cmd {
    Cmd::with(id, params)
}

fn control(c: Cmd, icon: &str, label: &str, about: &str, on: bool) -> Control {
    Control::of(c).icon(crate::script_icons::generic(icon)).label(label).about(about).on(on)
}

/// The tooltip for command `id`: its name and key, then `about`, then why it cannot run now.
pub fn tip(rb: &Rb<'_>, id: &str, about: &str) -> String {
    w::tip(rb.shell, rb.ed, id, about)
}

/// Note that a control running command `id` (none: a drop-down) was drawn at `r`.
pub fn record(rb: &mut Rb<'_>, id: Option<&str>, r: Rect) {
    w::record(rb.shell, id, r);
}

/// A large button: icon over a label. `on` draws it pressed (a toggle).
pub fn big(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, icon: &str, label: &str, about: &str, on: bool) -> bool {
    w::big(ui, rb.shell, rb.ed, &control(c, icon, label, about, on))
}

/// A small button: icon and label (an empty label shows the icon alone).
pub fn small(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, icon: &str, label: &str, about: &str, on: bool) -> bool {
    w::small(ui, rb.shell, rb.ed, &control(c, icon, label, about, on))
}

/// A drop-down button in a group; `add` draws its items.
pub fn drop(ui: &mut Ui, rb: &mut Rb<'_>, label: impl Into<egui::RichText>, about: &str, add: impl FnOnce(&mut Ui, &mut Rb<'_>)) {
    let (ed, data) = (rb.ed, rb.data.clone());
    w::drop(ui, rb.shell, label, about, |ui, shell| add(ui, &mut Rb { shell, ed, data }));
}

/// The menu under `resp` (a drop-down button or chip), opened by a click or its key tip.
pub fn popup_menu(ui: &mut Ui, rb: &mut Rb<'_>, resp: &egui::Response, name: &str, add: impl FnOnce(&mut Ui, &mut Rb<'_>)) {
    let (ed, data) = (rb.ed, rb.data.clone());
    w::popup_menu(ui, rb.shell, resp, name, |ui, shell| add(ui, &mut Rb { shell, ed, data }));
}

/// A menu of the menu bar, or a submenu.
pub fn menu(ui: &mut Ui, rb: &mut Rb<'_>, title: &str, add: impl FnOnce(&mut Ui, &mut Rb<'_>)) {
    let (ed, data) = (rb.ed, rb.data.clone());
    w::menu(ui, rb.shell, title, |ui, shell| add(ui, &mut Rb { shell, ed, data }));
}

/// A plain button that runs `c`.
pub fn button(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, label: &str, about: &str) -> bool {
    w::button(ui, rb.shell, rb.ed, &Control::of(c).label(label).about(about))
}

/// An item of a drop-down menu, with its key, greyed out when it cannot run.
pub fn item(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, label: &str) -> bool {
    w::item(ui, rb.shell, rb.ed, &Control::of(c).label(label))
}

/// A menu item that is on or off.
pub fn check(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, label: &str, on: bool) -> bool {
    w::check(ui, rb.shell, rb.ed, &Control::of(c).label(label).on(on))
}

/// A choice among several in a menu (one is current).
pub fn choice(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, label: &str, on: bool) -> bool {
    w::choice(ui, rb.shell, rb.ed, &Control::of(c).label(label).on(on))
}

/// A group: its controls in a row with its title under them; it shrinks or folds to fit.
pub fn group(ui: &mut Ui, rb: &mut Rb<'_>, title: &str, add: impl FnOnce(&mut Ui, &mut Rb<'_>)) {
    let (ed, data) = (rb.ed, rb.data.clone());
    w::group(ui, rb.shell, title, |ui, shell| add(ui, &mut Rb { shell, ed, data }));
}

/// A tile in a gallery: icon, name and a second line (a key, a hint). `on` marks the current one.
pub fn tile(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, icon: &str, text: (&str, &str), about: &str, on: bool) -> bool {
    w::tile(ui, rb.shell, rb.ed, &control(c, icon, text.0, about, on).second(text.1))
}

/// A tile `width` wide. With no second line, the name sits in the middle.
pub fn tile_sized(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, icon: &str, text: (&str, &str), about: &str, width: f32) -> bool {
    w::tile(ui, rb.shell, rb.ed, &control(c, icon, text.0, about, false).second(text.1).width(width))
}

/// A control drawn as text (the status bar).
pub fn link(ui: &mut Ui, rb: &mut Rb<'_>, c: Cmd, text: impl Into<egui::RichText>, about: &str) -> bool {
    w::link(ui, rb.shell, rb.ed, &Control::of(c).about(about), text)
}

/// A color chip (the current revision set); its response, for a menu under it.
pub fn chip(ui: &mut Ui, color: Color32, label: &str, about: &str) -> egui::Response {
    w::chip(ui, color, label, about)
}
