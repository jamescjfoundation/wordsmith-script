// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Wordsmith Script mark, wordmark and app icon (`assets/brand/`). The interface's colors are
//! the design tokens (`assets/brand/script/tokens.json`); the few here are the logo's own.
//!
//! The mark is a W followed by a brass caret bar. The W and the wordmark's letters are drawn from
//! white masks tinted at paint time, so one picture serves every theme; the caret bar is a plain
//! rectangle in brass.

use egui::{Color32, Context, Id, Painter, Pos2, Rect, TextureHandle, TextureOptions, pos2, vec2};

/// Iron Gall Ink: the mark, text, the File page.
pub const INK: Color32 = Color32::from_rgb(0x1C, 0x23, 0x40);
/// Page White: the light surfaces, text on ink.
pub const PAGE_WHITE: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xF7);
/// Brad Brass: the caret bar and other small highlights.
pub const BRASS: Color32 = Color32::from_rgb(0xB8, 0x86, 0x2B);
/// Night: the dark theme's ground.
pub const NIGHT: Color32 = Color32::from_rgb(0x12, 0x16, 0x2B);

/// The app icon at 256 pixels (the window's icon).
pub const ICON_PNG: &[u8] = include_bytes!("../../../assets/brand/icon-256.png");

const MARK_MASK: &[u8] = include_bytes!("../../../assets/brand/mark-mask.png");
const WORDMARK_MASK: &[u8] = include_bytes!("../../../assets/brand/wordmark-mask.png");

/// A picture drawn from an SVG: its view box, and where the caret bar sits in it.
struct Art {
    key: &'static str,
    png: &'static [u8],
    size: (f32, f32),
    /// x, width and height of the caret bar (it starts at the top).
    bar: (f32, f32, f32),
}

const MARK: Art = Art { key: "mark", png: MARK_MASK, size: (313.0, 239.8), bar: (251.84, 61.20, 239.84) };
const WORDMARK: Art = Art { key: "wordmark", png: WORDMARK_MASK, size: (977.7, 80.0), bar: (962.38, 15.30, 59.96) };

/// The app icon for the window, decoded from its PNG.
pub fn icon() -> Option<egui::IconData> {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?.into_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

/// The mask's texture, loaded once per context. The second value is the share of the texture
/// the view box covers (the PNG is rounded up to whole pixels).
fn texture(ctx: &Context, art: &Art) -> Option<(TextureHandle, egui::Vec2)> {
    let id = Id::unique(("wordsmith-brand", art.key));
    let tex = match ctx.data(|d| d.get_temp::<Option<TextureHandle>>(id)) {
        Some(tex) => tex,
        None => {
            let tex = image::load_from_memory_with_format(art.png, image::ImageFormat::Png).ok().map(|img| {
                let img = img.into_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                // White with the mask's coverage, so a tint paints it in any color.
                let pixels = img.pixels().map(|p| Color32::from_white_alpha(p[3])).collect();
                ctx.load_texture(format!("brand-{}", art.key), egui::ColorImage::new(size, pixels), TextureOptions::LINEAR)
            });
            ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
            tex
        }
    }?;
    let [w, h] = tex.size();
    let scale = w as f32 / art.size.0;
    let cover = vec2(1.0, (art.size.1 * scale / h as f32).min(1.0));
    Some((tex, cover))
}

fn paint(p: &Painter, art: &Art, min: Pos2, height: f32, ink: Color32, bar: Color32) -> Rect {
    let s = height / art.size.1;
    let rect = Rect::from_min_size(min, vec2(art.size.0 * s, height));
    if let Some((tex, cover)) = texture(p.ctx(), art) {
        p.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(cover.x, cover.y)), ink);
    }
    let (x, w, h) = art.bar;
    p.rect_filled(Rect::from_min_size(pos2(min.x + x * s, min.y), vec2(w * s, h * s)), 0.0, bar);
    rect
}

/// Paint the mark (W and caret bar) `height` points tall from `min`; returns the rectangle it took.
pub fn paint_mark(p: &Painter, min: Pos2, height: f32, ink: Color32) -> Rect {
    paint(p, &MARK, min, height, ink, BRASS)
}

/// The mark's width at `height`.
pub fn mark_width(height: f32) -> f32 {
    height * MARK.size.0 / MARK.size.1
}

/// Paint the wordmark `width` points wide from `min`; returns the rectangle it took.
pub fn paint_wordmark(p: &Painter, min: Pos2, width: f32, ink: Color32) -> Rect {
    paint(p, &WORDMARK, min, width * WORDMARK.size.1 / WORDMARK.size.0, ink, BRASS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_and_masks_decode() {
        let icon = icon().unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        // The tile is ink at its center-left, and transparent in its rounded corner.
        let at = |x: u32, y: u32| &icon.rgba[((y * 256 + x) * 4) as usize..][..4];
        assert_eq!(at(20, 128), [0x1C, 0x23, 0x40, 0xFF]);
        assert_eq!(at(0, 0)[3], 0);
        for art in [&MARK, &WORDMARK] {
            let img = image::load_from_memory(art.png).unwrap();
            // Rendered from the view box at its aspect ratio.
            let ratio = art.size.0 / art.size.1;
            assert!((img.width() as f32 / img.height() as f32 - ratio).abs() < 0.2, "{}", art.key);
        }
    }

    #[test]
    fn the_file_page_shows_the_mark_in_ink_and_brass() {
        let mut w = crate::tests::window();
        w.shell.open_file_page(jcj_shell::Page::About);
        let pic = crate::snapshot::render(&mut w.shell, &mut w.ed, egui::vec2(900.0, 600.0), 1.0, Vec::new(), 3);
        let count = |c: Color32| {
            let want = [c.r(), c.g(), c.b(), 255];
            (0..pic.height).flat_map(|y| (0..pic.width).map(move |x| (x, y))).filter(|&(x, y)| pic.at(x, y) == want).count()
        };
        // The column is ink; the caret bars (the marks, the current page) are brass.
        assert!(count(INK) > 10_000, "no ink column");
        assert!(count(BRASS) > 500, "no brass caret");
    }
}
