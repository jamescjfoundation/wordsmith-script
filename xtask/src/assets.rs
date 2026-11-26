// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask assets`: every file an app ships under `assets/` has its source and license on
//! record, and the legal files are there.
//!
//! - Every file under `assets/` that Git tracks or would add is named in `ATTRIBUTION.md`, in
//!   backticks: by its own path (`` `assets/fonts/Serif.otf` ``) or by a folder above it
//!   (`` `assets/fonts/` ``). The folder `assets/` itself names nothing: it would cover every
//!   file ever added.
//! - No asset is a symbolic link, which could lead out of the repository.
//! - The legal files every release carries exist ([`LEGAL`], and the ones the app adds).
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::Path;

use super::util::{TaskResult, git_output, report};

/// The folder checked, and the file that must name what is in it.
pub const ASSETS: &str = "assets";
pub const ATTRIBUTION: &str = "ATTRIBUTION.md";
/// Files every app repository carries.
pub const LEGAL: &[&str] = &["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", ATTRIBUTION, "SECURITY.md"];
/// Files that hold a folder open and are nobody's work.
const PLACEHOLDERS: &[&str] = &[".gitkeep"];

/// Whether `file`, or a folder above it and below `assets/`, appears in the attribution text in
/// backticks.
pub fn listed(attribution: &str, file: &str) -> bool {
    let mut path = file;
    loop {
        let is_top = path.trim_end_matches('/') == ASSETS;
        if !is_top && (attribution.contains(&format!("`{path}`")) || attribution.contains(&format!("`{path}/`"))) {
            return true;
        }
        match path.rfind('/') {
            Some(i) => path = &path[..i],
            None => return false,
        }
    }
}

/// The assets among `files` (paths from the root, `/` between parts) that `attribution` does not
/// name. Placeholder files are left out.
pub fn unlisted(files: &[String], attribution: &str) -> Vec<String> {
    let is_placeholder = |f: &str| PLACEHOLDERS.contains(&f.rsplit('/').next().unwrap_or_default());
    files.iter().filter(|f| !is_placeholder(f) && !listed(attribution, f)).cloned().collect()
}

/// The files under `assets/` that Git tracks or would add, and that are on disk.
fn asset_files(root: &Path) -> Result<Vec<String>, String> {
    let out = git_output(root, &["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", ASSETS])?;
    let mut files: Vec<String> = out.split('\0').filter(|f| !f.is_empty()).map(str::to_owned).collect();
    files.retain(|f| std::fs::symlink_metadata(root.join(f)).is_ok());
    files.sort();
    files.dedup();
    Ok(files)
}

/// `cargo xtask assets`. `more_legal` are the legal files this app carries beyond [`LEGAL`].
pub fn task(root: &Path, more_legal: &[&str]) -> TaskResult {
    let mut failures = Vec::new();
    for file in LEGAL.iter().chain(more_legal) {
        if !root.join(file).is_file() {
            failures.push(format!("{file} is missing"));
        }
    }
    let files = asset_files(root)?;
    for file in &files {
        if std::fs::symlink_metadata(root.join(file)).is_ok_and(|m| m.file_type().is_symlink()) {
            failures.push(format!("{file} is a symbolic link; an asset is a real file"));
        }
    }
    let attribution = std::fs::read_to_string(root.join(ATTRIBUTION)).unwrap_or_default();
    for file in unlisted(&files, &attribution) {
        failures.push(format!("{file} is not listed in {ATTRIBUTION} (in backticks, by its path or a folder above it)"));
    }
    report(failures, &format!("{} asset files are listed in {ATTRIBUTION}, and the legal files exist", files.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_asset_is_listed_by_its_path_or_a_folder_above_it() {
        let text = "| `assets/fonts/` | open fonts |\n| `assets/brand/tokens.json` | ours |\n| assets/brand/bare.svg | no backticks |";
        assert!(listed(text, "assets/fonts/serif/a.otf"));
        assert!(listed(text, "assets/brand/tokens.json"));
        assert!(!listed(text, "assets/brand/mark.svg"));
        assert!(!listed(text, "assets/brand/bare.svg"));
        assert!(listed("see `assets/print` for the rules", "assets/print/rules.json"));
        // Naming the whole folder covers nothing.
        assert!(!listed("everything in `assets/` is ours, and `assets` too", "assets/brand/mark.svg"));
    }

    #[test]
    fn placeholders_need_no_entry() {
        let files = ["assets/fonts/a.otf", "assets/empty/.gitkeep", "assets/brand/mark.svg"].map(str::to_owned);
        assert_eq!(unlisted(&files, "`assets/fonts/a.otf`"), ["assets/brand/mark.svg"]);
        assert!(unlisted(&files, "`assets/fonts/` and `assets/brand/`").is_empty());
    }
}
