// SPDX-License-Identifier: MIT OR Apache-2.0
//! The interface drawn offscreen (decision 0029): run the editor headlessly for a few frames and
//! rasterize egui's output on the CPU (its triangles, their vertex colors and its textures: the
//! font atlas and the page tiles), so the README's pictures and visual checks come from the app
//! itself, with no window and no GPU.

use std::collections::HashMap;

use egui::epaint::{ClippedPrimitive, Primitive, TextureId, Vertex};
use egui::{Color32, Event, Pos2, RawInput, Rect, TexturesDelta, Vec2};

use crate::editor::Editor;

/// An RGBA picture, straight (not premultiplied) alpha.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// The picture as a PNG file's bytes.
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let Some(img) = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone()) else { return out };
        let _ = image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png);
        out
    }

    /// The color at pixel (x, y).
    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }
}

/// Textures as the frames left them: id → (width, height, premultiplied texels).
#[derive(Default)]
struct Textures(HashMap<TextureId, (usize, usize, Vec<Color32>)>);

impl Textures {
    fn apply(&mut self, mut delta: TexturesDelta) {
        for (id, deltas) in std::mem::take(&mut delta.set) {
            for d in deltas {
                let egui::ImageData::Color(img) = d.image;
                let [w, h] = img.size;
                match d.pos {
                    None => {
                        self.0.insert(id, (w, h, img.pixels.clone()));
                    }
                    Some([x0, y0]) => {
                        if let Some((tw, th, px)) = self.0.get_mut(&id) {
                            for y in 0..h {
                                for x in 0..w {
                                    if x0 + x < *tw && y0 + y < *th {
                                        px[(y0 + y) * *tw + x0 + x] = img.pixels[y * w + x];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for id in std::mem::take(&mut delta.free) {
            self.0.remove(&id);
        }
    }

    /// The texel at `uv`, nearest, premultiplied.
    fn sample(&self, id: TextureId, uv: Pos2) -> [f32; 4] {
        let Some((w, h, px)) = self.0.get(&id) else { return [1.0; 4] };
        let x = ((uv.x * *w as f32) as isize).clamp(0, *w as isize - 1) as usize;
        let y = ((uv.y * *h as f32) as isize).clamp(0, *h as isize - 1) as usize;
        let c = px[y * w + x];
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, c.a() as f32 / 255.0]
    }
}

fn color(v: &Vertex) -> [f32; 4] {
    [v.color.r() as f32 / 255.0, v.color.g() as f32 / 255.0, v.color.b() as f32 / 255.0, v.color.a() as f32 / 255.0]
}

/// Draw one triangle into `buf` (premultiplied RGBA floats), clipped to `clip` (pixels).
fn triangle(buf: &mut [[f32; 4]], size: (usize, usize), clip: Rect, tex: &Textures, id: TextureId, vs: [&Vertex; 3], ppp: f32) {
    let p = vs.map(|v| v.pos * ppp);
    let area = (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[2].x - p[0].x) * (p[1].y - p[0].y);
    if area.abs() < 1e-6 {
        return;
    }
    let min_x = p.iter().map(|q| q.x).fold(f32::INFINITY, f32::min).max(clip.min.x).max(0.0).floor() as usize;
    let max_x = p.iter().map(|q| q.x).fold(f32::NEG_INFINITY, f32::max).min(clip.max.x).min(size.0 as f32).ceil() as usize;
    let min_y = p.iter().map(|q| q.y).fold(f32::INFINITY, f32::min).max(clip.min.y).max(0.0).floor() as usize;
    let max_y = p.iter().map(|q| q.y).fold(f32::NEG_INFINITY, f32::max).min(clip.max.y).min(size.1 as f32).ceil() as usize;
    let cs = vs.map(color);
    for y in min_y..max_y {
        for x in min_x..max_x {
            let c = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = ((p[1].x - c.x) * (p[2].y - c.y) - (p[2].x - c.x) * (p[1].y - c.y)) / area;
            let w1 = ((p[2].x - c.x) * (p[0].y - c.y) - (p[0].x - c.x) * (p[2].y - c.y)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let uv = Pos2::new(vs[0].uv.x * w0 + vs[1].uv.x * w1 + vs[2].uv.x * w2, vs[0].uv.y * w0 + vs[1].uv.y * w1 + vs[2].uv.y * w2);
            let t = tex.sample(id, uv);
            let src: [f32; 4] = std::array::from_fn(|k| (cs[0][k] * w0 + cs[1][k] * w1 + cs[2][k] * w2) * t[k]);
            let dst = &mut buf[y * size.0 + x];
            for k in 0..4 {
                dst[k] = src[k] + dst[k] * (1.0 - src[3]);
            }
        }
    }
}

/// Rasterize tessellated primitives at `ppp` pixels per point onto a `size` pixel canvas.
fn rasterize(prims: &[ClippedPrimitive], tex: &Textures, size: (usize, usize), ppp: f32, background: Color32) -> Picture {
    let bg = [background.r() as f32 / 255.0, background.g() as f32 / 255.0, background.b() as f32 / 255.0, 1.0];
    let mut buf = vec![bg; size.0 * size.1];
    for prim in prims {
        let Primitive::Mesh(mesh) = &prim.primitive else { continue };
        let clip = Rect::from_min_max(prim.clip_rect.min * ppp, prim.clip_rect.max * ppp);
        for [i, j, k] in mesh.indices.as_chunks::<3>().0 {
            let (Some(a), Some(b), Some(c)) = (mesh.vertices.get(*i as usize), mesh.vertices.get(*j as usize), mesh.vertices.get(*k as usize)) else {
                continue;
            };
            triangle(&mut buf, size, clip, tex, mesh.texture_id, [a, b, c], ppp);
        }
    }
    let rgba = buf
        .iter()
        .flat_map(|p| {
            let a = p[3].clamp(0.0, 1.0);
            let un = |v: f32| if a > 0.0 { (v / a).clamp(0.0, 1.0) } else { 0.0 };
            [(un(p[0]) * 255.0).round() as u8, (un(p[1]) * 255.0).round() as u8, (un(p[2]) * 255.0).round() as u8, (a * 255.0).round() as u8]
        })
        .collect();
    Picture { width: size.0 as u32, height: size.1 as u32, rgba }
}

/// The editor drawn offscreen at `size` points and `ppp` pixels per point, after `frames` frames
/// (the first feeding `events`), so layouts and textures settle.
pub fn render(ed: &mut Editor, size: Vec2, ppp: f32, events: Vec<Event>, frames: usize) -> Picture {
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::fonts::definitions());
    let mut textures = Textures::default();
    let mut shapes = Vec::new();
    let mut events = Some(events);
    for _ in 0..frames.max(1) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events: events.take().unwrap_or_default(),
            viewports: std::iter::once((egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(ppp), ..Default::default() }))
                .collect(),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| ed.ui(ui));
        textures.apply(out.textures_delta);
        shapes = out.shapes;
    }
    let prims = ctx.tessellate(shapes, ppp);
    let px = ((size.x * ppp).round() as usize, (size.y * ppp).round() as usize);
    let bg = ctx.style_of(ctx.theme()).visuals.panel_fill;
    rasterize(&prims, &textures, px, ppp, bg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_draws_its_shapes_and_text() {
        let mut ed = crate::editor::Editor::new(crate::new_session());
        ed.mac = false;
        let pic = render(&mut ed, Vec2::new(800.0, 600.0), 1.0, vec![Event::Text("int".into())], 3);
        assert_eq!((pic.width, pic.height), (800, 600));
        // The page is white paper; the interface around it is not.
        let mut colors = std::collections::HashSet::new();
        for y in (0..600).step_by(7) {
            for x in (0..800).step_by(7) {
                colors.insert(pic.at(x, y));
            }
        }
        assert!(colors.len() > 20, "a real picture, not a flat fill: {}", colors.len());
        assert!(colors.contains(&[255, 255, 255, 255]), "the page");
        let png = pic.to_png();
        assert!(png.starts_with(b"\x89PNG"));
    }
}
