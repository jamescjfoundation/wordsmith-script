// SPDX-License-Identifier: MIT OR Apache-2.0
//! Checking for, downloading and installing updates with craft-update (decision 0038).
//!
//! Releases come only from this project's GitHub repository. Every download is checked against
//! the release's SHA-256 before anything is replaced, and a copy that can't replace itself (a
//! development build, an unsigned Windows copy, a bundle we don't manage) is offered the release
//! page instead. The work runs on threads; the window only sees `wordsmith_ui::updates`.

use std::sync::mpsc::{Receiver, Sender, channel};

use craft_update::self_update::{self, Outcome, Restart};
use craft_update::{App, Http, JobPhase, Paths, Platform, Release, Stop};
use wordsmith_ui::updates::{Offer, Request, Status, Updates};

/// Wordsmith Script as craft-update knows it. The id is the stem of every release file
/// (`wordsmith-script-<version>-<os>-<arch>.<ext>`); the repository is the only place releases
/// are read and downloaded from.
pub const APP: App = App {
    bundle_id_override: Some("org.wordsmith-script.app"),
    developer: "the Wordsmith Script contributors",
    ..App::new("wordsmith-script", "Wordsmith Script", "Write screenplays", "jamescjfoundation/wordsmith-script")
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether this platform can install an update in place. Not macOS: craft-update (at our pin)
/// accepts a new macOS app only when it carries the Developer ID of ArtCraft's own team
/// (`install::macos::TEAM_ID`), which no copy of ours ever will, signed or not. The writer gets
/// the release page there instead (decision 0038).
const INSTALLS_IN_PLACE: bool = !cfg!(target_os = "macos");

enum Message {
    Checked { found: Result<Option<Release>, String>, asked: bool },
    Phase(String),
    Installed(Result<Restart, String>),
}

/// The desktop side of updates: the worker threads' answers and the update waiting for the exit.
pub struct Updater {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    /// The release last found newer, for Install.
    offered: Option<Release>,
    /// The helper that starts the new version once this process ends.
    restart: Option<Restart>,
    checked_at_start: bool,
}

impl Updater {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Updater { tx, rx, offered: None, restart: None, checked_at_start: false }
    }

    /// What the last run's update came to, shown once in this run.
    pub fn read_outcome(&self, ui: &mut Updates) {
        let Ok(paths) = Paths::for_app(&APP) else { return };
        match self_update::take_outcome(&paths, VERSION) {
            Some(Outcome::Installed(v)) => ui.status = Status::Updated(v.to_string()),
            Some(Outcome::Failed(why)) => {
                ui.status = Status::Failed(why);
                ui.prompt = true;
            }
            None => {}
        }
    }

    /// Takes the window's requests and the workers' answers. Returns true when the window should
    /// close so the installed version starts.
    pub fn tick(&mut self, ui: &mut Updates, ctx: &egui::Context) -> bool {
        if !self.checked_at_start {
            self.checked_at_start = true;
            if ui.check_at_start && ui.status == Status::Idle {
                ui.status = Status::Checking;
                self.check(ctx, false);
            }
        }
        let mut close = false;
        for request in ui.take_requests() {
            match request {
                Request::Check => self.check(ctx, true),
                Request::Install => self.install(ui, ctx),
                Request::Restart => close = self.restart.is_some(),
            }
        }
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::Checked { found, asked } => self.checked(ui, found, asked),
                Message::Phase(words) => {
                    if matches!(ui.status, Status::Installing(_)) {
                        ui.status = Status::Installing(words);
                    }
                }
                Message::Installed(Ok(restart)) => {
                    self.restart = Some(restart);
                    let v = self.offered.as_ref().map_or_else(String::new, Release::version_label);
                    ui.status = Status::Ready(v);
                    ui.prompt = true;
                }
                Message::Installed(Err(why)) => {
                    ui.status = Status::Failed(why);
                    ui.prompt = true;
                }
            }
        }
        if matches!(ui.status, Status::Checking | Status::Installing(_)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        close
    }

    /// Starts the helper of an installed update. Call once the exit is certain.
    pub fn finish(&mut self) {
        if let Some(restart) = self.restart.take()
            && let Err(e) = restart.start()
        {
            eprintln!("wordsmith: the updated version could not be started: {e}");
        }
    }

    fn check(&self, ctx: &egui::Context, asked: bool) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let found = http().and_then(|http| craft_update::check_for_update(&APP, &http, VERSION));
            let _ = tx.send(Message::Checked { found, asked });
            ctx.request_repaint();
        });
        if let Err(e) = spawned {
            let _ = self.tx.send(Message::Checked { found: Err(format!("could not start the check ({e})")), asked });
        }
    }

    fn checked(&mut self, ui: &mut Updates, found: Result<Option<Release>, String>, asked: bool) {
        match found {
            Ok(Some(release)) => {
                let installs = INSTALLS_IN_PLACE && self_update::is_self_updating(&APP) && self_update::release_installs(&APP, &release);
                let page = if release.html_url.starts_with("https://github.com/") { release.html_url.clone() } else { APP.releases_url() };
                ui.status = Status::Available(Offer { version: release.version_label(), page, installs });
                self.offered = Some(release);
                ui.prompt = true;
            }
            Ok(None) => {
                ui.status = Status::Current;
                ui.prompt |= asked;
            }
            // A check at start that fails says nothing: the writer didn't ask, and may be offline.
            Err(why) if asked => {
                ui.status = Status::Failed(why);
                ui.prompt = true;
            }
            Err(_) => ui.status = Status::Idle,
        }
    }

    fn install(&mut self, ui: &mut Updates, ctx: &egui::Context) {
        let Some(release) = self.offered.clone() else {
            ui.status = Status::Failed("there is no update to install; check again".into());
            return;
        };
        let installed = if INSTALLS_IN_PLACE { self_update::current(&APP) } else { None };
        let Some(installed) = installed else {
            ui.status = Status::Failed("this copy can't replace itself; download the new version from the release page".into());
            return;
        };
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("update-install".into()).spawn(move || {
            let result = install(&installed, release, &tx, &ctx);
            let _ = tx.send(Message::Installed(result));
            ctx.request_repaint();
        });
        if let Err(e) = spawned {
            ui.status = Status::Failed(format!("could not start the update ({e})"));
        }
    }
}

fn http() -> Result<Http, String> {
    // The product and its version, nothing about the writer.
    Http::new(&format!("WordsmithScript/{VERSION}"))
}

fn install(installed: &self_update::Install, mut release: Release, tx: &Sender<Message>, ctx: &egui::Context) -> Result<Restart, String> {
    let http = http()?;
    // A release found by the list may come without its files; read it again in full.
    if release.assets.is_empty()
        && let Some(full) = craft_update::check_for_update(&APP, &http, VERSION)?
    {
        release = full;
    }
    let paths = Paths::for_app(&APP)?;
    let platform = Platform::current().ok_or("updates aren't available on this platform")?;
    let mut phase = |p: JobPhase| {
        let _ = tx.send(Message::Phase(describe(&p)));
        ctx.request_repaint();
    };
    self_update::prepare_update(&APP, installed, &release, platform, &http, &paths, &Stop::default(), &mut phase, &|| Ok(()), Some(VERSION))
}

fn describe(phase: &JobPhase) -> String {
    let mb = |b: u64| b as f64 / 1_048_576.0;
    match phase {
        JobPhase::Downloading { done, total } if *total > 0 => format!("Downloading… {:.1} of {:.1} MB", mb(*done), mb(*total)),
        JobPhase::Downloading { .. } => "Downloading…".into(),
        JobPhase::Reconnecting { .. } => "The connection dropped; trying again…".into(),
        JobPhase::Verifying => "Checking the download…".into(),
        JobPhase::Waiting => "Waiting for the system installer…".into(),
        JobPhase::Installing => "Installing…".into(),
        JobPhase::Removing => "Removing the old version…".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_come_from_our_repository_under_our_name() {
        assert_eq!(APP.releases_url(), "https://github.com/jamescjfoundation/wordsmith-script/releases");
        assert_eq!(APP.bundle_id(), "org.wordsmith-script.app");
        assert_eq!(APP.stems().collect::<Vec<_>>(), ["wordsmith-script"]);
        assert!(!APP.developer.contains("ArtCraft"));
    }

    #[test]
    fn macos_is_sent_to_the_release_page() {
        let mut u = Updater::new();
        let mut ui = Updates::default();
        let release = Release { tag: "v99.0.0".into(), version: craft_update::parse_version("99.0.0"), ..Default::default() };
        u.checked(&mut ui, Ok(Some(release)), true);
        let Status::Available(offer) = ui.status.clone() else { panic!("{:?}", ui.status) };
        assert_eq!(offer.version, "99.0.0");
        if cfg!(target_os = "macos") {
            assert!(!offer.installs);
            u.install(&mut ui, &egui::Context::default());
            assert!(matches!(ui.status, Status::Failed(_)));
        }
        assert!(offer.page.starts_with("https://github.com/jamescjfoundation/wordsmith-script/releases"));
    }

    #[test]
    fn a_check_at_start_that_fails_stays_quiet_and_an_asked_one_says_why() {
        let mut u = Updater::new();
        let mut ui = Updates::default();
        u.checked(&mut ui, Err("offline".into()), false);
        assert_eq!((ui.status.clone(), ui.prompt), (Status::Idle, false));
        u.checked(&mut ui, Err("offline".into()), true);
        assert_eq!((ui.status.clone(), ui.prompt), (Status::Failed("offline".into()), true));
        u.checked(&mut ui, Ok(None), false);
        assert_eq!(ui.status, Status::Current);
    }
}
