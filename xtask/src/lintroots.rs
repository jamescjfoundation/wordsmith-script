// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every product crate root must forbid `unsafe` and deny the panicking lints (core decision 0003).
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Core decisions: <https://github.com/jamescjfoundation/open-source-core/tree/main/docs/decisions>.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::util::{TaskResult, report, workspace_members};

/// Lints every product crate root must deny.
pub const REQUIRED_DENIES: &[&str] =
    &["clippy::unwrap_used", "clippy::expect_used", "clippy::panic", "clippy::unimplemented", "clippy::todo", "clippy::unreachable"];

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

/// The inner attributes at the top of a crate root (`#![...]`), each as its text between the
/// brackets with whitespace removed. Comments are skipped; reading stops at the first thing that
/// is neither, so an attribute that is commented out, or that appears later, does not count.
fn inner_attributes(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source.trim_start_matches('\u{feff}');
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix("//") {
            rest = r.split_once('\n').map_or("", |(_, after)| after);
        } else if let Some(r) = rest.strip_prefix("/*") {
            // Block comments nest in Rust.
            let mut depth = 1usize;
            let mut chars = r.char_indices();
            let mut end = r.len();
            while let Some((i, c)) = chars.next() {
                let next = r.get(i + c.len_utf8()..).and_then(|s| s.chars().next());
                match (c, next) {
                    ('/', Some('*')) => {
                        depth += 1;
                        chars.next();
                    }
                    ('*', Some('/')) => {
                        depth -= 1;
                        chars.next();
                        if depth == 0 {
                            end = i + 2;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            rest = r.get(end..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix("#!").map(str::trim_start).and_then(|r| r.strip_prefix('[')) {
            let mut depth = 1usize;
            let mut end = None;
            for (i, c) in r.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            out.push(r.get(..end).unwrap_or("").split_whitespace().collect());
            rest = r.get(end + 1..).unwrap_or("");
        } else {
            break;
        }
    }
    out
}

/// The lints an inner attribute list sets to `level` (`deny`, `forbid`).
fn lints_at(attrs: &[String], level: &str) -> BTreeSet<String> {
    attrs
        .iter()
        .filter_map(|a| a.strip_prefix(level)?.strip_prefix('(')?.strip_suffix(')'))
        .flat_map(|list| list.split(',').map(str::to_owned))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Problems with one crate root's lint attributes, empty if it is fine. `no_indexing` also
/// requires `clippy::indexing_slicing`.
pub fn lint_root_problems(source: &str, no_indexing: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let attrs = inner_attributes(source);
    let forbidden = lints_at(&attrs, "forbid");
    if !forbidden.contains("unsafe_code") {
        problems.push("missing #![forbid(unsafe_code)]".to_owned());
    }
    // A forbidden lint is denied too, and more.
    let denied: BTreeSet<String> = lints_at(&attrs, "deny").into_iter().chain(forbidden).collect();
    let extra: &[&str] = if no_indexing { &["clippy::indexing_slicing"] } else { &[] };
    for lint in REQUIRED_DENIES.iter().chain(extra) {
        if !denied.contains(*lint) {
            problems.push(format!("missing #![deny({lint})]"));
        }
    }
    problems
}

pub fn check_lint_roots(root: &Path, no_indexing_crates: &[&str], skip_crates: &[&str]) -> TaskResult {
    let mut failures = Vec::new();
    for member in workspace_members(root)? {
        if skip_crates.contains(&member.name.as_str()) {
            continue;
        }
        let roots = crate_roots(&member.dir);
        if roots.is_empty() {
            failures.push(format!("{}: no crate root found", member.name));
        }
        for file in roots {
            let source = std::fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            for problem in lint_root_problems(&source, no_indexing_crates.contains(&member.name.as_str())) {
                failures.push(format!("{}: {problem}", file.display()));
            }
        }
    }
    report(failures, "crate roots carry the required lint attributes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_roots_detect_missing_attributes() {
        let good = "#![forbid(unsafe_code)]\n#![deny(\n clippy::unwrap_used,\n clippy::expect_used,\n clippy::panic,\n clippy::unimplemented,\n clippy::todo,\n clippy::unreachable\n)]\n";
        assert!(lint_root_problems(good, false).is_empty());

        let missing_todo = good.replace("clippy::todo,", "");
        assert_eq!(lint_root_problems(&missing_todo, false), vec!["missing #![deny(clippy::todo)]"]);

        let no_forbid = good.replace("#![forbid(unsafe_code)]", "");
        assert_eq!(lint_root_problems(&no_forbid, false), vec!["missing #![forbid(unsafe_code)]"]);

        // Lints named outside a deny block do not count.
        let warn_only = good.replace("#![deny(", "#![warn(");
        assert_eq!(lint_root_problems(&warn_only, false).len(), REQUIRED_DENIES.len());

        // Crates that may not index must deny that too.
        assert_eq!(lint_root_problems(good, true), vec!["missing #![deny(clippy::indexing_slicing)]"]);
        let with_indexing = format!("{good}#![deny(clippy::indexing_slicing)]\n");
        assert!(lint_root_problems(&with_indexing, true).is_empty());
    }

    #[test]
    fn lint_roots_read_attributes_not_text() {
        let good = "// SPDX-License-Identifier: MIT\n//! Docs.\n/* a /* nested */ comment */\n#![forbid(unsafe_code)]\n#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]\n\npub mod a;\n";
        assert!(lint_root_problems(good, false).is_empty(), "{:?}", lint_root_problems(good, false));
        // Commented out, the attributes do not count.
        let commented = good.replace("#![forbid", "// #![forbid").replace("#![deny", "/* #![deny").replace("unreachable)]\n", "unreachable)] */\n");
        assert_eq!(lint_root_problems(&commented, false).len(), 1 + REQUIRED_DENIES.len(), "{commented}");
        // Nor do they after the first item, or inside a string.
        let late = "pub mod a;\n#![forbid(unsafe_code)]\n";
        assert!(lint_root_problems(late, false).contains(&"missing #![forbid(unsafe_code)]".to_owned()));
        let in_string = "const S: &str = \"#![forbid(unsafe_code)]\";\n";
        assert!(lint_root_problems(in_string, false).contains(&"missing #![forbid(unsafe_code)]".to_owned()));
        // A lint whose name only contains a required one does not count for it.
        let longer = good.replace("clippy::todo,", "clippy::todo_extra,");
        assert_eq!(lint_root_problems(&longer, false), vec!["missing #![deny(clippy::todo)]"]);
        // A forbidden lint counts as denied; an attribute split over lines still reads.
        let forbid_all = "#![forbid(\n unsafe_code,\n clippy::unwrap_used, clippy::expect_used, clippy::panic,\n clippy::unimplemented, clippy::todo, clippy::unreachable\n)]\n";
        assert!(lint_root_problems(forbid_all, false).is_empty());
        assert_eq!(inner_attributes("#![doc = \"[x]\"]\n#![allow(dead_code)]"), vec!["doc=\"[x]\"".to_owned(), "allow(dead_code)".to_owned()]);
    }
}
