// SPDX-License-Identifier: MIT OR Apache-2.0
//! The core's patch block, and dev-linking an app to local checkouts of what it builds on.
//!
//! - **Patch block.** Cargo honours `[patch]` only in the top-level project, so every app carries
//!   an exact copy of the core's block (between the `BEGIN`/`END` marker lines of the core's
//!   `Cargo.toml`). [`extract_block`] reads it; [`replace_block`] writes it into an app manifest.
//! - **Dev-link.** While building a core change and an app change together, the app builds
//!   against a local core checkout: `cargo xtask dev-link ../open-source-core`. An app that also
//!   builds on another app's crates (core decision 0015) links that provider the same way, by the
//!   name its `xtask/src/main.rs` declares: `cargo xtask dev-link <name>=<folder>`.
//!   `cargo xtask dev-unlink` removes every link.
//!
//! A link changes no file Git tracks. It is two files in `.cargo/dev-link/`, which the shared
//! block of `.gitignore` ignores:
//!
//! - `config.toml`, a Cargo configuration file with a `[patch]` table per linked address. The
//!   tracked `.cargo/config.toml` includes it when it exists (`include`, with `optional`), so
//!   plain `cargo build`, `cargo xtask ci --linked` and an editor's rust-analyzer all build
//!   against the link with no flag to remember.
//! - `Cargo.lock`, the lock file of the linked build (`resolver.lockfile-path` in that same
//!   file). It starts as a copy of the app's own lock file, so every other dependency stays at
//!   the version the app pins, and the app's `Cargo.lock` is never rewritten while linked.
//!
//! So link, build, unlink leaves `git status` clean. What a tracked file may never hold is
//! checked by [`check_tracked`] (the pre-commit hook): a `[patch]` or `paths` entry in
//! `.cargo/config.toml`, or a `[patch]` entry with a local path in `Cargo.toml`. The gate's
//! `unlinked` task ([`check_unlinked`]) also fails while a link is in place, because the gate
//! says what CI will say and CI builds the pinned releases; `cargo xtask ci --linked` runs the
//! gate against the link on purpose.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Core decisions: <https://github.com/jamescjfoundation/open-source-core/tree/main/docs/decisions>.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use super::json;
use super::util::{TaskResult, cargo_output, git_output};

pub const BLOCK_BEGIN: &str = "# BEGIN jcj patch block";
pub const BLOCK_END: &str = "# END jcj patch block";
/// The markers of the link section the tool wrote into `.cargo/config.toml` up to core 0.1.19.
/// Still recognised, so that an old link is refused and `dev-unlink` removes it.
pub const OLD_LINK_BEGIN: &str = "# BEGIN jcj dev-link";
pub const OLD_LINK_END: &str = "# END jcj dev-link";
/// The core's address, as apps name it in their dependencies.
pub const CORE_GIT: &str = "https://github.com/jamescjfoundation/open-source-core";
/// The name the core is linked under.
pub const CORE_NAME: &str = "core";
/// The folder a link lives in, from the app's root. Git ignores it (the shared `.gitignore` block).
pub const LINK_DIR: &str = ".cargo/dev-link";
/// The line of the tracked `.cargo/config.toml` that reads a link when there is one.
pub const INCLUDE: &str = r#"include = [{ path = "dev-link/config.toml", optional = true }]"#;
/// How deep under a checkout's root its packages are looked for.
const MAX_DEPTH: usize = 4;

/// A repository an app builds on besides the core (core decision 0015): the name the app's xtask
/// knows it by, and the git address the app's dependencies on it carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    pub name: &'static str,
    pub git: &'static str,
}

/// One linked address: its name, the git address it stands in for, and the local checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub name: String,
    pub git: String,
    pub dir: PathBuf,
}

/// The text between (and including) the marker lines.
pub fn extract_block(manifest: &str, begin: &str, end: &str) -> Option<String> {
    let start = manifest.find(begin)?;
    let stop = manifest[start..].find(end)? + start + end.len();
    Some(manifest[start..stop].to_owned())
}

/// `manifest` with the marked block replaced by `block`, or `block` appended when absent.
pub fn replace_block(manifest: &str, block: &str, begin: &str, end: &str) -> String {
    match (manifest.find(begin), manifest.find(end)) {
        (Some(s), Some(e)) if e > s => format!("{}{}{}", &manifest[..s], block, &manifest[e + end.len()..]),
        _ => {
            let sep = if manifest.is_empty() || manifest.ends_with("\n\n") {
                ""
            } else if manifest.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            format!("{manifest}{sep}{block}\n")
        }
    }
}

/// `text` without the lines from the one that starts with `begin` to the one that starts with
/// `end` (and the blank lines before them).
pub fn remove_lines(text: &str, begin: &str, end: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if !inside && line.trim_start().starts_with(begin) {
            inside = true;
            while out.last().is_some_and(|l| l.trim().is_empty()) {
                out.pop();
            }
        } else if inside && line.trim_start().starts_with(end) {
            inside = false;
        } else if !inside {
            out.push(line);
        }
    }
    if out.is_empty() { String::new() } else { format!("{}\n", out.join("\n")) }
}

/// Whether two git addresses are the same repository (a trailing `/` or `.git` and letter case aside).
pub fn same_address(a: &str, b: &str) -> bool {
    let bare = |s: &str| s.trim_end_matches('/').trim_end_matches(".git").to_ascii_lowercase();
    bare(a) == bare(b)
}

/// The value of `key = "..."` on a line; `key` must be a whole word.
pub fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
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

/// The packages a manifest takes from the git address `git`: for each line that names the
/// address, its `package`, else the dependency's own name.
pub fn git_dependencies(manifest: &str, git: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('#') || !value_of(line, "git").is_some_and(|address| same_address(address, git)) {
            continue;
        }
        let name = line.split('=').next().unwrap_or_default().trim().trim_matches('"');
        match value_of(line, "package") {
            Some(package) => out.insert(package.to_owned()),
            // `git = "..."` on a line of its own: a `[dependencies.name]` table. The lock file names it.
            None if name == "git" || name.is_empty() => false,
            None => out.insert(name.to_owned()),
        };
    }
    out
}

/// The packages a lock file builds from the git address `git`.
pub fn locked_from(lock: &str, git: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut name = "";
    for line in lock.lines().map(str::trim) {
        if line == "[[package]]" {
            name = "";
        } else if let Some(n) = line.strip_prefix("name = \"").and_then(|v| v.strip_suffix('"')) {
            name = n;
        } else if let Some(source) = line.strip_prefix("source = \"git+") {
            let address = source.split(['?', '#', '"']).next().unwrap_or_default();
            if same_address(address, git) && !name.is_empty() {
                out.insert(name.to_owned());
            }
        }
    }
    out
}

/// A lock file without the packages it builds from the git addresses in `linked`. Cargo keeps
/// what a lock file holds, and a package it still holds from an address is not replaced by a
/// linked checkout of another version; taken out, each is resolved again, to the checkout.
/// Everything else stays locked as it was.
pub fn without_linked(lock: &str, linked: &[&str]) -> String {
    let from_linked = |block: &str| {
        let source = block.lines().find_map(|l| l.trim().strip_prefix("source = \"git+"));
        source.is_some_and(|s| linked.iter().any(|git| same_address(s.split(['?', '#', '"']).next().unwrap_or_default(), git)))
    };
    let mut out = String::new();
    let mut block = String::new();
    for line in lock.split_inclusive('\n') {
        if line.trim() == "[[package]]" || (line.trim().starts_with('[') && !block.is_empty()) {
            if !from_linked(&block) {
                out.push_str(&block);
            }
            block.clear();
        }
        block.push_str(line);
    }
    if !from_linked(&block) {
        out.push_str(&block);
    }
    out
}

/// The name in the `[package]` table of a manifest.
fn package_name(manifest: &str) -> Option<&str> {
    let mut in_package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && line.split('=').next().is_some_and(|key| key.trim() == "name") {
            return value_of(line, "name");
        }
    }
    None
}

fn collect_packages(dir: &Path, depth: usize, out: &mut BTreeMap<String, PathBuf>) {
    if let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml"))
        && let Some(name) = package_name(&manifest)
    {
        out.entry(name.to_owned()).or_insert_with(|| dir.to_path_buf());
    }
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for sub in dirs {
        let name = sub.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !name.starts_with('.') && name != "target" && name != "node_modules" {
            collect_packages(&sub, depth + 1, out);
        }
    }
}

/// Every package in a checkout: name → its folder. Build output, hidden folders and anything
/// deeper than a workspace keeps its members are not read.
pub fn packages_in(checkout: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    if !checkout.join("Cargo.toml").is_file() {
        return Err(format!("{} is not a Cargo checkout (no Cargo.toml)", checkout.display()));
    }
    let mut out = BTreeMap::new();
    collect_packages(checkout, 0, &mut out);
    Ok(out)
}

/// `path` as an absolute path without `.` and `..` parts, read from `base` when it is relative.
/// Nothing is asked of the file system, so a link through a symlinked folder stays as written.
pub fn absolute(base: &Path, path: &str) -> PathBuf {
    let joined = base.join(path);
    let mut out = PathBuf::new();
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// A path as a TOML string. Forward slashes, which Cargo reads on every platform.
fn toml_string(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('\\', "/").replace('"', "\\\""))
}

/// The link file: what is linked (as comment lines [`read_links`] reads back), where the linked
/// build keeps its lock file, and a `[patch]` table per link.
pub fn render(app: &Path, links: &[(Link, BTreeMap<String, PathBuf>)]) -> String {
    let mut s = String::from(
        "# Written by `cargo xtask dev-link`; `cargo xtask dev-unlink` removes it. Local only: Git ignores this\n\
         # folder, and the paths below are this machine's.\n",
    );
    for (link, _) in links {
        s.push_str(&format!("# link: {} | {} | {}\n", link.name, link.git, link.dir.to_string_lossy().replace('\\', "/")));
    }
    s.push_str(&format!("\n[resolver]\nlockfile-path = {}\n", toml_string(&app.join(LINK_DIR).join("Cargo.lock"))));
    for (link, packages) in links {
        s.push_str(&format!("\n[patch.\"{}\"]\n", link.git));
        for (name, dir) in packages {
            s.push_str(&format!("{name} = {{ path = {} }}\n", toml_string(dir)));
        }
    }
    s
}

/// The links a link file records.
pub fn read_links(text: &str) -> Vec<Link> {
    let lines = text.lines().filter_map(|l| l.strip_prefix("# link: "));
    let parts = lines.filter_map(|l| {
        let mut parts = l.splitn(3, " | ");
        Some(Link { name: parts.next()?.to_owned(), git: parts.next()?.to_owned(), dir: PathBuf::from(parts.next()?) })
    });
    parts.collect()
}

/// The links in place in an app; empty when it is not linked.
pub fn links(app: &Path) -> Vec<Link> {
    read_links(&std::fs::read_to_string(app.join(LINK_DIR).join("config.toml")).unwrap_or_default())
}

/// One line for a person: `core -> /path, other -> /path`.
pub fn describe(links: &[Link]) -> String {
    links.iter().map(|l| format!("{} -> {}", l.name, l.dir.display())).collect::<Vec<_>>().join(", ")
}

/// What `cargo xtask dev-link` is asked to link: `<folder>` is the core, `<name>=<folder>` a
/// provider the app declares. A relative folder is read from the app's root, whatever folder
/// the command was run in.
pub fn parse_request(app: &Path, arg: &str, providers: &[Provider]) -> Result<Link, String> {
    let (name, git, folder) = match arg.split_once('=') {
        None => (CORE_NAME, CORE_GIT, arg),
        Some((name, folder)) if name == CORE_NAME => (CORE_NAME, CORE_GIT, folder),
        Some((name, folder)) => match providers.iter().find(|p| p.name == name) {
            Some(p) => (p.name, p.git, folder),
            None => {
                let known: Vec<&str> = std::iter::once(CORE_NAME).chain(providers.iter().map(|p| p.name)).collect();
                return Err(format!(
                    "'{name}' is not something this app builds on (known: {}). A provider is declared in `providers` in xtask/src/main.rs",
                    known.join(", ")
                ));
            }
        },
    };
    if folder.is_empty() {
        return Err(format!("{arg}: no folder given"));
    }
    Ok(Link { name: name.to_owned(), git: git.to_owned(), dir: absolute(app, folder) })
}

/// The packages to patch for one link: what the app takes from the address (its manifest and its
/// lock file), each found in the checkout.
pub fn linked_packages(link: &Link, manifest: &str, lock: &str) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut wanted = git_dependencies(manifest, &link.git);
    wanted.extend(locked_from(lock, &link.git));
    if wanted.is_empty() {
        return Err(format!("this app takes nothing from {} (no dependency names that address)", link.git));
    }
    let found = packages_in(&link.dir)?;
    let mut out = BTreeMap::new();
    let mut missing = Vec::new();
    for name in wanted {
        match found.get(&name) {
            Some(dir) => {
                out.insert(name, dir.clone());
            }
            None => missing.push(name),
        }
    }
    if missing.is_empty() {
        Ok(out)
    } else {
        Err(format!("{} is not a checkout of {}: it has no package {}", link.dir.display(), link.git, missing.join(", ")))
    }
}

/// The packages among `names` that `cargo metadata` output still takes from a git address: with
/// a link in effect each is a local folder, which has no source.
pub fn still_from_git(meta: &json::Value, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for p in meta.get("packages").map(json::Value::items).unwrap_or_default() {
        if let Some(name) = p.str_of("name").filter(|n| names.contains(n))
            && p.str_of("source").is_some_and(|source| source.starts_with("git+"))
        {
            out.push(name.to_owned());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Why a link cannot be written into this app yet, if it cannot.
fn not_ready(app: &Path) -> Option<String> {
    let config = std::fs::read_to_string(app.join(".cargo/config.toml")).unwrap_or_default();
    if !config.lines().any(|l| l.trim() == INCLUDE) {
        return Some(format!(
            ".cargo/config.toml does not read a link yet. It needs the core's `cargo` block, which holds the line `{INCLUDE}`: \
             `cargo xtask sync` writes it"
        ));
    }
    // 0: ignored. 1: not ignored. Anything else: no repository here, so nothing can be committed.
    let probe = format!("{LINK_DIR}/config.toml");
    let ignored = std::process::Command::new("git").args(["check-ignore", "--quiet", &probe]).current_dir(app).status();
    if ignored.is_ok_and(|s| s.code() == Some(1)) {
        return Some(format!("Git does not ignore {LINK_DIR}/ here. The core's block in .gitignore does: `cargo xtask sync` writes it"));
    }
    None
}

/// `cargo xtask dev-link [<core folder>] [<name>=<folder> ...]`. With nothing to link it says
/// what is linked.
pub fn link_task(app: &Path, args: &[String], providers: &[Provider]) -> TaskResult {
    if args.is_empty() {
        let now = links(app);
        let names: Vec<String> = providers.iter().map(|p| format!(" [{}=<folder>]", p.name)).collect();
        println!("usage: cargo xtask dev-link [<folder of open-source-core>]{}", names.concat());
        println!("       cargo xtask dev-unlink");
        println!(
            "{}",
            if now.is_empty() { "not linked: this app builds on its pinned releases".to_owned() } else { format!("linked: {}", describe(&now)) }
        );
        return Ok(());
    }
    if let Some(why) = not_ready(app) {
        return Err(why);
    }
    let read = |p: &str| std::fs::read_to_string(app.join(p)).map_err(|e| format!("cannot read {p}: {e}"));
    let (manifest, lock) = (read("Cargo.toml")?, read("Cargo.lock")?);
    let mut all = links(app);
    for arg in args {
        let link = parse_request(app, arg, providers)?;
        all.retain(|l| l.name != link.name);
        all.push(link);
    }
    let mut with_packages = Vec::new();
    for link in all {
        let packages = linked_packages(&link, &manifest, &lock)?;
        with_packages.push((link, packages));
    }
    let dir = app.join(LINK_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    // The linked build starts from the app's own lock file, less what is linked, so that only
    // the linked packages are resolved again.
    let addresses: Vec<&str> = with_packages.iter().map(|(link, _)| link.git.as_str()).collect();
    std::fs::write(dir.join("Cargo.lock"), without_linked(&lock, &addresses)).map_err(|e| format!("cannot write {LINK_DIR}/Cargo.lock: {e}"))?;
    std::fs::write(dir.join("config.toml"), render(app, &with_packages)).map_err(|e| format!("cannot write {LINK_DIR}/config.toml: {e}"))?;
    // Bring the linked lock file in line now, so every `--locked` command of the gate works, and
    // see that Cargo took the link.
    let wanted: Vec<&str> = with_packages.iter().flat_map(|(_, packages)| packages.keys().map(String::as_str)).collect();
    let taken =
        cargo_output(app, &["metadata", "--format-version", "1"]).and_then(|out| json::parse(&out)).map(|meta| still_from_git(&meta, &wanted));
    // A Cargo older than the one rust-toolchain.toml names ignores `resolver.lockfile-path` and
    // writes the app's own lock file. Put that back, and do not link with that Cargo.
    let rewritten = std::fs::read_to_string(app.join("Cargo.lock")).is_ok_and(|now| now != lock);
    if rewritten {
        std::fs::write(app.join("Cargo.lock"), &lock).map_err(|e| format!("cannot restore Cargo.lock: {e}"))?;
    }
    let older = "It needs the Cargo that rust-toolchain.toml names: an older one does not read an included configuration file, or does not \
                 keep the linked build's lock file apart";
    let problem = match taken {
        Ok(left) if left.is_empty() && !rewritten => None,
        Ok(left) if left.is_empty() => Some(format!("Cargo wrote the app's own Cargo.lock instead of the linked build's (it is restored). {older}")),
        Ok(left) => Some(format!("Cargo did not take the link: {} still come from git. {older}", left.join(", "))),
        Err(e) => Some(format!("the link does not resolve: {e}")),
    };
    if let Some(problem) = problem {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!("{problem}. The link was not kept"));
    }
    for (link, packages) in &with_packages {
        println!("linked {} -> {} ({} packages)", link.name, link.dir.display(), packages.len());
    }
    println!(
        "The link is in {LINK_DIR}/, which Git ignores; Cargo.lock is not touched while linked. `cargo build`, tests and an editor now build \
         against it.\n`cargo xtask ci --linked` runs the gate against the link; `cargo xtask dev-unlink` removes it."
    );
    Ok(())
}

/// `cargo xtask dev-link <core dir>`, as an app's `main.rs` from before the shared gate calls it.
pub fn dev_link(app: &Path, core: &str) -> TaskResult {
    link_task(app, &[core.to_owned()], &[])
}

/// `cargo xtask dev-unlink`: remove every link, and a link section an older tool wrote into
/// `.cargo/config.toml`. The app's own `Cargo.lock` was never changed, so there is nothing to
/// restore; if the manifests changed while linked, it says how to bring the lock file in line.
pub fn dev_unlink(app: &Path) -> TaskResult {
    let dir = app.join(LINK_DIR);
    let was_linked = dir.exists();
    if was_linked {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("cannot remove {}: {e}", dir.display()))?;
    }
    let config = app.join(".cargo/config.toml");
    if let Ok(old) = std::fs::read_to_string(&config)
        && old.contains(OLD_LINK_BEGIN)
    {
        let new = remove_lines(&old, OLD_LINK_BEGIN, OLD_LINK_END);
        std::fs::write(&config, new).map_err(|e| format!("cannot write {}: {e}", config.display()))?;
        println!(
            "removed the link section an older `dev-link` wrote into .cargo/config.toml; if Cargo.lock changed while it was there, `git restore Cargo.lock`"
        );
    }
    println!("unlinked: the app builds on its pinned releases again");
    if was_linked && cargo_output(app, &["metadata", "--format-version", "1", "--locked"]).is_err() {
        println!(
            "note: Cargo.lock does not match the manifests (a dependency changed while linked?). \
             `cargo metadata --format-version 1 >/dev/null` brings it in line, once everything the manifests name is released"
        );
    }
    Ok(())
}

/// What `.cargo/config.toml` holds that points a build at a local folder.
pub fn config_problems(config: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, line) in config.lines().enumerate() {
        let line = line.trim();
        let at = format!(".cargo/config.toml:{}", i + 1);
        if line.starts_with(OLD_LINK_BEGIN) {
            out.push(format!("{at}: a dev-link section written by an older tool (`cargo xtask dev-unlink` removes it)"));
        } else if line.starts_with("[patch") {
            out.push(format!("{at}: a [patch] table. A dev-link lives in {LINK_DIR}/, never in a tracked file"));
        } else if line.contains('=') && line.split('=').next().is_some_and(|key| key.trim() == "paths") {
            out.push(format!("{at}: a `paths` override. A dev-link lives in {LINK_DIR}/, never in a tracked file"));
        }
    }
    out
}

/// The `[patch]` entries of a manifest that point at a local folder.
pub fn manifest_problems(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_patch = false;
    for (i, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            in_patch = line.starts_with("[patch");
        } else if in_patch && !line.starts_with('#') && value_of(line, "path").is_some() {
            out.push(format!("Cargo.toml:{}: a [patch] entry with a local path. Use `cargo xtask dev-link`, which changes no tracked file", i + 1));
        }
    }
    out
}

/// What the tracked files hold that only a link may hold.
fn tracked_problems(app: &Path) -> Vec<String> {
    let read = |p: &str| std::fs::read_to_string(app.join(p)).unwrap_or_default();
    let mut out = config_problems(&read(".cargo/config.toml"));
    out.extend(manifest_problems(&read("Cargo.toml")));
    if git_output(app, &["ls-files", "--", LINK_DIR]).is_ok_and(|tracked| !tracked.trim().is_empty()) {
        out.push(format!("{LINK_DIR}/ is tracked by Git. It holds this machine's paths: `git rm -r --cached {LINK_DIR}`"));
    }
    out
}

fn fail(problems: Vec<String>) -> TaskResult {
    for p in &problems {
        eprintln!("  - {p}");
    }
    Err(format!("{} problem(s): a build pointed at a local folder must never be committed", problems.len()))
}

/// `cargo xtask unlinked --tracked` (the pre-commit hook): no tracked file holds a link. A link
/// in `.cargo/dev-link/` is fine here, because Git ignores it and a commit cannot carry it.
// An app's `main.rs` from before the shared gate never calls this, and declares this module
// itself; allowed here so that it builds without a warning until it is moved over.
#[allow(dead_code)]
pub fn check_tracked(app: &Path) -> TaskResult {
    let problems = tracked_problems(app);
    if !problems.is_empty() {
        return fail(problems);
    }
    match links(app) {
        now if now.is_empty() => println!("ok: no dev-link in a tracked file"),
        now => println!("ok: no dev-link in a tracked file (linked locally: {})", describe(&now)),
    }
    Ok(())
}

/// The gate's `unlinked` task: no tracked file holds a link, and no link is in place. The gate
/// says what CI will say, and CI builds the pinned releases.
pub fn check_unlinked(app: &Path) -> TaskResult {
    let problems = tracked_problems(app);
    if !problems.is_empty() {
        return fail(problems);
    }
    let now = links(app);
    if !now.is_empty() {
        return Err(format!(
            "this app is linked to local checkouts ({}), and CI builds the pinned releases. `cargo xtask dev-unlink` removes the link; \
             `cargo xtask ci --linked` runs the gate against it",
            describe(&now)
        ));
    }
    println!("ok: no dev-link");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "[workspace]\n\n# BEGIN jcj patch block\n[patch.x]\na = 1\n# END jcj patch block\n";
    const OTHER: &str = "https://example.invalid/other-app";

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jcj-link-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(root: &Path, file: &str, text: &str) {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn blocks_extract_and_replace() {
        let b = extract_block(M, BLOCK_BEGIN, BLOCK_END).unwrap();
        assert!(b.starts_with(BLOCK_BEGIN) && b.ends_with(BLOCK_END));
        let new = "# BEGIN jcj patch block\n[patch.y]\n# END jcj patch block";
        let r = replace_block(M, new, BLOCK_BEGIN, BLOCK_END);
        assert!(r.contains("[patch.y]") && !r.contains("[patch.x]"));
        let appended = replace_block("[workspace]\n", new, BLOCK_BEGIN, BLOCK_END);
        assert_eq!(appended, format!("[workspace]\n\n{new}\n"));
    }

    #[test]
    fn an_old_link_section_is_removed_whole() {
        let old = "[alias]\nxtask = \"run -p xtask --\"\n\n# BEGIN jcj dev-link (local only)\n[patch.\"x\"]\njcj-doc = { path = \"/c\" }\n# END jcj dev-link\n";
        assert_eq!(remove_lines(old, OLD_LINK_BEGIN, OLD_LINK_END), "[alias]\nxtask = \"run -p xtask --\"\n");
        assert_eq!(remove_lines("x\n", OLD_LINK_BEGIN, OLD_LINK_END), "x\n");
    }

    #[test]
    fn a_relative_folder_is_read_from_the_app_root() {
        let app = Path::new("/work/app");
        assert_eq!(absolute(app, "../open-source-core"), PathBuf::from("/work/open-source-core"));
        assert_eq!(absolute(app, "./vendor/../core/"), PathBuf::from("/work/app/core"));
        assert_eq!(absolute(app, "/elsewhere/core"), PathBuf::from("/elsewhere/core"));
        let link = parse_request(app, "../open-source-core", &[]).unwrap();
        assert_eq!((link.name.as_str(), link.git.as_str(), link.dir), (CORE_NAME, CORE_GIT, PathBuf::from("/work/open-source-core")));
    }

    #[test]
    fn a_provider_is_linked_by_the_name_the_app_declares() {
        let app = Path::new("/work/app");
        let providers = [Provider { name: "other", git: OTHER }];
        let link = parse_request(app, "other=../other-app", &providers).unwrap();
        assert_eq!(link, Link { name: "other".to_owned(), git: OTHER.to_owned(), dir: PathBuf::from("/work/other-app") });
        assert_eq!(parse_request(app, "core=../c", &providers).unwrap().git, CORE_GIT);
        let err = parse_request(app, "third=../x", &providers).unwrap_err();
        assert!(err.contains("known: core, other") && err.contains("providers"), "{err}");
        assert!(parse_request(app, "other=", &providers).is_err());
    }

    #[test]
    fn what_an_app_takes_from_an_address_is_read_from_its_manifest_and_lock() {
        let manifest = format!(
            "jcj-doc = {{ git = \"{CORE_GIT}\", tag = \"v0.1.9\" }}\n# jcj-old = {{ git = \"{CORE_GIT}\" }}\n\
             model = {{ package = \"other-model\", git = \"{OTHER}.git\", tag = \"v1\" }}\nserde = \"1\"\n\
             [dependencies.jcj-proof]\ngit = \"{CORE_GIT}\"\n"
        );
        assert_eq!(git_dependencies(&manifest, CORE_GIT).into_iter().collect::<Vec<_>>(), ["jcj-doc"]);
        assert_eq!(git_dependencies(&manifest, OTHER).into_iter().collect::<Vec<_>>(), ["other-model"]);
        let lock = format!(
            "[[package]]\nname = \"jcj-upstream\"\nversion = \"0.1.9\"\nsource = \"git+{CORE_GIT}?tag=v0.1.9#abc\"\n\n\
             [[package]]\nname = \"app\"\nversion = \"1.0.0\"\n\n\
             [[package]]\nname = \"serde\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n\
             [[package]]\nname = \"other-format\"\nversion = \"1.0.0\"\nsource = \"git+{OTHER}?tag=v1#def\"\n"
        );
        assert_eq!(locked_from(&lock, CORE_GIT).into_iter().collect::<Vec<_>>(), ["jcj-upstream"]);
        assert_eq!(locked_from(&lock, OTHER).into_iter().collect::<Vec<_>>(), ["other-format"]);
    }

    #[test]
    fn the_linked_lock_file_lets_go_of_what_is_linked_and_nothing_else() {
        let registry = "registry+https://github.com/rust-lang/crates.io-index";
        let package = |name: &str, source: &str| {
            format!("[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\nsource = \"{source}\"\ndependencies = [\n \"serde\",\n]\n\n")
        };
        let app = "[[package]]\nname = \"app\"\nversion = \"1.0.0\"\ndependencies = [\n \"jcj-doc\",\n]\n\n";
        let (core, other, serde) = (
            package("jcj-doc", &format!("git+{CORE_GIT}?tag=v0.1.9#abc")),
            package("other-model", &format!("git+{OTHER}?tag=v1#def")),
            package("serde", registry),
        );
        let lock = format!("# made by Cargo\nversion = 4\n\n{app}{core}{other}{serde}[metadata]\nx = \"y\"\n");
        assert_eq!(without_linked(&lock, &[CORE_GIT]), format!("# made by Cargo\nversion = 4\n\n{app}{other}{serde}[metadata]\nx = \"y\"\n"));
        assert_eq!(without_linked(&lock, &[CORE_GIT, OTHER]), format!("# made by Cargo\nversion = 4\n\n{app}{serde}[metadata]\nx = \"y\"\n"));
        assert_eq!(without_linked(&lock, &[]), lock);
        // The last package of a file with nothing after it.
        assert_eq!(without_linked(&format!("version = 4\n\n{serde}{core}"), &[CORE_GIT]), format!("version = 4\n\n{serde}"));
    }

    #[test]
    fn the_link_file_names_each_address_and_reads_back() {
        let tmp = temp("render");
        let (core, other, app) = (tmp.join("core"), tmp.join("other"), tmp.join("app"));
        put(&core, "Cargo.toml", "[workspace]\n");
        put(&core, "crates/jcj-doc/Cargo.toml", "[package]\nname = \"jcj-doc\"\n");
        put(&core, "crates/jcj-upstream/Cargo.toml", "[package]\nname = \"jcj-upstream\"\n");
        put(&core, "crates/jcj-board/Cargo.toml", "[package]\nname = \"jcj-board\"\n");
        put(&core, "target/debug/build/x/Cargo.toml", "[package]\nname = \"not-read\"\n");
        // A package whose folder is not its name, as an app's crates often are.
        put(&other, "Cargo.toml", "[workspace]\n");
        put(&other, "crates/model/Cargo.toml", "[package]\nedition = \"2024\"\nname = \"other-model\"\n\n[dependencies]\nname = \"1\"\n");
        let manifest = format!("jcj-doc = {{ git = \"{CORE_GIT}\", tag = \"v0.1.9\" }}\nother-model = {{ git = \"{OTHER}\", tag = \"v1\" }}\n");
        let lock = format!("[[package]]\nname = \"jcj-upstream\"\nversion = \"0.1.9\"\nsource = \"git+{CORE_GIT}?tag=v0.1.9#abc\"\n");
        let core_link = Link { name: CORE_NAME.to_owned(), git: CORE_GIT.to_owned(), dir: core.clone() };
        let other_link = Link { name: "other".to_owned(), git: OTHER.to_owned(), dir: other.clone() };

        // Only what the app takes is patched: jcj-board is in the checkout and not in the app.
        let core_packages = linked_packages(&core_link, &manifest, &lock).unwrap();
        assert_eq!(core_packages.keys().collect::<Vec<_>>(), ["jcj-doc", "jcj-upstream"]);
        let other_packages = linked_packages(&other_link, &manifest, &lock).unwrap();
        assert_eq!(other_packages.get("other-model"), Some(&other.join("crates/model")));
        assert!(!packages_in(&core).unwrap().contains_key("not-read"));

        let text = render(&app, &[(core_link.clone(), core_packages), (other_link.clone(), other_packages)]);
        assert!(text.contains(&format!("[patch.\"{CORE_GIT}\"]\njcj-doc = {{ path = ")), "{text}");
        assert!(text.contains(&format!("[patch.\"{OTHER}\"]\nother-model = {{ path = ")), "{text}");
        assert!(text.contains("[resolver]\nlockfile-path = ") && text.contains(".cargo/dev-link/Cargo.lock"), "{text}");
        assert_eq!(read_links(&text), [core_link.clone(), other_link]);

        // A folder that is not the repository asked for is refused, and names what is missing.
        let wrong = Link { dir: other.clone(), ..core_link };
        let err = linked_packages(&wrong, &manifest, &lock).unwrap_err();
        assert!(err.contains("no package jcj-doc, jcj-upstream"), "{err}");
        assert!(linked_packages(&wrong, "serde = \"1\"\n", "").unwrap_err().contains("takes nothing"));
        assert!(packages_in(&tmp.join("nowhere")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn a_link_cargo_did_not_take_is_noticed() {
        let meta = |doc_source: &str| {
            let text = format!(
                r#"{{"packages":[{{"name":"jcj-doc","source":{doc_source}}},{{"name":"jcj-shell","source":null}},{{"name":"serde","source":"registry+https://example.invalid/index"}}]}}"#
            );
            json::parse(&text).unwrap()
        };
        let names = ["jcj-doc", "jcj-shell"];
        assert!(still_from_git(&meta("null"), &names).is_empty());
        assert_eq!(still_from_git(&meta(r#""git+https://example.invalid/core?tag=v0.1.9#abc""#), &names), ["jcj-doc"]);
    }

    #[test]
    fn a_tracked_file_may_not_hold_a_link() {
        let fine = format!("# BEGIN open-source-core: cargo\n{INCLUDE}\n\n[alias]\nxtask = \"run -p xtask --\"\n# END open-source-core: cargo\n");
        assert!(config_problems(&fine).is_empty());
        let old = format!("{fine}\n{OLD_LINK_BEGIN} (local only)\n[patch.\"{CORE_GIT}\"]\njcj-doc = {{ path = \"/c/jcj-doc\" }}\n{OLD_LINK_END}\n");
        let problems = config_problems(&old);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("older tool") && problems[1].contains("[patch] table"));
        assert_eq!(config_problems("paths = [\"/c/jcj-doc\"]\n").len(), 1);
        assert!(config_problems("# paths = []\n[env]\nSEARCH_PATHS = \"x\"\n").is_empty());

        // The patch block points at git commits and is fine; a hand-written local path is not.
        assert!(manifest_problems(M).is_empty());
        assert!(manifest_problems("[patch.crates-io]\nkrilla = { git = \"https://example.invalid/k\", rev = \"abc\" }\n").is_empty());
        let by_hand =
            format!("[dependencies]\nmine = {{ path = \"crates/mine\" }}\n\n[patch.\"{OTHER}\"]\nother-model = {{ path = \"../other/model\" }}\n");
        let problems = manifest_problems(&by_hand);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("Cargo.toml:5:"));
    }

    #[test]
    fn the_gate_refuses_a_link_and_the_commit_hook_does_not() {
        let app = temp("checks");
        put(&app, ".cargo/config.toml", &format!("{INCLUDE}\n"));
        put(&app, "Cargo.toml", "[workspace]\n");
        assert!(check_unlinked(&app).is_ok() && check_tracked(&app).is_ok());
        let link = Link { name: CORE_NAME.to_owned(), git: CORE_GIT.to_owned(), dir: PathBuf::from("/work/core") };
        put(&app, &format!("{LINK_DIR}/config.toml"), &render(&app, &[(link, BTreeMap::new())]));
        assert_eq!(describe(&links(&app)), "core -> /work/core");
        let err = check_unlinked(&app).unwrap_err();
        assert!(err.contains("core -> /work/core") && err.contains("ci --linked"), "{err}");
        assert!(check_tracked(&app).is_ok());
        // Unlinking removes the folder and changes no other file.
        dev_unlink(&app).unwrap();
        assert!(!app.join(LINK_DIR).exists());
        assert_eq!(std::fs::read_to_string(app.join(".cargo/config.toml")).unwrap(), format!("{INCLUDE}\n"));
        assert!(check_unlinked(&app).is_ok());
        // A link in a tracked file fails both.
        put(&app, ".cargo/config.toml", "[patch.crates-io]\nx = { path = \"/x\" }\n");
        assert!(check_unlinked(&app).is_err() && check_tracked(&app).is_err());
        let _ = std::fs::remove_dir_all(&app);
    }

    #[test]
    fn a_link_is_not_written_where_cargo_would_not_read_it() {
        let app = temp("ready");
        put(&app, ".cargo/config.toml", "[alias]\nxtask = \"run -p xtask --\"\n");
        let err = link_task(&app, &["../core".to_owned()], &[]).unwrap_err();
        assert!(err.contains("`cargo` block"), "{err}");
        assert!(!app.join(LINK_DIR).exists());
        let _ = std::fs::remove_dir_all(&app);
    }
}
