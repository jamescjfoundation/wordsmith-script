// SPDX-License-Identifier: MIT OR Apache-2.0
//! The interface's fonts, handed to the shell (`Shell::with_fonts`): Inter for text and JetBrains
//! Mono for monospace, both OFL-1.1 (`ATTRIBUTION.md`). The page font, Courier Prime, is the
//! layout's (`wordsmith_script_model::fonts`).

use jcj_theme::FontFace;

static INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
static INTER_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Inter-Medium.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
static MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");

/// The interface's faces, under the names the tokens give them.
pub fn faces() -> Vec<FontFace> {
    vec![
        FontFace::from_static("Inter", INTER),
        FontFace::from_static("Inter Medium", INTER_MEDIUM),
        FontFace::from_static("Inter SemiBold", INTER_SEMIBOLD),
        FontFace::from_static("JetBrains Mono", MONO),
    ]
}

/// Install the interface fonts in `ctx` (the shell does it on its first frame; this is for
/// drawing without one).
pub fn install(ctx: &egui::Context) {
    jcj_theme::install_fonts(ctx, &crate::theme::tokens().fonts, &faces());
}
