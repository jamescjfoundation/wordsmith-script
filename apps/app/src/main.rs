// SPDX-License-Identifier: MIT OR Apache-2.0
//! A new foundation app, started from open-source-core's template. It opens a window on the core's
//! shell with one document, shown as rendered pages, and saves it as a project file. Replace the
//! names, add the app's commands, tabs and panels, and keep the app's own rules here; anything a
//! second app would want belongs in the core (AGENTS.md).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;
use std::process::ExitCode;

use jcj_shell::commands::ids;
use jcj_shell::egui;
use jcj_shell::{About, App, Cmd, CommandInfo, Commands, Shell, theme::Tokens};
use serde_json::json;

/// This app's identity in project files: change the id, and raise the version when the app's own
/// data changes shape (with a migration, `jcj_project::App::with_migration`).
const APP: jcj_project::App = jcj_project::App::new("org.example.app", 1);
const NAME: &str = "App";

struct Doc {
    session: jcj_engine::Session,
    path: PathBuf,
    revision: u64,
    saved: u64,
    page: Option<(u64, egui::TextureHandle)>,
}

impl Doc {
    fn new(path: PathBuf) -> Doc {
        let registry = std::sync::Arc::new(jcj_engine::registry_with(jcj_engine::commands::merged(jcj_features::commands())));
        let doc = match jcj_project::files::open(&path, &APP) {
            Ok(project) => project.documents().next().map(|(_, _, d)| d.clone()).unwrap_or_default(),
            Err(_) => jcj_doc::new_document(),
        };
        Doc { session: jcj_engine::session_with(doc, registry), path, revision: 0, saved: 0, page: None }
    }

    fn save(&mut self) -> Result<(), String> {
        let project = jcj_project::Project::with_document(&APP, "Main", self.session.doc.clone()).map_err(|e| e.to_string())?;
        jcj_project::files::save(&self.path, &project).map_err(|e| e.to_string())?;
        self.saved = self.revision;
        Ok(())
    }
}

impl Commands for Doc {
    fn command(&self, id: &str) -> Option<CommandInfo> {
        self.all_commands().into_iter().find(|c| c.id == id)
    }

    fn all_commands(&self) -> Vec<CommandInfo> {
        vec![
            CommandInfo { icon: "save".into(), ..CommandInfo::new(ids::SAVE, "Save", "Mod+S") },
            CommandInfo { icon: "undo".into(), ..CommandInfo::new(ids::UNDO, "Undo", "Mod+Z") },
            CommandInfo { icon: "redo".into(), ..CommandInfo::new(ids::REDO, "Redo", "Mod+Shift+Z") },
            CommandInfo::new("app.hello", "Insert Greeting", "Mod+G"),
        ]
    }
}

impl App for Doc {
    fn document_name(&self) -> String {
        self.path.file_name().map_or_else(|| "Untitled".into(), |n| n.to_string_lossy().into_owned())
    }

    fn is_dirty(&self) -> bool {
        self.revision != self.saved
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn run(&mut self, cmd: &Cmd, _shell: &mut Shell) -> Result<(), String> {
        match cmd.id.as_str() {
            ids::SAVE => return self.save(),
            ids::UNDO | ids::REDO => {
                self.session.run(&cmd.id, &json!({})).map_err(|e| format!("{e:?}"))?;
            }
            "app.hello" => {
                self.session.run("text.insert", &json!({"text": "Hello from a new app. "})).map_err(|e| format!("{e:?}"))?;
            }
            other => return Err(format!("unknown command {other}")),
        }
        self.revision += 1;
        Ok(())
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        let stale = self.page.as_ref().is_none_or(|(rev, _)| *rev != self.revision);
        if stale {
            let mut cache = jcj_layout::LayoutCache::new();
            let layout = jcj_layout::print_layout(&self.session.doc, &mut cache);
            if let Some(img) = jcj_output::render_page(&self.session.doc, &layout, 0, 1.0, &jcj_output::RenderSettings::default())
                && let (Ok(w), Ok(h)) = (usize::try_from(img.width), usize::try_from(img.height))
            {
                let color = egui::ColorImage::from_rgba_unmultiplied([w, h], &img.straight_rgba());
                self.page = Some((self.revision, ui.ctx().load_texture("page", color, egui::TextureOptions::LINEAR)));
            }
        }
        egui::ScrollArea::both().show(ui, |ui| {
            if let Some((_, tex)) = &self.page {
                ui.add(egui::Image::new(tex).fit_to_original_size(1.0));
            }
        });
    }

    fn about(&self) -> About {
        About { summary: "A foundation app.".into(), credit: "Based on WordCraft by the ArtCraft team.".into(), ..About::default() }
    }
}

struct Window {
    shell: Shell,
    doc: Doc,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.doc);
    }
}

fn main() -> ExitCode {
    let path = std::env::args().nth(1).map_or_else(|| PathBuf::from("untitled.app"), PathBuf::from);
    // The app's own look: replace with the product's tokens.json (jcj-theme's TOKENS.md).
    let shell = Shell::new(NAME, env!("CARGO_PKG_VERSION"), Tokens::default());
    let window = Window { shell, doc: Doc::new(path) };
    match eframe::run_native(NAME, eframe::NativeOptions::default(), Box::new(|_cc| Ok(Box::new(window)))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("could not open a window: {e}");
            ExitCode::FAILURE
        }
    }
}
