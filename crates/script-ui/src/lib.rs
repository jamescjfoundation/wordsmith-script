// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface layer for Wordsmith Script: the script editor and its screenplay views, ribbon tabs,
//! panels, File pages and dialogs, as an app on the foundation's shell (`jcj_shell`), which draws
//! and runs the window around them (ribbon, panels, File page, palette, preferences, auto-save,
//! updates). Only the app crates may depend on it; `cargo xtask ci` enforces that with its
//! layering check.
//!
//! The interface owns no behavior. Every action, its own included (opening the command palette,
//! copying), is a command in the session's registry, run by id.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod app;
pub mod backstage;
pub mod board;
pub mod brand;
pub mod chrome;
pub mod commands;
pub mod dialogs;
pub mod editor;
pub mod fonts;
pub mod outline;
pub mod prefs;
pub mod production;
pub mod reports;
pub mod ribbon;
pub mod script_icons;
pub mod sidebar;
pub mod snapshot;
pub mod strip;
pub mod structure;
pub mod tables;
pub mod theme;
pub mod tools;
pub mod workspace;

pub use app::shell;
pub use commands::{new_session, registry};
pub use editor::Editor;

/// Where Script's releases come from, for updates (decision 0038): this project's GitHub
/// releases, with files named `wordsmith-script-<version>-<os>-<arch>.<ext>`.
#[cfg(not(target_arch = "wasm32"))]
pub const RELEASES: jcj_shell::updates::desktop::ReleaseSource = jcj_shell::updates::desktop::ReleaseSource {
    id: app::ID,
    name: app::NAME,
    tagline: "Write screenplays",
    repo: "jamescjfoundation/wordsmith-script",
    developer: "the Wordsmith Script contributors",
    version: wordsmith_script_model::VERSION,
    // Not macOS: the updater (at the core's pin) accepts a new macOS app only when it carries the
    // Developer ID of its makers' own team, which no copy of ours ever will. The writer gets the
    // release page there instead.
    installs_in_place: !cfg!(target_os = "macos"),
};

/// The About page's first lines: the name, version and license.
pub fn about_summary() -> String {
    format!("{} {}\nLicensed under MIT OR Apache-2.0.", wordsmith_script_model::PRODUCT_NAME, wordsmith_script_model::VERSION)
}

/// The open-source work the app is built on, for the About page's Acknowledgments.
pub fn acknowledgments() -> String {
    "Built on open-source-core, and with craft-libs, egui, AccessKit, HarfRust, Skrifa, Vello CPU and kurbo. The fonts (Courier Prime, Inter, JetBrains Mono, Source Sans 3 and Source Serif 4) are under the SIL Open Font License 1.1. Spelling uses the public-domain Moby word list by Grady Ward. The license text of every library the app is built from is in THIRD-PARTY-LICENSES.txt, which comes with the app, and the fonts' are in the licenses folder beside it.".to_string()
}

/// Text for `--version`: the summary, the credit, then the acknowledgments.
pub fn about_text() -> String {
    format!("{}\n{}\n\nAcknowledgments\n{}", about_summary(), app::CREDIT, acknowledgments())
}

/// Seconds since 1970, for the dates notes and statistics keep (Script's clock, which tests fix).
pub fn now_secs() -> u64 {
    wordsmith_script_model::clock::now()
}

#[cfg(test)]
pub(crate) mod tests;
