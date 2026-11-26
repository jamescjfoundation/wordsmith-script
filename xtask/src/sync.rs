// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask sync`: keep an app repository's shared settings identical to the core's.
//!
//! Every app carries the same toolchain, lint, cargo-deny and license settings, the same shared
//! xtask modules, the same workflows and an exact copy of the core's `[patch]` block. This module
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
    ("template/xtask/src/corepin.rs", "xtask/src/corepin.rs"),
    ("template/.github/workflows/ci.yml", ".github/workflows/ci.yml"),
    ("template/.github/workflows/core-pin.yml", ".github/workflows/core-pin.yml"),
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

/// The core checkout an app builds with: the folder Cargo fetched the pinned `jcj-` crates from.
pub fn locate_core(app: &Path) -> Result<PathBuf, String> {
    let out = cargo_output(app, &["metadata", "--format-version", "1", "--locked"])?;
    choose_core(&json::parse(&out)?)
}

/// The core checkout in `cargo metadata` output. The graph may hold more than one core release
/// when an app depends on another app's crates that pin an older core; the app's own pin is the
/// release its workspace members depend on directly (the newest, should they name several).
pub fn choose_core(meta: &json::Value) -> Result<PathBuf, String> {
    let members: Vec<&str> =
        meta.get("workspace_members").map(json::Value::items).unwrap_or_default().iter().filter_map(json::Value::as_str).collect();
    let mut direct: Vec<&str> = Vec::new();
    for node in meta.get("resolve").and_then(|r| r.get("nodes")).map(json::Value::items).unwrap_or_default() {
        if node.str_of("id").is_some_and(|id| members.contains(&id)) {
            direct.extend(node.get("deps").map(json::Value::items).unwrap_or_default().iter().filter_map(|d| d.str_of("pkg")));
        }
    }
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for pass_direct_only in [true, false] {
        for p in meta.get("packages").map(json::Value::items).unwrap_or_default() {
            let Some(name) = p.str_of("name") else { continue };
            if !name.starts_with("jcj-") || name == "jcj-upstream" {
                continue;
            }
            if pass_direct_only && !p.str_of("id").is_some_and(|id| direct.contains(&id)) {
                continue;
            }
            let manifest = PathBuf::from(p.str_of("manifest_path").ok_or_else(|| format!("{name} has no manifest_path"))?);
            // <core>/crates/<name>/Cargo.toml
            let Some(core) = manifest.ancestors().nth(3).map(Path::to_path_buf) else { continue };
            let version: Vec<u64> = p.str_of("version").unwrap_or("0").split(['.', '-', '+']).map(|n| n.parse().unwrap_or(0)).collect();
            if best.as_ref().is_none_or(|(v, _)| version > *v) {
                best = Some((version, core));
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, core)| core).ok_or_else(|| "this workspace does not depend on open-source-core (no jcj- package)".to_owned())
}

/// What the template's `main.rs` runs that an app's own `main.rs`, started from an older template,
/// may not run yet: the call to look for, and what it is. A managed module that nothing calls
/// checks nothing, and `main.rs` is the app's own, so `sync` can only say so.
const RUN_FROM_MAIN: &[(&str, &str)] = &[("corepin::task", "the core-pin check")];

/// The entries of [`RUN_FROM_MAIN`] an app's `xtask/src/main.rs` does not call.
pub fn not_run(main: &str) -> Vec<String> {
    let missing = RUN_FROM_MAIN.iter().filter(|(call, _)| !main.contains(call));
    missing.map(|(call, what)| format!("xtask/src/main.rs does not run {what} (`{call}`); the core's CHANGELOG lists the lines to add")).collect()
}

/// `cargo xtask sync [--check]` in an app: against the pinned core release.
pub fn app_task(app: &Path, check: bool) -> TaskResult {
    let core = locate_core(app)?;
    for note in not_run(&std::fs::read_to_string(app.join("xtask/src/main.rs")).unwrap_or_default()) {
        println!("note: {note}");
    }
    if check {
        return report(differences(&core, app)?, "shared settings match the pinned core release");
    }
    write(&core, app)?;
    // This program was built from the app's sync.rs as it was, so a file the new list adds is not
    // written until the next run.
    if std::fs::read(core.join("template/xtask/src/sync.rs")).is_ok_and(|new| new != include_bytes!("sync.rs")) {
        println!("note: sync.rs itself changed, and its list of managed files may have grown: run `cargo xtask sync` again");
    }
    Ok(())
}

/// `cargo xtask sync <app dir> [--check]` in the core.
pub fn core_task(core: &Path, app: &str, check: bool) -> TaskResult {
    let app = PathBuf::from(app);
    if check { report(differences(core, &app)?, "the app's shared settings match this core") } else { write(core, &app) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(packages: &[(&str, &str, &str, &str)], direct: &[&str]) -> json::Value {
        let pkgs: Vec<String> = packages
            .iter()
            .map(|(id, name, version, path)| format!(r#"{{"id":"{id}","name":"{name}","version":"{version}","manifest_path":"{path}"}}"#))
            .collect();
        let deps: Vec<String> = direct.iter().map(|d| format!(r#"{{"pkg":"{d}"}}"#)).collect();
        let text = format!(
            r#"{{"packages":[{}],"workspace_members":["app"],"resolve":{{"nodes":[{{"id":"app","deps":[{}]}},{{"id":"other","deps":[]}}]}}}}"#,
            pkgs.join(","),
            deps.join(",")
        );
        json::parse(&text).unwrap()
    }

    #[test]
    fn the_apps_own_core_wins_over_an_older_one_in_the_graph() {
        let packages = [
            ("old-doc", "jcj-doc", "0.1.4", "/c/old/crates/jcj-doc/Cargo.toml"),
            ("new-doc", "jcj-doc", "0.1.5", "/c/new/crates/jcj-doc/Cargo.toml"),
            ("new-shell", "jcj-shell", "0.1.5", "/c/new/crates/jcj-shell/Cargo.toml"),
        ];
        assert_eq!(choose_core(&meta(&packages, &["new-shell"])).unwrap(), PathBuf::from("/c/new"));
        // Only the older core is a direct dependency: it is the app's pin.
        assert_eq!(choose_core(&meta(&packages, &["old-doc"])).unwrap(), PathBuf::from("/c/old"));
        // No direct jcj- dependency: the newest in the graph.
        assert_eq!(choose_core(&meta(&packages, &[])).unwrap(), PathBuf::from("/c/new"));
        assert!(choose_core(&meta(&[], &[])).is_err());
    }

    #[test]
    fn says_what_an_older_main_does_not_run() {
        assert!(not_run("\"core-pin\" => corepin::task(&root, rest),").is_empty());
        let notes = not_run("mod sync;");
        assert_eq!(notes.len(), RUN_FROM_MAIN.len());
        assert!(notes[0].contains("the core-pin check"));
    }
}
