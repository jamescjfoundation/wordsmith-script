// SPDX-License-Identifier: MIT OR Apache-2.0
//! Which workspace packages a change can affect, so local gates and CI test only those.
//!
//! - A changed file inside a package affects that package and every workspace package that
//!   depends on it (directly or not).
//! - A changed file that another package's Rust sources name affects that package too: a design
//!   or a font under the root `assets/`, a fixture or a golden file under the root `tests/`, a
//!   page of `docs/` that a test compares, a file of another package. See [`named_by`] for what
//!   counts as naming a file.
//! - A changed file under the root `assets/` or `tests/` that no package names affects
//!   everything: those folders hold what packages read, so a reader this module failed to find
//!   is still tested.
//! - A change to a workspace-wide file (the manifests, the lock file, the toolchain or lint
//!   settings) affects everything.
//! - Anything else (documentation no test reads, workflows, scripts) affects nothing that needs
//!   building. The checks that read such files need no build and always run.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::json::{self, Value};
use crate::util::{TaskResult, cargo_output, git_output};

/// Files whose change can affect every package.
const GLOBAL: &[&str] =
    &["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml", "deny.toml", ".cargo/config.toml", "build.rs"];

/// Root folders that hold what packages read: designs, fonts, translations, fixtures, golden
/// files. A changed file there that no package's sources name affects everything.
const SHARED_INPUTS: &[&str] = &["assets", "tests"];

/// The package that is the gate itself. The files it names are the ones its checks read, and
/// the checks always run, so they are not inputs of its tests.
const GATE: &str = "xtask";

/// The workspace packages: name → (directory relative to the root, workspace packages it uses).
pub type Graph = BTreeMap<String, (String, BTreeSet<String>)>;

/// What each package's sources name outside its own folder: package → paths from the root.
pub type Named = BTreeMap<String, BTreeSet<String>>;

/// What a change affects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Affected {
    /// Every package.
    All,
    /// These packages (sorted; possibly empty when the change touches nothing a package reads).
    Some(Vec<String>),
}

impl Affected {
    /// `-p name` arguments for cargo, or `--workspace`.
    pub fn cargo_args(&self) -> Vec<String> {
        match self {
            Affected::All => vec!["--workspace".to_owned()],
            Affected::Some(names) => names.iter().flat_map(|n| ["-p".to_owned(), n.clone()]).collect(),
        }
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Affected::Some(v) if v.is_empty())
    }
}

/// The workspace packages: name → (directory relative to the root, workspace packages it uses).
pub fn workspace_graph(root: &Path) -> Result<Graph, String> {
    let out = cargo_output(root, &["metadata", "--format-version", "1", "--locked"])?;
    graph_from_metadata(&json::parse(&out)?, root)
}

fn graph_from_metadata(meta: &Value, root: &Path) -> Result<Graph, String> {
    let members: BTreeSet<&str> = meta.get("workspace_members").map(Value::items).unwrap_or_default().iter().filter_map(Value::as_str).collect();
    let mut names = BTreeMap::new();
    let mut graph = BTreeMap::new();
    let root_s = root.to_string_lossy().replace('\\', "/");
    for p in meta.get("packages").map(Value::items).unwrap_or_default() {
        let Some(id) = p.str_of("id") else { continue };
        if !members.contains(id) {
            continue;
        }
        let name = p.str_of("name").ok_or("a package without a name")?.to_owned();
        let manifest = p.str_of("manifest_path").ok_or("a package without a manifest")?.replace('\\', "/");
        let dir = manifest.trim_end_matches("/Cargo.toml").trim_start_matches(&root_s).trim_start_matches('/').to_owned();
        names.insert(id.to_owned(), name.clone());
        graph.insert(name, (dir, BTreeSet::new()));
    }
    for node in meta.get("resolve").and_then(|r| r.get("nodes")).map(Value::items).unwrap_or_default() {
        let Some(name) = node.str_of("id").and_then(|id| names.get(id)) else { continue };
        let deps: BTreeSet<String> = node
            .get("deps")
            .map(Value::items)
            .unwrap_or_default()
            .iter()
            .filter_map(|d| d.str_of("pkg"))
            .filter_map(|id| names.get(id).cloned())
            .collect();
        if let Some(entry) = graph.get_mut(name) {
            entry.1 = deps;
        }
    }
    Ok(graph)
}

/// The files changed since `base` (committed, staged, unstaged and untracked).
pub fn changed_files(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let mut files: BTreeSet<String> = git_output(root, &["diff", "--name-only", base])?.lines().map(str::to_owned).collect();
    files.extend(git_output(root, &["ls-files", "--others", "--exclude-standard"])?.lines().map(str::to_owned));
    Ok(files.into_iter().filter(|f| !f.is_empty()).collect())
}

/// The string literals of Rust source `src`, each with whether it is the argument of
/// `include_str!`, `include_bytes!`, `include!` or `#[path = ...]`, which name a file from the
/// folder of the source file. Comments are left out. A literal is given as written, escapes
/// and all.
fn string_literals(src: &str) -> Vec<(&str, bool)> {
    let b = src.as_bytes();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match at(i) {
            b'/' if at(i + 1) == b'/' => {
                while i < b.len() && at(i) != b'\n' {
                    i += 1;
                }
            }
            b'/' if at(i + 1) == b'*' => {
                let mut depth = 1;
                i += 2;
                while i < b.len() && depth > 0 {
                    match (at(i), at(i + 1)) {
                        (b'/', b'*') => (depth, i) = (depth + 1, i + 2),
                        (b'*', b'/') => (depth, i) = (depth - 1, i + 2),
                        _ => i += 1,
                    }
                }
            }
            // A character (`'"'`, `'\''`, `'é'`) is stepped over; a lifetime is one quote.
            b'\'' if at(i + 1) == b'\\' => {
                i += 3;
                while i < b.len() && at(i) != b'\'' {
                    i += 1;
                }
                i += 1;
            }
            b'\'' => {
                let len = match at(i + 1) {
                    c if c >= 0xF0 => 4,
                    c if c >= 0xE0 => 3,
                    c if c >= 0xC0 => 2,
                    _ => 1,
                };
                i += if at(i + 1 + len) == b'\'' { len + 2 } else { 1 };
            }
            b'"' => {
                // `r"..."`, `r#"..."#`, with `b` or `c` before the `r`: no escapes inside.
                let hashes = b.get(..i).map_or(0, |before| before.iter().rev().take_while(|c| **c == b'#').count());
                let r = i - hashes;
                let raw =
                    r > 0 && at(r - 1) == b'r' && (r < 2 || !ident(at(r - 2)) || (matches!(at(r - 2), b'b' | b'c') && (r < 3 || !ident(at(r - 3)))));
                let prefix = if raw { r - 1 } else { i };
                let start = i + 1;
                let mut end = start;
                if raw {
                    while end < b.len() && !(at(end) == b'"' && (1..=hashes).all(|h| at(end + h) == b'#')) {
                        end += 1;
                    }
                } else {
                    while end < b.len() && at(end) != b'"' {
                        end += if at(end) == b'\\' { 2 } else { 1 };
                    }
                }
                let end = end.min(b.len());
                let before = src.get(..prefix).unwrap_or_default().trim_end();
                let from_file = ["include_str!(", "include_bytes!(", "include!("].iter().any(|m| before.ends_with(m))
                    || before.strip_suffix('=').is_some_and(|b| b.trim_end().strip_suffix("path").is_some_and(|b| b.trim_end().ends_with("#[")));
                if let Some(text) = src.get(start..end) {
                    out.push((text, from_file));
                }
                i = end + 1 + if raw { hashes } else { 0 };
            }
            _ => i += 1,
        }
    }
    out
}

/// A literal as path segments: leading empty and placeholder segments (`"{}/../x"`, `"/../x"`)
/// are dropped, and the path ends before the first later segment with a placeholder or a
/// wildcard (`"fixtures/{name}.txt"` names `fixtures`). `None` for what cannot be a path.
fn segments(literal: &str) -> Option<Vec<&str>> {
    if literal.is_empty() || literal.len() > 240 || literal.contains(['\\', '\n', '"']) || literal.contains("://") {
        return None;
    }
    let open = |s: &str| s.contains(['{', '}', '*']);
    let segs: Vec<&str> = literal.split('/').skip_while(|s| s.is_empty() || open(s)).take_while(|s| !open(s)).filter(|s| !s.is_empty()).collect();
    if segs.is_empty() { None } else { Some(segs) }
}

/// `segs` followed from the folder `base` (a path from the root, `""` for the root itself), as
/// a path from the root. `None` when it leaves the root.
fn resolve(base: &str, segs: &[&str]) -> Option<String> {
    let mut path: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in segs {
        match *seg {
            "." => {}
            ".." => {
                path.pop()?;
            }
            s => path.push(s),
        }
    }
    Some(path.join("/"))
}

/// The paths outside its own folder that a package names, as paths from the root. `dir` is the
/// package's folder and `sources` its Rust files: (path from the root, text).
///
/// Sources are read as text, not compiled, so this is a careful guess with rules that err on
/// the side of testing too much:
///
/// - the argument of `include_str!`, `include_bytes!`, `include!` and `#[path = ...]` is
///   followed from the source file's folder;
/// - any other string literal is followed from the package's folder, which is where
///   `CARGO_MANIFEST_DIR` points and where tests run, so `"../../tests/fixtures"` names the
///   root's `tests/fixtures`;
/// - a literal without `..` is also taken from the root and from each folder the package names
///   by dots alone (`"../.."`), since tests build paths as `root().join("docs/guide/keys.md")`.
///   Not a bare word, though: `"assets"` or `"tools"` is far more often a key or a label than
///   a folder at the root, and counting it would hide a shared input that nobody names.
///
/// A path named this way stands for the file or for everything under the folder. What is not
/// seen: a path put together from pieces that are not literals in the package, or from bare
/// words (`root().join("docs").join("guide")`).
pub fn named_by(dir: &str, sources: &[(&str, &str)]) -> BTreeSet<String> {
    let mut bases: BTreeSet<String> = BTreeSet::from([String::new()]);
    let mut plain: Vec<Vec<&str>> = Vec::new();
    let mut named = BTreeSet::new();
    for (file, text) in sources {
        let folder = file.rsplit_once('/').map_or("", |(folder, _)| folder);
        for (literal, from_file) in string_literals(text) {
            let Some(segs) = segments(literal) else { continue };
            if from_file {
                named.extend(resolve(folder, &segs));
            } else if segs.iter().all(|s| matches!(*s, "." | "..")) {
                bases.extend(resolve(dir, &segs));
            } else {
                named.extend(resolve(dir, &segs));
                if !segs.contains(&"..") && (segs.len() > 1 || segs.iter().any(|s| s.contains('.'))) {
                    plain.push(segs);
                }
            }
        }
    }
    for segs in &plain {
        named.extend(bases.iter().filter_map(|base| resolve(base, segs)));
    }
    // What lies in the package itself says nothing new, and the root is everything.
    named.retain(|p| !p.is_empty() && p != dir && !p.starts_with(&format!("{dir}/")));
    named
}

/// The Rust files under `folder` (`rel` from the root), leaving out build output, hidden
/// folders and the folders of other packages.
fn rust_files(folder: &Path, rel: &str, other_packages: &BTreeSet<&str>, depth: usize, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        match entry.file_type() {
            Ok(t) if t.is_dir() && depth < 24 && !name.starts_with('.') && name != "target" && !other_packages.contains(child.as_str()) => {
                rust_files(&entry.path(), &child, other_packages, depth + 1, out);
            }
            Ok(t) if t.is_file() && name.ends_with(".rs") => out.push(child),
            _ => {}
        }
    }
}

/// [`named_by`] for every package of the workspace but the gate's own.
pub fn named_paths(root: &Path, graph: &Graph) -> Named {
    let dirs: BTreeSet<&str> = graph.values().map(|(dir, _)| dir.as_str()).collect();
    let mut out = Named::new();
    for (name, (dir, _)) in graph {
        if name == GATE || dir.is_empty() {
            continue;
        }
        let mut others = dirs.clone();
        others.remove(dir.as_str());
        let mut files = Vec::new();
        rust_files(&root.join(dir), dir, &others, 0, &mut files);
        let texts: Vec<(String, String)> =
            files.into_iter().filter_map(|f| std::fs::read_to_string(root.join(&f)).ok().map(|text| (f, text))).collect();
        let sources: Vec<(&str, &str)> = texts.iter().map(|(f, text)| (f.as_str(), text.as_str())).collect();
        out.insert(name.clone(), named_by(dir, &sources));
    }
    out
}

/// What the files hit before dependents are added: package → the first file that hit it and
/// why. `Err` holds the file that affects everything, and why.
fn hits(files: &[String], graph: &Graph, named: &Named) -> Result<BTreeMap<String, String>, String> {
    let mut hit = BTreeMap::new();
    for f in files {
        if GLOBAL.contains(&f.as_str()) || f.starts_with(".cargo/") {
            return Err(format!("{f} is a workspace-wide file"));
        }
        let mut found = false;
        // The deepest package directory that holds the file.
        let owner = graph
            .iter()
            .filter(|(_, (dir, _))| !dir.is_empty() && (f == dir || f.starts_with(&format!("{dir}/"))))
            .max_by_key(|(_, (dir, _))| dir.len())
            .map(|(name, _)| name.clone());
        match owner {
            Some(name) => {
                found = true;
                hit.entry(name).or_insert_with(|| format!("{f} is in it"));
            }
            // Shared xtask modules live in template/ and are compiled into the core's xtask.
            None if f.starts_with("template/xtask/") && graph.contains_key(GATE) => {
                found = true;
                hit.entry(GATE.to_owned()).or_insert_with(|| format!("{f} is compiled into it"));
            }
            None => {}
        }
        // Every package whose sources name the file, or a folder it is in.
        let folders: Vec<&str> = f.match_indices('/').filter_map(|(i, _)| f.get(..i)).chain([f.as_str()]).collect();
        for (name, paths) in named {
            if folders.iter().any(|p| paths.contains(*p)) {
                found = true;
                hit.entry(name.clone()).or_insert_with(|| format!("its sources name {f} or a folder it is in"));
            }
        }
        if !found && SHARED_INPUTS.iter().any(|folder| f.starts_with(&format!("{folder}/"))) {
            return Err(format!("{f} is a shared input no package's sources name"));
        }
    }
    Ok(hit)
}

/// What `files` affect, given the workspace graph and what each package's sources name.
pub fn affected_by(files: &[String], graph: &Graph, named: &Named) -> Affected {
    let Ok(direct) = hits(files, graph, named) else { return Affected::All };
    let mut hit: BTreeSet<String> = direct.into_keys().collect();
    // Add everything that depends on a hit package, until nothing changes.
    loop {
        let before = hit.len();
        for (name, (_, deps)) in graph {
            if deps.iter().any(|d| hit.contains(d)) {
                hit.insert(name.clone());
            }
        }
        if hit.len() == before {
            break;
        }
    }
    Affected::Some(hit.into_iter().collect())
}

/// The packages a change since `base` affects.
pub fn affected(root: &Path, base: &str) -> Result<Affected, String> {
    let files = changed_files(root, base)?;
    if files.is_empty() {
        return Ok(Affected::Some(Vec::new()));
    }
    let graph = workspace_graph(root)?;
    Ok(affected_by(&files, &graph, &named_paths(root, &graph)))
}

/// `--base <ref>` from `args`, or `origin/main` when this clone has it, else `HEAD`.
pub fn base_ref(root: &Path, args: &[String]) -> String {
    if let Some(base) = args.iter().position(|a| a == "--base").and_then(|i| args.get(i + 1)) {
        return base.clone();
    }
    match git_output(root, &["rev-parse", "--verify", "--quiet", "origin/main"]) {
        Ok(_) => "origin/main".to_owned(),
        Err(_) => "HEAD".to_owned(),
    }
}

/// `cargo xtask affected [--base <ref>]`: what `ci --changed` would build and test, as one line
/// a script can read: `all`, the packages separated by spaces, or an empty line when the change
/// affects no package. Why each package is in it goes to standard error. The shared CI workflow
/// asks this before it restores a build cache.
pub fn task(root: &Path, args: &[String]) -> TaskResult {
    let base = base_ref(root, args);
    let files = changed_files(root, &base)?;
    if files.is_empty() {
        println!();
        return Ok(());
    }
    let graph = workspace_graph(root)?;
    let named = named_paths(root, &graph);
    match hits(&files, &graph, &named) {
        Err(why) => eprintln!("every package: {why}"),
        Ok(direct) => direct.iter().for_each(|(name, why)| eprintln!("{name}: {why}")),
    }
    match affected_by(&files, &graph, &named) {
        Affected::All => println!("all"),
        Affected::Some(packages) => println!("{}", packages.join(" ")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> Graph {
        let mut g = BTreeMap::new();
        let deps = |d: &[&str]| d.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
        g.insert("a".to_owned(), ("crates/a".to_owned(), deps(&[])));
        g.insert("b".to_owned(), ("crates/b".to_owned(), deps(&["a"])));
        g.insert("c".to_owned(), ("crates/c".to_owned(), deps(&["b"])));
        g.insert("d".to_owned(), ("crates/d".to_owned(), deps(&[])));
        g.insert("xtask".to_owned(), ("xtask".to_owned(), deps(&[])));
        g
    }

    fn files(f: &[&str]) -> Vec<String> {
        f.iter().map(|s| s.to_string()).collect()
    }

    fn named(n: &[(&str, &[&str])]) -> Named {
        n.iter().map(|(name, paths)| (name.to_string(), paths.iter().map(|p| p.to_string()).collect())).collect()
    }

    fn nothing() -> Named {
        Named::new()
    }

    #[test]
    fn a_change_reaches_its_dependents() {
        assert_eq!(affected_by(&files(&["crates/a/src/lib.rs"]), &graph(), &nothing()), Affected::Some(files(&["a", "b", "c"])));
        assert_eq!(affected_by(&files(&["crates/c/src/lib.rs"]), &graph(), &nothing()), Affected::Some(files(&["c"])));
        assert_eq!(affected_by(&files(&["crates/d/tests/x.rs", "crates/b/src/x.rs"]), &graph(), &nothing()), Affected::Some(files(&["b", "c", "d"])));
    }

    #[test]
    fn docs_affect_nothing_and_manifests_everything() {
        assert!(affected_by(&files(&["README.md", "docs/x.md", ".github/workflows/ci.yml", "tools/make.py"]), &graph(), &nothing()).is_empty());
        assert_eq!(affected_by(&files(&["Cargo.lock"]), &graph(), &nothing()), Affected::All);
        assert_eq!(affected_by(&files(&[".cargo/config.toml"]), &graph(), &nothing()), Affected::All);
        assert_eq!(affected_by(&files(&["template/xtask/src/util.rs"]), &graph(), &nothing()), Affected::Some(files(&["xtask"])));
        // A file whose name only starts like a package directory is not in it.
        assert!(affected_by(&files(&["crates/ab/x.rs"]), &graph(), &nothing()).is_empty());
    }

    #[test]
    fn a_file_a_package_names_is_its_input() {
        let named =
            named(&[("b", &["assets/designs", "docs/guide/keys.md", "tests/fixtures"]), ("d", &["crates/a/tests/golden", "assets/fonts/x.ttf"])]);
        // A built-in design under a named folder: the reader and what depends on it.
        assert_eq!(affected_by(&files(&["assets/designs/classic.toml"]), &graph(), &named), Affected::Some(files(&["b", "c"])));
        // A font named as one file, a fixture deep under a named folder.
        assert_eq!(affected_by(&files(&["assets/fonts/x.ttf"]), &graph(), &named), Affected::Some(files(&["d"])));
        assert_eq!(affected_by(&files(&["tests/fixtures/novels/one/text.md"]), &graph(), &named), Affected::Some(files(&["b", "c"])));
        // A page a test compares is an input; the page beside it is not.
        assert_eq!(affected_by(&files(&["docs/guide/keys.md"]), &graph(), &named), Affected::Some(files(&["b", "c"])));
        assert!(affected_by(&files(&["docs/guide/start.md", "docs/guide/keys.md.bak"]), &graph(), &named).is_empty());
        // A file of one package that another package reads: both, and their dependents.
        assert_eq!(affected_by(&files(&["crates/a/tests/golden/one.pdf"]), &graph(), &named), Affected::Some(files(&["a", "b", "c", "d"])));
    }

    #[test]
    fn a_shared_input_nobody_names_affects_everything() {
        let named = named(&[("b", &["assets/designs"])]);
        assert_eq!(affected_by(&files(&["assets/brand/icon.png"]), &graph(), &named), Affected::All);
        assert_eq!(affected_by(&files(&["tests/golden/one.pdf"]), &graph(), &nothing()), Affected::All);
        // Only the root's folders: a package's own tests/ belongs to the package.
        assert_eq!(affected_by(&files(&["crates/d/tests/golden/one.pdf"]), &graph(), &nothing()), Affected::Some(files(&["d"])));
        assert!(affected_by(&files(&["assets.md", "testsuite/x"]), &graph(), &nothing()).is_empty());
    }

    #[test]
    fn says_why() {
        let named = named(&[("b", &["docs/guide"])]);
        let direct = hits(&files(&["docs/guide/keys.md", "crates/a/src/lib.rs"]), &graph(), &named).unwrap();
        assert_eq!(direct.get("a").unwrap(), "crates/a/src/lib.rs is in it");
        assert!(direct.get("b").unwrap().contains("docs/guide/keys.md"));
        assert!(hits(&files(&["rustfmt.toml"]), &graph(), &named).unwrap_err().contains("workspace-wide"));
        assert!(hits(&files(&["assets/x"]), &graph(), &named).unwrap_err().contains("no package's sources name"));
    }

    #[test]
    fn finds_string_literals_and_not_comments() {
        let src = r##"
            // a "comment" is not a literal
            /* nor /* a nested */ "one" */
            const A: &str = include_str!("../a.txt");
            const B: &[u8] = include_bytes!(
                "../b.bin"
            );
            #[path = "../shared/mod.rs"]
            mod shared;
            fn f<'a>(x: &'a str) -> char { let _ = ('"', '\'', b'"', 'é', '\u{e9}'); let _ = "an \" escaped"; x.len(); 'x' }
            const R: &str = r#"raw "quoted" text"#;
            const P: &str = "plain";
            const S: &[u8] = br"bytes";
        "##;
        let found = string_literals(src);
        let texts: Vec<&str> = found.iter().map(|(t, _)| *t).collect();
        assert_eq!(texts, ["../a.txt", "../b.bin", "../shared/mod.rs", r#"an \" escaped"#, r#"raw "quoted" text"#, "plain", "bytes"]);
        let from_file: Vec<bool> = found.iter().map(|(_, f)| *f).collect();
        assert_eq!(from_file, [true, true, true, false, false, false, false]);
        // Source that ends inside a string or a comment is read to its end, not past it.
        assert_eq!(string_literals("let a = \"open"), [("open", false)]);
        assert!(string_literals("/* open").is_empty());
    }

    #[test]
    fn follows_a_literal_as_a_path() {
        assert_eq!(segments("../../tests/fixtures/"), Some(vec!["..", "..", "tests", "fixtures"]));
        assert_eq!(segments("{}/../../crates/x/golden/{name}.txt"), Some(vec!["..", "..", "crates", "x", "golden"]));
        assert_eq!(segments("/../.."), Some(vec!["..", ".."]));
        assert_eq!(segments("{name}.txt"), None);
        assert_eq!(segments("https://example.invalid/a"), None);
        assert_eq!(segments("C:\\x"), None);
        assert_eq!(resolve("crates/a", &["..", "..", "tests"]), Some("tests".to_owned()));
        assert_eq!(resolve("crates/a/src", &[".", "x.txt"]), Some("crates/a/src/x.txt".to_owned()));
        assert_eq!(resolve("crates/a", &["..", ".."]), Some(String::new()));
        assert_eq!(resolve("crates/a", &["..", "..", ".."]), None);
    }

    #[test]
    fn names_what_a_package_reads_outside_itself() {
        let lib = r#"
            static FONT: &[u8] = include_bytes!("../../../assets/fonts/One.ttf");
            const OWN: &str = include_str!("../tokens/own.json");
        "#;
        let test = r#"
            fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..") }
            fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures") }
            fn keys() { let _ = root().join("docs/guide/keys.md"); let _ = std::fs::read_dir(root().join("docs/guide")); }
            fn golden(name: &str) -> String { format!("{}/../other/tests/golden/{name}.pdf", env!("CARGO_MANIFEST_DIR")) }
            fn own() -> PathBuf { Path::new("tests/golden").join("x") }
            fn sibling() { let up = Path::new(env!("CARGO_MANIFEST_DIR")).join(".."); let _ = up.join("third/data"); }
            const FOLDERS: [&str; 2] = ["assets", "tools"];
            fn readme() { let _ = root().join("README.md"); let _ = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging"); }
        "#;
        let named = named_by("crates/a", &[("crates/a/src/lib.rs", lib), ("crates/a/tests/all.rs", test)]);
        let expected = [
            "assets/fonts/One.ttf",
            "tests/fixtures",
            "docs/guide/keys.md",
            "docs/guide",
            "crates/other/tests/golden",
            "crates/third/data",
            "README.md",
            "packaging",
        ];
        for path in expected {
            assert!(named.contains(path), "{path} in {named:?}");
        }
        // A bare word is not taken for a folder at the root.
        assert!(!named.contains("assets") && !named.contains("tools"), "{named:?}");
        // Nothing inside the package, and never the root itself.
        assert!(!named.iter().any(|p| p.is_empty() || p == "crates/a" || p.starts_with("crates/a/")), "{named:?}");
        // "tests/golden" is also taken from the root: a guess that can only add a test run.
        assert!(named.contains("tests/golden"));
    }

    #[test]
    fn cargo_arguments() {
        assert_eq!(Affected::All.cargo_args(), vec!["--workspace"]);
        assert_eq!(Affected::Some(files(&["a", "b"])).cargo_args(), vec!["-p", "a", "-p", "b"]);
    }
}
