// SPDX-License-Identifier: MIT OR Apache-2.0
//! The theme as the screenplay views draw with it. The colors, fonts and sizes are Wordsmith
//! Script's design tokens (`assets/brand/script/tokens.json`), turned into a theme by `jcj-theme`;
//! this module only gives the views short names for what they read.

use egui::Context;

pub use jcj_theme::{medium, mono, regular, semibold};

/// The theme in use (its colors: `canvas`, `surface`, `text`, `accent`, …).
pub type Tokens = jcj_theme::Theme;

/// Body text size, points.
pub const BODY: f32 = 12.5;
/// Caption size, points.
pub const CAPTION: f32 = 11.0;
/// Title size, points.
pub const TITLE: f32 = 13.5;

/// The theme in use this frame.
pub fn get(ctx: &Context) -> Tokens {
    jcj_theme::Theme::get(ctx)
}

/// Wordsmith Script's design tokens ([`crate::brand::BRAND`]; the neutral set if the file does
/// not read, which a test rules out), read once.
pub fn tokens() -> jcj_theme::Tokens {
    static TOKENS: std::sync::OnceLock<jcj_theme::Tokens> = std::sync::OnceLock::new();
    TOKENS.get_or_init(|| crate::brand::BRAND.tokens()).clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tokens_read_and_carry_the_brand() {
        let t = crate::brand::BRAND.read_tokens().unwrap();
        let light = t.theme(jcj_theme::Mode::Light);
        assert_eq!(light.brand, crate::brand::INK);
        assert_eq!(light.highlight, crate::brand::BRASS);
        assert_eq!(light.surface, crate::brand::PAGE_WHITE);
        assert_eq!(t.theme(jcj_theme::Mode::Dark).canvas, crate::brand::NIGHT);
        assert_eq!(t.fonts.regular.as_deref(), Some("Inter"));
    }
}
