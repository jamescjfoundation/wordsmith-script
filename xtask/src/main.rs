// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>`: the Wordsmith Script CI gate.
//!
//! `cargo xtask ci` runs, in order: rustfmt, the crate-root lint check,
//! clippy, all tests, the attribution check, the layering check and the
//! wasm build. Each task can also be run on its own. Outside the gate:
//! `cargo xtask screenshots` redraws the README's pictures with the app's
//! offscreen renderer, and `cargo xtask bundle` builds a release bundle for
//! the host.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod bundle;
mod layers;
mod spdx;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

type TaskResult = Result<(), String>;

const TASKS: &[(&str, &str)] = &[
    ("fmt", "rustfmt in check mode"),
    ("lint-roots", "every product crate root denies panicking lints and unsafe"),
    ("clippy", "clippy on all targets, warnings are errors"),
    ("test", "all tests in the workspace"),
    ("attribution", "dependency licenses on the allowlist; assets listed in ATTRIBUTION.md"),
    ("layering", "workspace dependencies only point down; nothing below wordsmith-ui uses it"),
    ("wasm", "build wordsmith-web for wasm32-unknown-unknown"),
];

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const WASM_CRATE: &str = "wordsmith-web";

/// Lints every product crate root must deny, per the ground rules.
const REQUIRED_DENIES: &[&str] =
    &["clippy::unwrap_used", "clippy::expect_used", "clippy::panic", "clippy::unimplemented", "clippy::todo", "clippy::unreachable"];

/// Bounds for walking `assets/`, which is treated like any other input.
const MAX_ASSET_DEPTH: usize = 12;
const MAX_ASSET_FILES: usize = 10_000;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    let root = workspace_root();
    let result = match task {
        "ci" => run_all(&root),
        "screenshots" => screenshots(&root),
        "bundle" => bundle::bundle(&root),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        name => match TASKS.iter().find(|(n, _)| *n == name) {
            Some((n, _)) => run_task(n, &root),
            None => {
                print_help();
                Err(format!("unknown task '{name}'"))
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

fn print_help() {
    println!("usage: cargo xtask <task>\n\n  ci             run every task below, in order");
    for (name, about) in TASKS {
        println!("  {name:<14} {about}");
    }
    println!("\n  screenshots    redraw docs/images with the app's offscreen renderer");
    println!("  bundle         build a release bundle for this computer in dist/ (nothing is published)");
}

/// Redraw the README's pictures (docs/images) from the sample script.
fn screenshots(root: &Path) -> TaskResult {
    let out = root.join("docs/images");
    let status = Command::new(cargo_bin())
        .current_dir(root)
        .args(["run", "--release", "--locked", "-p", "wordsmith-ui", "--example", "screenshots", "--"])
        .arg(&out)
        .status()
        .map_err(|e| format!("cargo run: {e}"))?;
    if status.success() { Ok(()) } else { Err("the screenshots example failed".into()) }
}

fn run_all(root: &Path) -> TaskResult {
    for (name, _) in TASKS {
        run_task(name, root)?;
    }
    println!("\nxtask: all checks passed");
    Ok(())
}

fn run_task(name: &str, root: &Path) -> TaskResult {
    println!("\n==> {name}");
    let result = match name {
        "fmt" => cargo(root, &["fmt", "--all", "--", "--check"]),
        "lint-roots" => check_lint_roots(root),
        "clippy" => cargo(root, &["clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
        "test" => cargo(root, &["test", "--workspace", "--locked"]),
        "attribution" => check_attribution(root),
        "layering" => check_layering(root),
        "wasm" => cargo(root, &["build", "--locked", "-p", WASM_CRATE, "--target", WASM_TARGET])
            .map_err(|e| format!("{e} (is the target installed? rustup target add {WASM_TARGET})")),
        other => Err(format!("no runner for task '{other}'")),
    };
    result.map_err(|e| format!("{name}: {e}"))
}

fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    match manifest.parent() {
        Some(p) => p.to_path_buf(),
        None => manifest.to_path_buf(),
    }
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn cargo(root: &Path, args: &[&str]) -> TaskResult {
    let status = Command::new(cargo_bin()).args(args).current_dir(root).status().map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("`cargo {}` exited with {status}", args.join(" "))) }
}

fn cargo_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cargo_bin()).args(args).current_dir(root).output().map_err(|e| format!("could not run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!("`cargo {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("cargo printed non-UTF-8 output: {e}"))
}

/// A workspace member: package name and directory.
struct Member {
    name: String,
    dir: PathBuf,
}

/// Parses a `cargo tree` package line: `name vX.Y.Z (/path)` with an
/// optional trailing ` (*)` marker. Returns the name and the path if any.
fn parse_pkg(line: &str) -> Option<(String, Option<PathBuf>)> {
    let line = line.trim().trim_end_matches("(*)").trim_end();
    let mut parts = line.splitn(3, ' ');
    let name = parts.next()?.to_owned();
    let _version = parts.next()?;
    let path = parts
        .next()
        .and_then(|rest| rest.strip_prefix('('))
        .and_then(|rest| rest.strip_suffix(')'))
        .filter(|p| p.starts_with('/') || p.contains(":\\") || p.starts_with("\\\\"))
        .map(PathBuf::from);
    Some((name, path))
}

fn workspace_members(root: &Path) -> Result<Vec<Member>, String> {
    let out = cargo_output(root, &["tree", "--workspace", "--locked", "--depth", "0", "--prefix", "none", "--format", "{p}"])?;
    let mut members = Vec::new();
    let mut seen = BTreeSet::new();
    for line in out.lines().filter(|l| !l.trim().is_empty()) {
        let (name, path) = parse_pkg(line).ok_or_else(|| format!("unexpected cargo tree line: {line}"))?;
        let dir = path.ok_or_else(|| format!("workspace member without a path: {line}"))?;
        if seen.insert(name.clone()) {
            members.push(Member { name, dir });
        }
    }
    if members.is_empty() {
        return Err("cargo tree listed no workspace members".to_owned());
    }
    Ok(members)
}

fn crate_roots(dir: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ["src/lib.rs", "src/main.rs"].iter().map(|p| dir.join(p)).filter(|p| p.is_file()).collect();
    if let Ok(entries) = std::fs::read_dir(dir.join("src/bin")) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "rs") {
                roots.push(p);
            } else if p.join("main.rs").is_file() {
                roots.push(p.join("main.rs"));
            }
        }
    }
    roots
}

/// Problems with one crate root's lint attributes, empty if it is fine.
fn lint_root_problems(source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    // Only look at inner attributes, which must come before any item.
    let compact: String = source.split_whitespace().collect();
    if !compact.contains("#![forbid(unsafe_code)]") {
        problems.push("missing #![forbid(unsafe_code)]".to_owned());
    }
    let deny_blocks: Vec<&str> = compact
        .match_indices("#![deny(")
        .filter_map(|(i, _)| {
            let rest = compact.get(i..)?;
            let end = rest.find(")]")?;
            rest.get(..end)
        })
        .collect();
    for lint in REQUIRED_DENIES {
        if !deny_blocks.iter().any(|b| b.contains(lint)) {
            problems.push(format!("missing #![deny({lint})]"));
        }
    }
    problems
}

fn check_lint_roots(root: &Path) -> TaskResult {
    let mut failures = Vec::new();
    for member in workspace_members(root)? {
        let roots = crate_roots(&member.dir);
        if roots.is_empty() {
            failures.push(format!("{}: no crate root found", member.name));
        }
        for file in roots {
            let source = std::fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            for problem in lint_root_problems(&source) {
                failures.push(format!("{}: {problem}", file.display()));
            }
        }
    }
    report(failures, "crate roots carry the required lint attributes")
}

fn check_attribution(root: &Path) -> TaskResult {
    let mut failures = Vec::new();

    // 1. Every package in the build graph, for every target, must be usable
    //    under allowlisted licenses.
    let out = cargo_output(
        root,
        &["tree", "--workspace", "--locked", "--target", "all", "-e", "normal,build,dev", "--prefix", "none", "--format", "{p}\t{l}"],
    )?;
    let mut checked = BTreeSet::new();
    for line in out.lines().filter(|l| !l.trim().is_empty()) {
        let line = line.trim().trim_end_matches("(*)").trim_end();
        let Some((pkg, license)) = line.split_once('\t') else {
            failures.push(format!("unexpected cargo tree line: {line}"));
            continue;
        };
        if !checked.insert(pkg.to_owned()) {
            continue;
        }
        let license = license.trim();
        if license.is_empty() {
            failures.push(format!("{pkg}: no SPDX license in its manifest"));
            continue;
        }
        match spdx::is_allowed(license) {
            Ok(true) => {}
            Ok(false) => failures.push(format!("{pkg}: license '{license}' is not on the allowlist")),
            Err(e) => failures.push(format!("{pkg}: cannot read license '{license}': {e}")),
        }
    }
    println!("checked {} packages against the license allowlist", checked.len());

    // 2. Every file under assets/ must be named in ATTRIBUTION.md.
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
    println!("checked {listed} asset files against ATTRIBUTION.md");

    // 3. The legal files the ground rules require must exist.
    for file in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md", "UPSTREAM.md"] {
        if !root.join(file).is_file() {
            failures.push(format!("{file} is missing"));
        }
    }

    report(failures, "licenses and assets are accounted for")
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

fn check_layering(root: &Path) -> TaskResult {
    let members = workspace_members(root)?;
    let names: BTreeSet<&str> = members.iter().map(|m| m.name.as_str()).collect();
    let mut failures = Vec::new();

    for name in &names {
        if !layers::UNLAYERED.contains(name) && layers::layer_of(name).is_none() {
            failures.push(format!("{name} has no layer in xtask/src/layers.rs"));
        }
    }

    let mut edges = 0usize;
    for member in &members {
        let out = cargo_output(
            root,
            &[
                "tree",
                "--locked",
                "-p",
                &member.name,
                "--depth",
                "1",
                "-e",
                "normal,build,dev",
                "--target",
                "all",
                "--prefix",
                "depth",
                "--format",
                "{p}",
            ],
        )?;
        for line in out.lines() {
            // With --prefix depth each package line starts with its depth.
            let Some(rest) = line.strip_prefix('1') else {
                continue;
            };
            let Some((dep, _)) = parse_pkg(rest) else {
                continue;
            };
            if names.contains(dep.as_str()) {
                edges += 1;
                if let Err(e) = layers::check_edge(&member.name, &dep) {
                    failures.push(e);
                }
            }
        }
    }
    println!("checked {edges} workspace dependency edges across {} crates", members.len());
    report(failures, "workspace layering holds")
}

fn report(failures: Vec<String>, ok_message: &str) -> TaskResult {
    if failures.is_empty() {
        println!("ok: {ok_message}");
        return Ok(());
    }
    for f in &failures {
        eprintln!("  - {f}");
    }
    Err(format!("{} problem(s)", failures.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_tree_lines() {
        let (name, path) = parse_pkg("wordsmith-ui v0.1.0-dev (/repo/crates/wordsmith-ui)").unwrap();
        assert_eq!(name, "wordsmith-ui");
        assert_eq!(path, Some(PathBuf::from("/repo/crates/wordsmith-ui")));

        let (name, path) = parse_pkg("serde v1.0.0 (*)").unwrap();
        assert_eq!(name, "serde");
        assert_eq!(path, None);

        let (name, path) = parse_pkg(r"wordsmith v0.1.0 (C:\src\ws\apps\wordsmith)").unwrap();
        assert_eq!(name, "wordsmith");
        assert!(path.is_some());

        // A git source is not a local path.
        let (_, path) = parse_pkg("craft-ui v0.1.0 (https://example.invalid/x?tag=v0.1.0#abc)").unwrap();
        assert_eq!(path, None);
        assert!(parse_pkg("").is_none());
    }

    #[test]
    fn lint_roots_detect_missing_attributes() {
        let good = "#![forbid(unsafe_code)]\n#![deny(\n clippy::unwrap_used,\n clippy::expect_used,\n clippy::panic,\n clippy::unimplemented,\n clippy::todo,\n clippy::unreachable\n)]\n";
        assert!(lint_root_problems(good).is_empty());

        let missing_todo = good.replace("clippy::todo,", "");
        assert_eq!(lint_root_problems(&missing_todo), vec!["missing #![deny(clippy::todo)]"]);

        let no_forbid = good.replace("#![forbid(unsafe_code)]", "");
        assert_eq!(lint_root_problems(&no_forbid), vec!["missing #![forbid(unsafe_code)]"]);

        // Lints named outside a deny block do not count.
        let warn_only = good.replace("#![deny(", "#![warn(");
        assert_eq!(lint_root_problems(&warn_only).len(), REQUIRED_DENIES.len());
    }

    #[test]
    fn task_list_is_complete() {
        for (name, _) in TASKS {
            assert_ne!(*name, "ci");
        }
        assert_eq!(TASKS.len(), 7);
    }
}
