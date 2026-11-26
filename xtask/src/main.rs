// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for Wordsmith Script, built on open-source-core.
//!
//! Started from the core's `template/`. This file and `bundle.rs` are the app's own; the other
//! modules in this folder are owned by the core and kept identical by `cargo xtask sync`.
//!
//! `cargo xtask ci` runs the core's gate plus the app's `assets` check, and `lint-roots` also
//! requires `clippy::indexing_slicing` in the crates that may not index. Outside the gate:
//! `bundle` (a release bundle for this computer), `screenshots` (the README's pictures) and
//! `wasm` (the web build).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod bundle;
mod changed;
mod corepin;
mod docs;
mod json;
mod licenses;
mod link;
mod lintroots;
mod privacy;
mod spdx;
// `core_task` is the core's side of sync.
#[allow(dead_code)]
mod sync;
mod util;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use changed::Affected;
use util::{TaskResult, cargo, cargo_output, report, workspace_members};

/// The programs this app ships (for THIRD-PARTY-LICENSES.txt).
const SHIPPED: &[&str] = &["wordsmith", "wordsmith-cli"];
/// The product name used in generated files.
const PRODUCT: &str = "Wordsmith Script";

/// Crates whose product code may not index or slice (`clippy.toml`): their roots must also deny
/// `clippy::indexing_slicing`.
const NO_INDEXING_CRATES: &[&str] = &["wordsmith-script", "wordsmith-format"];

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const WASM_CRATE: &str = "wordsmith-web";

/// Files the repository must carry (the `assets` check).
const LEGAL_FILES: &[&str] = &["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md", "UPSTREAM.md"];

/// Bounds for walking `assets/`, which is treated like any other input.
const MAX_ASSET_DEPTH: usize = 12;
const MAX_ASSET_FILES: usize = 10_000;

const TASKS: &[(&str, &str)] = &[
    ("fmt", "rustfmt in check mode"),
    ("lint-roots", "every crate root forbids unsafe and denies the panicking lints (and indexing where barred)"),
    ("unlinked", "no local core dev-link is left in .cargo/config.toml"),
    ("core-pin", "every jcj- dependency names one core release tag, and Cargo.lock holds one core"),
    ("sync", "shared settings and the patch block match the pinned core release"),
    ("docs", "links in Markdown lead somewhere, decision references name a record, core-candidate markers name an issue"),
    ("layers", "only jcj- interfaces of the core are used, never upstream crates or jcj-upstream"),
    ("privacy", "no personal details or secrets in files or commit identities"),
    ("licenses", "every dependency's license is on the allowlist"),
    ("assets", "every file under assets/ is listed in ATTRIBUTION.md; the legal files exist"),
    ("clippy", "clippy on all targets, warnings are errors"),
    ("test", "all tests"),
];

/// The app's tasks that `ci` does not run.
const OTHER_TASKS: &[(&str, &str)] = &[
    ("bundle", "build a release bundle for this computer in dist/ (nothing is published)"),
    ("screenshots", "redraw docs/images with the app's offscreen renderer"),
    ("wasm", "build wordsmith-web for wasm32-unknown-unknown"),
    ("third-party-licenses", "[file] write the dependency license texts (default dist/THIRD-PARTY-LICENSES.txt)"),
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    let rest = args.get(1..).unwrap_or_default();
    let root = workspace_root();
    let result = match task {
        "ci" => ci(&root, rest),
        "dev-link" => match rest.first() {
            Some(a) if a == "--check" => link::check_unlinked(&root),
            Some(core) => link::dev_link(&root, core),
            None => Err("usage: cargo xtask dev-link <path to open-source-core>".to_owned()),
        },
        "dev-unlink" => link::dev_unlink(&root),
        "sync" => sync::app_task(&root, rest.iter().any(|a| a == "--check")),
        "core-pin" => corepin::task(&root, rest),
        "affected" => changed::task(&root, rest),
        "deny" => cargo(&root, &["deny", "--locked", "check"]),
        "install-hooks" => util::git_output(&root, &["config", "core.hooksPath", "hooks"]).map(|_| println!("git hooks on")),
        "third-party-licenses" => licenses::task(&root, SHIPPED, PRODUCT, rest.first().map(String::as_str)),
        "privacy" => privacy::task(&root, rest),
        "bundle" => bundle::bundle(&root),
        "screenshots" => screenshots(&root),
        "wasm" => wasm(&root),
        name => match TASKS.iter().find(|(n, _)| *n == name) {
            Some((n, _)) => run_task(n, &root, &Affected::All),
            None => {
                println!("usage: cargo xtask <ci [--changed] [--base <ref>] | dev-link <core> | dev-unlink | sync [--check] | deny | install-hooks>");
                println!("\nchecks (`ci` runs them all, in this order; --skip <task> leaves one out):");
                for (name, about) in TASKS {
                    println!("  {name:<12} {about}");
                }
                println!("\nother tasks:");
                for (name, about) in OTHER_TASKS {
                    println!("  {name:<12} {about}");
                }
                if name == "help" { Ok(()) } else { Err(format!("unknown task '{name}'")) }
            }
        },
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("\nxtask: FAILED: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

fn ci(root: &Path, args: &[String]) -> TaskResult {
    let names: Vec<&str> = TASKS.iter().map(|(n, _)| *n).collect();
    let mut skip_args = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--changed" => {}
            "--base" => {
                it.next();
            }
            _ => skip_args.push(a.clone()),
        }
    }
    let skip = util::skipped(&skip_args, &names)?;
    util::warn_if_hooks_off(root, "hooks");
    let scope = if args.iter().any(|a| a == "--changed") {
        let base = args.iter().position(|a| a == "--base").and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| "origin/main".to_owned());
        changed::affected(root, &base)?
    } else {
        Affected::All
    };
    for (name, _) in TASKS {
        if skip.iter().any(|s| s == name) {
            continue;
        }
        run_task(name, root, &scope)?;
    }
    println!("\nxtask: all checks passed");
    Ok(())
}

fn run_task(name: &str, root: &Path, scope: &Affected) -> TaskResult {
    println!("\n==> {name}");
    let result = match name {
        "fmt" => cargo(root, &["fmt", "--all", "--", "--check"]),
        "lint-roots" => lintroots::check_lint_roots(root, NO_INDEXING_CRATES, &[]),
        "unlinked" => link::check_unlinked(root),
        "core-pin" => corepin::task(root, &[]),
        "sync" => sync::app_task(root, true),
        "docs" => docs::app_task(root),
        "layers" => check_layers(root),
        "privacy" => privacy::task(root, &[]),
        "licenses" => check_licenses(root),
        "assets" => check_assets(root),
        "clippy" => scoped(root, scope, &["clippy", "--locked", "--all-targets"], &["--", "-D", "warnings"]),
        "test" => scoped(root, scope, &["test", "--locked"], &[]),
        other => Err(format!("no runner for task '{other}'")),
    };
    result.map_err(|e| format!("{name}: {e}"))
}

fn scoped(root: &Path, scope: &Affected, head: &[&str], tail: &[&str]) -> TaskResult {
    if scope.is_empty() {
        println!("nothing to do: no package affected");
        return Ok(());
    }
    let pkgs = scope.cargo_args();
    let mut args: Vec<&str> = head.to_vec();
    args.extend(pkgs.iter().map(String::as_str));
    args.extend(tail);
    cargo(root, &args)
}

/// Apps reach upstream only through the core's `jcj-` interfaces (core plan, "Things agents must
/// not do").
fn check_layers(root: &Path) -> TaskResult {
    let members = workspace_members(root)?;
    let names: BTreeSet<&str> = members.iter().map(|m| m.name.as_str()).collect();
    let mut failures = Vec::new();
    for member in &members {
        if member.name == "xtask" {
            continue;
        }
        for dep in util::direct_deps(root, &member.name)? {
            if names.contains(dep.as_str()) {
                continue;
            }
            if dep.starts_with("wordcraft-") || dep.starts_with("craft-") || dep == "jcj-upstream" {
                failures.push(format!("{} depends on {dep}; use the core's jcj- interfaces", member.name));
            }
        }
    }
    report(failures, "the app uses only the core's jcj- interfaces")
}

fn check_licenses(root: &Path) -> TaskResult {
    let out = cargo_output(
        root,
        &["tree", "--workspace", "--locked", "--target", "all", "-e", "normal,build,dev", "--prefix", "none", "--format", "{p}\t{l}"],
    )?;
    let mut failures = Vec::new();
    let mut checked = BTreeSet::new();
    for line in out.lines().filter(|l| !l.trim().is_empty()) {
        let line = line.trim().trim_end_matches("(*)").trim_end();
        let Some((pkg, license)) = line.split_once('\t') else { continue };
        if !checked.insert(pkg.to_owned()) {
            continue;
        }
        match spdx::is_allowed(license.trim()) {
            Ok(true) => {}
            Ok(false) => failures.push(format!("{pkg}: license '{license}' is not on the allowlist")),
            Err(e) => failures.push(format!("{pkg}: {e}")),
        }
    }
    report(failures, "licenses are on the allowlist")
}

/// Redraw the README's pictures (docs/images) from the sample script.
fn screenshots(root: &Path) -> TaskResult {
    let out = root.join("docs/images");
    let out = out.to_str().ok_or_else(|| format!("{} is not UTF-8", out.display()))?;
    cargo(root, &["run", "--release", "--locked", "-p", "wordsmith-ui", "--example", "screenshots", "--", out])
}

/// The web build. Not part of `ci`.
fn wasm(root: &Path) -> TaskResult {
    cargo(root, &["build", "--locked", "-p", WASM_CRATE, "--target", WASM_TARGET])
        .map_err(|e| format!("{e} (is the target installed? rustup target add {WASM_TARGET})"))
}

/// Every file under `assets/` is named in ATTRIBUTION.md, and the legal files exist.
fn check_assets(root: &Path) -> TaskResult {
    let (failures, listed) = asset_problems(root)?;
    println!("checked {listed} asset files against ATTRIBUTION.md");
    report(failures, "assets are attributed and the legal files exist")
}

/// The problems the `assets` check finds, and how many asset files it checked.
fn asset_problems(root: &Path) -> Result<(Vec<String>, usize), String> {
    let mut failures = Vec::new();
    let attribution = std::fs::read_to_string(root.join("ATTRIBUTION.md")).map_err(|e| format!("cannot read ATTRIBUTION.md: {e}"))?;
    let mut assets = Vec::new();
    collect_files(&root.join("assets"), 0, &mut assets)?;
    let mut listed = 0usize;
    for asset in &assets {
        let rel = asset.strip_prefix(root).unwrap_or(asset);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if is_ignored_file(&rel) {
            continue;
        }
        listed += 1;
        if !attribution.contains(&rel) {
            failures.push(format!("{rel}: asset not listed in ATTRIBUTION.md"));
        }
    }
    for file in LEGAL_FILES {
        if !root.join(file).is_file() {
            failures.push(format!("{file} is missing"));
        }
    }
    Ok((failures, listed))
}

fn is_ignored_file(rel: &str) -> bool {
    rel.ends_with("/.gitkeep") || rel.ends_with("/.DS_Store")
}

fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> TaskResult {
    if depth > MAX_ASSET_DEPTH {
        return Err(format!("{} is nested too deeply", dir.display()));
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if out.len() >= MAX_ASSET_FILES {
            return Err(format!("more than {MAX_ASSET_FILES} files under assets/"));
        }
        // Do not follow symlinks out of the tree.
        let is_link = std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink());
        if is_link {
            return Err(format!("{} is a symlink; assets must be real files", path.display()));
        }
        if path.is_dir() {
            collect_files(&path, depth + 1, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn task_lists_are_complete_and_apart() {
        let names: Vec<&str> = TASKS.iter().map(|(n, _)| *n).collect();
        for app_check in ["assets", "lint-roots"] {
            assert!(names.contains(&app_check), "{app_check} is not in ci");
        }
        assert!(!names.contains(&"ci"));
        // Release and slow builds stay out of the gate.
        for (name, _) in OTHER_TASKS {
            assert!(!names.contains(name), "{name} is both a ci check and another task");
        }
        assert!(OTHER_TASKS.iter().any(|(n, _)| *n == "wasm"));
        assert_eq!(SHIPPED, ["wordsmith", "wordsmith-cli"]);
    }

    #[test]
    fn no_indexing_crates_are_workspace_members() {
        let manifest = std::fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
        for name in NO_INDEXING_CRATES {
            assert!(manifest.contains(&format!("\"crates/{name}\"")), "{name} is not a workspace member");
        }
    }

    #[test]
    fn every_asset_must_be_attributed() {
        let root = temp_root("assets");
        for file in LEGAL_FILES {
            write(&root.join(file), file);
        }
        write(&root.join("ATTRIBUTION.md"), "| `assets/fonts/A.ttf` | a font |\n");
        write(&root.join("assets/fonts/A.ttf"), "");
        write(&root.join("assets/.gitkeep"), "");
        assert_eq!(asset_problems(&root).unwrap(), (vec![], 1));

        write(&root.join("assets/brand/icon.png"), "");
        let (problems, listed) = asset_problems(&root).unwrap();
        assert_eq!(listed, 2);
        assert_eq!(problems, ["assets/brand/icon.png: asset not listed in ATTRIBUTION.md"]);

        std::fs::remove_file(root.join("NOTICE")).unwrap();
        assert!(asset_problems(&root).unwrap().0.contains(&"NOTICE is missing".to_owned()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn assets_may_not_be_symlinks() {
        let root = temp_root("assets-link");
        write(&root.join("ATTRIBUTION.md"), "");
        write(&root.join("outside.txt"), "");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::os::unix::fs::symlink(root.join("outside.txt"), root.join("assets/link.txt")).unwrap();
        assert!(asset_problems(&root).unwrap_err().contains("symlink"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
