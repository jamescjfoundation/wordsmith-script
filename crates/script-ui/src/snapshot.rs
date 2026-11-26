// SPDX-License-Identifier: MIT OR Apache-2.0
//! The window drawn offscreen, for the README's pictures and visual checks: the shell and the
//! editor run headlessly for a few frames and are rasterized on the CPU (`jcj_shell::snapshot`).

use egui::{Event, Vec2};
use jcj_shell::Shell;
use jcj_shell::snapshot::Picture;

use crate::editor::Editor;

/// The window (`shell` around `ed`) drawn at `size` points and `ppp` pixels per point, after
/// `frames` frames, the first feeding `events`.
pub fn render(shell: &mut Shell, ed: &mut Editor, size: Vec2, ppp: f32, events: Vec<Event>, frames: usize) -> Picture {
    // Page images belong to one egui context; this picture has a new one.
    ed.forget_drawn_pages();
    let ctx = egui::Context::default();
    // The shell installs its fonts and theme on its first frame only; this context is new.
    crate::fonts::install(&ctx);
    jcj_theme::apply(&ctx, &shell.theme());
    let pic = jcj_shell::snapshot::render(&ctx, size, ppp, events, frames, |ui| shell.ui(ui, ed));
    ed.forget_drawn_pages();
    pic
}
