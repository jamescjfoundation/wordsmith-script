// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared helpers for the xtask of open-source-core and of every app repository.
//!
//! This file is owned by open-source-core (`template/xtask/src/`) and copied into apps by
//! `cargo xtask sync`. Change it in the core, never in an app.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type TaskResult = Result<(), String>;

pub fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// Run cargo, streaming its output.
pub fn cargo(root: &Path, args: &[&str]) -> TaskResult {
    let status = Command::new(cargo_bin()).args(args).current_dir(root).status().map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("`cargo {}` exited with {status}", args.join(" "))) }
}

/// Run cargo and capture its standard output.
pub fn cargo_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cargo_bin()).args(args).current_dir(root).output().map_err(|e| format!("could not run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!("`cargo {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("cargo printed non-UTF-8 output: {e}"))
}

/// Run git and capture its standard output.
pub fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").args(args).current_dir(root).output().map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err(format!("`git {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("git printed non-UTF-8 output: {e}"))
}

/// A workspace member: package name and directory.
pub struct Member {
    pub name: String,
    pub dir: PathBuf,
}

/// Parses a `cargo tree` package line: `name vX.Y.Z (/path)` with an optional trailing ` (*)`
/// marker. Returns the name and the local path if any (a git or registry source is not a path).
pub fn parse_pkg(line: &str) -> Option<(String, Option<PathBuf>)> {
    let line = line.trim().trim_end_matches("(*)").trim_end();
    let mut parts = line.splitn(3, ' ');
    let name = parts.next().filter(|n| !n.is_empty())?.to_owned();
    let _version = parts.next()?;
    let path = parts
        .next()
        .and_then(|rest| rest.strip_prefix('('))
        .and_then(|rest| rest.strip_suffix(')'))
        .filter(|p| p.starts_with('/') || p.contains(":\\") || p.starts_with("\\\\"))
        .map(PathBuf::from);
    Some((name, path))
}

pub fn workspace_members(root: &Path) -> Result<Vec<Member>, String> {
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

/// The direct dependencies (normal, build and dev, every target) of one workspace member.
pub fn direct_deps(root: &Path, member: &str) -> Result<Vec<String>, String> {
    let out = cargo_output(
        root,
        &["tree", "--locked", "-p", member, "--depth", "1", "-e", "normal,build,dev", "--target", "all", "--prefix", "depth", "--format", "{p}"],
    )?;
    let mut deps = BTreeSet::new();
    for line in out.lines() {
        // With --prefix depth each package line starts with its depth.
        let Some(rest) = line.strip_prefix('1') else { continue };
        if let Some((dep, _)) = parse_pkg(rest) {
            deps.insert(dep);
        }
    }
    Ok(deps.into_iter().collect())
}

pub fn report(failures: Vec<String>, ok_message: &str) -> TaskResult {
    if failures.is_empty() {
        println!("ok: {ok_message}");
        return Ok(());
    }
    for f in &failures {
        eprintln!("  - {f}");
    }
    Err(format!("{} problem(s)", failures.len()))
}

/// The tasks a `ci` run is told to leave out: `--skip a --skip b` or `--skip a,b`.
pub fn skipped(args: &[String], known: &[&str]) -> Result<Vec<String>, String> {
    let mut skip = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let list = match arg.strip_prefix("--skip=") {
            Some(list) => list,
            None if arg == "--skip" => args.next().ok_or("--skip needs a task name")?,
            None => return Err(format!("unknown argument '{arg}' for ci")),
        };
        for name in list.split(',').filter(|n| !n.is_empty()) {
            if !known.contains(&name) {
                return Err(format!("--skip: there is no task '{name}'"));
            }
            skip.push(name.to_owned());
        }
    }
    Ok(skip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_tree_lines() {
        let (name, path) = parse_pkg("jcj-doc v0.1.0 (/repo/crates/jcj-doc)").unwrap();
        assert_eq!(name, "jcj-doc");
        assert_eq!(path, Some(PathBuf::from("/repo/crates/jcj-doc")));
        let (name, path) = parse_pkg("serde v1.0.0 (*)").unwrap();
        assert_eq!(name, "serde");
        assert_eq!(path, None);
        let (_, path) = parse_pkg(r"app v0.1.0 (C:\src\app)").unwrap();
        assert!(path.is_some());
        let (_, path) = parse_pkg("craft-ui v0.1.0 (https://example.invalid/x?tag=v0.1.0#abc)").unwrap();
        assert_eq!(path, None);
        assert!(parse_pkg("").is_none());
    }

    #[test]
    fn ci_can_leave_tasks_out() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let known = ["fmt", "test"];
        assert_eq!(skipped(&args(&[]), &known), Ok(vec![]));
        assert_eq!(skipped(&args(&["--skip", "test"]), &known), Ok(vec!["test".to_owned()]));
        assert_eq!(skipped(&args(&["--skip=fmt,test"]), &known), Ok(vec!["fmt".to_owned(), "test".to_owned()]));
        assert!(skipped(&args(&["--skip"]), &known).is_err());
        assert!(skipped(&args(&["--skip", "nothing"]), &known).is_err());
        assert!(skipped(&args(&["fmt"]), &known).is_err());
    }
}
