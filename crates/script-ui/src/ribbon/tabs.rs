// SPDX-License-Identifier: MIT OR Apache-2.0
//! The ribbon's tabs and their groups (decision 0037). Each control runs a command from the
//! registry; `every_drawn_control_runs_a_registered_command` keeps it so.

use egui::Ui;
use serde_json::json;
use wordsmith_script_model::{ElementKind, Elements};

use jcj_shell::workspace::Side;

use super::tab;
use super::widgets::{Rb, big, check, chip, choice, cmd, cmd_with, drop, group, item, popup_menu, small, stack, tile, tile_sized};
use crate::editor::{Editor, View};

/// The command that turns text style `style` (as `format.state` names it) on or off.
pub fn style_cmd(style: &str, on: bool) -> jcj_shell::Cmd {
    let underline = |kind: &str| cmd_with("format.underline", json!({"style": kind, "value": on}));
    match style {
        "strike" => cmd_with("format.strikethrough", json!({"value": on})),
        "doubleUnderline" => underline("double"),
        "dottedUnderline" => underline("dotted"),
        "wordUnderline" => underline("words"),
        "caps" => cmd_with("format.allCaps", json!({"value": on})),
        other => cmd_with(format!("format.{other}"), json!({"value": on})),
    }
}

/// Draw tab `id`'s groups.
pub fn draw(id: &str, ed: &mut Rb<'_>, ui: &mut Ui) {
    match id {
        tab::WRITE => write(ed, ui),
        tab::INSERT => insert(ed, ui),
        tab::PLAN => plan(ed, ui),
        tab::PRODUCTION => production(ed, ui),
        tab::REVIEW => review(ed, ui),
        tab::VIEW => view(ed, ui),
        tab::BOARD => board(ed, ui),
        tab::CARDS => cards(ed, ui),
        _ => {}
    }
}

/// What the ribbon shows that depends on the script and the caret, worked out once a revision
/// (reading settings from the script's data on every frame would cost typing time).
pub struct RibbonData {
    pub here: ElementKind,
    pub flows: Vec<(ElementKind, String)>,
    pub keys: Vec<(String, String)>,
    pub styles: Vec<String>,
    pub typing: wordsmith_script_model::typing::Typing,
    pub macros: wordsmith_script_model::macros::Macros,
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
            typing: wordsmith_script_model::typing::Typing::of_doc(&ed.session.doc),
            macros: wordsmith_script_model::macros::Macros::of_doc(&ed.session.doc),
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

fn write(ed: &mut Rb<'_>, ui: &mut Ui) {
    group(ui, ed, "Clipboard", |ui, ed| {
        big(ui, ed, cmd("edit.paste"), "", "Paste", "Put in what the clipboard holds.", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("edit.cut"), "", "Cut", "Cut the selection to the clipboard.", false);
            small(ui, ed, cmd("edit.copy"), "", "Copy", "Copy the selection to the clipboard.", false);
            small(ui, ed, cmd("select.all"), "select", "Select All", "", false);
        });
    });
    let data = ed.data.clone();
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
                tile(ui, ed, cmd(*id), "", (name, &key), &about, here == *kind);
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
            drop(ui, ed, egui::RichText::new("More ▾").size(11.5), "More elements", |ui, ed| {
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
                    item(ui, ed, cmd(id), label);
                }
                ui.separator();
                item(ui, ed, cmd("view.elementSettings"), "Element Settings…");
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
                small(ui, ed, style_cmd("strike", !is("strike")), "strike", "", "Strike through.", is("strike"));
            });
            ui.horizontal(|ui| {
                drop(ui, ed, "Highlight ▾", "Highlight the selection", |ui, ed| {
                    for (name, _) in wordsmith_script_model::commands::format::HIGHLIGHTS {
                        item(ui, ed, cmd_with("format.highlight", json!({"color": name})), name);
                    }
                    item(ui, ed, cmd_with("format.highlight", json!({"color": "none"})), "None");
                });
                drop(ui, ed, "More ▾", "More text styles", |ui, ed| {
                    for (style, label) in [
                        ("doubleUnderline", "Double Underline"),
                        ("wordUnderline", "Underline Words"),
                        ("dottedUnderline", "Dotted Underline"),
                        ("superscript", "Raised"),
                        ("subscript", "Lowered"),
                        ("caps", "Capitals"),
                        ("smallCaps", "Small Capitals"),
                    ] {
                        check(ui, ed, style_cmd(style, !is(style)), label, is(style));
                    }
                    ui.separator();
                    item(ui, ed, cmd_with("format.changeCase", json!({"mode": "lower"})), "Small Letters");
                    ui.separator();
                    for (name, hex) in [("Black ink", "#000000"), ("Red ink", "#C0392B"), ("Blue ink", "#1F5FA8"), ("Automatic ink", "auto")] {
                        item(ui, ed, cmd_with("format.color", json!({"color": hex})), name);
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
            small(ui, ed, cmd("format.toggleCapitals"), "", "Toggle Case", "Capitals, or back to small letters.", false);
        });
        big(ui, ed, cmd("element.reset"), "", "Reset to Element", "Drop formatting set by hand.", false);
    });
    group(ui, ed, "Find & Go", |ui, ed| {
        big(ui, ed, cmd("view.find"), "", "Find", "Find and replace.", ed.find_bar);
        big(ui, ed, cmd("view.goto"), "", "Go To", "A page, a scene, a bookmark.", false);
        stack(ui, |ui| {
            small(ui, ed, cmd("bookmark.add"), "", "Add Bookmark", "Bookmark this paragraph.", false);
            small(ui, ed, cmd("bookmark.next"), "bookmark", "Next Bookmark", "", false);
            small(ui, ed, cmd("bookmark.previous"), "bookmark", "Previous Bookmark", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("view.bookmarks"), "", "All Bookmarks", "The Script panel's bookmarks.", ed.shell.layout.is_open("bookmarks"));
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
                    cmd(crate::commands::macro_id(i + 1)),
                    "macro",
                    &m.name,
                    &format!("Run this macro ({}).", m.shortcut.clone().unwrap_or_default()),
                    false,
                );
            }
            drop(ui, ed, "All Macros ▾", "Run a macro", |ui, ed| {
                for (i, m) in all.slots.iter().enumerate().filter(|(_, m)| !m.is_empty()) {
                    item(ui, ed, cmd(crate::commands::macro_id(i + 1)), &format!("{}. {}", i + 1, m.name));
                }
                ui.separator();
                item(ui, ed, cmd("view.macros"), "Edit Macros…");
            });
        });
    });
}

fn insert(ed: &mut Rb<'_>, ui: &mut Ui) {
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

fn plan(ed: &mut Rb<'_>, ui: &mut Ui) {
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
            let shows = wordsmith_script_model::outline::shows_outline(&ed.session.doc);
            small(ui, ed, cmd("outline.show"), "", "Show Outline", "Outline paragraphs in the pages.", shows);
        });
    });
}

fn production(ed: &mut Rb<'_>, ui: &mut Ui) {
    let rev = wordsmith_script_model::revisions::Revisions::of_doc(&ed.session.doc);
    group(ui, ed, "Revisions", |ui, ed| {
        big(ui, ed, cmd("revision.mode"), "", "Revision Mode", "Mark every change in the current set.", rev.mode);
        stack(ui, |ui| {
            let (name, color) = rev.current_index().and_then(|i| rev.set(i)).map_or(("Original".to_string(), egui::Color32::WHITE), |s| {
                (
                    s.name.clone(),
                    wordsmith_script_model::revisions::parse_color(&s.color).map_or(egui::Color32::WHITE, |c| egui::Color32::from_rgb(c.0, c.1, c.2)),
                )
            });
            let resp = chip(ui, color, &name, "The current set. Click to choose another.");
            popup_menu(ui, ed, &resp, "Revision set", |ui, ed| {
                for set in &rev.sets {
                    choice(ui, ed, cmd_with("revision.setCurrent", json!({"name": set.name})), &set.name, set.name == rev.current);
                }
                ui.separator();
                item(ui, ed, cmd("view.revisions"), "Revision Sets…");
            });
            small(ui, ed, cmd("revision.next"), "", "Next Set", "Start the next set in the color cycle.", false);
            small(ui, ed, cmd("revision.markRevised"), "", "Mark Revised", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("revision.clearRevised"), "", "Clear Revised", "", false);
            small(ui, ed, cmd("revision.gotoPrevious"), "", "Previous", "Previous revised text.", false);
            small(ui, ed, cmd("revision.gotoNext"), "", "Next", "Next revised text.", false);
        });
    });
    let numbers = wordsmith_script_model::scene_numbers::SceneNumbers::of_doc(&ed.session.doc);
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
    let locked = wordsmith_script_model::locks::Locks::of_doc(&ed.session.doc).is_locked();
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
            small(ui, ed, cmd("lock.unlockLevel"), "", "Unlock a Level", "The newest lettered pages flow freely again.", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("lock.unlockAll"), "", "Unlock All", "", false);
        });
    });
    let tracking = wordsmith_script_model::tracking::is_on(&ed.session.doc);
    group(ui, ed, "Changes", |ui, ed| {
        big(ui, ed, cmd("track.toggle"), "", "Track Changes", "Keep who changed what, and when.", tracking);
        stack(ui, |ui| {
            small(ui, ed, cmd("track.accept"), "", "Accept", "", false);
            small(ui, ed, cmd("track.reject"), "", "Reject", "", false);
            small(ui, ed, cmd("track.next"), "", "Next Change", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("track.acceptAll"), "", "Accept All", "", false);
            small(ui, ed, cmd("track.rejectAll"), "", "Reject All", "", false);
            small(ui, ed, cmd("track.previous"), "", "Previous Change", "", false);
        });
        big(ui, ed, cmd("compare.withFile"), "", "Compare", "With an earlier version.", false);
    });
    group(ui, ed, "Print", |ui, ed| {
        big(ui, ed, cmd("view.print"), "", "Print…", "Ranges, sets, sides, copies.", false);
        big(ui, ed, cmd("file.printRevised"), "", "Revised Pages", "Print only the revised pages.", false);
    });
}

fn review(ed: &mut Rb<'_>, ui: &mut Ui) {
    group(ui, ed, "Proofing", |ui, ed| {
        big(ui, ed, cmd("view.checker"), "", "Check Format", "Empty paragraphs, names with no speech…", false);
        let spelling = ed.spelling;
        big(ui, ed, cmd("view.spelling"), "spelling", "Spelling", "Underline misspelled words.", spelling);
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
        // The reports gallery: each opens the Reports window on that report.
        egui::Grid::new("reports-gallery").spacing([3.0, 3.0]).show(ui, |ui| {
            let kinds = [
                ("scenes", "Scenes"),
                ("locations", "Locations"),
                ("cast", "Cast"),
                ("character", "Character"),
                ("statistics", "Statistics"),
                ("notes", "Notes"),
                ("filtered", "Filtered"),
                ("representation", "Represent."),
            ];
            for (i, (kind, label)) in kinds.iter().enumerate() {
                let about = format!("The {} report.", label.to_lowercase());
                tile_sized(ui, ed, cmd_with("view.reports", json!({"kind": kind})), "report", (label, ""), &about, 78.0);
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("view.character"), "", "Characters", "Traits and highlights.", false);
            small(ui, ed, cmd("report.exportCharacters"), "export", "Character CSV", "", false);
        });
    });
    group(ui, ed, "Habits", |ui, ed| {
        big(ui, ed, cmd("view.stats"), "", "Statistics", "Streaks, words, pages, time, goals.", false);
        let on = crate::workspace::sprint_left(ed.ed).is_some();
        big(ui, ed, cmd("view.sprint"), "", "Sprint", "Write against the clock.", on);
    });
}

fn view(ed: &mut Rb<'_>, ui: &mut Ui) {
    let v = ed.view;
    group(ui, ed, "Script View", |ui, ed| {
        big(ui, ed, cmd("view.page"), "", "Page", "Pages as they print.", v == View::Page && ed.structure.is_none());
        big(ui, ed, cmd("view.normal"), "", "Normal", "One continuous column.", v == View::Normal && ed.structure.is_none());
        big(ui, ed, cmd("view.speed"), "", "Draft", "Plain and fast.", v == View::Draft && ed.structure.is_none());
    });
    group(ui, ed, "Panels", |ui, ed| {
        let left = ed.shell.layout.open_on(Side::Left).is_some();
        let right = ed.shell.layout.open_on(Side::Right).is_some();
        big(ui, ed, cmd("view.scriptPanel"), "", "Script Panel", "Scenes, outline, bookmarks, notes, characters, find.", left);
        big(ui, ed, cmd("view.inspector"), "", "Inspector", "The element and scene at the caret, revisions, the page…", right);
        stack(ui, |ui| {
            small(ui, ed, cmd("view.tables"), "", "Tables", "Scenes, characters and notes as tables.", ed.tables.is_some());
            small(ui, ed, cmd("view.strip"), "", "Structure Strip", "", ed.tools.strip);
            let menu = ed.menu_bar;
            small(ui, ed, cmd("view.menuBar"), "", "Menu Bar", "The menus under the quick-access row.", menu);
        });
    });
    group(ui, ed, "Split", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("view.splitCards"), "", "Cards Beside", "", false);
            small(ui, ed, cmd("view.splitBoard"), "", "Board Beside", "", false);
            small(ui, ed, cmd("view.splitOutline"), "", "Outline Beside", "", false);
        });
        stack(ui, |ui| {
            small(ui, ed, cmd("view.splitScenes"), "", "Scenes Beside", "", false);
            small(ui, ed, cmd("view.splitScript"), "", "Script Beside", "", false);
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
            let now = ed.zoom;
            drop(ui, ed, format!("{:.0}% ▾", now * 100.0), "Zoom to a size", |ui, ed| {
                for z in crate::editor::ZOOM_STEPS {
                    let pct = (z * 100.0).round();
                    choice(ui, ed, cmd_with("view.zoomTo", json!({"percent": pct})), &format!("{pct:.0}%"), (now - z).abs() < 0.01);
                }
            });
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
            small(ui, ed, cmd("view.lightTheme"), "", "Light", "", !dark);
            small(ui, ed, cmd("view.darkTheme"), "", "Dark", "", dark && !extra);
            small(ui, ed, cmd("view.extraDark"), "", "Extra Dark", "", dark && extra);
        });
    });
    group(ui, ed, "Tabs", |ui, ed| {
        stack(ui, |ui| {
            small(ui, ed, cmd("file.newTab"), "", "New Tab", "Another script in a tab.", false);
            small(ui, ed, cmd("view.nextTab"), "newtab", "Next Tab", "", false);
            small(ui, ed, cmd("file.closeTab"), "close", "Close Tab", "", false);
        });
    });
}

fn board(ed: &mut Rb<'_>, ui: &mut Ui) {
    let sb = wordsmith_script_model::storyboard::Storyboard::of_doc(&ed.session.doc);
    let current = ed.board.current;
    group(ui, ed, "Beats", |ui, ed| {
        big(ui, ed, cmd_with("beat.add", json!({"board": current})), "", "Add Beat", "A new beat on this board.", false);
        let sel = ed.board.selected();
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
        let ids = ed.board.ids();
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

fn cards(ed: &mut Rb<'_>, ui: &mut Ui) {
    group(ui, ed, "Show", |ui, ed| {
        stack(ui, |ui| {
            let per_row = ed.tools.cards.per_row;
            let shown = if per_row == 0 { "Fit".to_string() } else { per_row.to_string() };
            drop(ui, ed, format!("Per row: {shown} ▾"), "Cards in each row", |ui, ed| {
                choice(ui, ed, cmd_with("cards.perRow", json!({"count": 0})), "Fit", per_row == 0);
                for n in 1..=9 {
                    choice(ui, ed, cmd_with("cards.perRow", json!({"count": n})), &n.to_string(), per_row == n);
                }
            });
            let script = ed.tools.cards.script_text;
            small(ui, ed, cmd_with("cards.scriptText", json!({"show": "summaries"})), "el.outline", "Summaries", "Each scene's summary.", !script);
            small(
                ui,
                ed,
                cmd_with("cards.scriptText", json!({"show": "script"})),
                "view.scenes",
                "Script Text",
                "Each scene's opening text.",
                script,
            );
        });
    });
    group(ui, ed, "Cards", |ui, ed| {
        big(ui, ed, cmd("view.sceneProperties"), "characters", "Properties", "The scene at the caret.", false);
        let printing = ed.tools.cards.printing;
        big(ui, ed, cmd("cards.print"), "", "Print Cards", "3 × 5 cards or a grid, as a PDF.", printing);
    });
}
