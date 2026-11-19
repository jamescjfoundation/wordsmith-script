// SPDX-License-Identifier: MIT OR Apache-2.0
//! The script editor: pages drawn from the layout, the caret and selection, and every key press,
//! click and menu choice turned into a command run through the session's registry.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Event, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordsmith_doc::StoryRef;
use wordsmith_engine::Session;
use wordsmith_script::paginate::Paginated;
use wordsmith_script::{ElementKind, Elements};

use crate::dialogs::{Palette, RecoveryPrompt, SettingsDialog};
use crate::keys;

/// Space between pages and around them, in points on screen.
const GAP: f32 = 16.0;
/// Zoom limits.
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.5..=3.0;

/// A page's raster, for one document revision and one scale.
struct PageTexture {
    rev: u64,
    scale_bits: u32,
    texture: TextureHandle,
}

/// One script window's editor.
pub struct Editor {
    pub session: Session,
    pages: HashMap<usize, PageTexture>,
    /// Points on screen per document point.
    pub zoom: f32,
    /// Apply macOS key conventions.
    pub mac: bool,
    pub palette: Option<Palette>,
    pub settings: Option<SettingsDialog>,
    pub recovery: Option<RecoveryPrompt>,
    pub show_about: bool,
    /// Scroll the caret into view on the next frame (after a keyboard command).
    scroll_to_caret: bool,
    /// The last error a command reported, for the status bar.
    pub message: String,
    /// Text a copy or cut asked to put on the clipboard.
    clipboard: Option<String>,
    /// Pagination, kept between edits so only changed paragraphs are measured again.
    paginator: wordsmith_script::paginate::Paginator,
    /// The paginated script for the session's current revision.
    paginated: Option<(u64, Arc<Paginated>)>,
}

impl Editor {
    pub fn new(session: Session) -> Self {
        Editor {
            session,
            pages: HashMap::new(),
            zoom: 1.0,
            mac: cfg!(target_os = "macos"),
            palette: None,
            settings: None,
            recovery: None,
            show_about: false,
            scroll_to_caret: true,
            message: String::new(),
            clipboard: None,
            paginator: Default::default(),
            paginated: None,
        }
    }

    /// Run a command; failures go to the status bar, never panic.
    pub fn run(&mut self, id: &str, params: Value) {
        match self.session.run(id, &params) {
            Ok(_) => self.message.clear(),
            Err(e) => self.message = e.to_string(),
        }
        self.scroll_to_caret = true;
        self.take_ui_requests();
    }

    /// Act on what commands asked of the interface (open a dialog, copy text).
    fn take_ui_requests(&mut self) {
        for req in std::mem::take(&mut self.session.ui_requests) {
            if let Some(text) = req.get("copy").and_then(Value::as_str) {
                self.clipboard = Some(text.to_string());
            }
            match req.get("open").and_then(Value::as_str) {
                Some("palette") => self.palette = Some(Palette::default()),
                Some("elementSettings") => {
                    let current = Elements::of_doc(&self.session.doc).of_para_at(&self.session).to_string();
                    self.settings = Some(SettingsDialog::new(&mut self.session, &current));
                }
                Some("about") => self.show_about = true,
                Some("recover") => self.recovery = Some(RecoveryPrompt),
                _ => {}
            }
        }
    }

    /// Text waiting for the clipboard (for tests).
    pub fn pending_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    /// Whether the editor should take keyboard input now (no dialog or text field has it).
    fn has_keyboard(&self, ui: &Ui) -> bool {
        self.palette.is_none() && self.settings.is_none() && !ui.ctx().text_edit_focused()
    }

    /// Turn this frame's keyboard events into commands.
    pub fn handle_keys(&mut self, ui: &Ui) {
        if !self.has_keyboard(ui) {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.palette = None;
                self.settings = None;
            }
            return;
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                Event::Text(text) => self.run("text.insert", json!({"text": text})),
                Event::Paste(text) => self.run("text.insert", json!({"text": text})),
                Event::Copy => self.run("edit.copy", json!({})),
                Event::Cut => self.run("edit.cut", json!({})),
                Event::Key { key, pressed: true, modifiers, .. } => {
                    let Some(shortcut) = keys::shortcut(key, modifiers, self.mac) else { continue };
                    self.press(&shortcut);
                }
                _ => {}
            }
        }
        // Copy and cut put text on the clipboard through the commands' requests.
        if let Some(text) = self.clipboard.take() {
            ui.ctx().copy_text(text);
        }
        // Tab and arrows are ours, not egui's focus navigation.
        ui.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { key: egui::Key::Tab, .. })));
    }

    /// Run the command bound to `shortcut`: a custom element's own shortcut first, then the
    /// registry's.
    pub fn press(&mut self, shortcut: &str) {
        let elements = Elements::of_doc(&self.session.doc);
        if let Some(def) =
            elements.all().iter().find(|d| d.kind == ElementKind::Custom && d.shortcut.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(shortcut)))
        {
            let id = def.id.clone();
            self.run("element.set", json!({"element": id}));
            return;
        }
        let id = self.session.registry.by_shortcut(shortcut).map(|spec| spec.id);
        if let Some(id) = id {
            self.run(id, json!({}));
        }
    }

    /// The paginated script for the current revision, paginating again after an edit.
    pub fn paginated(&mut self) -> Arc<Paginated> {
        let rev = self.session.rev();
        if let Some((r, p)) = &self.paginated
            && *r == rev
        {
            return p.clone();
        }
        let p = Arc::new(self.paginator.paginate(&self.session.doc));
        self.paginated = Some((rev, p.clone()));
        p
    }

    /// A page's texture at `scale` pixels per document point, drawn again when the document or
    /// scale changed.
    fn texture(
        &mut self,
        ctx: &egui::Context,
        doc: &wordsmith_doc::Document,
        index: usize,
        page: &wordsmith_layout::Page,
        scale: f32,
    ) -> TextureHandle {
        let rev = self.session.rev();
        let bits = scale.to_bits();
        if let Some(t) = self.pages.get(&index)
            && t.rev == rev
            && t.scale_bits == bits
        {
            return t.texture.clone();
        }
        let opts = wordsmith_render::RenderOptions::default();
        let r = wordsmith_render::render_page(doc, page, scale, &opts);
        let image = egui::ColorImage::from_rgba_premultiplied([r.width as usize, r.height as usize], &r.pixels);
        let texture = match self.pages.remove(&index) {
            Some(mut t) => {
                t.texture.set(image, TextureOptions::LINEAR);
                t.texture
            }
            None => ctx.load_texture(format!("page-{index}"), image, TextureOptions::LINEAR),
        };
        self.pages.insert(index, PageTexture { rev, scale_bits: bits, texture: texture.clone() });
        texture
    }

    /// The pages, caret and selection, with mouse handling. The pages are the paginated script
    /// (with its (MORE) and (CONT'D) lines and page numbers); the caret and clicks map to the
    /// script itself.
    pub fn page_view(&mut self, ui: &mut Ui) {
        let pag = self.paginated();
        let layout = pag.layout.clone();
        let zoom = self.zoom;
        let ppp = ui.ctx().pixels_per_point();
        let widest = layout.pages.iter().map(|p| p.w).fold(0.0f32, f32::max) * zoom;
        let height: f32 = layout.pages.iter().map(|p| p.h * zoom + GAP).sum::<f32>() + GAP;
        let width = (widest + 2.0 * GAP).max(ui.available_width());
        let (area, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        // The page view keeps keyboard focus, and Tab and the arrows stay ours instead of moving
        // egui's focus between widgets.
        if ui.memory(|m| m.focused().is_none()) || response.clicked() {
            response.request_focus();
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(response.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false })
        });
        let painter = ui.painter_at(area);
        painter.rect_filled(area, 0.0, Color32::from_gray(96));

        let mut origins: Vec<Rect> = Vec::with_capacity(layout.pages.len());
        let mut y = area.top() + GAP;
        for page in &layout.pages {
            let size = vec2(page.w * zoom, page.h * zoom);
            origins.push(Rect::from_min_size(pos2(area.center().x - size.x / 2.0, y), size));
            y += size.y + GAP;
        }
        for (i, page) in layout.pages.iter().enumerate() {
            let Some(rect) = origins.get(i).copied() else { continue };
            if ui.is_rect_visible(rect) {
                let tex = self.texture(ui.ctx(), &pag.doc, i, page, zoom * ppp);
                painter.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            } else {
                painter.rect_filled(rect, 0.0, Color32::WHITE);
            }
        }
        let to_screen = |page: usize, x: f32, y: f32| origins.get(page).map(|r| r.min + vec2(x * zoom, y * zoom));

        // Selection, then the caret.
        if !self.session.sel.is_collapsed() {
            let (a, b) = self.session.sel.ordered();
            let (a, b) = (pag.to_output(&a).unwrap_or(a), pag.to_output(&b).unwrap_or(b));
            for (page, r) in layout.selection_rects(&pag.doc, &a, &b, 0) {
                if let Some(min) = to_screen(page, r.x, r.y) {
                    painter.rect_filled(
                        Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)),
                        0.0,
                        Color32::from_rgba_unmultiplied(64, 128, 255, 70),
                    );
                }
            }
        }
        let mut caret_rect = None;
        if let Some(c) = pag.to_output(&self.session.sel.focus).and_then(|at| layout.caret(&at))
            && let Some(top) = to_screen(c.page, c.x, c.top)
        {
            let r = Rect::from_min_size(top, vec2(1.5, c.height * zoom));
            painter.line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.5, Color32::BLACK));
            caret_rect = Some(r.expand(24.0));
        }
        if std::mem::take(&mut self.scroll_to_caret)
            && let Some(r) = caret_rect
        {
            ui.scroll_to_rect(r, None);
        }

        // Mouse: a click places the caret, Shift+click or a drag extends, a double-click selects
        // a word.
        let hit = |p: Pos2| -> Option<Value> {
            let page = origins.iter().position(|r| r.contains(p)).or_else(|| nearest_page(&origins, p))?;
            let r = origins.get(page)?;
            let (x, y) = ((p.x - r.min.x) / zoom, (p.y - r.min.y) / zoom);
            layout.hit(page, x, y, StoryRef::Body).and_then(|pos| pag.to_source(&pos)).and_then(|pos| serde_json::to_value(pos).ok())
        };
        if let Some(p) = response.interact_pointer_pos() {
            let shift = ui.input(|i| i.modifiers.shift);
            if response.double_clicked() {
                if let Some(pos) = hit(p) {
                    self.run("select.wordAt", json!({"pos": pos}));
                }
            } else if response.drag_started() || response.clicked() {
                if let Some(pos) = hit(p) {
                    self.run("caret.to", json!({"pos": pos, "extend": shift}));
                }
            } else if response.dragged()
                && let Some(pos) = hit(p)
            {
                self.run("caret.to", json!({"pos": pos, "extend": true}));
            }
            self.scroll_to_caret = false;
        }
    }

    /// The whole window: menus, toolbar, status bar, pages and dialogs.
    pub fn ui(&mut self, ui: &mut Ui) {
        self.handle_keys(ui);
        crate::chrome::menu_bar(self, ui);
        crate::chrome::toolbar(self, ui);
        crate::chrome::status_bar(self, ui);
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| self.page_view(ui));
        });
        crate::dialogs::show(self, ui.ctx());
    }
}

/// The page whose vertical span is nearest `p` (a click in the gap between pages).
fn nearest_page(origins: &[Rect], p: Pos2) -> Option<usize> {
    origins
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.center().y - p.y).abs();
            let db = (b.center().y - p.y).abs();
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

/// Lookups the editor needs on the registry.
trait ElementsExt {
    fn of_para_at(&self, s: &Session) -> &str;
}

impl ElementsExt for Elements {
    fn of_para_at(&self, s: &Session) -> &str {
        match s.doc.para_at(&s.sel.focus) {
            Some(p) => &self.of_para(p).id,
            None => self.id_of(ElementKind::General),
        }
    }
}
