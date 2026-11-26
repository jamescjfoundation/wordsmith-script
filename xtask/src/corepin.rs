// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask core-pin`: an app builds on one core release, and how far behind that release is.
//!
//! An app names the core in its root `Cargo.toml`, every `jcj-` line at the same release tag, and
//! moves to the newest core before new work starts on it and before its own release (core
//! decision 0011). Nothing in Cargo keeps that true, so this module checks it:
//!
//! - **One tag.** Every line that names the core's git address carries a release tag
//!   (`tag = "vX.Y.Z"`), and all of them the same one.
//! - **One core in the graph.** `Cargo.lock` holds packages from one ref of the core's address. A
//!   second one arrives when a dependency (another app's crates, say) pins a different core
//!   release: the two cores' types do not mix, and the older one lacks the newer one's fixes.
//! - **A temporary pin, on a branch only.** An app change that needs a core change which is not
//!   released yet can be checked before the release: on a branch, every `jcj-` line names the
//!   same full commit of the core (`rev = "<40 characters>"`) in place of the tag. The gate
//!   passes there with a warning, so a pull request shows green against that commit. On `main`,
//!   on a tag and anywhere this cannot be told to be a branch, the same pin fails: `main` and
//!   releases build on a core release (core decision 0013).
//! - **Distance** (`--behind`). The core's release tags are listed with `git ls-remote`, ordered
//!   as versions, and the pin's place among them is printed. `--max-behind <n>` fails when the pin
//!   is more than `n` releases behind. This needs the network, so it is not part of `ci`; the
//!   scheduled `core-pin.yml` workflow runs it and opens an issue.
//!
//! The first two read two files and run in every `ci`. A workspace with no core pin (the core
//! itself, or a copy that uses the core by path) passes all three.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Core decisions: <https://github.com/jamescjfoundation/open-source-core/tree/main/docs/decisions>.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::link::{CORE_GIT, same_address, value_of};
use super::util::{TaskResult, git_output, report};

/// The branch that builds on releases only.
const MAIN: &str = "main";

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
    same_address(address, CORE_GIT)
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
    /// Its `rev`, when the line has one: a commit, which only a temporary pin names.
    pub rev: Option<String>,
    /// Its `branch`, when the line has one. A branch moves, so it is never a pin.
    pub branch: Option<String>,
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
        let key = |k: &str| value_of(line, k).map(str::to_owned);
        out.push(PinLine { number: i + 1, name, tag: key("tag"), rev: key("rev"), branch: key("branch") });
    }
    out
}

/// Whether `rev` is a whole commit id: 40 hexadecimal characters. A shorter one can come to
/// mean another commit.
fn is_full_commit(rev: &str) -> bool {
    rev.len() == 40 && rev.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The commit of a temporary pin: every line names the same whole commit and none a tag or a
/// branch. `None` for every other manifest, the usual one at a release tag included.
pub fn temporary_pin(lines: &[PinLine]) -> Option<&str> {
    let first = lines.first()?.rev.as_deref().filter(|rev| is_full_commit(rev))?;
    let same = |l: &PinLine| l.tag.is_none() && l.branch.is_none() && l.rev.as_deref().is_some_and(|rev| rev.eq_ignore_ascii_case(first));
    lines.iter().all(same).then_some(first)
}

/// What is wrong with how a manifest names the core: a line without a release tag, or lines at
/// different tags (the lines behind the newest tag named are the ones reported). A temporary pin
/// ([`temporary_pin`]) is not a problem here; where it is allowed is [`Place`]'s business.
pub fn manifest_problems(lines: &[PinLine]) -> Vec<String> {
    if temporary_pin(lines).is_some() {
        return Vec::new();
    }
    let newest = lines.iter().filter_map(|l| l.tag.as_deref().and_then(release)).max();
    let mut out = Vec::new();
    for line in lines {
        let at = format!("Cargo.toml:{}: {}", line.number, line.name);
        if let Some(branch) = &line.branch {
            out.push(format!("{at} follows the branch `{branch}`, which moves: name a release tag (`tag = \"vX.Y.Z\"`)"));
            continue;
        }
        if let (Some(rev), None) = (&line.rev, &line.tag) {
            out.push(if is_full_commit(rev) {
                format!("{at} is at commit {rev} while other lines are not: every jcj- line carries the same tag, or on a branch the same commit")
            } else {
                format!("{at} is at `{rev}`, which is not a whole commit id: a temporary pin names all 40 characters")
            });
            continue;
        }
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

/// Where a gate is running, as far as a temporary pin cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// A pull request's run in GitHub Actions.
    PullRequest,
    /// A branch other than `main`, here or in GitHub Actions.
    Branch(String),
    /// `main`, a tag, a merge queue, or somewhere that cannot be told to be a branch. The reason.
    Refused(String),
}

/// Where this run is: from GitHub Actions' variables when it runs there (`env` reads one), else
/// from the branch checked out (`branch`; `None` or `HEAD` when there is none).
pub fn place(env: &dyn Fn(&str) -> Option<String>, branch: Option<&str>) -> Place {
    if env("GITHUB_ACTIONS").as_deref() == Some("true") {
        let event = env("GITHUB_EVENT_NAME").unwrap_or_default();
        let name = env("GITHUB_REF_NAME").unwrap_or_default();
        return match (event.as_str(), env("GITHUB_REF_TYPE").as_deref()) {
            ("pull_request" | "pull_request_target", _) => Place::PullRequest,
            ("merge_group", _) => Place::Refused("this run is a merge queue's, about to land on main".to_owned()),
            (_, Some("tag")) => Place::Refused(format!("this run is on the tag {name}")),
            (_, Some("branch")) if !name.is_empty() && name != MAIN => Place::Branch(name),
            (_, Some("branch")) if name == MAIN => Place::Refused(format!("this run is on {MAIN}")),
            _ => Place::Refused("this run names no branch".to_owned()),
        };
    }
    match branch.map(str::trim) {
        Some(name) if name == MAIN => Place::Refused(format!("this checkout is on {MAIN}")),
        Some(name) if !name.is_empty() && name != "HEAD" => Place::Branch(name.to_owned()),
        _ => Place::Refused("this checkout is on no branch".to_owned()),
    }
}

/// What to say about a temporary pin at `rev`: a warning where it is allowed, an error elsewhere.
pub fn temporary_pin_verdict(rev: &str, place: &Place) -> Result<String, String> {
    let rule = "main and releases build on a core release (core decision 0013)";
    match place {
        Place::Refused(why) => Err(format!(
            "every jcj- line is at commit {rev}, not a release, and {why}. A temporary pin is for a branch and its pull request only: {rule}. \
             Move every jcj- line to `tag = \"vX.Y.Z\"`"
        )),
        Place::PullRequest | Place::Branch(_) => Ok(format!(
            "temporary core pin: every jcj- line is at commit {rev}, not a release. That is allowed on a branch, so this change can be checked \
             against a core change before its release; {rule}. Move every jcj- line to the release's tag before this lands."
        )),
    }
}

/// A package in a lock file.
#[derive(Debug, Default)]
pub struct Locked {
    pub name: String,
    pub version: String,
    /// Empty for a package of the workspace or one used by path.
    pub source: String,
    /// The entries of its `dependencies` list, as written.
    pub deps: Vec<String>,
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
/// bring them in. `temporary` is the commit of the manifest's temporary pin, when it has one:
/// that one ref is expected.
pub fn lock_problems(packages: &[Locked], temporary: Option<&str>) -> Vec<String> {
    let cores = cores(packages);
    let expected = temporary.map(|rev| format!("rev={rev}"));
    let mut out = Vec::new();
    for (name, _) in cores.iter().filter(|(name, _)| release(name).is_none() && expected.as_deref() != Some(name.as_str())) {
        out.push(format!("Cargo.lock builds the core from `{name}`, which is not a core release tag (vX.Y.Z)"));
    }
    if let Some(expected) = &expected
        && !cores.is_empty()
        && !cores.iter().any(|(name, _)| name == expected)
    {
        out.push(format!(
            "Cargo.toml pins the core at a commit and Cargo.lock does not build it from `{expected}`: `cargo metadata --format-version 1 >/dev/null` brings it in line"
        ));
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
    let temporary = temporary_pin(&lines);
    let mut failures = manifest_problems(&lines);
    failures.extend(lock_problems(&packages, temporary));
    if let Some(rev) = temporary {
        let branch = git_output(app, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
        match temporary_pin_verdict(rev, &place(&|key| std::env::var(key).ok(), branch.as_deref())) {
            Err(refused) => failures.push(refused),
            Ok(warning) => {
                println!("warning: {warning}");
                if std::env::var_os("GITHUB_ACTIONS").is_some() {
                    // Shown on the pull request's checks page, not only in the log.
                    println!("::warning title=Temporary core pin::{warning}");
                }
            }
        }
    }
    let pin = pinned(&lines, &cores);
    let what = match (temporary, pin) {
        (Some(rev), _) => format!("one core, at the commit {rev} (a temporary pin)"),
        (None, pin) => format!("one core release{}", pin.map(|p| format!(", {}", tag_of(p))).unwrap_or_default()),
    };
    let checked = report(failures, &what);
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
        assert!(temporary_pin(&lines).is_none());
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
        assert!(problems[1].starts_with("Cargo.toml:3: jcj-board is at `abc`, which is not a whole commit id"));
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
        assert!(lock_problems(&packages, None).is_empty());
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
        let problems = lock_problems(&packages, None);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.lock holds two cores, v0.1.10 and v0.1.9. From v0.1.9: jcj-doc, jcj-upstream; used by: other-model."));
        // Two packages of one name and version are told apart by their source.
        let same_version = lock(&[
            ("app", "1.0.0", "", &["jcj-doc 0.1.9 (git+https://github.com/jamescjfoundation/open-source-core?rev=abc)"]),
            ("jcj-doc", "0.1.9", "git+https://github.com/jamescjfoundation/open-source-core?rev=abc#abc", &[]),
            ("jcj-doc", "0.1.9", OLD, &[]),
            ("other-model", "9.9.9", OTHER, &["jcj-doc 0.1.9 (git+https://github.com/jamescjfoundation/open-source-core?tag=v0.1.9)"]),
        ]);
        let problems = lock_problems(&locked(&same_version), None);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.lock builds the core from `rev=abc`"));
        assert!(problems[1].contains("From rev=abc: jcj-doc; used by: app."));
    }

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    fn pinned_at_a_commit() -> (Vec<PinLine>, String) {
        let core = "https://github.com/jamescjfoundation/open-source-core";
        let manifest = format!("jcj-doc = {{ git = \"{core}\", rev = \"{COMMIT}\" }}\njcj-shell = {{ git = \"{core}\", rev = \"{COMMIT}\" }}\n");
        (pin_lines(&manifest), format!("git+{core}?rev={COMMIT}#{COMMIT}"))
    }

    #[test]
    fn a_temporary_pin_is_every_line_at_the_same_whole_commit() {
        let (lines, source) = pinned_at_a_commit();
        assert_eq!(temporary_pin(&lines), Some(COMMIT));
        assert!(manifest_problems(&lines).is_empty());
        // Its lock file builds the core from that commit and nothing else.
        let one = locked(&lock(&[("app", "1.0.0", "", &["jcj-doc"]), ("jcj-doc", "0.1.10", &source, &[])]));
        assert!(lock_problems(&one, Some(COMMIT)).is_empty());
        assert_eq!(lock_problems(&one, None).len(), 1);
        // A lock file still at the tag is stale, and a second core is as wrong as ever.
        let stale = locked(&lock(&[("jcj-doc", "0.1.10", NEW, &[])]));
        assert!(lock_problems(&stale, Some(COMMIT))[0].contains("does not build it from"));
        let two = locked(&lock(&[("jcj-doc", "0.1.10", &source, &[]), ("jcj-doc", "0.1.9", OLD, &[])]));
        assert!(lock_problems(&two, Some(COMMIT)).iter().any(|p| p.starts_with("Cargo.lock holds two cores")));

        // Not a temporary pin: one line at a tag, two commits, a short commit, a branch.
        let core = "https://github.com/jamescjfoundation/open-source-core";
        let other = COMMIT.replace('0', "f");
        let cases = [
            (format!("a = {{ git = \"{core}\", rev = \"{COMMIT}\" }}\nb = {{ git = \"{core}\", tag = \"v0.1.10\" }}\n"), "while other lines are not"),
            (format!("a = {{ git = \"{core}\", rev = \"{COMMIT}\" }}\nb = {{ git = \"{core}\", rev = \"{other}\" }}\n"), "while other lines are not"),
            (format!("a = {{ git = \"{core}\", rev = \"0123456\" }}\n"), "not a whole commit id"),
            (format!("a = {{ git = \"{core}\", branch = \"main\" }}\n"), "follows the branch `main`"),
        ];
        for (manifest, why) in cases {
            let lines = pin_lines(&manifest);
            assert!(temporary_pin(&lines).is_none(), "{manifest}");
            assert!(manifest_problems(&lines).iter().any(|p| p.contains(why)), "{manifest}: {:?}", manifest_problems(&lines));
        }
    }

    #[test]
    fn a_temporary_pin_is_allowed_on_a_branch_and_refused_on_main() {
        let actions = |event: &'static str, kind: &'static str, name: &'static str| {
            move |key: &str| match key {
                "GITHUB_ACTIONS" => Some("true".to_owned()),
                "GITHUB_EVENT_NAME" => Some(event.to_owned()),
                "GITHUB_REF_TYPE" => Some(kind.to_owned()),
                "GITHUB_REF_NAME" => Some(name.to_owned()),
                _ => None,
            }
        };
        let local = |_: &str| None;
        // In GitHub Actions the checkout of a pull request is on no branch: the event decides.
        assert_eq!(place(&actions("pull_request", "branch", "12/merge"), Some("HEAD")), Place::PullRequest);
        assert_eq!(place(&actions("push", "branch", "shell/tree-drag"), None), Place::Branch("shell/tree-drag".to_owned()));
        assert_eq!(place(&actions("workflow_dispatch", "branch", "shell/tree-drag"), None), Place::Branch("shell/tree-drag".to_owned()));
        for refused in [
            place(&actions("push", "branch", "main"), Some("topic")),
            place(&actions("schedule", "branch", "main"), None),
            place(&actions("push", "tag", "v1.2.3"), None),
            place(&actions("merge_group", "branch", "gh-readonly-queue/main/pr-12"), None),
            place(&actions("push", "", ""), None),
            place(&local, Some("main\n")),
            place(&local, Some("HEAD")),
            place(&local, None),
        ] {
            assert!(matches!(refused, Place::Refused(_)), "{refused:?}");
            let err = temporary_pin_verdict(COMMIT, &refused).unwrap_err();
            assert!(err.contains(COMMIT) && err.contains("for a branch and its pull request only"), "{err}");
        }
        assert_eq!(place(&local, Some("shell/tree-drag\n")), Place::Branch("shell/tree-drag".to_owned()));
        let warning = temporary_pin_verdict(COMMIT, &Place::PullRequest).unwrap();
        assert!(warning.contains("allowed on a branch") && warning.contains("before this lands"), "{warning}");
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
