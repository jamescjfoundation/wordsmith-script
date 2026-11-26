// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask core-pin`: an app builds on one core release, and how far behind that release is.
//!
//! An app names the core in its root `Cargo.toml`, every `jcj-` line at the same release tag, and
//! moves to the newest core before new work starts on it and before its own release (decision
//! 0011). Nothing in Cargo keeps that true, so this module checks it:
//!
//! - **One tag.** Every line that names the core's git address carries a release tag
//!   (`tag = "vX.Y.Z"`), and all of them the same one.
//! - **One core in the graph.** `Cargo.lock` holds packages from one ref of the core's address. A
//!   second one arrives when a dependency (another app's crates, say) pins a different core
//!   release: the two cores' types do not mix, and the older one lacks the newer one's fixes.
//! - **Distance** (`--behind`). The core's release tags are listed with `git ls-remote`, ordered
//!   as versions, and the pin's place among them is printed. `--max-behind <n>` fails when the pin
//!   is more than `n` releases behind. This needs the network, so it is not part of `ci`; the
//!   scheduled `core-pin.yml` workflow runs it and opens an issue.
//!
//! The first two read two files and run in every `ci`. A workspace with no core pin (the core
//! itself, or a copy that uses the core by path) passes all three.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::link::CORE_GIT;
use crate::util::{TaskResult, git_output, report};

/// A core release: `v0.1.15` is `(0, 1, 15)`. Tuples order as versions do, so `v0.1.9` comes
/// before `v0.1.10`.
pub type Release = (u64, u64, u64);

/// The release a tag names, when the tag is exactly `vX.Y.Z`.
pub fn release(tag: &str) -> Option<Release> {
    let mut parts = tag.strip_prefix('v')?.split('.').map(|p| p.parse::<u64>().ok());
    let found = (parts.next()??, parts.next()??, parts.next()??);
    // Writing it back rejects a fourth part, a sign and leading zeros, which the parse lets through.
    (tag_of(found) == tag).then_some(found)
}

/// The tag of a release.
pub fn tag_of(release: Release) -> String {
    format!("v{}.{}.{}", release.0, release.1, release.2)
}

/// Whether a git address is the core's (a trailing `/` or `.git` and letter case aside).
fn is_core(address: &str) -> bool {
    address.trim_end_matches('/').trim_end_matches(".git").eq_ignore_ascii_case(CORE_GIT)
}

/// The value of `key = "..."` on a line; `key` must be a whole word.
fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(at) = line.get(from..).and_then(|rest| rest.find(key)).map(|i| i + from) {
        from = at + key.len();
        let (before, after) = (&line[..at], &line[from..]);
        if before.chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        let Some(value) = after.trim_start().strip_prefix('=').and_then(|v| v.trim_start().strip_prefix('"')) else { continue };
        if let Some((value, _)) = value.split_once('"') {
            return Some(value);
        }
    }
    None
}

/// A manifest line that names the core as a git dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinLine {
    /// The line's number, from 1.
    pub number: usize,
    /// The dependency's name.
    pub name: String,
    /// Its `tag`, when the line has one.
    pub tag: Option<String>,
}

/// Every line of a manifest that names the core's git address (comments aside).
pub fn pin_lines(manifest: &str) -> Vec<PinLine> {
    let mut out = Vec::new();
    for (i, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('#') || !value_of(line, "git").is_some_and(is_core) {
            continue;
        }
        let name = line.split('=').next().unwrap_or_default().trim().trim_matches('"').to_owned();
        out.push(PinLine { number: i + 1, name, tag: value_of(line, "tag").map(str::to_owned) });
    }
    out
}

/// What is wrong with how a manifest names the core: a line without a release tag, or lines at
/// different tags (the lines behind the newest tag named are the ones reported).
pub fn manifest_problems(lines: &[PinLine]) -> Vec<String> {
    let newest = lines.iter().filter_map(|l| l.tag.as_deref().and_then(release)).max();
    let mut out = Vec::new();
    for line in lines {
        let at = format!("Cargo.toml:{}: {}", line.number, line.name);
        match (line.tag.as_deref(), line.tag.as_deref().and_then(release)) {
            (None, _) => out.push(format!("{at} names the core without a release tag on the same line (`tag = \"vX.Y.Z\"`)")),
            (Some(tag), None) => out.push(format!("{at} is at `{tag}`, which is not a core release tag (vX.Y.Z)")),
            (Some(tag), found) if found != newest => {
                let other = newest.map(tag_of).unwrap_or_default();
                out.push(format!("{at} is at {tag}, other lines are at {other}: every jcj- line carries the same tag"));
            }
            _ => {}
        }
    }
    out
}

/// A package in a lock file.
#[derive(Debug, Default)]
pub struct Locked {
    name: String,
    version: String,
    /// Empty for a package of the workspace or one used by path.
    source: String,
    /// The entries of its `dependencies` list, as written.
    deps: Vec<String>,
}

/// The packages of a lock file.
pub fn locked(lock: &str) -> Vec<Locked> {
    let mut out: Vec<Locked> = Vec::new();
    let (mut in_package, mut in_deps) = (false, false);
    for line in lock.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[[package]]";
            in_deps = false;
            if in_package {
                out.push(Locked::default());
            }
            continue;
        }
        let Some(package) = out.last_mut().filter(|_| in_package) else { continue };
        let field = |key: &str| line.strip_prefix(key).and_then(|v| v.strip_prefix(" = \"")).and_then(|v| v.strip_suffix('"'));
        if in_deps {
            match line {
                "]" => in_deps = false,
                entry => package.deps.push(entry.trim_end_matches(',').trim_matches('"').to_owned()),
            }
        } else if line == "dependencies = [" {
            in_deps = true;
        } else if let Some(v) = field("name") {
            package.name = v.to_owned();
        } else if let Some(v) = field("version") {
            package.version = v.to_owned();
        } else if let Some(v) = field("source") {
            package.source = v.to_owned();
        }
    }
    out
}

/// The ref of the core a lock-file source names: the tag (`v0.1.4`), else `rev=...`, `branch=...`
/// or the commit. `None` when the source is not the core's address.
pub fn core_ref(source: &str) -> Option<String> {
    let rest = source.strip_prefix("git+")?;
    let (rest, commit) = rest.split_once('#').unwrap_or((rest, ""));
    let (address, query) = rest.split_once('?').unwrap_or((rest, ""));
    if !is_core(address) {
        return None;
    }
    Some(match query.strip_prefix("tag=") {
        Some(tag) => tag.to_owned(),
        None if query.is_empty() => format!("commit {commit}"),
        None => query.to_owned(),
    })
}

/// The cores a lock file holds: each ref of the core's address with its packages, ordered as
/// versions with the newest last (a ref that is not a release tag comes before every release).
pub fn cores(packages: &[Locked]) -> Vec<(String, Vec<&Locked>)> {
    let mut by_ref: BTreeMap<(Option<Release>, String), Vec<&Locked>> = BTreeMap::new();
    for package in packages {
        if let Some(name) = core_ref(&package.source) {
            by_ref.entry((release(&name), name)).or_default().push(package);
        }
    }
    by_ref.into_iter().map(|((_, name), from)| (name, from)).collect()
}

/// Whether a lock-file dependency entry (`name`, `name version` or `name version (source)`) means
/// `package`.
fn names(entry: &str, package: &Locked) -> bool {
    let mut parts = entry.splitn(3, ' ');
    parts.next() == Some(package.name.as_str())
        && parts.next().is_none_or(|version| version == package.version)
        && parts.next().is_none_or(|source| package.source.starts_with(source.trim_matches(['(', ')'])))
}

/// What is wrong with the cores in a lock file: a ref that is not a release tag, or more than one
/// ref. For each ref but the newest it names the packages built from it and the packages that
/// bring them in.
pub fn lock_problems(packages: &[Locked]) -> Vec<String> {
    let cores = cores(packages);
    let mut out = Vec::new();
    for (name, _) in cores.iter().filter(|(name, _)| release(name).is_none()) {
        out.push(format!("Cargo.lock builds the core from `{name}`, which is not a core release tag (vX.Y.Z)"));
    }
    let Some(((newest, _), older)) = cores.split_last() else { return out };
    for (name, from) in older {
        let built: Vec<&str> = from.iter().map(|p| p.name.as_str()).collect();
        let users: BTreeSet<&str> = packages
            .iter()
            .filter(|p| core_ref(&p.source).as_ref() != Some(name))
            .filter(|p| p.deps.iter().any(|entry| from.iter().any(|f| names(entry, f))))
            .map(|p| p.name.as_str())
            .collect();
        let users: Vec<&str> = users.into_iter().collect();
        out.push(format!(
            "Cargo.lock holds two cores, {newest} and {name}. From {name}: {}; used by: {}. A graph holds one core release: \
             move the pin, or move those users to a release of theirs built on the same core",
            built.join(", "),
            if users.is_empty() { "nothing else in the lock file".to_owned() } else { users.join(", ") },
        ));
    }
    out
}

/// The release an app is pinned to: the tag its manifest lines carry (the oldest, should they
/// differ), else the oldest release among the lock file's cores.
pub fn pinned(lines: &[PinLine], cores: &[(String, Vec<&Locked>)]) -> Option<Release> {
    let named = lines.iter().filter_map(|l| l.tag.as_deref().and_then(release)).min();
    named.or_else(|| cores.iter().filter_map(|(name, _)| release(name)).min())
}

/// The release tags in `git ls-remote --tags` output, oldest first, each once.
pub fn releases(ls_remote: &str) -> Vec<Release> {
    let found: BTreeSet<Release> =
        ls_remote.lines().filter_map(|l| l.split("refs/tags/").nth(1)).filter_map(|tag| release(tag.trim().trim_end_matches("^{}"))).collect();
    found.into_iter().collect()
}

/// How many of `releases` are newer than `pin`, and the newest of them all.
pub fn behind(pin: Release, releases: &[Release]) -> Result<(usize, Release), String> {
    let newest = releases.iter().max().copied().ok_or("the core has no release tags")?;
    if !releases.contains(&pin) {
        return Err(format!("the core has no release {}", tag_of(pin)));
    }
    Ok((releases.iter().filter(|r| **r > pin).count(), newest))
}

/// `--behind` and `--max-behind <n>` (which asks for the distance too).
fn options(args: &[String]) -> Result<(bool, Option<usize>), String> {
    let (mut measure, mut max) = (false, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let number = match arg.strip_prefix("--max-behind=") {
            Some(n) => n,
            None if arg == "--max-behind" => args.next().ok_or("--max-behind needs a number")?,
            None if arg == "--behind" => {
                measure = true;
                continue;
            }
            None => return Err(format!("unknown argument '{arg}' for core-pin (--behind, --max-behind <n>)")),
        };
        max = Some(number.parse::<usize>().map_err(|_| format!("--max-behind: '{number}' is not a number"))?);
        measure = true;
    }
    Ok((measure, max))
}

/// `cargo xtask core-pin [--behind] [--max-behind <n>]`.
///
/// With `--behind` it also prints `pinned=`, `newest=` and `behind=` lines for a workflow to read.
pub fn task(app: &Path, args: &[String]) -> TaskResult {
    let (measure, max) = options(args)?;
    let read = |p: &str| std::fs::read_to_string(app.join(p)).map_err(|e| format!("cannot read {p}: {e}"));
    let lines = pin_lines(&read("Cargo.toml")?);
    let packages = locked(&read("Cargo.lock")?);
    let cores = cores(&packages);
    if lines.is_empty() && cores.is_empty() {
        println!("ok: no core pin here (this workspace is the core, or uses the core by path)");
        return Ok(());
    }
    let mut failures = manifest_problems(&lines);
    failures.extend(lock_problems(&packages));
    let pin = pinned(&lines, &cores);
    let checked = report(failures, &format!("one core release{}", pin.map(|p| format!(", {}", tag_of(p))).unwrap_or_default()));
    if !measure {
        return checked;
    }
    let pin = pin.ok_or("there is no core release tag to measure from")?;
    let listed = git_output(app, &["ls-remote", "--tags", "--refs", CORE_GIT])?;
    let (count, newest) = behind(pin, &releases(&listed))?;
    let (pin, newest) = (tag_of(pin), tag_of(newest));
    match count {
        0 => println!("core pin {pin} is the newest core release"),
        1 => println!("core pin {pin} is 1 release behind {newest}"),
        n => println!("core pin {pin} is {n} releases behind {newest}"),
    }
    println!("pinned={pin}\nnewest={newest}\nbehind={count}");
    checked?;
    match max {
        Some(max) if count > max => Err(format!(
            "the core pin is {count} releases behind, more than {max}: move every jcj- dependency to `tag = \"{newest}\"`, then `cargo xtask sync`"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[workspace.package]
repository = "https://github.com/jamescjfoundation/open-source-core"

[workspace.dependencies]
# jcj-old = { git = "https://github.com/jamescjfoundation/open-source-core", tag = "v0.1.0" }
jcj-doc = { git = "https://github.com/jamescjfoundation/open-source-core", tag = "v0.1.10" }
jcj-shell = { tag = "v0.1.10", git = "https://github.com/JamesCJFoundation/open-source-core.git" }
other-model = { git = "https://example.invalid/other-app", tag = "v9.9.9" }
serde = "1"
"#;

    fn lock(packages: &[(&str, &str, &str, &[&str])]) -> String {
        let mut out = String::from("version = 4\n");
        for (name, version, source, deps) in packages {
            out.push_str(&format!("\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"));
            if !source.is_empty() {
                out.push_str(&format!("source = \"{source}\"\nchecksum = \"00\"\n"));
            }
            if !deps.is_empty() {
                out.push_str("dependencies = [\n");
                deps.iter().for_each(|d| out.push_str(&format!(" \"{d}\",\n")));
                out.push_str("]\n");
            }
        }
        out.push_str("\n[metadata]\nname = \"not a package\"\n");
        out
    }

    const NEW: &str = "git+https://github.com/jamescjfoundation/open-source-core?tag=v0.1.10#aaaa";
    const OLD: &str = "git+https://github.com/jamescjfoundation/open-source-core?tag=v0.1.9#bbbb";
    const OTHER: &str = "git+https://example.invalid/other-app?tag=v9.9.9#cccc";

    #[test]
    fn release_tags_order_as_versions() {
        assert_eq!(release("v0.1.15"), Some((0, 1, 15)));
        for not_a_release in ["0.1.15", "v0.1", "v0.1.2.3", "v0.1.x", "v0.1.2-rc.1", "v0.1.+2", "v0.01.2", "nightly", ""] {
            assert_eq!(release(not_a_release), None, "{not_a_release}");
        }
        let out = "a\trefs/tags/v0.1.9\nb\trefs/tags/v0.1.10\nc\trefs/tags/v0.1.10^{}\nd\trefs/tags/v0.2.0\ne\trefs/tags/v0.1.2\n\
                   f\trefs/tags/nightly\ng\trefs/tags/v0.3.0-rc.1\nh\trefs/heads/v9.9.9\n";
        let all = releases(out);
        assert_eq!(all, vec![(0, 1, 2), (0, 1, 9), (0, 1, 10), (0, 2, 0)]);
        assert_eq!(behind((0, 1, 9), &all), Ok((2, (0, 2, 0))));
        assert_eq!(behind((0, 2, 0), &all), Ok((0, (0, 2, 0))));
        assert!(behind((0, 1, 3), &all).is_err());
        assert!(behind((0, 1, 9), &[]).is_err());
    }

    #[test]
    fn finds_the_lines_that_name_the_core() {
        let lines = pin_lines(MANIFEST);
        let found: Vec<(usize, &str, Option<&str>)> = lines.iter().map(|l| (l.number, l.name.as_str(), l.tag.as_deref())).collect();
        assert_eq!(found, vec![(7, "jcj-doc", Some("v0.1.10")), (8, "jcj-shell", Some("v0.1.10"))]);
        assert!(manifest_problems(&lines).is_empty());
        assert_eq!(pinned(&lines, &[]), Some((0, 1, 10)));
        assert_eq!(value_of(r#"x = { legit = "no", git="yes" }"#, "git"), Some("yes"));
        assert_eq!(value_of(r#"homepage = "https://github.com/x""#, "git"), None);
    }

    #[test]
    fn every_line_carries_the_same_release_tag() {
        let core = "https://github.com/jamescjfoundation/open-source-core";
        let manifest = format!(
            "jcj-doc = {{ git = \"{core}\", tag = \"v0.1.9\" }}\njcj-shell = {{ git = \"{core}\", tag = \"v0.1.10\" }}\n\
             jcj-board = {{ git = \"{core}\", rev = \"abc\" }}\njcj-theme = {{ git = \"{core}\", tag = \"next\" }}\n\
             [dependencies.jcj-proof]\ngit = \"{core}\"\ntag = \"v0.1.10\"\n"
        );
        let lines = pin_lines(&manifest);
        let problems = manifest_problems(&lines);
        assert_eq!(problems.len(), 4, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.toml:1: jcj-doc is at v0.1.9, other lines are at v0.1.10"));
        assert!(problems[1].starts_with("Cargo.toml:3: jcj-board names the core without a release tag"));
        assert!(problems[2].starts_with("Cargo.toml:4: jcj-theme is at `next`"));
        assert!(problems[3].starts_with("Cargo.toml:6: git names the core without a release tag"));
        // The distance is measured from the oldest tag named.
        assert_eq!(pinned(&lines, &[]), Some((0, 1, 9)));
    }

    #[test]
    fn the_lock_file_holds_one_core() {
        let one = lock(&[
            ("app", "1.0.0", "", &["jcj-doc", "serde"]),
            ("jcj-doc", "0.1.10", NEW, &["jcj-upstream"]),
            ("jcj-upstream", "0.1.10", NEW, &[]),
            ("serde", "1.0.0", "registry+https://github.com/rust-lang/crates.io-index", &[]),
        ]);
        let packages = locked(&one);
        assert_eq!(packages.len(), 4);
        assert!(lock_problems(&packages).is_empty());
        let found = cores(&packages);
        assert_eq!(found.len(), 1);
        assert_eq!(pinned(&[], &found), Some((0, 1, 10)));
        // No core at all: the core itself, or a workspace linked to a local checkout.
        assert!(cores(&locked(&lock(&[("jcj-doc", "0.1.10", "", &[])]))).is_empty());
    }

    #[test]
    fn a_second_core_is_named_with_its_packages_and_their_users() {
        let two = lock(&[
            ("app", "1.0.0", "", &["jcj-doc 0.1.10", "other-model"]),
            ("jcj-doc", "0.1.10", NEW, &["jcj-upstream 0.1.10"]),
            ("jcj-doc", "0.1.9", OLD, &["jcj-upstream 0.1.9"]),
            ("jcj-upstream", "0.1.10", NEW, &[]),
            ("jcj-upstream", "0.1.9", OLD, &[]),
            ("other-model", "9.9.9", OTHER, &["jcj-doc 0.1.9"]),
        ]);
        let packages = locked(&two);
        let problems = lock_problems(&packages);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.lock holds two cores, v0.1.10 and v0.1.9. From v0.1.9: jcj-doc, jcj-upstream; used by: other-model."));
        // Two packages of one name and version are told apart by their source.
        let same_version = lock(&[
            ("app", "1.0.0", "", &["jcj-doc 0.1.9 (git+https://github.com/jamescjfoundation/open-source-core?rev=abc)"]),
            ("jcj-doc", "0.1.9", "git+https://github.com/jamescjfoundation/open-source-core?rev=abc#abc", &[]),
            ("jcj-doc", "0.1.9", OLD, &[]),
            ("other-model", "9.9.9", OTHER, &["jcj-doc 0.1.9 (git+https://github.com/jamescjfoundation/open-source-core?tag=v0.1.9)"]),
        ]);
        let problems = lock_problems(&locked(&same_version));
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.lock builds the core from `rev=abc`"));
        assert!(problems[1].contains("From rev=abc: jcj-doc; used by: app."));
    }

    #[test]
    fn reads_its_arguments() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(options(&args(&[])), Ok((false, None)));
        assert_eq!(options(&args(&["--behind"])), Ok((true, None)));
        assert_eq!(options(&args(&["--max-behind", "3"])), Ok((true, Some(3))));
        assert_eq!(options(&args(&["--behind", "--max-behind=0"])), Ok((true, Some(0))));
        assert!(options(&args(&["--max-behind"])).is_err());
        assert!(options(&args(&["--max-behind", "three"])).is_err());
        assert!(options(&args(&["--ahead"])).is_err());
    }

    #[test]
    fn a_workspace_without_a_core_pin_passes() {
        let dir = std::env::temp_dir().join(format!("jcj-corepin-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[workspace.dependencies]\njcj-doc = { path = \"crates/jcj-doc\" }\n").unwrap();
        std::fs::write(dir.join("Cargo.lock"), lock(&[("jcj-doc", "0.1.10", "", &[])])).unwrap();
        // With nothing to measure the network is never asked.
        assert_eq!(task(&dir, &["--max-behind".to_owned(), "0".to_owned()]), Ok(()));
        std::fs::write(dir.join("Cargo.toml"), MANIFEST).unwrap();
        std::fs::write(dir.join("Cargo.lock"), lock(&[("jcj-doc", "0.1.10", NEW, &[]), ("jcj-doc", "0.1.9", OLD, &[])])).unwrap();
        assert!(task(&dir, &[]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
