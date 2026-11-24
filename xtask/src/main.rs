// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for a foundation app built on open-source-core.
//!
//! Started from the core's `template/`. This file is the app's own (add app tasks here); the other
//! modules in this folder are owned by the core and kept identical by `cargo xtask sync`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod changed;
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

/// The programs this app ships (for THIRD-PARTY-LICENSES.txt). Change to the app's binaries.
const SHIPPED: &[&str] = &["app"];
/// The product name used in generated files. Change to the app's name.
const PRODUCT: &str = "This app";

const TASKS: &[(&str, &str)] = &[
    ("fmt", "rustfmt in check mode"),
    ("lint-roots", "every crate root forbids unsafe and denies the panicking lints"),
    ("unlinked", "no local core dev-link is left in .cargo/config.toml"),
    ("sync", "shared settings and the patch block match the pinned core release"),
    ("layers", "only jcj- interfaces of the core are used, never upstream crates or jcj-upstream"),
    ("privacy", "no personal details or secrets in files or commit identities"),
    ("licenses", "every dependency's license is on the allowlist"),
    ("clippy", "clippy on all targets, warnings are errors"),
    ("test", "all tests"),
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
        "deny" => cargo(&root, &["deny", "--locked", "check"]),
        "install-hooks" => util::git_output(&root, &["config", "core.hooksPath", "hooks"]).map(|_| println!("git hooks on")),
        "third-party-licenses" => licenses::task(&root, SHIPPED, PRODUCT, rest.first().map(String::as_str)),
        "privacy" => privacy::task(&root, rest),
        name => match TASKS.iter().find(|(n, _)| *n == name) {
            Some((n, _)) => run_task(n, &root, &Affected::All),
            None => {
                println!("usage: cargo xtask <ci [--changed] [--base <ref>] | dev-link <core> | dev-unlink | sync [--check] | deny | install-hooks>");
                for (name, about) in TASKS {
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
        "lint-roots" => lintroots::check_lint_roots(root, &[], &[]),
        "unlinked" => link::check_unlinked(root),
        "sync" => sync::app_task(root, true),
        "layers" => check_layers(root),
        "privacy" => privacy::task(root, &[]),
        "licenses" => check_licenses(root),
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
