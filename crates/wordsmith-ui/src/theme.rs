// SPDX-License-Identifier: MIT OR Apache-2.0
//! Design tokens (v0.3 plan, "Look"): every color, size and space the interface's chrome uses,
//! in three themes. The idea follows WordCraft's `Tokens`; the values are our own.

use egui::{Color32, Context, FontFamily, FontId, Id};

/// The interface's colors and sizes for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Behind the page.
    pub canvas: Color32,
    /// Ribbon, panels, status bar.
    pub surface: Color32,
    /// The quick-access row and tab strip.
    pub surface_high: Color32,
    /// Fields and list rows.
    pub input: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    /// The current tab, controls that are on, focus.
    pub accent: Color32,
    pub accent_soft: Color32,
    /// Hovered controls.
    pub hover: Color32,
    pub danger: Color32,
    pub dark: bool,
}

pub const LIGHT: Tokens = Tokens {
    canvas: Color32::from_rgb(0xE9, 0xEA, 0xEE),
    surface: Color32::from_rgb(0xF7, 0xF7, 0xF9),
    surface_high: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    input: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    border: Color32::from_rgb(0xD9, 0xDA, 0xE0),
    text: Color32::from_rgb(0x1E, 0x21, 0x28),
    text_dim: Color32::from_rgb(0x6A, 0x6F, 0x7A),
    accent: Color32::from_rgb(0x2F, 0x5F, 0xC8),
    accent_soft: Color32::from_rgb(0xDD, 0xE6, 0xF8),
    hover: Color32::from_rgb(0xEC, 0xEE, 0xF3),
    danger: Color32::from_rgb(0xC0, 0x39, 0x2B),
    dark: false,
};

pub const DARK: Tokens = Tokens {
    canvas: Color32::from_rgb(0x1C, 0x1E, 0x23),
    surface: Color32::from_rgb(0x26, 0x28, 0x2E),
    surface_high: Color32::from_rgb(0x2D, 0x30, 0x37),
    input: Color32::from_rgb(0x1F, 0x21, 0x26),
    border: Color32::from_rgb(0x3A, 0x3D, 0x45),
    text: Color32::from_rgb(0xE6, 0xE7, 0xEA),
    text_dim: Color32::from_rgb(0x9A, 0x9E, 0xA8),
    accent: Color32::from_rgb(0x7A, 0xA2, 0xF0),
    accent_soft: Color32::from_rgb(0x2E, 0x3A, 0x55),
    hover: Color32::from_rgb(0x33, 0x36, 0x3E),
    danger: Color32::from_rgb(0xE5, 0x73, 0x6A),
    dark: true,
};

pub const EXTRA_DARK: Tokens = Tokens {
    canvas: Color32::from_rgb(0x06, 0x06, 0x08),
    surface: Color32::from_rgb(0x10, 0x11, 0x13),
    surface_high: Color32::from_rgb(0x16, 0x17, 0x1A),
    input: Color32::from_rgb(0x0B, 0x0B, 0x0D),
    border: Color32::from_rgb(0x26, 0x28, 0x2D),
    text: Color32::from_rgb(0xD4, 0xD5, 0xD9),
    text_dim: Color32::from_rgb(0x84, 0x88, 0x91),
    accent: Color32::from_rgb(0x6C, 0x93, 0xE0),
    accent_soft: Color32::from_rgb(0x1E, 0x27, 0x3B),
    hover: Color32::from_rgb(0x1D, 0x1F, 0x23),
    danger: Color32::from_rgb(0xD9, 0x6A, 0x60),
    dark: true,
};

/// Type sizes.
pub const BODY: f32 = 12.5;
pub const CAPTION: f32 = 11.0;
pub const TITLE: f32 = 13.5;

fn id() -> Id {
    Id::new("wordsmith-theme-tokens")
}

impl Tokens {
    /// The tokens for the theme the editor shows.
    pub fn of(dark: bool, extra_dark: bool) -> Tokens {
        match (dark, extra_dark) {
            (true, true) => EXTRA_DARK,
            (true, false) => DARK,
            _ => LIGHT,
        }
    }

    /// The tokens in use this frame (light until the editor sets them).
    pub fn get(ctx: &Context) -> Tokens {
        ctx.data(|d| d.get_temp::<Tokens>(id())).unwrap_or(LIGHT)
    }

    /// Put these tokens in use.
    pub fn set(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(id(), self));
    }
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

/// egui's visuals for these tokens.
pub fn visuals(t: &Tokens) -> egui::Visuals {
    let mut v = if t.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = t.surface;
    v.window_fill = t.surface_high;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.hover;
    v.window_stroke = egui::Stroke::new(1.0, t.border);
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = egui::Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, t.border);
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_fill = t.hover;
    v.window_corner_radius = egui::CornerRadius::same(8);
    v
}
