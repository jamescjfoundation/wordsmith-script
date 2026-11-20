// SPDX-License-Identifier: MIT OR Apache-2.0
//! The interface's fonts. egui's bundled fonts are switched off (their licenses are not all on
//! our allowlist), so the interface uses fonts we already ship: Inter for text and JetBrains Mono
//! for monospace, both OFL-1.1 (`ATTRIBUTION.md`).

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

static INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
static MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
static INTER_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Inter-Medium.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

/// The interface's font definitions.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert("Inter".into(), Arc::new(FontData::from_static(INTER)));
    fonts.font_data.insert("JetBrains Mono".into(), Arc::new(FontData::from_static(MONO)));
    fonts.families.insert(FontFamily::Proportional, vec!["Inter".into(), "JetBrains Mono".into()]);
    fonts.families.insert(FontFamily::Monospace, vec!["JetBrains Mono".into(), "Inter".into()]);
    // The interface's heavier weights (ribbon labels, panel titles).
    fonts.font_data.insert("Inter Medium".into(), Arc::new(FontData::from_static(INTER_MEDIUM)));
    fonts.font_data.insert("Inter SemiBold".into(), Arc::new(FontData::from_static(INTER_SEMIBOLD)));
    fonts.families.insert(FontFamily::Name("medium".into()), vec!["Inter Medium".into(), "Inter".into(), "JetBrains Mono".into()]);
    fonts.families.insert(FontFamily::Name("semibold".into()), vec!["Inter SemiBold".into(), "Inter".into(), "JetBrains Mono".into()]);
    fonts
}

/// Install the interface fonts.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}
