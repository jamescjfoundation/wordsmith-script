// SPDX-License-Identifier: MIT OR Apache-2.0
//! Which workspace packages a change can affect, so local gates and CI test only those.
//!
//! A changed file inside a package affects that package and every workspace package that depends
//! on it (directly or not). A change to a workspace-wide file (the manifests, the lock file, the
//! toolchain or lint settings) affects everything. A change to documentation alone affects nothing
//! that needs building.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::json::{self, Value};
use crate::util::{cargo_output, git_output};

/// Files whose change can affect every package.
const GLOBAL: &[&str] =
    &["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml", "deny.toml", ".cargo/config.toml", "build.rs"];

/// What a change affects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Affected {
    /// Every package.
    All,
    /// These packages (sorted; possibly empty when only documentation changed).
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
pub fn workspace_graph(root: &Path) -> Result<BTreeMap<String, (String, BTreeSet<String>)>, String> {
    let out = cargo_output(root, &["metadata", "--format-version", "1", "--locked"])?;
    graph_from_metadata(&json::parse(&out)?, root)
}

fn graph_from_metadata(meta: &Value, root: &Path) -> Result<BTreeMap<String, (String, BTreeSet<String>)>, String> {
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

/// What `files` affect, given the workspace graph.
pub fn affected_by(files: &[String], graph: &BTreeMap<String, (String, BTreeSet<String>)>) -> Affected {
    let mut hit = BTreeSet::new();
    for f in files {
        if GLOBAL.contains(&f.as_str()) || f.starts_with(".cargo/") {
            return Affected::All;
        }
        // The deepest package directory that holds the file.
        let owner = graph
            .iter()
            .filter(|(_, (dir, _))| !dir.is_empty() && (f == dir || f.starts_with(&format!("{dir}/"))))
            .max_by_key(|(_, (dir, _))| dir.len())
            .map(|(name, _)| name.clone());
        match owner {
            Some(name) => {
                hit.insert(name);
            }
            // Shared xtask modules live in template/ and are compiled into the core's xtask.
            None if f.starts_with("template/xtask/") && graph.contains_key("xtask") => {
                hit.insert("xtask".to_owned());
            }
            None => {}
        }
    }
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
    Ok(affected_by(&files, &workspace_graph(root)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> BTreeMap<String, (String, BTreeSet<String>)> {
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

    #[test]
    fn a_change_reaches_its_dependents() {
        assert_eq!(affected_by(&files(&["crates/a/src/lib.rs"]), &graph()), Affected::Some(files(&["a", "b", "c"])));
        assert_eq!(affected_by(&files(&["crates/c/src/lib.rs"]), &graph()), Affected::Some(files(&["c"])));
        assert_eq!(affected_by(&files(&["crates/d/tests/x.rs", "crates/b/src/x.rs"]), &graph()), Affected::Some(files(&["b", "c", "d"])));
    }

    #[test]
    fn docs_affect_nothing_and_manifests_everything() {
        assert!(affected_by(&files(&["README.md", "docs/x.md"]), &graph()).is_empty());
        assert_eq!(affected_by(&files(&["Cargo.lock"]), &graph()), Affected::All);
        assert_eq!(affected_by(&files(&[".cargo/config.toml"]), &graph()), Affected::All);
        assert_eq!(affected_by(&files(&["template/xtask/src/util.rs"]), &graph()), Affected::Some(files(&["xtask"])));
        // A file whose name only starts like a package directory is not in it.
        assert!(affected_by(&files(&["crates/ab/x.rs"]), &graph()).is_empty());
    }

    #[test]
    fn cargo_arguments() {
        assert_eq!(Affected::All.cargo_args(), vec!["--workspace"]);
        assert_eq!(Affected::Some(files(&["a", "b"])).cargo_args(), vec!["-p", "a", "-p", "b"]);
    }
}
