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

/// The find bar's fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FindBar {
    pub query: String,
    pub with: String,
    pub match_case: bool,
    pub whole_word: bool,
    /// Matches found for the current query (shown in the bar).
    pub count: Option<usize>,
}

impl FindBar {
    pub fn params(&self) -> Value {
        json!({"query": self.query, "with": self.with, "matchCase": self.match_case, "wholeWord": self.whole_word})
    }
}

/// Autocomplete suggestions showing at the caret.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestions {
    pub candidates: Vec<String>,
    /// The highlighted one, which Tab or Enter accepts.
    pub selected: usize,
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
    /// The find bar, while it is open.
    pub find: Option<FindBar>,
    /// Underline misspelled words.
    pub spelling: bool,
    spell_cache: wordsmith_script::spelling::SpellCache,
    /// The misspelling a right-click opened the menu on, with its suggestions.
    spell_menu: Option<(wordsmith_script::spelling::Misspelling, Vec<String>)>,
    /// Autocomplete suggestions at the caret, if any are showing.
    pub suggestions: Option<Suggestions>,
    /// Where suggestions were hidden with Escape: (revision, caret). They stay hidden there.
    dismissed: Option<(u64, wordsmith_doc::Pos)>,
    /// The caret's rectangle on screen in the last frame, for placing the suggestion list.
    caret_screen: Option<Rect>,
    /// Native file dialogs; each answer runs the command named with it, with the chosen path.
    #[cfg(not(target_arch = "wasm32"))]
    pub dialogs: craft_dialogs::Dialogs<&'static str>,
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
            find: None,
            spelling: true,
            spell_cache: Default::default(),
            spell_menu: None,
            suggestions: None,
            dismissed: None,
            caret_screen: None,
            paginator: Default::default(),
            paginated: None,
            #[cfg(not(target_arch = "wasm32"))]
            dialogs: craft_dialogs::Dialogs::new(Arc::new(|| {})),
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
        self.refresh_suggestions();
    }

    /// Ask the script what is offered at the caret now. Suggestions hidden with Escape stay
    /// hidden until the text or the caret changes.
    pub fn refresh_suggestions(&mut self) {
        let here = (self.session.rev(), self.session.sel.focus.clone());
        if self.dismissed.as_ref() == Some(&here) {
            self.suggestions = None;
            return;
        }
        self.dismissed = None;
        let offered = self.session.run("autocomplete.suggest", &json!({})).ok();
        let candidates: Vec<String> =
            offered.and_then(|v| v.get("candidates").cloned()).and_then(|c| serde_json::from_value(c).ok()).unwrap_or_default();
        self.suggestions = match (&self.suggestions, candidates.is_empty()) {
            (_, true) => None,
            // Keep the highlight on the same entry while it is still offered.
            (Some(old), false) => {
                let keep = old.candidates.get(old.selected).and_then(|c| candidates.iter().position(|n| n == c)).unwrap_or(0);
                Some(Suggestions { candidates, selected: keep })
            }
            (None, false) => Some(Suggestions { candidates, selected: 0 }),
        };
    }

    /// Keys the suggestion list takes while it shows: Tab and Enter accept, Up and Down move,
    /// Escape hides. Returns whether the key was taken.
    fn suggestion_key(&mut self, shortcut: &str) -> bool {
        let Some(sug) = self.suggestions.clone() else { return false };
        match shortcut {
            "Tab" | "Enter" => {
                if let Some(c) = sug.candidates.get(sug.selected) {
                    self.run("autocomplete.accept", json!({"candidate": c}));
                }
                true
            }
            "Down" => {
                self.suggestions = Some(Suggestions { selected: (sug.selected + 1) % sug.candidates.len().max(1), ..sug });
                true
            }
            "Up" => {
                let n = sug.candidates.len().max(1);
                self.suggestions = Some(Suggestions { selected: (sug.selected + n - 1) % n, ..sug });
                true
            }
            "Escape" => {
                self.dismissed = Some((self.session.rev(), self.session.sel.focus.clone()));
                self.suggestions = None;
                true
            }
            _ => false,
        }
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
                Some("exportPdf") => self.ask_export_path(),
                Some("find") => {
                    let query = self.session.selected_text();
                    let bar = self.find.take().unwrap_or_default();
                    self.find = Some(FindBar { query: if query.is_empty() || query.contains('\n') { bar.query.clone() } else { query }, ..bar });
                }
                _ => {}
            }
        }
    }

    /// Text waiting for the clipboard (for tests).
    pub fn pending_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ask_export_path(&mut self) {
        let dialog = craft_dialogs::FileDialog::save().title("Export PDF").filter("PDF", &["pdf"]).file_name("Script.pdf");
        self.dialogs.ask(dialog, "file.exportPdf");
    }

    #[cfg(target_arch = "wasm32")]
    fn ask_export_path(&mut self) {
        self.message = "Export PDF is not available in the web build yet.".into();
    }

    /// Show a waiting native dialog over `parent` (the window), and act on answers: each runs the
    /// command it was asked for, with the chosen path. Call once a frame, after [`Editor::ui`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_dialogs<W: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle>(&mut self, parent: Option<&W>) {
        self.dialogs.show_pending(parent);
        for (then, answer) in self.dialogs.take_answers() {
            match answer {
                craft_dialogs::Answer::Paths(paths) => {
                    if let Some(path) = paths.first() {
                        self.run(then, json!({"path": path}));
                    }
                }
                craft_dialogs::Answer::Cancelled | craft_dialogs::Answer::Message(_) => {}
                other => self.message = format!("{other:?}"),
            }
        }
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
                    if !self.suggestion_key(&shortcut) {
                        self.press(&shortcut);
                    }
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

        // Every match of the find bar's query, on the pages in view.
        if let Some(bar) = self.find.as_ref().filter(|b| !b.query.is_empty()) {
            let q = wordsmith_script::commands::find::Query { text: bar.query.clone(), match_case: bar.match_case, whole_word: bar.whole_word };
            let fill = Color32::from_rgba_unmultiplied(255, 210, 0, 90);
            for (a, b) in wordsmith_script::commands::find::find_all(&self.session, &q).into_iter().take(5_000) {
                let (Some(a), Some(b)) = (pag.to_output(&a), pag.to_output(&b)) else { continue };
                for (page, r) in layout.selection_rects(&pag.doc, &a, &b, 0) {
                    if let Some(min) = to_screen(page, r.x, r.y) {
                        painter.rect_filled(Rect::from_min_size(min, vec2(r.w * zoom, r.h * zoom)), 0.0, fill);
                    }
                }
            }
        }
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
        // Misspelled words: a dotted red line under each, on the pages in view.
        if self.spelling {
            let found = wordsmith_script::spelling::check(&self.session.doc, &mut self.spell_cache);
            let red = Stroke::new(1.5, Color32::from_rgb(220, 40, 40));
            for m in &found {
                let at = |off| wordsmith_doc::Pos { story: StoryRef::Body, path: wordsmith_doc::Path::top(m.block), off };
                let (Some(a), Some(b)) = (pag.to_output(&at(m.start)), pag.to_output(&at(m.end))) else { continue };
                for (page, r) in layout.selection_rects(&pag.doc, &a, &b, 0) {
                    let Some(min) = to_screen(page, r.x, r.y + r.h - 1.0) else { continue };
                    if !ui.is_rect_visible(Rect::from_min_size(min, vec2(r.w * zoom, 2.0))) {
                        continue;
                    }
                    let (x0, x1, y) = (min.x, min.x + r.w * zoom, min.y);
                    let mut x = x0;
                    while x < x1 {
                        painter.line_segment([pos2(x, y), pos2((x + 2.0).min(x1), y)], red);
                        x += 4.0;
                    }
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
            self.caret_screen = Some(r);
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
        // Right-click on a misspelled word: its suggestions, Ignore, Add to Dictionary.
        if response.secondary_clicked()
            && let Some(p) = response.interact_pointer_pos()
            && let Some(pos) = hit(p).and_then(|v| serde_json::from_value::<wordsmith_doc::Pos>(v).ok())
        {
            let found = wordsmith_script::spelling::check(&self.session.doc, &mut self.spell_cache);
            self.spell_menu = found.into_iter().find(|m| m.block == pos.path.last() && (m.start..=m.end).contains(&pos.off)).map(|m| {
                let s = wordsmith_script::spelling::suggest(&m.word, 6);
                (m, s)
            });
        }
        let mut menu_action: Option<(&'static str, Value)> = None;
        if let Some((m, suggestions)) = self.spell_menu.clone() {
            response.context_menu(|ui| {
                for s in &suggestions {
                    if ui.button(s.as_str()).clicked() {
                        menu_action =
                            Some(("spelling.replace", json!({"block": m.block, "start": m.start, "end": m.end, "word": m.word, "with": s})));
                        ui.close();
                    }
                }
                if suggestions.is_empty() {
                    ui.weak("No suggestions");
                }
                ui.separator();
                if ui.button("Ignore in This Script").clicked() {
                    menu_action = Some(("spelling.ignore", json!({"word": m.word})));
                    ui.close();
                }
                if ui.button("Add to Dictionary").clicked() {
                    menu_action = Some(("spelling.addWord", json!({"word": m.word})));
                    ui.close();
                }
            });
        }
        if let Some((id, params)) = menu_action {
            self.spell_menu = None;
            self.run(id, params);
        }
        if let Some(p) = response.interact_pointer_pos().filter(|_| !response.secondary_clicked()) {
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
        crate::chrome::find_bar(self, ui);
        crate::chrome::status_bar(self, ui);
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| self.page_view(ui));
        });
        self.suggestion_list(ui.ctx());
        crate::dialogs::show(self, ui.ctx());
    }

    /// The suggestion list, just below the caret. Clicking an entry accepts it.
    fn suggestion_list(&mut self, ctx: &egui::Context) {
        let (Some(sug), Some(caret)) = (self.suggestions.clone(), self.caret_screen) else { return };
        let mut chosen = None;
        egui::Area::new(egui::Id::new("autocomplete")).order(egui::Order::Foreground).fixed_pos(caret.left_bottom() + vec2(0.0, 4.0)).show(
            ctx,
            |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (i, c) in sug.candidates.iter().enumerate() {
                        if ui.selectable_label(i == sug.selected, c.as_str()).clicked() {
                            chosen = Some(c.clone());
                        }
                    }
                    ui.weak("Tab or Enter to accept · Esc to hide");
                });
            },
        );
        if let Some(c) = chosen {
            self.run("autocomplete.accept", json!({"candidate": c}));
        }
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
