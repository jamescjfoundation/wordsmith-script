// SPDX-License-Identifier: MIT OR Apache-2.0
//! Menus, toolbar and status bar. Every item runs a command by id; labels and shortcuts come from
//! the registry, so menus, keys, the palette, the CLI and MCP never disagree.

use egui::Ui;
use serde_json::json;
use wordsmith_script::{ElementKind, Elements};

use crate::editor::Editor;
use crate::ribbon::widgets::{self, Cmd, cmd, cmd_with, link, menu};

/// A menu item for command `id`, labelled and with its shortcut from the registry; greyed out
/// when the command says it cannot run.
fn item(ed: &mut Editor, ui: &mut Ui, id: &'static str) {
    let label = ed.session.registry.get(id).map_or(id, |s| s.label);
    widgets::item(ui, ed, cmd(id), label);
}

pub fn menu_bar(ed: &mut Editor, ui: &mut Ui) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            menu(ui, ed, "File", |ui, ed| {
                for id in ["file.new", "view.templates", "file.newTab", "file.open", "file.import"] {
                    item(ed, ui, id);
                }
                if !ed.recent.is_empty() {
                    menu(ui, ed, "Open Recent", |ui, ed| {
                        for r in ed.recent.clone() {
                            let name = std::path::Path::new(&r).file_name().and_then(|n| n.to_str()).unwrap_or(&r).to_string();
                            widgets::item(ui, ed, cmd_with("file.openPath", json!({"path": r})), &name);
                        }
                    });
                }
                item(ed, ui, "view.start");
                ui.separator();
                for id in ["file.save", "file.saveAs", "file.saveAsTemplate", "file.revert", "file.closeTab"] {
                    item(ed, ui, id);
                }
                ui.separator();
                menu(ui, ed, "Export", |ui, ed| {
                    for id in
                        ["file.exportPdfAs", "file.exportRevisedPdfAs", "file.exportFountain", "file.exportText", "file.exportRtf", "file.exportHtml"]
                    {
                        item(ed, ui, id);
                    }
                });
                for id in ["view.print", "file.printPreview", "file.print", "file.printRevised"] {
                    item(ed, ui, id);
                }
                ui.separator();
                for id in ["app.preferences", "view.keys", "file.recoverPrompt"] {
                    item(ed, ui, id);
                }
            });
            menu(ui, ed, "Edit", |ui, ed| {
                for id in ["edit.undo", "edit.redo", "edit.cut", "edit.copy", "edit.paste", "select.all", "select.scene", "view.find", "view.note"] {
                    item(ed, ui, id);
                }
            });
            menu(ui, ed, "Script", |ui, ed| {
                for id in ["view.sceneProperties", "view.character", "view.highlights"] {
                    item(ed, ui, id);
                }
            });
            menu(ui, ed, "Insert", |ui, ed| {
                for id in ["scene.insert", "text.pageBreak", "bookmark.add"] {
                    item(ed, ui, id);
                }
            });
            menu(ui, ed, "Go", |ui, ed| {
                for id in ["view.goto", "view.bookmarks", "bookmark.next", "bookmark.previous", "note.next", "note.previous"] {
                    item(ed, ui, id);
                }
            });
            menu(ui, ed, "Format", |ui, ed| {
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
                menu(ui, ed, "Structure and Outline", |ui, ed| {
                    for id in ["element.sequence", "element.outline1", "element.outline2", "element.outline3", "element.note", "element.summary"] {
                        item(ed, ui, id);
                    }
                });
                menu(ui, ed, "Text Style", |ui, ed| {
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
                        let is = on.iter().any(|x| x == style);
                        widgets::check(ui, ed, cmd_with("format.style", json!({"style": style, "on": !is})), label, is);
                    }
                    ui.separator();
                    item(ed, ui, "format.lowercase");
                    item(ed, ui, "format.plain");
                });
                menu(ui, ed, "Highlight", |ui, ed| {
                    for (name, _) in wordsmith_script::commands::format::HIGHLIGHTS {
                        widgets::item(ui, ed, cmd_with("format.highlight", json!({"color": name})), name);
                    }
                    let mut custom = egui::Color32::from_rgb(255, 230, 150);
                    ui.horizontal(|ui| {
                        ui.label("Any color");
                        let resp = ui.color_edit_button_srgba(&mut custom);
                        widgets::record(ed, Some("format.highlight"), resp.rect);
                        if resp.changed() {
                            ed.run("format.highlight", json!({"color": format!("#{:02X}{:02X}{:02X}", custom.r(), custom.g(), custom.b())}));
                        }
                    });
                    widgets::item(ui, ed, cmd_with("format.highlight", json!({"color": "none"})), "None");
                });
                menu(ui, ed, "Text Color", |ui, ed| {
                    for (name, hex) in [
                        ("Black", "#000000"),
                        ("Red", "#C0392B"),
                        ("Blue", "#1F5FA8"),
                        ("Green", "#2E7D32"),
                        ("Gray", "#707070"),
                        ("Automatic", "auto"),
                    ] {
                        widgets::item(ui, ed, cmd_with("format.color", json!({"color": hex})), name);
                    }
                });
                item(ed, ui, "dialogue.dual");
                item(ed, ui, "element.reset");
                item(ed, ui, "text.toggleCase");
                ui.separator();
                let typing = wordsmith_script::typing::Typing::of_doc(&ed.session.doc);
                widgets::check(ui, ed, cmd("typing.smartQuotes"), "Smart Quotes", typing.smart_quotes);
                widgets::check(ui, ed, cmd("typing.autoCapitalize"), "Capitalize Sentences", typing.auto_capitalize);
                ui.separator();
                item(ed, ui, "view.elementSettings");
                item(ed, ui, "view.titlePage");
                item(ed, ui, "view.headerFooter");
            });
            menu(ui, ed, "Production", |ui, ed| {
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
                menu(ui, ed, "Locked Pages", |ui, ed| {
                    for id in ["lock.pages", "lock.relock", "lock.lockPage", "lock.unlockPage", "lock.unlockLevel", "lock.unlockAll"] {
                        item(ed, ui, id);
                    }
                    widgets::item(ui, ed, cmd_with("lock.adjustBreak", json!({"by": 1})), "Move Page Start Down");
                    widgets::item(ui, ed, cmd_with("lock.adjustBreak", json!({"by": -1})), "Move Page Start Up");
                });
                menu(ui, ed, "Track Changes", |ui, ed| {
                    let on = wordsmith_script::tracking::is_on(&ed.session.doc);
                    widgets::check(ui, ed, cmd("track.toggle"), "Track Changes", on);
                    for id in ["track.next", "track.previous", "track.accept", "track.reject", "track.acceptAll", "track.rejectAll"] {
                        item(ed, ui, id);
                    }
                });
                item(ed, ui, "compare.withFile");
                item(ed, ui, "view.watermark");
                ui.separator();
                item(ed, ui, "report.exportCharacters");
            });
            menu(ui, ed, "View", |ui, ed| {
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
                menu(ui, ed, "Split", |ui, ed| {
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
                for id in ["view.commandPalette", "view.zoomIn", "view.zoomOut"] {
                    item(ed, ui, id);
                }
                menu(ui, ed, "Zoom To", |ui, ed| {
                    let now = ed.zoom;
                    for z in crate::editor::ZOOM_STEPS {
                        let pct = (z * 100.0).round();
                        widgets::choice(ui, ed, cmd_with("view.zoomTo", json!({"percent": pct})), &format!("{pct:.0}%"), (now - z).abs() < 0.01);
                    }
                });
            });
            menu(ui, ed, "Tools", |ui, ed| {
                for id in ["view.reports", "view.stats", "view.sprint", "view.checker", "view.reformat", "view.macros"] {
                    item(ed, ui, id);
                }
                menu(ui, ed, "Run Macro", |ui, ed| {
                    let all = wordsmith_script::macros::Macros::of_doc(&ed.session.doc);
                    for (i, m) in all.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
                        let key = m.shortcut.clone().unwrap_or_default();
                        widgets::item_with_key(ui, ed, cmd_with("macro.run", json!({"slot": i + 1})), &format!("{}. {}", i + 1, m.name), &key);
                    }
                });
            });
            menu(ui, ed, "Help", |ui, ed| {
                item(ed, ui, "help.checkForUpdates");
                item(ed, ui, "help.about");
            });
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
/// It shares its query with the Script panel's Find panel (`Editor::find`): closing it leaves the
/// panel's matches, and Escape closes it only from its own fields or when nothing else is open.
pub fn find_bar(ed: &mut Editor, ui: &mut Ui) {
    if !ed.find_bar {
        return;
    }
    let mut bar = ed.find.clone().unwrap_or_default();
    let mut focused = false;
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
            let w = ui.add(egui::TextEdit::singleline(&mut bar.with).desired_width(160.0));
            // Escape takes the focus from a field as it is pressed.
            focused = [&q, &w].iter().any(|r| r.has_focus() || r.lost_focus());
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
            let escape = ui.input(|i| i.key_pressed(egui::Key::Escape)) && (focused || ed.escape_for_find_bar);
            if ui.button("Close").clicked() || escape {
                close = true;
            }
        });
    });
    if close {
        ed.find_bar = false;
        // The Find panel keeps showing its matches.
        if ed.chrome.left != Some(crate::ribbon::Panel::Find) {
            ed.find = None;
        }
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
    ed.chrome.place = crate::ribbon::Place::Status;
    let bar = egui::Panel::bottom("status").frame(frame).show(ui, |ui| {
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
            link(ui, ed, cmd("view.goto"), dim(page_text), "Go to a page or scene.");
            let words = ed.stats.words_now.1;
            link(ui, ed, cmd("view.stats"), dim(format!("{words} words")), "Writing statistics.");
            let all = wordsmith_script::scenes(&ed.session.doc);
            let here = wordsmith_script::scenes::scene_at(&all, ed.session.sel.focus.path.last()).map(|s| s.index + 1);
            let scene_text = match here {
                Some(n) => format!("Scene {n} of {}", all.len()),
                None => format!("{} scenes", all.len()),
            };
            link(ui, ed, cmd_with("view.scriptPanel", json!({"panel": "scenes"})), dim(scene_text), "Show the scenes.");
            ui.separator();
            // What Return and Tab make from here (SPEC-2 3.2).
            if let Ok(h) = ed.session.run("element.hint", &json!({})) {
                let tab = h["tab"].as_str().unwrap_or("nothing");
                let element = egui::RichText::new(h["element"].as_str().unwrap_or("")).font(crate::theme::medium(12.0)).color(t.text);
                ui.add(egui::Label::new(element).selectable(false));
                ui.add(egui::Label::new(dim(format!("Return: {}  ·  Tab: {tab}", h["enter"].as_str().unwrap_or("")))).selectable(false));
            }
            let rev = wordsmith_script::revisions::Revisions::of_doc(&ed.session.doc);
            // A chip says what is on; one that opens a panel runs that command.
            let chip = |ui: &mut Ui, ed: &mut Editor, text: &str, color: egui::Color32, tip: &str, opens: Option<Cmd>| {
                let g = ui.painter().layout_no_wrap(text.to_string(), crate::theme::regular(11.5), t.text);
                let sense = if opens.is_some() { egui::Sense::click() } else { egui::Sense::hover() };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(g.size().x + 22.0, 18.0), sense);
                let hovered = resp.hovered() && opens.is_some();
                ui.painter().rect_filled(r, 9.0, if hovered { t.hover } else { t.surface_high });
                ui.painter().rect_stroke(r, 9.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                ui.painter().circle_filled(egui::pos2(r.min.x + 9.0, r.center().y), 4.0, color);
                ui.painter().galley(egui::pos2(r.min.x + 16.0, r.center().y - g.size().y / 2.0), g, t.text);
                if let Some(c) = opens {
                    widgets::record(ed, Some(c.id), r);
                    if resp.on_hover_text(widgets::tip(ed, c.id, tip)).clicked() {
                        ed.run(c.id, c.params);
                    }
                } else {
                    resp.on_hover_text(tip);
                }
            };
            if rev.mode {
                let color = rev
                    .current_index()
                    .and_then(|i| rev.set(i))
                    .and_then(|s| wordsmith_script::revisions::parse_color(&s.color))
                    .map_or(t.accent, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
                let opens = cmd_with("view.inspector", json!({"panel": "revisions"}));
                chip(ui, ed, &format!("Revising: {}", rev.current), color, "Revision sets.", Some(opens));
            }
            if wordsmith_script::tracking::is_on(&ed.session.doc) {
                chip(ui, ed, "Tracking changes", t.danger, "Tracked changes.", Some(cmd_with("view.inspector", json!({"panel": "changes"}))));
            }
            if !wordsmith_script::paginate::dual::pairs(&ed.session.doc).is_empty() {
                chip(ui, ed, "Dual dialogue", t.accent, "The script has speeches side by side", None);
            }
            if wordsmith_script::templates::is_text(&ed.session.doc) {
                chip(ui, ed, "Text document", t.text_dim, "Prose: no screenplay pagination", None);
            }
            if !ed.message.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(ed.message.as_str()).color(t.danger)).selectable(false));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add(egui::Label::new(dim(format!("{:.0}%", ed.zoom * 100.0))).selectable(false));
                link(ui, ed, cmd("view.zoomIn"), "+", "");
                let mut pct = ed.zoom * 100.0;
                let slider = ui.add(egui::Slider::new(&mut pct, 75.0..=300.0).step_by(25.0).show_value(false));
                widgets::record(ed, Some("view.zoomTo"), slider.rect);
                if slider.on_hover_text(widgets::tip(ed, "view.zoomTo", "")).changed() {
                    ed.run("view.zoomTo", json!({"percent": pct}));
                }
                link(ui, ed, cmd("view.zoomOut"), "−", "");
                ui.separator();
                for (id, icon, tip, on) in [
                    ("view.speed", "view.draft", "Draft view", ed.view == crate::editor::View::Draft),
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
                    widgets::record(ed, Some(id), r);
                    if resp.on_hover_text(widgets::tip(ed, id, tip)).clicked() {
                        ed.run(id, json!({}));
                    }
                }
                ui.separator();
                let (theme, other) = if ed.dark { ("Light", "view.lightTheme") } else { ("Dark", "view.darkTheme") };
                link(ui, ed, cmd(other), dim(theme.to_string()), "Switch theme.");
                link(ui, ed, cmd("view.focusMode"), dim("Focus".into()), "Only the page.");
                let sprint = match crate::workspace::sprint_left(ed) {
                    Some(left) => format!("Sprint {}:{:02}", left.as_secs() / 60, left.as_secs() % 60),
                    None => "Sprint".into(),
                };
                link(ui, ed, cmd("view.sprint"), dim(sprint), "Start or stop a writing sprint.");
            });
        });
    });
    ed.chrome.areas.push((crate::ribbon::Place::Status, bar.response.rect));
    ed.chrome.place = crate::ribbon::Place::Other;
}
