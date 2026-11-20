// SPDX-License-Identifier: MIT OR Apache-2.0
//! Menus, toolbar and status bar. Every item runs a command by id; labels and shortcuts come from
//! the registry, so menus, keys, the palette, the CLI and MCP never disagree.

use egui::Ui;
use serde_json::json;
use wordsmith_script::{ElementKind, Elements};

use crate::editor::{Editor, ZOOM_RANGE};

/// A menu item for command `id`, labelled and with its shortcut from the registry; greyed out
/// when the command says it cannot run.
fn item(ed: &mut Editor, ui: &mut Ui, id: &str) {
    let Some(spec) = ed.session.registry.get(id) else { return };
    let (label, shortcut) = (spec.label, spec.shortcut);
    let enabled = ed.session.why_disabled(id).is_none();
    let button = egui::Button::new(label).shortcut_text(shortcut.split(" / ").next().unwrap_or(""));
    if ui.add_enabled(enabled, button).clicked() {
        ed.run(id, json!({}));
        ui.close();
    }
}

pub fn menu_bar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                for id in ["file.new", "view.templates", "file.newTab", "file.open", "file.import"] {
                    item(ed, ui, id);
                }
                if !ed.recent.is_empty() {
                    ui.menu_button("Open Recent", |ui| {
                        for r in ed.recent.clone() {
                            let name = std::path::Path::new(&r).file_name().and_then(|n| n.to_str()).unwrap_or(&r).to_string();
                            if ui.button(name).on_hover_text(&r).clicked() {
                                ed.run("file.openPath", json!({"path": r}));
                                ui.close();
                            }
                        }
                    });
                }
                item(ed, ui, "view.start");
                ui.separator();
                item(ed, ui, "file.save");
                item(ed, ui, "file.saveAs");
                item(ed, ui, "file.saveAsTemplate");
                item(ed, ui, "file.revert");
                item(ed, ui, "file.closeTab");
                ui.separator();
                ui.menu_button("Export", |ui| {
                    for id in
                        ["file.exportPdfAs", "file.exportRevisedPdfAs", "file.exportFountain", "file.exportText", "file.exportRtf", "file.exportHtml"]
                    {
                        item(ed, ui, id);
                    }
                });
                item(ed, ui, "view.print");
                item(ed, ui, "file.printPreview");
                item(ed, ui, "file.print");
                item(ed, ui, "file.printRevised");
                ui.separator();
                item(ed, ui, "app.preferences");
                item(ed, ui, "view.keys");
                item(ed, ui, "file.recoverPrompt");
            });
            ui.menu_button("Edit", |ui| {
                for id in ["edit.undo", "edit.redo", "edit.cut", "edit.copy", "select.all", "select.scene", "view.find", "view.note"] {
                    item(ed, ui, id);
                }
            });
            ui.menu_button("Script", |ui| {
                for id in ["view.sceneProperties", "view.character", "view.highlights"] {
                    item(ed, ui, id);
                }
            });
            ui.menu_button("Insert", |ui| {
                for id in ["scene.insert", "text.pageBreak", "bookmark.add"] {
                    item(ed, ui, id);
                }
            });
            ui.menu_button("Go", |ui| {
                for id in ["view.goto", "view.bookmarks", "bookmark.next", "bookmark.previous", "note.next", "note.previous"] {
                    item(ed, ui, id);
                }
            });
            ui.menu_button("Format", |ui| {
                ui.label("Element");
                for id in [
                    "element.general",
                    "element.sceneHeading",
                    "element.action",
                    "element.character",
                    "element.parenthetical",
                    "element.dialogue",
                    "element.transition",
                    "element.shot",
                    "element.castList",
                    "element.newAct",
                    "element.endOfAct",
                ] {
                    item(ed, ui, id);
                }
                ui.menu_button("Structure and Outline", |ui| {
                    for id in ["element.sequence", "element.outline1", "element.outline2", "element.outline3", "element.note", "element.summary"] {
                        item(ed, ui, id);
                    }
                });
                ui.menu_button("Text Style", |ui| {
                    let on = ed.session.run("format.state", &json!({})).ok().and_then(|v| v["styles"].as_array().cloned()).unwrap_or_default();
                    for (style, label) in [
                        ("bold", "Bold"),
                        ("italic", "Italic"),
                        ("underline", "Underline"),
                        ("doubleUnderline", "Double Underline"),
                        ("wordUnderline", "Underline Words"),
                        ("dottedUnderline", "Dotted Underline"),
                        ("strike", "Strike Through"),
                        ("superscript", "Raised"),
                        ("subscript", "Lowered"),
                        ("caps", "Capitals"),
                        ("smallCaps", "Small Capitals"),
                    ] {
                        let mut checked = on.iter().any(|x| x == style);
                        if ui.checkbox(&mut checked, label).clicked() {
                            ed.run("format.style", json!({"style": style, "on": checked}));
                        }
                    }
                    ui.separator();
                    item(ed, ui, "format.lowercase");
                    item(ed, ui, "format.plain");
                });
                ui.menu_button("Highlight", |ui| {
                    for (name, _) in wordsmith_script::commands::format::HIGHLIGHTS {
                        if ui.button(name).clicked() {
                            ed.run("format.highlight", json!({"color": name}));
                            ui.close();
                        }
                    }
                    let mut custom = egui::Color32::from_rgb(255, 230, 150);
                    ui.horizontal(|ui| {
                        ui.label("Any color");
                        if ui.color_edit_button_srgba(&mut custom).changed() {
                            ed.run("format.highlight", json!({"color": format!("#{:02X}{:02X}{:02X}", custom.r(), custom.g(), custom.b())}));
                        }
                    });
                    if ui.button("None").clicked() {
                        ed.run("format.highlight", json!({"color": "none"}));
                        ui.close();
                    }
                });
                ui.menu_button("Text Color", |ui| {
                    for (name, hex) in [("Black", "#000000"), ("Red", "#C0392B"), ("Blue", "#1F5FA8"), ("Green", "#2E7D32"), ("Gray", "#707070")] {
                        if ui.button(name).clicked() {
                            ed.run("format.color", json!({"color": hex}));
                            ui.close();
                        }
                    }
                    if ui.button("Automatic").clicked() {
                        ed.run("format.color", json!({"color": "auto"}));
                        ui.close();
                    }
                });
                item(ed, ui, "dialogue.dual");
                item(ed, ui, "element.reset");
                item(ed, ui, "text.toggleCase");
                ui.separator();
                let typing = wordsmith_script::typing::Typing::of_doc(&ed.session.doc);
                let mut smart = typing.smart_quotes;
                if ui.checkbox(&mut smart, "Smart Quotes").clicked() {
                    ed.run("typing.smartQuotes", json!({}));
                }
                let mut caps = typing.auto_capitalize;
                if ui.checkbox(&mut caps, "Capitalize Sentences").clicked() {
                    ed.run("typing.autoCapitalize", json!({}));
                }
                ui.separator();
                item(ed, ui, "view.elementSettings");
                item(ed, ui, "view.titlePage");
                item(ed, ui, "view.headerFooter");
            });
            ui.menu_button("Production", |ui| {
                for id in
                    ["revision.mode", "revision.next", "revision.markRevised", "revision.clearRevised", "revision.gotoNext", "revision.gotoPrevious"]
                {
                    item(ed, ui, id);
                }
                ui.separator();
                item(ed, ui, "view.revisions");
                ui.separator();
                for id in ["view.sceneNumbers", "scene.showNumbers", "scene.lockNumbers", "scene.renumber", "scene.omit", "scene.recall"] {
                    item(ed, ui, id);
                }
                ui.separator();
                ui.menu_button("Locked Pages", |ui| {
                    for id in ["lock.pages", "lock.relock", "lock.lockPage", "lock.unlockPage", "lock.unlockLevel", "lock.unlockAll"] {
                        item(ed, ui, id);
                    }
                    let enabled = ed.session.why_disabled("lock.adjustBreak").is_none();
                    if ui.add_enabled(enabled, egui::Button::new("Move Page Start Down")).clicked() {
                        ed.run("lock.adjustBreak", json!({"by": 1}));
                        ui.close();
                    }
                    if ui.add_enabled(enabled, egui::Button::new("Move Page Start Up")).clicked() {
                        ed.run("lock.adjustBreak", json!({"by": -1}));
                        ui.close();
                    }
                });
                ui.menu_button("Track Changes", |ui| {
                    let on = wordsmith_script::tracking::is_on(&ed.session.doc);
                    let mut checked = on;
                    if ui.checkbox(&mut checked, "Track Changes").clicked() {
                        ed.run("track.toggle", json!({}));
                    }
                    for id in ["track.next", "track.previous", "track.accept", "track.reject", "track.acceptAll", "track.rejectAll"] {
                        item(ed, ui, id);
                    }
                });
                item(ed, ui, "compare.withFile");
                item(ed, ui, "view.watermark");
                ui.separator();
                item(ed, ui, "report.exportCharacters");
            });
            ui.menu_button("View", |ui| {
                for id in [
                    "view.normal",
                    "view.page",
                    "view.speed",
                    "view.cards",
                    "view.scenes",
                    "view.board",
                    "view.outline",
                    "view.tables",
                    "outline.show",
                ] {
                    item(ed, ui, id);
                }
                ui.separator();
                for id in ["view.strip", "view.darkTheme", "view.extraDark", "view.typewriter", "view.focusMode"] {
                    item(ed, ui, id);
                }
                ui.menu_button("Split", |ui| {
                    for id in [
                        "view.splitCards",
                        "view.splitBoard",
                        "view.splitOutline",
                        "view.splitScenes",
                        "view.splitScript",
                        "view.splitBelow",
                        "view.splitSwap",
                        "view.unsplit",
                    ] {
                        item(ed, ui, id);
                    }
                });
                if ed.tabs.len() > 1 {
                    item(ed, ui, "view.nextTab");
                }
                ui.separator();
                item(ed, ui, "view.commandPalette");
                if ui.button("Zoom In").clicked() {
                    ed.zoom = (ed.zoom * 1.25).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                }
                if ui.button("Zoom Out").clicked() {
                    ed.zoom = (ed.zoom / 1.25).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                }
                ui.menu_button("Zoom To", |ui| {
                    for z in crate::editor::ZOOM_STEPS {
                        if ui.selectable_label((ed.zoom - z).abs() < 0.01, format!("{:.0}%", z * 100.0)).clicked() {
                            ed.zoom = z;
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button("Tools", |ui| {
                for id in ["view.reports", "view.stats", "view.sprint", "view.checker", "view.reformat", "view.macros"] {
                    item(ed, ui, id);
                }
                ui.menu_button("Run Macro", |ui| {
                    let all = wordsmith_script::macros::Macros::of_doc(&ed.session.doc);
                    for (i, m) in all.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
                        let button = egui::Button::new(format!("{}. {}", i + 1, m.name)).shortcut_text(m.shortcut.clone().unwrap_or_default());
                        if ui.add(button).clicked() {
                            ed.run("macro.run", json!({"slot": i + 1}));
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button("Help", |ui| item(ed, ui, "help.about"));
        });
    });
}

pub fn toolbar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.add_enabled(ed.session.can_undo(), egui::Button::new("Undo")).clicked() {
                ed.run("edit.undo", json!({}));
            }
            if ui.add_enabled(ed.session.can_redo(), egui::Button::new("Redo")).clicked() {
                ed.run("edit.redo", json!({}));
            }
            ui.separator();
            let elements = Elements::of_doc(&ed.session.doc);
            let current = match ed.session.doc.para_at(&ed.session.sel.focus) {
                Some(p) => elements.of_para(p).clone(),
                None => {
                    elements.of_kind(ElementKind::General).cloned().unwrap_or_else(|| wordsmith_script::element::builtin_def(ElementKind::General))
                }
            };
            let mut chosen = None;
            egui::ComboBox::from_id_salt("element").selected_text(current.name.as_str()).width(160.0).show_ui(ui, |ui| {
                for def in elements.all().iter().filter(|d| !d.kind.is_outline() || d.id == current.id) {
                    if ui.selectable_label(def.id == current.id, def.name.as_str()).clicked() {
                        chosen = Some(def.id.clone());
                    }
                }
            });
            if let Some(id) = chosen {
                ed.run("element.set", json!({"element": id}));
            }
            ui.separator();
            ui.label(format!("{:.0}%", ed.zoom * 100.0));
        });
    });
}

/// The find bar: query, replacement, options and their commands.
pub fn find_bar(ed: &mut Editor, ui: &mut Ui) {
    let Some(mut bar) = ed.find.clone() else { return };
    let mut action: Option<&'static str> = None;
    let mut close = false;
    egui::Panel::top("find").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Find");
            let q = ui.add(egui::TextEdit::singleline(&mut bar.query).desired_width(200.0));
            if q.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                action = Some("find.next");
                q.request_focus();
            }
            ui.label("Replace with");
            ui.add(egui::TextEdit::singleline(&mut bar.with).desired_width(160.0));
            ui.checkbox(&mut bar.match_case, "Match case");
            ui.checkbox(&mut bar.whole_word, "Whole word");
            let elements = Elements::of_doc(&ed.session.doc);
            let shown = bar.elements.first().and_then(|id| elements.get(id)).map_or("All elements".to_string(), |d| d.name.clone());
            egui::ComboBox::from_id_salt("find-in").selected_text(shown).width(130.0).show_ui(ui, |ui| {
                if ui.selectable_label(bar.elements.is_empty(), "All elements").clicked() {
                    bar.elements.clear();
                }
                for d in elements.all() {
                    if ui.selectable_label(bar.elements.first() == Some(&d.id), d.name.as_str()).clicked() {
                        bar.elements = vec![d.id.clone()];
                    }
                }
            });
            for (label, id) in [("Previous", "find.previous"), ("Next", "find.next"), ("Replace", "replace.one"), ("Replace All", "replace.all")] {
                if ui.button(label).clicked() {
                    action = Some(id);
                }
            }
            if let Some(n) = bar.count {
                ui.label(format!("{n} found"));
            }
            if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });
    });
    if close {
        ed.find = None;
        return;
    }
    if let Some(id) = action.filter(|_| !bar.query.is_empty()) {
        ed.run(id, bar.params());
    }
    bar.count = (!bar.query.is_empty())
        .then(|| ed.session.run("find.all", &bar.params()).ok())
        .flatten()
        .and_then(|v| v["count"].as_u64())
        .map(|n| n as usize);
    ed.find = Some(bar);
}

pub fn status_bar(ed: &mut Editor, ui: &mut Ui) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let frame = egui::Frame::NONE.fill(t.surface).stroke(egui::Stroke::new(1.0, t.border)).inner_margin(egui::Margin::symmetric(10, 3));
    egui::Panel::bottom("status").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let dim = |s: String| egui::RichText::new(s).color(t.text_dim).size(12.0);
            let pag = ed.paginated();
            let page = pag.to_output(&ed.session.sel.focus).and_then(|at| pag.layout.caret(&at)).map_or(1, |c| c.page + 1);
            let pages = pag.layout.pages.len();
            let page_text = match pag.labels.as_ref().and_then(|l| l.get(page - 1)) {
                Some(label) => format!("Page {label} (locked) · {page} of {pages}"),
                None => format!("Page {page} of {pages}"),
            };
            if ui.add(egui::Label::new(dim(page_text)).sense(egui::Sense::click())).on_hover_text("Go to a page or scene").clicked() {
                ed.run("view.goto", json!({}));
            }
            let words = ed.stats.words_now.1;
            if ui.add(egui::Label::new(dim(format!("{words} words"))).sense(egui::Sense::click())).on_hover_text("Writing statistics").clicked() {
                ed.run("view.stats", json!({}));
            }
            let all = wordsmith_script::scenes(&ed.session.doc);
            let here = wordsmith_script::scenes::scene_at(&all, ed.session.sel.focus.path.last()).map(|s| s.index + 1);
            let scene_text = match here {
                Some(n) => format!("Scene {n} of {}", all.len()),
                None => format!("{} scenes", all.len()),
            };
            if ui.add(egui::Label::new(dim(scene_text)).sense(egui::Sense::click())).on_hover_text("Show the scenes").clicked() {
                ed.chrome.left = Some(crate::ribbon::Panel::Scenes);
            }
            ui.separator();
            // What Return and Tab make from here (SPEC-2 3.2).
            if let Ok(h) = ed.session.run("element.hint", &json!({})) {
                let tab = h["tab"].as_str().unwrap_or("nothing");
                ui.label(egui::RichText::new(h["element"].as_str().unwrap_or("")).font(crate::theme::medium(12.0)).color(t.text));
                ui.label(dim(format!("Return: {}  ·  Tab: {tab}", h["enter"].as_str().unwrap_or(""))));
            }
            let rev = wordsmith_script::revisions::Revisions::of_doc(&ed.session.doc);
            let chip = |ui: &mut Ui, text: &str, color: egui::Color32, tip: &str| -> bool {
                let g = ui.painter().layout_no_wrap(text.to_string(), crate::theme::regular(11.5), t.text);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(g.size().x + 22.0, 18.0), egui::Sense::click());
                ui.painter().rect_filled(r, 9.0, if resp.hovered() { t.hover } else { t.surface_high });
                ui.painter().rect_stroke(r, 9.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                ui.painter().circle_filled(egui::pos2(r.min.x + 9.0, r.center().y), 4.0, color);
                ui.painter().galley(egui::pos2(r.min.x + 16.0, r.center().y - g.size().y / 2.0), g, t.text);
                resp.on_hover_text(tip).clicked()
            };
            if rev.mode {
                let color = rev
                    .current_index()
                    .and_then(|i| rev.set(i))
                    .and_then(|s| wordsmith_script::revisions::parse_color(&s.color))
                    .map_or(t.accent, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
                if chip(ui, &format!("Revising: {}", rev.current), color, "Revision sets") {
                    ed.chrome.right = Some(crate::ribbon::Panel::Revisions);
                }
            }
            if wordsmith_script::tracking::is_on(&ed.session.doc) && chip(ui, "Tracking changes", t.danger, "Tracked changes") {
                ed.chrome.right = Some(crate::ribbon::Panel::Changes);
            }
            if !wordsmith_script::paginate::dual::pairs(&ed.session.doc).is_empty() {
                chip(ui, "Dual dialogue", t.accent, "The script has speeches side by side");
            }
            if wordsmith_script::templates::is_text(&ed.session.doc) {
                chip(ui, "Text document", t.text_dim, "Prose: no screenplay pagination");
            }
            if !ed.message.is_empty() {
                ui.colored_label(t.danger, ed.message.as_str());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(dim(format!("{:.0}%", ed.zoom * 100.0)));
                if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                    ed.run("view.zoomIn", json!({}));
                }
                let mut pct = ed.zoom * 100.0;
                if ui.add(egui::Slider::new(&mut pct, 75.0..=300.0).step_by(25.0).show_value(false)).changed() {
                    ed.zoom = (pct / 100.0).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                }
                if ui.small_button("−").on_hover_text("Zoom out").clicked() {
                    ed.run("view.zoomOut", json!({}));
                }
                ui.separator();
                for (id, icon, tip, on) in [
                    ("view.speed", "view.speed", "Speed view", ed.view == crate::editor::View::Speed),
                    ("view.normal", "view.normal", "Normal view", ed.view == crate::editor::View::Normal),
                    ("view.page", "view.page", "Page view", ed.view == crate::editor::View::Page),
                ] {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), egui::Sense::click());
                    let on = on && ed.structure.is_none();
                    if on {
                        ui.painter().rect_filled(r, 4.0, t.accent_soft);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 4.0, t.hover);
                    }
                    crate::script_icons::paint(ui.painter(), r.shrink(3.0), icon, t.text, t.accent);
                    if resp.on_hover_text(tip).clicked() {
                        ed.structure = None;
                        ed.run(id, json!({}));
                    }
                }
                ui.separator();
                let theme = if ed.dark { "Light" } else { "Dark" };
                if ui.add(egui::Label::new(dim(theme.to_string())).sense(egui::Sense::click())).on_hover_text("Switch theme").clicked() {
                    ed.dark = !ed.dark;
                }
                if ui.add(egui::Label::new(dim("Focus".into())).sense(egui::Sense::click())).on_hover_text("Only the page (Mod+Shift+F)").clicked() {
                    ed.focus_mode = !ed.focus_mode;
                }
                let sprint = match crate::workspace::sprint_left(ed) {
                    Some(left) => format!("Sprint {}:{:02}", left.as_secs() / 60, left.as_secs() % 60),
                    None => "Sprint".into(),
                };
                if ui.add(egui::Label::new(dim(sprint)).sense(egui::Sense::click())).on_hover_text("Start or stop a writing sprint").clicked() {
                    crate::workspace::toggle_sprint(ed);
                }
            });
        });
    });
}
