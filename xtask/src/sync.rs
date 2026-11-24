// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask sync`: keep an app repository's shared settings identical to the core's.
//!
//! Every app carries the same toolchain, lint, cargo-deny and license settings, the same shared
//! xtask modules, the same CI workflow and an exact copy of the core's `[patch]` block. This module
//! copies them from a core checkout into an app (`sync`) or reports any difference (`--check`, run
//! by the app's CI).
//!
//! In an app, the core checkout is the one Cargo already fetched for the pinned core release
//! (found through `cargo metadata`), so `cargo xtask sync --check` compares against exactly the
//! release the app builds with. From the core, `cargo xtask sync <app dir>` writes into an app.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::{Path, PathBuf};

use crate::json;
use crate::link::{BLOCK_BEGIN, BLOCK_END, extract_block, replace_block};
use crate::util::{TaskResult, cargo_output, report};

/// (path in the core, path in an app). Apps must not edit these files; change them in the core.
pub const MANAGED: &[(&str, &str)] = &[
    ("rust-toolchain.toml", "rust-toolchain.toml"),
    ("rustfmt.toml", "rustfmt.toml"),
    ("clippy.toml", "clippy.toml"),
    ("deny.toml", "deny.toml"),
    ("template/xtask/src/util.rs", "xtask/src/util.rs"),
    ("template/xtask/src/lintroots.rs", "xtask/src/lintroots.rs"),
    ("template/xtask/src/privacy.rs", "xtask/src/privacy.rs"),
    ("template/xtask/src/link.rs", "xtask/src/link.rs"),
    ("template/xtask/src/sync.rs", "xtask/src/sync.rs"),
    ("template/xtask/src/spdx.rs", "xtask/src/spdx.rs"),
    ("template/xtask/src/json.rs", "xtask/src/json.rs"),
    ("template/xtask/src/licenses.rs", "xtask/src/licenses.rs"),
    ("template/xtask/src/changed.rs", "xtask/src/changed.rs"),
    ("template/.github/workflows/ci.yml", ".github/workflows/ci.yml"),
    ("template/hooks/pre-commit", "hooks/pre-commit"),
    ("template/hooks/pre-push", "hooks/pre-push"),
];

/// What differs between the core's managed files and an app's.
pub fn differences(core: &Path, app: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (from, to) in MANAGED {
        let want = std::fs::read(core.join(from)).map_err(|e| format!("core file {from}: {e}"))?;
        match std::fs::read(app.join(to)) {
            Ok(have) if have == want => {}
            Ok(_) => out.push(format!("{to} differs from the core's {from}")),
            Err(_) => out.push(format!("{to} is missing (the core's {from})")),
        }
    }
    let core_manifest = std::fs::read_to_string(core.join("Cargo.toml")).map_err(|e| format!("core Cargo.toml: {e}"))?;
    let block = extract_block(&core_manifest, BLOCK_BEGIN, BLOCK_END).ok_or("the core's Cargo.toml has no patch block")?;
    let app_manifest = std::fs::read_to_string(app.join("Cargo.toml")).map_err(|e| format!("app Cargo.toml: {e}"))?;
    match extract_block(&app_manifest, BLOCK_BEGIN, BLOCK_END) {
        Some(b) if b == block => {}
        Some(_) => out.push("Cargo.toml: the patch block differs from the core's".to_owned()),
        None => out.push("Cargo.toml: the core's patch block is missing".to_owned()),
    }
    Ok(out)
}

/// Write every managed file and the patch block from `core` into `app`.
pub fn write(core: &Path, app: &Path) -> TaskResult {
    for (from, to) in MANAGED {
        let bytes = std::fs::read(core.join(from)).map_err(|e| format!("core file {from}: {e}"))?;
        let dest = app.join(to);
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    let core_manifest = std::fs::read_to_string(core.join("Cargo.toml")).map_err(|e| format!("core Cargo.toml: {e}"))?;
    let block = extract_block(&core_manifest, BLOCK_BEGIN, BLOCK_END).ok_or("the core's Cargo.toml has no patch block")?;
    let path = app.join("Cargo.toml");
    let app_manifest = std::fs::read_to_string(&path).map_err(|e| format!("app Cargo.toml: {e}"))?;
    std::fs::write(&path, replace_block(&app_manifest, &block, BLOCK_BEGIN, BLOCK_END)).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("synced {} managed files and the patch block into {}", MANAGED.len(), app.display());
    Ok(())
}

/// The core checkout an app builds with: the folder Cargo fetched the pinned `jcj-doc` from.
pub fn locate_core(app: &Path) -> Result<PathBuf, String> {
    let out = cargo_output(app, &["metadata", "--format-version", "1", "--locked"])?;
    let meta = json::parse(&out)?;
    for p in meta.get("packages").map(json::Value::items).unwrap_or_default() {
        if p.str_of("name") == Some("jcj-doc") {
            let manifest = PathBuf::from(p.str_of("manifest_path").ok_or("jcj-doc has no manifest_path")?);
            // <core>/crates/jcj-doc/Cargo.toml
            return manifest.ancestors().nth(3).map(Path::to_path_buf).ok_or_else(|| "cannot find the core above jcj-doc".to_owned());
        }
    }
    Err("this workspace does not depend on open-source-core (no jcj-doc package)".to_owned())
}

/// `cargo xtask sync [--check]` in an app: against the pinned core release.
pub fn app_task(app: &Path, check: bool) -> TaskResult {
    let core = locate_core(app)?;
    if check { report(differences(&core, app)?, "shared settings match the pinned core release") } else { write(&core, app) }
}

/// `cargo xtask sync <app dir> [--check]` in the core.
pub fn core_task(core: &Path, app: &str, check: bool) -> TaskResult {
    let app = PathBuf::from(app);
    if check { report(differences(core, &app)?, "the app's shared settings match this core") } else { write(core, &app) }
}
