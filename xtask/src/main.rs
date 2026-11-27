// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for Wordsmith Script, built on open-source-core.
//!
//! This file is the app's own. It holds only what is Script's: its data (`APP`) and its own
//! tasks (`TASKS`). The gate, every shared task and the list of them are in the modules under
//! `shared`, which the core owns and `cargo xtask sync` keeps identical in every app;
//! `cargo xtask help` lists them.
//!
//! Script's part of the gate is data: `lint-roots` also requires `clippy::indexing_slicing` in
//! the crates that may not index, and `assets` also wants `UPSTREAM.md`. What a release carries
//! is data too (`package`): `cargo xtask package` and the release workflow are the core's (core
//! decision 0019). Outside the gate: `package` (this computer's release package), `wasm` (the
//! web build) and `screenshots` (the README's pictures).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod shared;

use std::path::Path;
use std::process::ExitCode;

use shared::gate::{self, App, Task};
use shared::package::{FileType, Package};
use shared::util::{TaskResult, cargo};

/// Crates whose product code may not index or slice (`clippy.toml`): their roots must also deny
/// `clippy::indexing_slicing`.
const NO_INDEXING_CRATES: &[&str] = &["wordsmith-script-model", "wordsmith-script-format"];

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const WASM_CRATE: &str = "wordsmith-script-web";

/// What the shared tasks need to know about Script.
const APP: App = App {
    product: "Wordsmith Script",
    // The packages that build the programs Script ships, the desktop program's first.
    shipped: &["wordsmith-script", "wordsmith-script-cli"],
    // What a release carries (the core's docs/app-releases.md says what each field is).
    package: Some(Package {
        // Also `wordsmith_script_ui::app::ID`, the id of the update source.
        id: "wordsmith-script",
        // Also `wordsmith_script_ui::app::APP_ID`, the id the window is given (decision 0045).
        app_id: "org.jcjfoundation.script",
        summary: "Write screenplays",
        categories: "Office;WordProcessor;",
        icons: &[(256, "assets/brand/icon-256.png"), (512, "assets/brand/icon-512.png"), (1024, "assets/brand/icon-1024.png")],
        file_types: &[
            FileType { name: "Wordsmith Script screenplay", extensions: &["wss"], mime: "application/x-wordsmith-script", ..FileType::DEFAULT },
            // Another program's format: Script is offered for it and never made its owner.
            FileType {
                name: "Fountain screenplay",
                extensions: &["fountain", "spmd"],
                mime: "text/x-fountain",
                parent: "text/plain",
                own: false,
                ..FileType::DEFAULT
            },
        ],
        // The fonts' license texts.
        license_files: &["assets/fonts"],
        ..Package::DEFAULT
    }),
    no_indexing: NO_INDEXING_CRATES,
    // Beyond the legal files every app carries (the `assets` check).
    legal_files: &["UPSTREAM.md"],
    // `cargo xtask screenshots` redraws docs/images from the sample script.
    screenshots: Some("wordsmith-script-ui"),
    tasks: TASKS,
    ..App::DEFAULT
};

/// Script's own task. It is not part of the gate: slow builds stay out of it.
const TASKS: &[Task] = &[Task::other("wasm", "build wordsmith-script-web for wasm32-unknown-unknown", |root, _| wasm(root))];

fn main() -> ExitCode {
    gate::main(&APP)
}

/// The web build. Not part of `ci`.
fn wasm(root: &Path) -> TaskResult {
    cargo(root, &["build", "--locked", "-p", WASM_CRATE, "--target", WASM_TARGET])
        .map_err(|e| format!("{e} (is the target installed? rustup target add {WASM_TARGET})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_table_is_sound_and_slow_builds_stay_out_of_the_gate() {
        assert_eq!(gate::table_problems(&APP), Vec::<String>::new());
        let in_gate = gate::gate_names(&APP);
        for check in ["lint-roots", "app-layers", "assets"] {
            assert!(in_gate.contains(&check), "{check} is not in ci");
        }
        for slow in ["package", "release", "wasm", "screenshots"] {
            assert!(!in_gate.contains(&slow), "{slow} is in ci");
        }
        assert!(in_gate.contains(&"release-data"), "release-data is not in ci");
    }

    #[test]
    fn the_package_and_the_window_carry_the_same_names() {
        // The gate has no dependencies, so the two ids are written here and in the interface
        // crate. The update check finds a release's files by the release id, and a Linux desktop
        // joins a window to its menu entry by the app id, so each pair must be the same.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let package = APP.package.unwrap();
        let app = std::fs::read_to_string(root.join("crates/script-ui/src/app.rs")).unwrap();
        assert!(app.contains(&format!("pub const ID: &str = \"{}\";", package.id)), "the update source's id is not {}", package.id);
        assert!(app.contains(&format!("pub const APP_ID: &str = \"{}\";", package.app_id)), "the window's app id is not {}", package.app_id);
        // The desktop program is named by the release id, the command line `<id>-cli`.
        assert_eq!(APP.shipped, [package.id, "wordsmith-script-cli"]);
        for (program, folder) in [(package.id, "apps/script"), ("wordsmith-script-cli", "apps/script-cli")] {
            let manifest = std::fs::read_to_string(root.join(folder).join("Cargo.toml")).unwrap();
            assert!(manifest.contains(&format!("name = \"{program}\"\n")), "{folder} is not {program}");
        }
    }

    #[test]
    fn script_owns_its_format_and_is_only_offered_for_fountain() {
        let types = APP.package.unwrap().file_types;
        let owned: Vec<&str> = types.iter().filter(|t| t.own).flat_map(|t| t.extensions.iter().copied()).collect();
        assert_eq!(owned, ["wss"]);
        let offered: Vec<&str> = types.iter().filter(|t| !t.own).flat_map(|t| t.extensions.iter().copied()).collect();
        assert_eq!(offered, ["fountain", "spmd"]);
    }

    #[test]
    fn no_indexing_crates_are_workspace_members() {
        let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml")).unwrap();
        for name in NO_INDEXING_CRATES {
            assert!(manifest.contains(&format!("\n{name} = {{ path = \"crates/")), "{name} is not a crate of this workspace");
        }
    }
}
