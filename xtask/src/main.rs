// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for Wordsmith Script, built on open-source-core.
//!
//! This file and `bundle.rs` are the app's own. They hold only what is Script's: its data (`APP`)
//! and its own tasks (`TASKS`). The gate, every shared task and the list of them are in the
//! modules under `shared`, which the core owns and `cargo xtask sync` keeps identical in every
//! app; `cargo xtask help` lists them.
//!
//! Script's part of the gate is data: `lint-roots` also requires `clippy::indexing_slicing` in
//! the crates that may not index, and `assets` also wants `UPSTREAM.md`. Outside the gate:
//! `bundle` (a release bundle for this computer), `wasm` (the web build) and `screenshots` (the
//! README's pictures).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod bundle;
mod shared;

use std::path::Path;
use std::process::ExitCode;

use shared::gate::{self, App, Task};
use shared::util::{TaskResult, cargo};

/// Crates whose product code may not index or slice (`clippy.toml`): their roots must also deny
/// `clippy::indexing_slicing`.
const NO_INDEXING_CRATES: &[&str] = &["wordsmith-script-model", "wordsmith-script-format"];

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const WASM_CRATE: &str = "wordsmith-script-web";

/// What the shared tasks need to know about Script.
const APP: App = App {
    product: "Wordsmith Script",
    // The programs Script ships (for THIRD-PARTY-LICENSES.txt).
    shipped: &["wordsmith-script", "wordsmith-script-cli"],
    no_indexing: NO_INDEXING_CRATES,
    // Beyond the legal files every app carries (the `assets` check).
    legal_files: &["UPSTREAM.md"],
    // `cargo xtask screenshots` redraws docs/images from the sample script.
    screenshots: Some("wordsmith-script-ui"),
    tasks: TASKS,
    ..App::DEFAULT
};

/// Script's own tasks. Neither is part of the gate: release and slow builds stay out of it.
const TASKS: &[Task] = &[
    Task::other("bundle", "build a release bundle for this computer in dist/ (nothing is published)", |root, _| bundle::bundle(root)),
    Task::other("wasm", "build wordsmith-script-web for wasm32-unknown-unknown", |root, _| wasm(root)),
];

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
        for slow in ["bundle", "wasm", "screenshots"] {
            assert!(!in_gate.contains(&slow), "{slow} is in ci");
        }
        assert_eq!(APP.shipped, ["wordsmith-script", "wordsmith-script-cli"]);
    }

    #[test]
    fn no_indexing_crates_are_workspace_members() {
        let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml")).unwrap();
        for name in NO_INDEXING_CRATES {
            assert!(manifest.contains(&format!("\n{name} = {{ path = \"crates/")), "{name} is not a crate of this workspace");
        }
    }
}
