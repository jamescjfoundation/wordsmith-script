// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ribbon's tabs and their groups (v0.3 plan, "Ribbon tabs"). Each group lists commands that
//! exist in the registry; `every_ribbon_command_is_registered` keeps it so.

use egui::Ui;
use serde_json::json;
use wordsmith_script::{ElementKind, Elements};

use super::Tab;
use super::widgets::{big, chip, cmd, cmd_with, group, small, stack, tile};
use crate::editor::{Editor, View};

pub fn draw(ed: &mut Editor, ui: &mut Ui) {
    match ed.chrome.tab {
        Tab::Write => write(ed, ui),
        Tab::Insert => insert(ed, ui),
        Tab::Plan => plan(ed, ui),
        Tab::Production => production(ed, ui),
        Tab::Review => review(ed, ui),
        Tab::View => view(ed, ui),
        Tab::Board => board(ed, ui),
        Tab::Cards => cards(ed, ui),
    }
}

/// What the ribbon shows that depends on the script and the caret, worked out once a revision
/// (reading settings from the script's data on every frame would cost typing time).
pub struct RibbonData {
    pub here: ElementKind,
    pub flows: Vec<(ElementKind, String)>,
    pub keys: Vec<(String, String)>,
    pub styles: Vec<String>,
    pub typing: wordsmith_script::typing::Typing,
    pub macros: wordsmith_script::macros::Macros,
}

impl RibbonData {
    pub fn of(ed: &mut Editor) -> Self {
        let kinds = [
            ElementKind::SceneHeading,
            ElementKind::Action,
            ElementKind::Character,
            ElementKind::Parenthetical,
            ElementKind::Dialogue,
            ElementKind::Transition,
            ElementKind::Shot,
        ];
        RibbonData {
            here: current_kind(ed),
            flows: kinds.iter().map(|k| (*k, flow(ed, *k))).collect(),
            keys: ed
                .session
                .registry
                .all()
                .iter()
                .filter(|s| s.id.starts_with("element."))
                .map(|s| (s.id.to_string(), s.shortcut.to_string()))
                .collect(),
            styles: styles_on(ed),
            typing: wordsmith_script::typing::Typing::of_doc(&ed.session.doc),
            macros: wordsmith_script::macros::Macros::of_doc(&ed.session.doc),
        }
    }
}

/// What Return and Tab make from element `kind`, by name.
fn flow(ed: &Editor, kind: ElementKind) -> String {
    let elements = Elements::of_doc(&ed.session.doc);
    let Some(def) = elements.of_kind(kind) else { return String::new() };
    let name = |id: &str| elements.get(id).map_or(id.to_string(), |d| d.name.clone());
    let enter = ed.session.doc.styles.get(&def.id).and_then(|s| s.next.clone()).map_or(def.name.clone(), |n| name(&n));
    let tab = def.tab.as_deref().map(name).unwrap_or_else(|| "nothing".into());
    format!("Return makes {enter}; Tab makes {tab}.")
}

fn current_kind(ed: &Editor) -> ElementKind {
    let elements = Elements::of_doc(&ed.session.doc);
    ed.session.doc.para_at(&ed.session.sel.focus).map_or(ElementKind::General, |p| elements.of_para(p).kind)
}

fn styles_on(ed: &mut Editor) -> Vec<String> {
    ed.session
        .run("format.state", &json!({}))
        .ok()
        .and_then(|v| v["styles"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect()
}

fn write(ed: &mut Editor, ui: &mut Ui) {
    group(ui, ed, "Clipboard", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("edit.cut"), "", "Cut", "Cut the selection to the clipboard.", false);
            small(ui, ed, cmd("edit.copy"), "", "Copy", "Copy the selection to the clipboard.", false);
            small(ui, ed, cmd("select.all"), "select", "Select All", "", false);
        });
    });
    let data = ed.ribbon_data();
    let here = data.here;
    group(ui, ed, "Elements", |ui, ed| {
        let main = [
            ("element.sceneHeading", ElementKind::SceneHeading, "Scene Heading"),
            ("element.action", ElementKind::Action, "Action"),
            ("element.character", ElementKind::Character, "Character"),
            ("element.parenthetical", ElementKind::Parenthetical, "Parenthetical"),
            ("element.dialogue", ElementKind::Dialogue, "Dialogue"),
            ("element.transition", ElementKind::Transition, "Transition"),
            ("element.shot", ElementKind::Shot, "Shot"),
        ];
        egui::Grid::new("element-gallery").spacing([3.0, 3.0]).show(ui, |ui| {
            for (i, (id, kind, name)) in main.iter().enumerate() {
                let key = data.keys.iter().find(|(k, _)| k == id).map_or(String::new(), |(_, v)| v.clone());
                let about = data.flows.iter().find(|(k, _)| k == kind).map_or(String::new(), |(_, v)| v.clone());
                tile(ui, ed, cmd(id), "", (name, &key), &about, here == *kind);
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
            ui.menu_button(egui::RichText::new("More ▾").size(11.5), |ui| {
                for id in [
                    "element.general",
                    "element.castList",
                    "element.newAct",
                    "element.endOfAct",
                    "element.sequence",
                    "element.outline1",
                    "element.outline2",
                    "element.outline3",
                    "element.note",
                    "element.summary",
                ] {
                    let label = ed.session.registry.get(id).map_or(id, |s| s.label);
                    if ui.button(label).clicked() {
                        ed.run(id, json!({}));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Element Settings…").clicked() {
                    ed.run("view.elementSettings", json!({}));
                    ui.close();
                }
            });
        });
    });
    let on = data.styles.clone();
    let is = |s: &str| on.iter().any(|x| x == s);
    group(ui, ed, "Text", |ui, ed| {
        stack(ui, |ui| {
            ui.horizontal(|ui| {
                small(ui, ed, cmd("format.bold"), "", "", "Bold.", is("bold"));
                small(ui, ed, cmd("format.italic"), "", "", "Italic.", is("italic"));
                small(ui, ed, cmd("format.underline"), "", "", "Underline.", is("underline"));
                small(ui, ed, cmd_with("format.style", json!({"style": "strike"})), "strike", "", "Strike through.", is("strike"));
            });
            ui.horizontal(|ui| {
                ui.menu_button("Highlight ▾", |ui| {
                    for (name, _) in wordsmith_script::commands::format::HIGHLIGHTS {
                        if ui.button(name).clicked() {
                            ed.run("format.highlight", json!({"color": name}));
                            ui.close();
                        }
                    }
                    if ui.button("None").clicked() {
                        ed.run("format.highlight", json!({"color": "none"}));
                        ui.close();
                    }
                });
                ui.menu_button("More ▾", |ui| {
                    for (style, label) in [
                        ("doubleUnderline", "Double Underline"),
                        ("wordUnderline", "Underline Words"),
                        ("dottedUnderline", "Dotted Underline"),
                        ("superscript", "Raised"),
                        ("subscript", "Lowered"),
                        ("caps", "Capitals"),
                        ("smallCaps", "Small Capitals"),
                    ] {
                        let mut checked = is(style);
                        if ui.checkbox(&mut checked, label).clicked() {
                            ed.run("format.style", json!({"style": style, "on": checked}));
                        }
                    }
                    ui.separator();
                    for (name, hex) in [("Black ink", "#000000"), ("Red ink", "#C0392B"), ("Blue ink", "#1F5FA8"), ("Automatic ink", "auto")] {
                        if ui.button(name).clicked() {
                            ed.run("format.color", json!({"color": hex}));
                            ui.close();
                        }
                    }
                });
            });
            small(ui, ed, cmd("format.plain"), "", "Plain Text", "Remove every style set by hand.", false);
        });
    });
    let typing = data.typing.clone();
    group(ui, ed, "Typing", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("typing.smartQuotes"), "", "Smart Quotes", "Curly quotes as you type.", typing.smart_quotes);
            small(
                ui,
                ed,
                cmd("typing.autoCapitalize"),
                "",
                "Capitalize Sentences",
                "A capital at the start of each sentence.",
                typing.auto_capitalize,
            );
            small(ui, ed, cmd("text.toggleCase"), "", "Toggle Case", "Capitals, or back to small letters.", false);
        });
        big(ui, ed, cmd("element.reset"), "", "Reset to Element", "Drop formatting set by hand.", false);
    });
    group(ui, ed, "Find & Go", |ui, ed| {
        big(ui, ed, cmd("view.find"), "", "Find", "Find and replace.", ed.find.is_some());
        big(ui, ed, cmd("view.goto"), "", "Go To", "A page, a scene, a bookmark.", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("bookmark.add"), "", "Add Bookmark", "Bookmark this paragraph.", false);
            small(ui, ed, cmd("bookmark.next"), "bookmark", "Next Bookmark", "", false);
            small(ui, ed, cmd("select.scene"), "select", "Select Scene", "", false);
        });
    });
    group(ui, ed, "Macros", |ui, ed| {
        let all = data.macros.clone();
        stack(ui, |ui| {
            for (i, m) in all.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()).take(2) {
                small(
                    ui,
                    ed,
                    cmd_with("macro.run", json!({"slot": i + 1})),
                    "macro",
                    &m.name,
                    &format!("Run this macro ({}).", m.shortcut.clone().unwrap_or_default()),
                    false,
                );
            }
            ui.menu_button("All Macros ▾", |ui| {
                for (i, m) in all.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
                    if ui.add(egui::Button::new(format!("{}. {}", i + 1, m.name)).shortcut_text(m.shortcut.clone().unwrap_or_default())).clicked() {
                        ed.run("macro.run", json!({"slot": i + 1}));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Edit Macros…").clicked() {
                    ed.run("view.macros", json!({}));
                    ui.close();
                }
            });
        });
    });
}

fn insert(ed: &mut Editor, ui: &mut Ui) {
    group(ui, ed, "Script", |ui, ed| {
        big(ui, ed, cmd("scene.insert"), "", "New Scene", "A scene heading after this scene.", false);
        big(ui, ed, cmd("text.pageBreak"), "", "Page Break", "Start a new page here.", false);
        big(ui, ed, cmd("dialogue.dual"), "", "Dual Dialogue", "Set this speech beside the one before.", false);
    });
    group(ui, ed, "Marks", |ui, ed| {
        big(ui, ed, cmd("view.note"), "", "Note", "A note on the selected text.", false);
        big(ui, ed, cmd("bookmark.add"), "", "Bookmark", "Bookmark this paragraph.", false);
        big(ui, ed, cmd("view.sceneProperties"), "characters", "Character in Scene", "Add someone who is in the scene but does not speak.", false);
    });
    group(ui, ed, "Pages", |ui, ed| {
        big(ui, ed, cmd("view.titlePage"), "", "Title Page", "", false);
        big(ui, ed, cmd("view.headerFooter"), "", "Header & Footer", "", false);
        big(ui, ed, cmd("view.watermark"), "", "Watermark", "", false);
    });
}

fn plan(ed: &mut Editor, ui: &mut Ui) {
    use crate::structure::Structure;
    let s = ed.structure;
    group(ui, ed, "Views", |ui, ed| {
        big(ui, ed, cmd("view.page"), "", "Script", "The script's pages.", s.is_none());
        big(ui, ed, cmd("view.cards"), "", "Index Cards", "A card for each scene.", s == Some(Structure::Cards));
        big(ui, ed, cmd("view.scenes"), "", "Scene View", "Scenes as blocks of text.", s == Some(Structure::Scenes));
        big(ui, ed, cmd("view.board"), "", "Story Board", "Beats on a canvas.", s == Some(Structure::Board));
        big(ui, ed, cmd("view.outline"), "", "Outline", "Beats in lanes.", s == Some(Structure::Outline));
        big(ui, ed, cmd("view.strip"), "", "Structure Strip", "Acts, sequences and scenes along the pages.", ed.tools.strip);
    });
    group(ui, ed, "Structure", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("element.sequence"), "", "Sequence", "", false);
            small(ui, ed, cmd("element.outline1"), "", "Outline 1", "", false);
            small(ui, ed, cmd("element.outline2"), "", "Outline 2", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("element.outline3"), "", "Outline 3", "", false);
            small(ui, ed, cmd("element.summary"), "", "Summary", "", false);
            let shows = wordsmith_script::outline::shows_outline(&ed.session.doc);
            small(ui, ed, cmd("outline.show"), "", "Show Outline", "Outline paragraphs in the pages.", shows);
        });
    });
}

fn production(ed: &mut Editor, ui: &mut Ui) {
    let rev = wordsmith_script::revisions::Revisions::of_doc(&ed.session.doc);
    group(ui, ed, "Revisions", |ui, ed| {
        big(ui, ed, cmd("revision.mode"), "", "Revision Mode", "Mark every change in the current set.", rev.mode);
        stack(ui, |ui| {
            let (name, color) = rev.current_index().and_then(|i| rev.set(i)).map_or(("Original".to_string(), egui::Color32::WHITE), |s| {
                (
                    s.name.clone(),
                    wordsmith_script::revisions::parse_color(&s.color).map_or(egui::Color32::WHITE, |c| egui::Color32::from_rgb(c.0, c.1, c.2)),
                )
            });
            chip(ui, ed, cmd("view.revisions"), color, &name, "The current set. Click for every set.");
            small(ui, ed, cmd("revision.next"), "", "Next Set", "Start the next set in the color cycle.", false);
            small(ui, ed, cmd("revision.markRevised"), "", "Mark Revised", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("revision.clearRevised"), "", "Clear Revised", "", false);
            small(ui, ed, cmd("revision.gotoPrevious"), "", "Previous", "Previous revised text.", false);
            small(ui, ed, cmd("revision.gotoNext"), "", "Next", "Next revised text.", false);
        });
    });
    let numbers = wordsmith_script::scene_numbers::SceneNumbers::of_doc(&ed.session.doc);
    group(ui, ed, "Scene Numbers", |ui, ed| {
        big(ui, ed, cmd("scene.showNumbers"), "", "Show Numbers", "", numbers.show);
        stack(ui, |ui| {
            small(ui, ed, cmd("scene.lockNumbers"), "", "Keep Numbers", "Lock them: added scenes take letters.", numbers.locked);
            small(ui, ed, cmd("scene.renumber"), "", "Renumber", "", false);
            small(ui, ed, cmd("view.sceneNumbers"), "", "Options…", "Start, letters, sides, font.", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("scene.omit"), "", "Omit Scene", "Leave OMITTED with its number.", false);
            small(ui, ed, cmd("scene.recall"), "", "Recall Scene", "", false);
        });
    });
    let locked = wordsmith_script::locks::Locks::of_doc(&ed.session.doc).is_locked();
    group(ui, ed, "Locked Pages", |ui, ed| {
        big(ui, ed, cmd("lock.pages"), "", "Lock Pages", "Freeze page numbers; added pages take letters.", locked);
        stack(ui, |ui| {
            small(ui, ed, cmd("lock.relock"), "", "Relock", "", false);
            small(ui, ed, cmd("lock.lockPage"), "", "Lock Page Here", "", false);
            small(ui, ed, cmd("lock.unlockPage"), "", "Unlock Page", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd_with("lock.adjustBreak", json!({"by": -1})), "lock", "Start Up", "Move this page's start up a paragraph.", false);
            small(ui, ed, cmd_with("lock.adjustBreak", json!({"by": 1})), "lock", "Start Down", "Move this page's start down a paragraph.", false);
            small(ui, ed, cmd("lock.unlockAll"), "", "Unlock All", "", false);
        });
    });
    let tracking = wordsmith_script::tracking::is_on(&ed.session.doc);
    group(ui, ed, "Changes", |ui, ed| {
        big(ui, ed, cmd("track.toggle"), "", "Track Changes", "Keep who changed what, and when.", tracking);
        stack(ui, |ui| {
            small(ui, ed, cmd("track.accept"), "", "Accept", "", false);
            small(ui, ed, cmd("track.reject"), "", "Reject", "", false);
            small(ui, ed, cmd("track.next"), "", "Next Change", "", false);
        });
        big(ui, ed, cmd("compare.withFile"), "", "Compare", "With an earlier version.", false);
    });
    group(ui, ed, "Print", |ui, ed| {
        big(ui, ed, cmd("view.print"), "", "Print…", "Ranges, sets, sides, copies.", false);
        big(ui, ed, cmd("file.printRevised"), "", "Revised Pages", "Print only the revised pages.", false);
    });
}

fn review(ed: &mut Editor, ui: &mut Ui) {
    group(ui, ed, "Proofing", |ui, ed| {
        big(ui, ed, cmd("view.checker"), "", "Check Format", "Empty paragraphs, names with no speech…", false);
        let spelling = ed.spelling;
        let (r, resp) = ui.allocate_exact_size(egui::vec2(super::widgets::BIG_W, super::widgets::CONTENT_H), egui::Sense::click());
        let t = crate::theme::Tokens::get(ui.ctx());
        if spelling {
            ui.painter().rect_filled(r, 4.0, t.accent_soft);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 4.0, t.hover);
        }
        crate::script_icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(egui::pos2(r.center().x, r.min.y + 19.0), egui::vec2(26.0, 26.0)),
            "editor",
            t.text,
            t.accent,
        );
        ui.painter().text(egui::pos2(r.center().x, r.min.y + 45.0), egui::Align2::CENTER_CENTER, "Spelling", crate::theme::regular(11.0), t.text);
        if resp.on_hover_text("Underline misspelled words").clicked() {
            ed.spelling = !ed.spelling;
        }
        big(ui, ed, cmd("view.reformat"), "", "Reformat", "Step through paragraphs and set each one's element.", false);
    });
    group(ui, ed, "Notes", |ui, ed| {
        big(ui, ed, cmd("view.note"), "", "New Note", "", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("note.previous"), "", "Previous", "", false);
            small(ui, ed, cmd("note.next"), "", "Next", "", false);
        });
    });
    group(ui, ed, "Reports", |ui, ed| {
        big(ui, ed, cmd("view.reports"), "", "Reports", "Scenes, locations, cast, statistics…", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("view.character"), "", "Characters", "Traits and highlights.", false);
            small(ui, ed, cmd("report.exportCharacters"), "export", "Character CSV", "", false);
        });
    });
    group(ui, ed, "Habits", |ui, ed| {
        big(ui, ed, cmd("view.stats"), "", "Statistics", "Streaks, words, pages, time, goals.", false);
        let on = crate::workspace::sprint_left(ed).is_some();
        big(ui, ed, cmd("view.sprint"), "", "Sprint", "Write against the clock.", on);
    });
}

fn view(ed: &mut Editor, ui: &mut Ui) {
    let v = ed.view;
    group(ui, ed, "Script View", |ui, ed| {
        big(ui, ed, cmd("view.page"), "", "Page", "Pages as they print.", v == View::Page && ed.structure.is_none());
        big(ui, ed, cmd("view.normal"), "", "Normal", "One continuous column.", v == View::Normal && ed.structure.is_none());
        big(ui, ed, cmd("view.speed"), "", "Speed", "Plain and fast.", v == View::Speed && ed.structure.is_none());
    });
    group(ui, ed, "Panels", |ui, ed| {
        let left = ed.chrome.left.is_some();
        let right = ed.chrome.right.is_some();
        if panel_button(ui, ed, "view.navigator", "Navigator", left) {
            ed.chrome.left = if left { None } else { Some(super::Panel::Scenes) };
        }
        if panel_button(ui, ed, "view.inspector", "Inspector", right) {
            ed.chrome.right = if right { None } else { Some(super::Panel::Element) };
        }
        stack(ui, |ui| {
            small(ui, ed, cmd("view.tables"), "", "Tables", "Scenes, characters and notes as tables.", ed.tables.is_some());
            small(ui, ed, cmd("view.strip"), "", "Structure Strip", "", ed.tools.strip);
            let mut menu = ed.chrome.menu_bar;
            if ui.checkbox(&mut menu, "Menu bar").changed() {
                ed.chrome.menu_bar = menu;
            }
        });
    });
    group(ui, ed, "Split", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("view.splitCards"), "", "Cards Beside", "", false);
            small(ui, ed, cmd("view.splitBoard"), "", "Board Beside", "", false);
            small(ui, ed, cmd("view.splitOutline"), "", "Outline Beside", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("view.splitBelow"), "view.split", "Below / Beside", "", false);
            small(ui, ed, cmd("view.splitSwap"), "view.split", "Swap", "", false);
            small(ui, ed, cmd("view.unsplit"), "", "Unsplit", "", ed.split.is_none());
        });
    });
    group(ui, ed, "Zoom", |ui, ed| {
        big(ui, ed, cmd("view.zoomIn"), "", "Zoom In", "", false);
        big(ui, ed, cmd("view.zoomOut"), "", "Zoom Out", "", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("view.zoomReset"), "view.zoomIn", "100%", "Actual size.", false);
            ui.label(format!("{:.0}%", ed.zoom * 100.0));
        });
    });
    group(ui, ed, "Writing", |ui, ed| {
        big(ui, ed, cmd("view.focusMode"), "", "Focus", "Only the page (Mod+Shift+F to leave).", ed.focus_mode);
        big(ui, ed, cmd("view.typewriter"), "", "Typewriter", "Keep the line at the middle.", ed.typewriter);
        big(ui, ed, cmd("view.highlights"), "", "Highlights", "Characters' highlighted speeches.", ed.show_highlights);
    });
    group(ui, ed, "Theme", |ui, ed| {
        let (dark, extra) = (ed.dark, ed.extra_dark);
        stack(ui, |ui| {
            if ui.selectable_label(!dark, "Light").clicked() {
                ed.dark = false;
                ed.extra_dark = false;
            }
            if ui.selectable_label(dark && !extra, "Dark").clicked() {
                ed.dark = true;
                ed.extra_dark = false;
            }
            if ui.selectable_label(dark && extra, "Extra Dark").clicked() {
                ed.dark = true;
                ed.extra_dark = true;
            }
        });
    });
}

/// A large toggle for a panel (no command of its own).
fn panel_button(ui: &mut Ui, ed: &mut Editor, icon: &str, label: &str, on: bool) -> bool {
    let t = crate::theme::Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(egui::vec2(super::widgets::BIG_W, super::widgets::CONTENT_H), egui::Sense::click());
    if on {
        ui.painter().rect_filled(r, 4.0, t.accent_soft);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, t.hover);
    }
    crate::script_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(egui::pos2(r.center().x, r.min.y + 19.0), egui::vec2(26.0, 26.0)),
        icon,
        t.text,
        t.accent,
    );
    ui.painter().text(egui::pos2(r.center().x, r.min.y + 45.0), egui::Align2::CENTER_CENTER, label, crate::theme::regular(11.0), t.text);
    let _ = ed;
    resp.on_hover_text(format!("Show or hide the {label}")).clicked()
}

fn board(ed: &mut Editor, ui: &mut Ui) {
    let sb = wordsmith_script::storyboard::Storyboard::of_doc(&ed.session.doc);
    let current = ed.board.current;
    group(ui, ed, "Beats", |ui, ed| {
        big(ui, ed, cmd_with("beat.add", json!({"board": current})), "", "Add Beat", "A new beat on this board.", false);
        let sel = ed.board.selected;
        if let Some(id) = sel {
            big(
                ui,
                ed,
                cmd_with("beat.toScript", json!({"id": id})),
                "el.outline",
                "To Script",
                "Its title as Outline 1, its body as Summary.",
                false,
            );
            big(ui, ed, cmd_with("beat.delete", json!({"id": id})), "reject", "Delete", "", false);
        }
    });
    group(ui, ed, "Arrange", |ui, ed| {
        let mut ids: Vec<u32> = ed.board.selected.into_iter().collect();
        ids.extend(ed.board.also.iter().copied());
        stack(ui, |ui| {
            small(
                ui,
                ed,
                cmd_with("beats.arrange", json!({"ids": ids, "how": "grid"})),
                "",
                "On the Grid",
                "Shift-click beats to arrange several.",
                false,
            );
            small(ui, ed, cmd_with("beats.arrange", json!({"ids": ids, "how": "column"})), "", "Column", "", false);
            small(ui, ed, cmd_with("beats.arrange", json!({"ids": ids, "how": "row"})), "", "Row", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd_with("beats.arrange", json!({"ids": ids, "how": "cascade"})), "", "Cascade", "", false);
            small(ui, ed, cmd_with("storyboard.configure", json!({"snap": !sb.snap})), "arrange", "Snap to Grid", "", sb.snap);
        });
    });
    group(ui, ed, "Boards", |ui, ed| {
        big(ui, ed, cmd("storyboard.addBoard"), "plus", "New Board", "", false);
        big(ui, ed, cmd_with("storyboard.exportPdf", json!({"board": current})), "", "Export PDF", "This board on one page.", false);
    });
}

fn cards(ed: &mut Editor, ui: &mut Ui) {
    group(ui, ed, "Show", |ui, ed| {
        stack(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Per row");
                let p = &mut ed.tools.cards.per_row;
                egui::ComboBox::from_id_salt("rib-per-row")
                    .selected_text(if *p == 0 { "Fit".to_string() } else { p.to_string() })
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(p, 0, "Fit");
                        for n in 1..=9 {
                            ui.selectable_value(p, n, n.to_string());
                        }
                    });
            });
            ui.selectable_value(&mut ed.tools.cards.script_text, false, "Summaries");
            ui.selectable_value(&mut ed.tools.cards.script_text, true, "Script text");
        });
    });
    group(ui, ed, "Cards", |ui, ed| {
        big(ui, ed, cmd("view.sceneProperties"), "characters", "Properties", "The scene at the caret.", false);
        let t = crate::theme::Tokens::get(ui.ctx());
        let (r, resp) = ui.allocate_exact_size(egui::vec2(super::widgets::BIG_W, super::widgets::CONTENT_H), egui::Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, 4.0, t.hover);
        }
        crate::script_icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(egui::pos2(r.center().x, r.min.y + 19.0), egui::vec2(26.0, 26.0)),
            "print",
            t.text,
            t.accent,
        );
        ui.painter().text(egui::pos2(r.center().x, r.min.y + 45.0), egui::Align2::CENTER_CENTER, "Print Cards", crate::theme::regular(11.0), t.text);
        if resp.on_hover_text("3 × 5 cards or a grid, as a PDF").clicked() {
            ed.tools.cards.printing = !ed.tools.cards.printing;
        }
    });
}

/// Every command id the ribbon can run (for the registry check).
pub const RIBBON_COMMANDS: &[&str] = &[
    "edit.cut",
    "edit.copy",
    "select.all",
    "element.sceneHeading",
    "element.action",
    "element.character",
    "element.parenthetical",
    "element.dialogue",
    "element.transition",
    "element.shot",
    "element.general",
    "element.castList",
    "element.newAct",
    "element.endOfAct",
    "element.sequence",
    "element.outline1",
    "element.outline2",
    "element.outline3",
    "element.note",
    "element.summary",
    "view.elementSettings",
    "format.bold",
    "format.italic",
    "format.underline",
    "format.style",
    "format.highlight",
    "format.color",
    "format.plain",
    "typing.smartQuotes",
    "typing.autoCapitalize",
    "text.toggleCase",
    "element.reset",
    "view.find",
    "view.goto",
    "bookmark.add",
    "bookmark.next",
    "select.scene",
    "macro.run",
    "view.macros",
    "scene.insert",
    "text.pageBreak",
    "dialogue.dual",
    "view.note",
    "view.sceneProperties",
    "view.titlePage",
    "view.headerFooter",
    "view.watermark",
    "view.page",
    "view.cards",
    "view.scenes",
    "view.board",
    "view.outline",
    "view.strip",
    "outline.show",
    "revision.mode",
    "view.revisions",
    "revision.next",
    "revision.markRevised",
    "revision.clearRevised",
    "revision.gotoPrevious",
    "revision.gotoNext",
    "scene.showNumbers",
    "scene.lockNumbers",
    "scene.renumber",
    "view.sceneNumbers",
    "scene.omit",
    "scene.recall",
    "lock.pages",
    "lock.relock",
    "lock.lockPage",
    "lock.unlockPage",
    "lock.adjustBreak",
    "lock.unlockAll",
    "track.toggle",
    "track.accept",
    "track.reject",
    "track.next",
    "compare.withFile",
    "view.print",
    "file.printRevised",
    "view.checker",
    "view.reformat",
    "note.previous",
    "note.next",
    "view.reports",
    "view.character",
    "report.exportCharacters",
    "view.stats",
    "view.sprint",
    "view.normal",
    "view.speed",
    "view.tables",
    "view.splitCards",
    "view.splitBoard",
    "view.splitOutline",
    "view.splitBelow",
    "view.splitSwap",
    "view.unsplit",
    "view.zoomIn",
    "view.zoomOut",
    "view.zoomReset",
    "view.focusMode",
    "view.typewriter",
    "view.highlights",
    "beat.add",
    "beat.toScript",
    "beat.delete",
    "beats.arrange",
    "storyboard.configure",
    "storyboard.addBoard",
    "storyboard.exportPdf",
    "file.save",
    "edit.undo",
    "edit.redo",
    "view.commandPalette",
];
