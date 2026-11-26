// SPDX-License-Identifier: MIT OR Apache-2.0
//! `THIRD-PARTY-LICENSES.txt`: every crate the shipped programs are built from, with its license
//! text, for the release bundles (core decision 0001). Many of those licenses (MIT, BSD, ISC,
//! Unicode-3.0) ask for their notice in binary copies too.
//!
//! The list comes from `cargo metadata` (the locked dependency graph of the shipped programs,
//! normal and build dependencies, for every target), and the texts from the
//! license files in each package's source, which Cargo has already downloaded. A package that
//! ships no license file gets the standard text of its license, said as such; a license we have
//! no standard text for stops the bundle.
//!
//! Core decisions: <https://github.com/jamescjfoundation/open-source-core/tree/main/docs/decisions>.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::json::{self, Value};
use crate::util::{TaskResult, cargo_output};

/// The file name in every bundle.
pub const FILE_NAME: &str = "THIRD-PARTY-LICENSES.txt";

/// Larger "license files" are not license texts.
const MAX_LICENSE_BYTES: u64 = 512 * 1024;

/// How far up a git checkout to look for the repository's license (crates in a workspace often
/// keep it only at the top).
const MAX_WALK_UP: usize = 6;

/// One third-party package.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub authors: Vec<String>,
    /// The folder holding its `Cargo.toml`.
    pub dir: PathBuf,
    /// From a git checkout (its license may sit at the checkout's top).
    pub git: bool,
}

impl Package {
    fn label(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

/// The third-party packages in the shipped programs' dependency graph, sorted.
/// `shipped`: the programs a bundle ships; their dependency graphs are what the file covers.
pub fn shipped_packages(root: &Path, shipped: &[&str]) -> Result<Vec<Package>, String> {
    let out = cargo_output(root, &["metadata", "--format-version", "1", "--locked"])?;
    packages_from_metadata(&json::parse(&out)?, shipped)
}

fn packages_from_metadata(meta: &Value, shipped: &[&str]) -> Result<Vec<Package>, String> {
    let mut by_id: BTreeMap<&str, &Value> = BTreeMap::new();
    for p in meta.get("packages").ok_or("cargo metadata has no packages")?.items() {
        by_id.insert(p.str_of("id").ok_or("a package without an id")?, p);
    }
    let mut deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in meta.get("resolve").and_then(|r| r.get("nodes")).ok_or("cargo metadata has no resolve graph")?.items() {
        let id = node.str_of("id").ok_or("a graph node without an id")?;
        let shipped_deps = node
            .get("deps")
            .map(Value::items)
            .unwrap_or_default()
            .iter()
            .filter(|d| {
                // Normal (null) and build dependencies; dev dependencies are not shipped.
                d.get("dep_kinds").map(Value::items).unwrap_or_default().iter().any(|k| k.str_of("kind").is_none_or(|kind| kind == "build"))
            })
            .filter_map(|d| d.str_of("pkg"))
            .collect();
        deps.insert(id, shipped_deps);
    }
    let mut todo: Vec<&str> = Vec::new();
    for name in shipped {
        let id = by_id
            .iter()
            .find(|(_, p)| p.str_of("name") == Some(name) && p.get("source") == Some(&Value::Null))
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("cargo metadata does not list the workspace package {name}"))?;
        todo.push(id);
    }
    let mut seen = BTreeSet::new();
    while let Some(id) = todo.pop() {
        if seen.insert(id) {
            todo.extend(deps.get(id).map(Vec::as_slice).unwrap_or_default());
        }
    }
    let mut packages = Vec::new();
    for id in seen {
        let p = by_id.get(id).ok_or_else(|| format!("the graph names {id}, which is not in the package list"))?;
        let source = match p.get("source") {
            Some(Value::String(s)) => s.as_str(),
            // Workspace and path packages are this project's own code.
            _ => continue,
        };
        let manifest = PathBuf::from(p.str_of("manifest_path").ok_or_else(|| format!("{id}: no manifest_path"))?);
        packages.push(Package {
            name: p.str_of("name").ok_or_else(|| format!("{id}: no name"))?.to_owned(),
            version: p.str_of("version").ok_or_else(|| format!("{id}: no version"))?.to_owned(),
            license: p.str_of("license").map(str::to_owned),
            license_file: p.str_of("license_file").map(str::to_owned),
            repository: p.str_of("repository").map(str::to_owned),
            authors: p.get("authors").map(Value::items).unwrap_or_default().iter().filter_map(Value::as_str).map(str::to_owned).collect(),
            dir: manifest.parent().map(Path::to_path_buf).ok_or_else(|| format!("{id}: manifest_path has no folder"))?,
            git: source.starts_with("git+"),
        });
    }
    packages.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(packages)
}

/// A file or folder whose name says it holds license terms or notices.
fn is_license_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["license", "licence", "copying", "copyright", "notice", "unlicense"].iter().any(|p| lower.starts_with(p))
}

/// The license files at the top of `dir`, including those in a `LICENSES/` folder, sorted.
fn license_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_name().to_str().is_some_and(is_license_name) {
            continue;
        }
        if path.is_file() {
            found.push(path);
        } else if path.is_dir()
            && let Ok(inner) = std::fs::read_dir(&path)
        {
            found.extend(inner.flatten().map(|e| e.path()).filter(|p| p.is_file()));
        }
    }
    found.sort();
    found
}

/// Where `package`'s license texts are: its own files, the manifest's `license-file`, or, for a
/// crate in a git checkout, the nearest folder above it that has them.
fn license_files(package: &Package) -> Vec<PathBuf> {
    let mut files = license_files_in(&package.dir);
    if let Some(named) = &package.license_file {
        let path = package.dir.join(named);
        if path.is_file() && !files.contains(&path) {
            files.push(path);
        }
    }
    if files.is_empty() && package.git {
        let mut dir = package.dir.as_path();
        for _ in 0..MAX_WALK_UP {
            // `.cargo-ok` marks the top of Cargo's checkout; never look above it.
            if dir.join(".cargo-ok").exists() || dir.join(".git").exists() {
                break;
            }
            let Some(parent) = dir.parent() else { break };
            dir = parent;
            files = license_files_in(dir);
            if !files.is_empty() {
                break;
            }
        }
    }
    files
}

/// Line endings and trailing spaces evened out, so identical texts are written once.
fn normalize(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let mut out = lines.join("\n").trim().to_owned();
    out.push('\n');
    out
}

fn read_license(path: &Path) -> Result<String, String> {
    let len = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?.len();
    if len > MAX_LICENSE_BYTES {
        return Err(format!("{}: {len} bytes is too large for a license file", path.display()));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(normalize(&String::from_utf8_lossy(&bytes)))
}

/// Standard texts for packages that ship none, in order of preference when a package offers a
/// choice. Apache-2.0 and MIT come from this repository's own license files.
struct Standard {
    apache: String,
    mit: String,
}

impl Standard {
    fn read(root: &Path) -> Result<Self, String> {
        let apache = read_license(&root.join("LICENSE-APACHE"))?;
        // Our MIT file without its copyright line: the package's own notice is not known.
        let mit = read_license(&root.join("LICENSE-MIT"))?;
        let mit = normalize(&mit.lines().filter(|l| !l.trim_start().starts_with("Copyright")).collect::<Vec<_>>().join("\n"));
        Ok(Standard { apache, mit })
    }

    fn text(&self, id: &str) -> Option<String> {
        match id {
            "Apache-2.0" => Some(self.apache.clone()),
            "MIT" => Some(self.mit.clone()),
            "BSD-3-Clause" => Some(normalize(BSD_3_CLAUSE)),
            "BSL-1.0" => Some(normalize(BSL_1_0)),
            _ => None,
        }
    }
}

const PREFERENCE: [&str; 4] = ["Apache-2.0", "MIT", "BSD-3-Clause", "BSL-1.0"];

/// The licenses whose standard texts stand in for a package's missing files: one choice of an
/// OR, every part of an AND.
fn standard_ids(expr: &str) -> Result<Vec<&'static str>, String> {
    let ids: Vec<&str> = expr
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '/'))
        .filter(|w| !w.is_empty() && !["AND", "OR", "WITH"].iter().any(|op| w.eq_ignore_ascii_case(op)))
        .collect();
    if expr.split_whitespace().any(|w| w.eq_ignore_ascii_case("AND")) {
        ids.iter().map(|id| PREFERENCE.iter().copied().find(|p| p == id).ok_or_else(|| format!("no standard text for {id} (in '{expr}')"))).collect()
    } else {
        PREFERENCE.iter().copied().find(|p| ids.contains(p)).map(|p| vec![p]).ok_or_else(|| format!("no standard text for any of '{expr}'"))
    }
}

/// Author names from a manifest, without email addresses.
fn author_names(authors: &[String]) -> String {
    authors.iter().map(|a| a.split('<').next().unwrap_or(a).trim()).filter(|a| !a.is_empty()).collect::<Vec<_>>().join(", ")
}

/// The file's text, for `packages` (sorted), with `standard` texts for those that ship none.
fn render(packages: &[Package], standard: &Standard, product: &str) -> Result<String, String> {
    // Each distinct text once, with the packages that carry it, in first-seen order.
    let mut texts: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut add = |heading: String, text: String, who: String| match texts.iter_mut().find(|(h, t, _)| *t == text && *h == heading) {
        Some((_, _, users)) => users.push(who),
        None => texts.push((heading, text, vec![who])),
    };
    let mut problems = Vec::new();
    let mut listing = String::new();
    for p in packages {
        let license = p.license.as_deref().unwrap_or("(none stated)");
        let repo = p.repository.as_deref().map(|r| format!("  {r}")).unwrap_or_default();
        listing.push_str(&format!("{}  {license}{repo}\n", p.label()));
        let files = license_files(p);
        if files.is_empty() {
            let Some(expr) = p.license.as_deref() else {
                problems.push(format!("{}: no license stated and no license file", p.label()));
                continue;
            };
            match standard_ids(expr) {
                Ok(ids) => {
                    for id in ids {
                        let text = standard.text(id).ok_or_else(|| format!("no standard text for {id}"))?;
                        let authors = author_names(&p.authors);
                        let who = if authors.is_empty() { p.label() } else { format!("{} (authors: {authors})", p.label()) };
                        add(format!("The standard {id} text, for packages that include no license file"), text, who);
                    }
                }
                Err(e) => problems.push(format!("{}: ships no license file, and {e}", p.label())),
            }
            continue;
        }
        for file in files {
            add("Used by".to_owned(), read_license(&file)?, p.label());
        }
    }
    if !problems.is_empty() {
        return Err(format!("third-party licenses: {}", problems.join("; ")));
    }
    let mut out = String::new();
    out.push_str("Third-party licenses\n====================\n\n");
    out.push_str(&format!(
        "{product} is built from the {} Rust packages listed below, besides its own code\n\
         (LICENSE-MIT, LICENSE-APACHE and NOTICE). Each is listed with the license its manifest\n\
         states. Their license texts follow, each written once with the packages that carry it.\n\
         The fonts built into the app are under the SIL Open Font License 1.1; their licenses are\n\
         the OFL-*.txt files beside this one.\n\n\
         Made by `cargo xtask third-party-licenses` from the project's Cargo.lock.\n\n",
        packages.len()
    ));
    out.push_str("Packages\n--------\n\n");
    out.push_str(&listing);
    out.push_str("\nLicense texts\n-------------\n");
    for (heading, text, users) in &texts {
        out.push_str(&format!("\n{}\n{heading}: {}\n{}\n\n{text}", "=".repeat(78), users.join(", "), "=".repeat(78)));
    }
    Ok(out)
}

/// The whole file for this workspace.
pub fn third_party_licenses(root: &Path, shipped: &[&str], product: &str) -> Result<String, String> {
    let packages = shipped_packages(root, shipped)?;
    if packages.is_empty() {
        return Err("no third-party packages found; cargo metadata looks wrong".into());
    }
    render(&packages, &Standard::read(root)?, product)
}

/// `cargo xtask third-party-licenses [file]`: writes the file (default `dist/THIRD-PARTY-LICENSES.txt`).
pub fn task(root: &Path, shipped: &[&str], product: &str, out: Option<&str>) -> TaskResult {
    let out = out.map_or_else(|| root.join("dist").join(FILE_NAME), PathBuf::from);
    let text = third_party_licenses(root, shipped, product)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("xtask: wrote {}", out.display());
    Ok(())
}

const BSD_3_CLAUSE: &str = "Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
";

const BSL_1_0: &str = "Boost Software License - Version 1.0 - August 17th, 2003

Permission is hereby granted, free of charge, to any person or organization obtaining a copy of the software and accompanying documentation covered by this license (the \"Software\") to use, reproduce, display, distribute, execute, and transmit the Software, and to prepare derivative works of the Software, and to permit third-parties to whom the Software is furnished to do so, all subject to the following:

The copyright notices in the Software and this entire statement, including the above license grant, this restriction and the following disclaimer, must be included in all copies of the Software, in whole or in part, and all derivative works of the Software, unless such copies or derivative works are solely in the form of machine-executable object code generated by a source language processor.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xtask-licenses-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn package(name: &str, license: &str, dir: PathBuf, git: bool) -> Package {
        Package {
            name: name.into(),
            version: "1.0.0".into(),
            license: Some(license.into()),
            license_file: None,
            repository: Some(format!("https://example.invalid/{name}")),
            authors: vec!["A. Writer <a@example.invalid>".into()],
            dir,
            git,
        }
    }

    fn standard() -> Standard {
        Standard { apache: "APACHE TEXT\n".into(), mit: "MIT TEXT\n".into() }
    }

    #[test]
    fn every_package_is_listed_with_its_texts_written_once() {
        let dir = temp("render");
        for name in ["a", "b"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            std::fs::write(dir.join(name).join("LICENSE-MIT"), "Copyright the authors\r\nPermission is granted.  \r\n").unwrap();
        }
        std::fs::create_dir_all(dir.join("c/LICENSES")).unwrap();
        std::fs::write(dir.join("c/LICENSES/BSD-3-Clause.txt"), "c's own BSD text").unwrap();
        std::fs::create_dir_all(dir.join("d")).unwrap();
        let packages = vec![
            package("a", "MIT", dir.join("a"), false),
            package("b", "MIT OR Apache-2.0", dir.join("b"), false),
            package("c", "BSD-3-Clause", dir.join("c"), false),
            package("d", "MIT/Apache-2.0", dir.join("d"), false),
        ];
        let text = render(&packages, &standard(), "Test").unwrap();
        assert!(text.contains("a 1.0.0  MIT  https://example.invalid/a\n"));
        assert!(text.contains("Used by: a 1.0.0, b 1.0.0\n"));
        assert_eq!(text.matches("Permission is granted.\n").count(), 1);
        assert!(text.contains("c's own BSD text"));
        // d ships no file: the standard Apache text stands in, said as such, without an email.
        assert!(text.contains("The standard Apache-2.0 text, for packages that include no license file: d 1.0.0 (authors: A. Writer)"));
        assert!(text.contains("APACHE TEXT"));
        assert!(!text.contains("example.invalid>"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_crate_in_a_git_checkout_uses_the_checkout_license() {
        let dir = temp("git");
        std::fs::write(dir.join(".cargo-ok"), "").unwrap();
        std::fs::write(dir.join("LICENSE-APACHE"), "the checkout's license").unwrap();
        std::fs::create_dir_all(dir.join("crates/x")).unwrap();
        let text = render(&[package("x", "MIT OR Apache-2.0", dir.join("crates/x"), true)], &standard(), "Test").unwrap();
        assert!(text.contains("the checkout's license") && text.contains("Used by: x 1.0.0"));
        // A registry crate never borrows from the folder above it.
        let text = render(&[package("x", "MIT", dir.join("crates/x"), false)], &standard(), "Test").unwrap();
        assert!(!text.contains("the checkout's license") && text.contains("MIT TEXT"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_license_without_a_text_stops_the_bundle() {
        let dir = temp("unknown");
        let err = render(&[package("z", "Zlib", dir.clone(), false)], &standard(), "Test").unwrap_err();
        assert!(err.contains("z 1.0.0") && err.contains("Zlib"), "{err}");
        assert_eq!(standard_ids("(MIT OR Apache-2.0) AND BSD-3-Clause"), Ok(vec!["MIT", "Apache-2.0", "BSD-3-Clause"]));
        assert_eq!(standard_ids("Zlib OR Apache-2.0 OR MIT"), Ok(vec!["Apache-2.0"]));
        assert!(standard_ids("MIT AND Unicode-3.0").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_graph_follows_shipped_dependencies_only() {
        let meta = json::parse(
            r#"{"packages":[
                {"id":"app","name":"app","version":"0.1.0","source":null,"manifest_path":"/w/apps/app/Cargo.toml","authors":[]},
                {"id":"cli","name":"app-cli","version":"0.1.0","source":null,"manifest_path":"/w/apps/cli/Cargo.toml","authors":[]},
                {"id":"lib","name":"lib","version":"1.0.0","source":"registry+https://github.com/rust-lang/crates.io-index","license":"MIT","manifest_path":"/r/lib/Cargo.toml","authors":[]},
                {"id":"gen","name":"gen","version":"2.0.0","source":"registry+https://github.com/rust-lang/crates.io-index","license":"MIT","manifest_path":"/r/gen/Cargo.toml","authors":[]},
                {"id":"testonly","name":"testonly","version":"1.0.0","source":"registry+https://github.com/rust-lang/crates.io-index","license":"MIT","manifest_path":"/r/t/Cargo.toml","authors":[]},
                {"id":"g","name":"craft-x","version":"0.1.0","source":"git+https://github.com/storytold/craft-libs?rev=1#1","license":"MIT","manifest_path":"/g/crates/x/Cargo.toml","authors":[]}
            ],"resolve":{"nodes":[
                {"id":"app","deps":[{"pkg":"lib","dep_kinds":[{"kind":null}]},{"pkg":"testonly","dep_kinds":[{"kind":"dev"}]}]},
                {"id":"cli","deps":[{"pkg":"g","dep_kinds":[{"kind":null,"target":"cfg(unix)"}]}]},
                {"id":"lib","deps":[{"pkg":"gen","dep_kinds":[{"kind":"build"}]}]},
                {"id":"gen","deps":[]},{"id":"testonly","deps":[]},{"id":"g","deps":[]}
            ]}}"#,
        )
        .unwrap();
        let names: Vec<(String, bool)> = packages_from_metadata(&meta, &["app", "app-cli"]).unwrap().into_iter().map(|p| (p.name, p.git)).collect();
        assert_eq!(names, [("craft-x".to_owned(), true), ("gen".to_owned(), false), ("lib".to_owned(), false)]);
    }
}
