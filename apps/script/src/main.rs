// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script desktop app: one window, a script in each tab, on the foundation's shell.
//!
//! `wordsmith-script [script]` opens the script (a `.wss` or Fountain file; plain text, RTF and
//! PDF are imported, asking whether text is a script). `wordsmith-script --version` prints the
//! version, the credit and the acknowledgments.
//!
//! The start-up is the core's (`jcj_shell::desktop`, core decision 0014): this file gives
//! Script's data ([`DESKTOP`]), and `wordsmith_script_ui::desktop` what only Script does. The
//! window's title, app id and icon, the settings folder, the preferences, the update check, the
//! close that starts an installed update and its helper, the file dialogs and the steps of every
//! frame and of the end are not written here, so none of them can be left out. Before anything
//! else, `main` hands its arguments to the updater, because on Windows the updater finishes
//! through a copy of this program (the core's `docs/app-releases.md`, "Updating in place").

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use jcj_shell::desktop::{Desktop, Releases};
use wordsmith_script_ui::app::{APP_ID, ID, NAME};

/// Script's data for the core's start-up (decision 0038): its names, its brand, where its releases
/// come from (this project's GitHub releases, with files named
/// `wordsmith-script-<version>-<os>-<arch>.<ext>`), the variable that names an isolated settings
/// folder, and the window's first size. Updates are installed in place on Linux and Windows; on
/// macOS the update check offers the release page, because the updater (at the core's pin)
/// accepts a new macOS app only when it carries the Developer ID of its makers' own team.
const DESKTOP: Desktop = Desktop {
    brand: wordsmith_script_ui::brand::BRAND,
    releases: Some(Releases {
        repo: "jamescjfoundation/wordsmith-script",
        tagline: "Write screenplays",
        developer: "the Wordsmith Script contributors",
    }),
    profile_env: Some("WORDSMITH_SCRIPT_PROFILE_DIR"),
    window_size: Some([1000.0, 900.0]),
    ..Desktop::new(NAME, ID, APP_ID, env!("CARGO_PKG_VERSION"))
};

/// What the command line asks for: the version text, or a window with the script named first.
#[derive(Debug, PartialEq)]
enum Asked {
    Version,
    Window(Vec<PathBuf>),
}

/// Read the command line (without the program's name) as Script always has: `--version` first
/// prints the version; otherwise the first argument is the script to open, kept as the system
/// gave it.
fn asked(args: Vec<OsString>) -> Asked {
    let first = args.into_iter().next();
    if first.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        return Asked::Version;
    }
    Asked::Window(first.map(PathBuf::from).into_iter().collect())
}

fn main() -> ExitCode {
    // First, before the program reads its own arguments: on Windows an update is finished by a
    // copy of this program, started with the updater's arguments. It is not an ordinary start.
    if let Some(code) = DESKTOP.finish_update() {
        return code;
    }
    match asked(std::env::args_os().skip(1).collect()) {
        Asked::Version => {
            println!("{}", wordsmith_script_ui::about_text());
            ExitCode::SUCCESS
        }
        Asked::Window(files) => match DESKTOP.run(files, wordsmith_script_ui::desktop::build) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{ID}: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use jcj_shell::egui;

    use super::*;

    #[test]
    fn releases_come_from_our_repository_under_our_name() {
        let r = DESKTOP.release_source().unwrap();
        assert_eq!(r.repo, "jamescjfoundation/wordsmith-script");
        assert_eq!(r.id, "wordsmith-script");
        assert_eq!(r.name, "Wordsmith Script");
        assert_eq!(r.tagline, "Write screenplays");
        assert_eq!(r.developer, "the Wordsmith Script contributors");
        assert_eq!(r.releases_url(), "https://github.com/jamescjfoundation/wordsmith-script/releases");
        assert_eq!(r.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn the_window_and_the_program_carry_scripts_own_names() {
        // The program is `wordsmith-script`, so `wordsmith` stays free for the launcher, and the
        // window's id is the one the menu entry and the macOS bundle carry (decision 0045).
        assert_eq!(env!("CARGO_PKG_NAME"), "wordsmith-script");
        assert_eq!(env!("CARGO_BIN_NAME"), "wordsmith-script");
        assert_eq!(APP_ID, "org.jcjfoundation.script");
        assert_eq!(ID, "wordsmith-script");
        // The window is the app's name at first, with its app id, icon and size.
        let viewport = DESKTOP.viewport();
        assert_eq!(viewport.title.as_deref(), Some("Wordsmith Script"));
        assert_eq!(viewport.app_id.as_deref(), Some(APP_ID));
        assert_eq!(viewport.inner_size, Some(egui::vec2(1000.0, 900.0)));
        assert!(viewport.icon.is_some(), "the brand's icon decodes");
    }

    #[test]
    fn macos_is_sent_to_the_release_page() {
        assert_eq!(DESKTOP.release_source().unwrap().installs_in_place, !cfg!(target_os = "macos"));
    }

    #[test]
    fn an_isolated_profile_is_used_whole() {
        assert_eq!(DESKTOP.profile_env, Some("WORDSMITH_SCRIPT_PROFILE_DIR"));
        assert_eq!(DESKTOP.settings_dir_from(Some("profile-folder".into())), Some(PathBuf::from("profile-folder")));
        // Otherwise the platform's folder for the app, as the shell names it.
        assert_eq!(DESKTOP.settings_dir_from(None), jcj_shell::prefs::settings_dir("wordsmith-script", "Wordsmith Script"));
        assert_eq!(DESKTOP.settings_dir_from(Some("".into())), DESKTOP.settings_dir_from(None));
    }

    #[test]
    fn the_command_line_is_the_version_or_a_script() {
        let os = |args: &[&str]| args.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(asked(os(&[])), Asked::Window(Vec::new()));
        assert_eq!(asked(os(&["--version"])), Asked::Version);
        assert_eq!(asked(os(&["a.wss", "b.wss"])), Asked::Window(vec![PathBuf::from("a.wss")]));
        assert_eq!(asked(os(&["a.wss", "--version"])), Asked::Window(vec![PathBuf::from("a.wss")]));
        // An ordinary start is not the updater's helper; the helper's start never opens a window.
        assert!(DESKTOP.finish_update_with(&os(&["a.wss"])).is_none());
        assert!(DESKTOP.finish_update_with(&os(&["--version"])).is_none());
        assert!(DESKTOP.finish_update_with(&os(&[jcj_shell::updates::desktop::FINISH_FLAG])).is_some());
    }

    /// The start-up without a window: Script's shell, its preferences file (0.1's shape read),
    /// its statistics and personal dictionary in the settings folder, the script named on the
    /// command line, and everything written again at the end.
    #[test]
    fn script_starts_from_its_data() {
        let dir = std::env::temp_dir().join(format!("wordsmith-script-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let settings = dir.join("settings");
        std::fs::create_dir_all(&settings).unwrap();
        std::fs::write(settings.join("preferences.json"), r#"{"app": {"view": {"zoom": 1.5}, "backups": {"folder": ""}}}"#).unwrap();
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/stats-0.9.0.json"), settings.join("stats.json")).unwrap();
        let ctx = egui::Context::default();
        let missing = dir.join("Nowhere.wss");
        let mut window =
            jcj_shell::desktop::Window::start(&DESKTOP, &ctx, Some(settings.clone()), vec![missing], wordsmith_script_ui::desktop::build);
        assert_eq!(window.shell.name, "Wordsmith Script");
        assert!(!window.shell.options.shortcuts, "Script's own shell, which leaves the keys to the editor");
        assert_eq!(window.app.zoom, 1.5);
        assert_eq!(window.app.backups.dir, Some(settings.join("Backups")));
        assert_eq!(window.app.settings_dir.as_deref(), Some(settings.as_path()));
        assert_eq!(window.app.stats.data.days.len(), 10, "the writer's statistics are read");
        assert!(!window.app.message.is_empty(), "a script that cannot be opened is said");
        window.app.stats.dirty = true;
        window.exit();
        assert!(settings.join("preferences.json").is_file());
        let written = std::fs::read_to_string(settings.join("stats.json")).unwrap();
        assert_eq!(jcj_shell::writing::stats::Statistics::from_json(&written).map(|s| s.days.len()), Some(10));
        drop(window);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
