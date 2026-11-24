// SPDX-License-Identifier: MIT OR Apache-2.0
//! Design tokens (decision 0037): every color, size and space the interface's chrome uses,
//! in three themes. The idea follows WordCraft's `Tokens`; the values are our own, drawn from the
//! Wordsmith Script palette (`crate::brand`): Iron Gall Ink, Page White, Brad Brass, Margin Blue,
//! Slate and Night.

use egui::{Color32, Context, FontFamily, FontId, Id};

use crate::brand;

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
    /// The File page's column and the File button: an ink blue in every theme.
    pub brand: Color32,
    /// Text and icons on `brand`.
    pub on_brand: Color32,
    pub dark: bool,
}

pub const LIGHT: Tokens = Tokens {
    canvas: Color32::from_rgb(0xE6, 0xE7, 0xEC),
    surface: brand::PAGE_WHITE,
    surface_high: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    input: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    border: Color32::from_rgb(0xD8, 0xDA, 0xE3),
    text: brand::INK,
    text_dim: brand::SLATE,
    accent: brand::MARGIN_BLUE,
    accent_soft: Color32::from_rgb(0xE1, 0xE5, 0xF5),
    hover: Color32::from_rgb(0xED, 0xEE, 0xF3),
    danger: Color32::from_rgb(0xB3, 0x36, 0x2A),
    brand: brand::INK,
    on_brand: brand::PAGE_WHITE,
    dark: false,
};

pub const DARK: Tokens = Tokens {
    canvas: brand::NIGHT,
    surface: Color32::from_rgb(0x1A, 0x1F, 0x37),
    surface_high: Color32::from_rgb(0x20, 0x26, 0x3F),
    input: Color32::from_rgb(0x15, 0x1A, 0x30),
    border: Color32::from_rgb(0x2F, 0x36, 0x53),
    text: Color32::from_rgb(0xEC, 0xEC, 0xE8),
    text_dim: Color32::from_rgb(0x9D, 0xA2, 0xBE),
    accent: Color32::from_rgb(0x8D, 0x9D, 0xE6),
    accent_soft: Color32::from_rgb(0x2A, 0x33, 0x66),
    hover: Color32::from_rgb(0x26, 0x2C, 0x48),
    danger: Color32::from_rgb(0xE5, 0x73, 0x6A),
    brand: Color32::from_rgb(0x27, 0x30, 0x5C),
    on_brand: brand::PAGE_WHITE,
    dark: true,
};

pub const EXTRA_DARK: Tokens = Tokens {
    canvas: Color32::from_rgb(0x07, 0x08, 0x0F),
    surface: Color32::from_rgb(0x0E, 0x10, 0x20),
    surface_high: Color32::from_rgb(0x13, 0x16, 0x2A),
    input: Color32::from_rgb(0x0A, 0x0C, 0x18),
    border: Color32::from_rgb(0x23, 0x27, 0x42),
    text: Color32::from_rgb(0xD6, 0xD7, 0xDE),
    text_dim: Color32::from_rgb(0x87, 0x8C, 0xA8),
    accent: Color32::from_rgb(0x7F, 0x8F, 0xD8),
    accent_soft: Color32::from_rgb(0x1C, 0x22, 0x46),
    hover: Color32::from_rgb(0x18, 0x1B, 0x30),
    danger: Color32::from_rgb(0xD9, 0x6A, 0x60),
    brand: brand::INK,
    on_brand: brand::PAGE_WHITE,
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
