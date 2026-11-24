// SPDX-License-Identifier: MIT OR Apache-2.0
//! Software updates as the window shows them (decision 0038).
//!
//! The desktop app does the work: it checks GitHub releases, downloads, verifies and installs with
//! craft-update on a thread of its own, and reports here. This module only holds what to show and
//! what the writer asked for, so the interface crate has no network code and the web build has
//! nothing to run (its `Updates` stays [`Status::Unsupported`]).

use egui::{Context, RichText, Ui};

use crate::editor::Editor;
use crate::theme::Tokens;

/// Where an update stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing checked yet in this run.
    #[default]
    Idle,
    Checking,
    /// The latest release is this version or older.
    Current,
    Available(Offer),
    /// Downloading, verifying or installing; the words say which.
    Installing(String),
    /// The new version is in place; it starts when the app restarts.
    Ready(String),
    Failed(String),
    /// This run is the version an update installed.
    Updated(String),
    /// This build can't check (the web build).
    Unsupported(String),
}

/// A newer release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// `0.2.0`
    pub version: String,
    /// The release's page, for its notes and its files.
    pub page: String,
    /// Whether this copy can install it itself. When it can't (an unsigned Windows copy, a
    /// development build, a bundle we don't replace), the writer gets the release page instead.
    pub installs: bool,
}

/// What the writer asked the desktop app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Check,
    Install,
    /// Close the window so the installed version starts.
    Restart,
}

/// The update state the window keeps.
#[derive(Debug, Default)]
pub struct Updates {
    pub status: Status,
    /// Check once when the app starts (a preference, on by default).
    pub check_at_start: bool,
    /// Requests the app has not taken yet.
    pub requests: Vec<Request>,
    /// Show the window offering the update: set when a check at start finds one, or when a
    /// requested check ends.
    pub prompt: bool,
}

impl Updates {
    /// The requests since the app last asked, oldest first.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// A check the writer asked for: its answer is always shown.
    pub fn check_now(&mut self) {
        if matches!(self.status, Status::Unsupported(_)) {
            self.prompt = true;
            return;
        }
        if !matches!(self.status, Status::Checking | Status::Installing(_)) {
            self.requests.push(Request::Check);
            self.status = Status::Checking;
        }
        self.prompt = true;
    }

    /// The words for the status, and whether it is a problem.
    pub fn describe(&self) -> (String, bool) {
        let name = wordsmith_script::PRODUCT_NAME;
        let running = wordsmith_script::VERSION;
        match &self.status {
            Status::Idle => (format!("{name} {running}."), false),
            Status::Checking => ("Checking for updates…".into(), false),
            Status::Current => (format!("{name} {running} is the latest version."), false),
            Status::Available(o) if o.installs => (format!("{name} {} is available. You have {running}.", o.version), false),
            Status::Available(o) => {
                (format!("{name} {} is available. You have {running}. Download it from the release page and replace this copy.", o.version), false)
            }
            Status::Installing(what) => (what.clone(), false),
            Status::Ready(v) => (format!("{name} {v} is installed. Restart to use it; your scripts are kept."), false),
            Status::Failed(why) => (format!("The update did not finish: {why}"), true),
            Status::Updated(v) => (format!("{name} was updated to {v}."), false),
            Status::Unsupported(why) => (why.clone(), false),
        }
    }
}

/// The buttons that fit the status, under its words. Shared by the prompt window and the About
/// page.
pub fn actions(ed: &mut Editor, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (text, problem) = ed.updates.describe();
    ui.label(RichText::new(text).color(if problem { t.danger } else { t.text }));
    ui.add_space(6.0);
    ui.horizontal(|ui| match ed.updates.status.clone() {
        Status::Available(offer) => {
            if offer.installs && ui.button("Install and Restart").clicked() {
                ed.updates.requests.push(Request::Install);
                ed.updates.status = Status::Installing("Downloading…".into());
            }
            if ui.button(if offer.installs { "Release Notes" } else { "Open Release Page" }).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&offer.page));
            }
        }
        Status::Ready(_) => {
            if ui.button("Restart Now").clicked() {
                ed.updates.requests.push(Request::Restart);
            }
        }
        Status::Checking | Status::Installing(_) => {
            ui.spinner();
        }
        Status::Unsupported(_) => {}
        _ => {
            crate::ribbon::widgets::button(ui, ed, crate::ribbon::widgets::cmd("help.checkForUpdates"), "Check for Updates", "");
        }
    });
}

/// The window that offers an update or answers a check.
pub fn window(ed: &mut Editor, ctx: &Context) {
    if !ed.updates.prompt {
        return;
    }
    let mut open = true;
    egui::Window::new("Software Update").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.set_max_width(380.0);
        actions(ed, ui);
    });
    if !open {
        ed.updates.prompt = false;
    }
}
