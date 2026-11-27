// SPDX-License-Identifier: MIT OR Apache-2.0
//! The xtask modules every app shares, under one name.
//!
//! An app's `xtask/src/main.rs` declares this module and nothing else of the core's:
//! `mod shared;`. The list of shared modules is here, so a core release that adds one needs no
//! edit in an app. The files sit beside this one in `xtask/src/`; a module names another as
//! `super::name`.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Change it in the core, never in an app.

// Each side uses a part: the core's own xtask the core's half of `sync` and `docs`, an app's the
// app's half and the gate.
#![allow(dead_code)]

#[path = "applayers.rs"]
pub mod applayers;
#[path = "assets.rs"]
pub mod assets;
#[path = "changed.rs"]
pub mod changed;
#[path = "corepin.rs"]
pub mod corepin;
#[path = "docs.rs"]
pub mod docs;
#[path = "gate.rs"]
pub mod gate;
#[path = "json.rs"]
pub mod json;
#[path = "licenses.rs"]
pub mod licenses;
#[path = "link.rs"]
pub mod link;
#[path = "lintroots.rs"]
pub mod lintroots;
#[path = "privacy.rs"]
pub mod privacy;
#[path = "spdx.rs"]
pub mod spdx;
#[path = "sync.rs"]
pub mod sync;
#[path = "util.rs"]
pub mod util;

// The two modules below are written here, not in files of their own, because a release that adds
// a file to this folder cannot be taken by an app's own `cargo xtask sync` (open-source-core#42):
// the app's older program would write this list and not the new file. They move to `release.rs`
// and `package.rs` beside this one, with no other change, once that is fixed.

/// `cargo xtask release`: an app's release commit, and what the release workflow asks about it.
///
/// A release of an app is cut from a release commit on `main`: the version under
/// `[workspace.package]` of `Cargo.toml` is the release's, and the changelog's section for it is
/// written and dated (`## 1.2.0 (2026-12-01)`). The shared release workflow checks that, runs
/// the gate, and only then makes the tag `v1.2.0` and publishes (the core's
/// `docs/app-releases.md`).
///
/// - `cargo xtask release <version> [--date YYYY-MM-DD]` makes the changes of a release commit:
///   the version in `Cargo.toml` and `Cargo.lock`, and the changelog's `## Unreleased` heading,
///   which becomes `## <version> (<date>)`.
/// - `cargo xtask release --check` says whether the checkout is a release commit and prints
///   `version=`, `tag=`, `id=` and `product=` lines for the workflow.
/// - `cargo xtask release --notes` prints the changelog section of the checkout's version: the
///   text of the GitHub release.
///
/// Nothing here tags, pushes or publishes. The helpers that read a manifest and a changelog are
/// the ones the core's own release task uses.
pub mod release {
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::corepin;
    use super::gate::App;
    use super::link;
    use super::util::{TaskResult, cargo_output};

    /// The heading of the section that holds what the next release changes.
    pub const UNRELEASED: &str = "Unreleased";

    const USAGE: &str = "usage: cargo xtask release <version> [--date YYYY-MM-DD]\n       cargo xtask release --check | --notes";

    /// A version as three numbers, when it is written `X.Y.Z` and nothing else.
    pub fn version(text: &str) -> Option<(u64, u64, u64)> {
        let digits = text.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
        if digits { corepin::release(&format!("v{text}")) } else { None }
    }

    /// Whether `text` is a date written `YYYY-MM-DD`.
    pub fn is_date(text: &str) -> bool {
        let b = text.as_bytes();
        b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
    }

    /// The date `days` days after 1970-01-01, as `YYYY-MM-DD`.
    pub fn date_of(days: u64) -> String {
        // Days to a date of the calendar in use everywhere, by whole 400-year eras from 0000-03-01.
        let z = days + 719_468;
        let (era, day_of_era) = (z / 146_097, z % 146_097);
        let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
        let year = year_of_era + era * 400 + u64::from(month <= 2);
        format!("{year:04}-{month:02}-{day:02}")
    }

    /// Today's date in UTC, the clock GitHub's own dates use.
    pub fn today() -> String {
        date_of(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) / 86_400)
    }

    /// The version under `[workspace.package]` of the root manifest.
    pub fn workspace_version(manifest: &str) -> Option<&str> {
        let mut in_table = false;
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                in_table = line == "[workspace.package]";
            } else if in_table && line.split('=').next().is_some_and(|key| key.trim() == "version") {
                return link::value_of(line, "version");
            }
        }
        None
    }

    /// The root manifest with `new` as the version under `[workspace.package]`.
    pub fn with_workspace_version(manifest: &str, new: &str) -> Option<String> {
        let old = workspace_version(manifest)?;
        let mut in_table = false;
        let mut out = String::with_capacity(manifest.len());
        let mut done = false;
        for line in manifest.split_inclusive('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_table = trimmed == "[workspace.package]";
            }
            if in_table && !done && trimmed.split('=').next().is_some_and(|key| key.trim() == "version") {
                out.push_str(&line.replacen(&format!("\"{old}\""), &format!("\"{new}\""), 1));
                done = true;
            } else {
                out.push_str(line);
            }
        }
        done.then_some(out)
    }

    /// One `## ` section of a changelog: the heading without its `## `, the line it is on (from
    /// 0) and the lines below it up to the next section.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Section<'a> {
        pub heading: &'a str,
        pub line: usize,
        pub body: Vec<&'a str>,
    }

    impl Section<'_> {
        /// The section's text without the blank lines around it.
        pub fn text(&self) -> String {
            self.body.join("\n").trim().to_owned()
        }
    }

    /// The sections of a changelog, newest first as written. A `## ` inside a fenced block of
    /// code is code, not a heading.
    pub fn sections(changelog: &str) -> Vec<Section<'_>> {
        let mut out: Vec<Section> = Vec::new();
        let mut in_code = false;
        for (i, line) in changelog.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
            }
            match line.strip_prefix("## ").filter(|_| !in_code) {
                Some(heading) => out.push(Section { heading: heading.trim(), line: i, body: Vec::new() }),
                None => {
                    if let Some(section) = out.last_mut() {
                        section.body.push(line);
                    }
                }
            }
        }
        out
    }

    /// The version and the date of a released section's heading, `X.Y.Z (YYYY-MM-DD)`.
    pub fn released_heading(heading: &str) -> Option<(&str, &str)> {
        let (number, date) = heading.split_once(" (")?;
        let date = date.strip_suffix(')')?;
        (version(number).is_some() && is_date(date)).then_some((number, date))
    }

    /// The section a release of `number` publishes: the newest section, or the one below a
    /// `## Unreleased` that holds no text yet (an app may open the next section in the release
    /// commit itself). `Err` says what the changelog has in its place.
    pub fn release_section<'a, 'b>(all: &'b [Section<'a>], number: &str) -> Result<&'b Section<'a>, String> {
        let is_ours = |s: &&Section| released_heading(s.heading).is_some_and(|(found, _)| found == number);
        let mut newest = all.iter().skip_while(|s| s.heading == UNRELEASED && s.text().is_empty());
        match newest.next() {
            Some(top) if is_ours(&top) => Ok(top),
            Some(top) if top.heading == UNRELEASED => Err(format!(
                "CHANGELOG.md: the newest section is still `## {UNRELEASED}`. `cargo xtask release {number}` turns it into `## {number} (date)`"
            )),
            Some(top) => Err(format!("CHANGELOG.md: the newest section is `## {}`, not `## {number} (YYYY-MM-DD)`", top.heading)),
            None => Err("CHANGELOG.md has no section".to_owned()),
        }
    }

    /// The changelog with its `## Unreleased` section turned into the section of `new`, dated
    /// `date`. Refuses a changelog whose newest section is not `Unreleased`, one that already
    /// has a section for `new`, and a section with nothing in it.
    pub fn with_release(changelog: &str, new: &str, date: &str) -> Result<String, String> {
        let all = sections(changelog);
        if all.iter().any(|s| released_heading(s.heading).is_some_and(|(number, _)| number == new)) {
            return Err(format!("CHANGELOG.md already has a section for {new}"));
        }
        let Some(top) = all.first().filter(|s| s.heading == UNRELEASED) else {
            return Err(format!(
                "CHANGELOG.md: the newest section is not `## {UNRELEASED}`. Write what this release changes under that heading first"
            ));
        };
        if top.text().is_empty() {
            return Err(format!("CHANGELOG.md: `## {UNRELEASED}` is empty. A release says what it changes"));
        }
        let mut out = String::with_capacity(changelog.len() + 16);
        for (i, line) in changelog.split_inclusive('\n').enumerate() {
            if i == top.line {
                out.push_str(&format!("## {new} ({date})\n"));
            } else {
                out.push_str(line);
            }
        }
        Ok(out)
    }

    /// The version of the release commit that `manifest` and `changelog` make, or what stops
    /// them from being one.
    pub fn release_problems(manifest: &str, changelog: &str) -> Result<String, Vec<String>> {
        let Some(now) = workspace_version(manifest).filter(|v| version(v).is_some()) else {
            return Err(vec![
                "Cargo.toml has no version X.Y.Z under [workspace.package] (a release has three numbers and nothing after them)".to_owned(),
            ]);
        };
        let all = sections(changelog);
        match release_section(&all, now) {
            Ok(section) if section.text().is_empty() => Err(vec![format!("CHANGELOG.md: the section of {now} is empty")]),
            Ok(_) => Ok(now.to_owned()),
            Err(problem) => Err(vec![problem]),
        }
    }

    /// The text of the GitHub release of `number`: its section of the changelog.
    pub fn notes_of(changelog: &str, number: &str) -> Result<String, String> {
        let all = sections(changelog);
        let section = all
            .iter()
            .find(|s| released_heading(s.heading).is_some_and(|(found, _)| found == number))
            .ok_or_else(|| format!("CHANGELOG.md has no section for {number}"))?;
        let text = section.text();
        if text.is_empty() { Err(format!("CHANGELOG.md: the section of {number} is empty")) } else { Ok(text) }
    }

    pub(super) fn read(root: &Path, file: &str) -> Result<String, String> {
        std::fs::read_to_string(root.join(file)).map_err(|e| format!("cannot read {file}: {e}"))
    }

    fn write(root: &Path, file: &str, text: &str) -> TaskResult {
        std::fs::write(root.join(file), text).map_err(|e| format!("cannot write {file}: {e}"))
    }

    /// The version of the checkout, from its root manifest.
    pub fn checkout_version(root: &Path) -> Result<String, String> {
        let manifest = read(root, "Cargo.toml")?;
        let found = workspace_version(&manifest).filter(|v| version(v).is_some());
        found.map(str::to_owned).ok_or_else(|| "Cargo.toml has no version X.Y.Z under [workspace.package]".to_owned())
    }

    /// `cargo xtask release <version>`: the files of an app's release commit.
    fn cut(root: &Path, app: &App, new: &str, date: &str) -> TaskResult {
        let wanted = version(new).ok_or_else(|| format!("'{new}' is not a version (X.Y.Z). {USAGE}"))?;
        if !is_date(date) {
            return Err(format!("'{date}' is not a date (YYYY-MM-DD)"));
        }
        let manifest = read(root, "Cargo.toml")?;
        let now = workspace_version(&manifest).ok_or("Cargo.toml has no version under [workspace.package]")?;
        // The version may already be there: the first release of an app is the version it has carried.
        if version(now).is_none_or(|now| wanted < now) {
            return Err(format!("{new} is older than {now}, the version in Cargo.toml"));
        }
        // Everything is worked out before anything is written, so a refusal changes no file.
        let new_manifest = with_workspace_version(&manifest, new).ok_or("Cargo.toml: could not write the version under [workspace.package]")?;
        let new_changelog = with_release(&read(root, "CHANGELOG.md")?, new, date)?;
        if new_manifest != manifest {
            write(root, "Cargo.toml", &new_manifest)?;
        }
        write(root, "CHANGELOG.md", &new_changelog)?;
        // Cargo brings the workspace's own packages to the new version in the lock file and
        // moves nothing else.
        cargo_output(root, &["metadata", "--format-version", "1"])?;
        println!("{} {new} ({date}): Cargo.toml, Cargo.lock and CHANGELOG.md are changed.", app.product);
        println!("Next:");
        println!("  1. cargo xtask ci");
        println!("  2. commit the files as \"{} {new}\" and land it on main through a pull request", app.product);
        println!("  3. gh workflow run release.yml --ref main");
        println!("     It runs the gate on Linux, macOS and Windows and builds the packages; when all of it has");
        println!("     passed it makes the tag v{new} and publishes the release. Do not make the tag by hand.");
        Ok(())
    }

    /// `cargo xtask release --check`: the checkout is a release commit of an app that can be
    /// packaged; its version.
    fn check(root: &Path, app: &App) -> Result<String, String> {
        let found = release_problems(&read(root, "Cargo.toml")?, &read(root, "CHANGELOG.md")?);
        let now = found.map_err(|problems| format!("not a release commit:\n  - {}", problems.join("\n  - ")))?;
        cargo_output(root, &["metadata", "--format-version", "1", "--locked"]).map_err(|e| {
            format!("Cargo.lock is not in line with the manifests (`cargo metadata --format-version 1 >/dev/null` brings it there): {e}")
        })?;
        super::package::check(root, app)?;
        Ok(now)
    }

    /// `cargo xtask release ...` in an app.
    pub fn task(root: &Path, app: &App, args: &[String]) -> TaskResult {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        match args.as_slice() {
            ["--check"] => {
                let version = check(root, app)?;
                super::package::describe(app, &version)
            }
            ["--notes"] => notes_of(&read(root, "CHANGELOG.md")?, &checkout_version(root)?).map(|text| println!("{text}")),
            [version] if !version.starts_with('-') => cut(root, app, version, &today()),
            [version, "--date", date] if !version.starts_with('-') => cut(root, app, version, date),
            _ => Err(USAGE.to_owned()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const LOG: &str = "# Changelog\n\nIntro.\n\n## Unreleased\n\n### Added\n\n- A thing.\n\n```toml\n## not a heading\n```\n\n\
                           ## 0.8.0 (2026-11-26)\n\n- The one before.\n";

        #[test]
        fn reads_versions_and_dates() {
            assert_eq!(version("0.9.0"), Some((0, 9, 0)));
            assert_eq!(version("12.0.3"), Some((12, 0, 3)));
            // Three numbers and nothing else: no `v`, no pre-release, no build text.
            for other in ["", "0.9", "0.9.0.1", "v0.9.0", "0.9.x", "0.9.0-rc.1", "0.9.0+build", "0..1", " 0.9.0", "01.2.3"] {
                assert_eq!(version(other), None, "{other}");
            }
            assert!(is_date("2026-11-26"));
            for other in ["2026-11-6", "26-11-2026", "2026/11/26", "today", ""] {
                assert!(!is_date(other), "{other}");
            }
            assert_eq!(date_of(0), "1970-01-01");
            assert_eq!(date_of(11_016), "2000-02-29");
            assert_eq!(date_of(20_783), "2026-11-26");
            assert_eq!(date_of(20_819), "2027-01-01");
        }

        #[test]
        fn reads_and_writes_the_workspace_version() {
            let manifest =
                "[package]\nversion = \"9.9.9\"\n\n[workspace.package]\nedition = \"2024\"\nversion = \"0.8.0\"\n\n[dependencies]\nversion = \"1\"\n";
            assert_eq!(workspace_version(manifest), Some("0.8.0"));
            let moved = with_workspace_version(manifest, "0.9.0").unwrap();
            assert_eq!(workspace_version(&moved), Some("0.9.0"));
            assert_eq!(moved.replace("0.9.0", "0.8.0"), manifest);
            assert_eq!(with_workspace_version("[package]\nversion = \"1.0.0\"\n", "2.0.0"), None);
        }

        #[test]
        fn unreleased_becomes_the_release_and_code_is_not_a_heading() {
            let all = sections(LOG);
            assert_eq!(all.iter().map(|s| s.heading).collect::<Vec<_>>(), ["Unreleased", "0.8.0 (2026-11-26)"]);
            let cut = with_release(LOG, "0.9.0", "2026-12-01").unwrap();
            assert!(cut.contains("\n## 0.9.0 (2026-12-01)\n\n### Added\n") && !cut.contains("## Unreleased"));
            assert_eq!(cut.lines().count(), LOG.lines().count());
            assert_eq!(notes_of(&cut, "0.9.0").unwrap(), "### Added\n\n- A thing.\n\n```toml\n## not a heading\n```");
            // Twice, nothing to release, and a changelog that does not start with Unreleased.
            assert!(with_release(&cut, "0.9.0", "2026-12-01").unwrap_err().contains("already has a section for 0.9.0"));
            assert!(with_release(&cut, "0.9.1", "2026-12-02").unwrap_err().contains("is not `## Unreleased`"));
            assert!(with_release("## Unreleased\n\n## 0.8.0 (2026-11-26)\n\n- x\n", "0.9.0", "2026-12-01").unwrap_err().contains("is empty"));
            assert!(notes_of(LOG, "0.9.0").unwrap_err().contains("no section for 0.9.0"));
        }

        #[test]
        fn knows_a_release_commit() {
            let manifest = |v: &str| format!("[workspace.package]\nversion = \"{v}\"\n");
            let cut = with_release(LOG, "0.9.0", "2026-12-01").unwrap();
            assert_eq!(release_problems(&manifest("0.9.0"), &cut), Ok("0.9.0".to_owned()));
            // The next section may be opened in the release commit itself, while it is empty.
            let opened = cut.replacen("## 0.9.0", "## Unreleased\n\n## 0.9.0", 1);
            assert_eq!(release_problems(&manifest("0.9.0"), &opened), Ok("0.9.0".to_owned()));
            let problems = |m: &str, log: &str| release_problems(m, log).unwrap_err().join("; ");
            assert!(problems(&manifest("0.9.0"), LOG).contains("still `## Unreleased`"));
            assert!(problems(&manifest("1.0.0"), &cut).contains("`## 0.9.0 (2026-12-01)`, not `## 1.0.0"));
            assert!(problems(&manifest("0.9.0-rc.1"), &cut).contains("three numbers"));
            assert!(problems(&manifest("0.9.0"), "# Changelog\n").contains("no section"));
            assert!(problems(&manifest("0.9.0"), "## 0.9.0 (2026-12-01)\n\n## 0.8.0 (2026-11-26)\n\n- x\n").contains("is empty"));
            // A heading that only looks like a release is not one.
            assert!(problems(&manifest("0.9.0"), "## 0.9.0 (unreleased)\n\n- x\n").contains("not `## 0.9.0 (YYYY-MM-DD)`"));
        }
    }
}

/// `cargo xtask package`: an app's release files, as the core's `docs/app-releases.md` names
/// and lays them out, so that the app's own update check, a launcher and a website find,
/// verify and install them without knowing the app.
///
/// - `cargo xtask package [<platform>] [--out <folder>] [--no-build]` builds the shipped
///   programs in release mode and writes the package of this system into `dist/`:
///   `<id>-<version>-linux-x86_64.tar.gz`, `<id>-<version>-macos-aarch64.dmg` or
///   `<id>-<version>-windows-x64-portable.zip`. A package is made on its own system.
/// - `cargo xtask package --sums <folder> [--complete]` checks that the folder holds release
///   files of this app's version and nothing else, and writes `SHA256SUMS.txt` beside them.
///   `--complete` also fails when a platform's file is missing.
/// - `cargo xtask package --describe` prints `id=`, `product=`, `version=`, `tag=` and `files=`
///   lines for a workflow.
///
/// What an app ships is data: [`Package`] in the app's `xtask/src/main.rs`, with the programs in
/// `App::shipped`. Signing is asked for through the environment and left out when it is not
/// (see [`sign`]). Nothing here tags, uploads or publishes.
pub mod package {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::gate::App;
    use super::json;
    use super::licenses;
    use super::release;
    use super::util::{TaskResult, cargo, cargo_output, report};

    /// What a release of an app carries besides its programs. An app writes the fields it has
    /// and takes the rest from [`Package::DEFAULT`], so a field added later breaks no app.
    #[derive(Clone, Copy, Debug)]
    pub struct Package {
        /// The release id: the stem of every release file, the name of the desktop program
        /// (the first of `App::shipped`), and the id the app gives its update source. Lower-case
        /// letters, digits and `-`.
        pub id: &'static str,
        /// The app's id on a desktop, in reverse-DNS form: the macOS bundle id, the name of the
        /// Linux menu entry and icon, and the app id the window is given.
        pub app_id: &'static str,
        /// What the app is for, in a few words (the Linux menu entry's comment).
        pub summary: &'static str,
        /// The Linux menu categories, each followed by `;`.
        pub categories: &'static str,
        /// The app icon as PNG files: (pixels a side, path from the root). Every size goes into
        /// the Linux package; 16 to 1024 also into the macOS icon unless `macos_icons` is set.
        pub icons: &'static [(u32, &'static str)],
        /// The macOS icon's PNG files, when the app draws that icon apart (its own margins).
        pub macos_icons: &'static [(u32, &'static str)],
        /// The `.ico` file the Windows program must carry in its resources (the app's build
        /// script embeds it; the package task checks that it is there).
        pub windows_icon: Option<&'static str>,
        /// The kinds of file the app opens.
        pub file_types: &'static [FileType],
        /// License texts of what the programs embed (fonts, data): files, or folders searched
        /// for files named like a license. Each is carried under `licenses/` by its own path.
        pub license_files: &'static [&'static str],
    }

    impl Package {
        /// A package with nothing of its own. Write `..Package::DEFAULT` after the fields an app
        /// sets; an app that sets every field leaves it out, because clippy refuses a `..` that
        /// fills nothing (`needless_update`).
        pub const DEFAULT: Package = Package {
            id: "",
            app_id: "",
            summary: "",
            categories: "Office;",
            icons: &[],
            macos_icons: &[],
            windows_icon: None,
            file_types: &[],
            license_files: &[],
        };
    }

    /// One kind of file an app opens.
    #[derive(Clone, Copy, Debug)]
    pub struct FileType {
        /// The kind's name as a desktop shows it.
        pub name: &'static str,
        /// File name extensions, without the dot. The first is the main one.
        pub extensions: &'static [&'static str],
        /// The media type.
        pub mime: &'static str,
        /// The media type this one is a kind of (`application/zip`, `text/plain`), or empty.
        pub parent: &'static str,
        /// A document of this kind is a folder that a desktop shows as one document.
        pub folder: bool,
        /// The kind is the app's own. Another program's format the app also opens is not: the
        /// app is offered for it and never made its owner.
        pub own: bool,
    }

    impl FileType {
        /// A file type of the app's own, kept in one file.
        pub const DEFAULT: FileType = FileType { name: "", extensions: &[], mime: "", parent: "", folder: false, own: true };
    }

    /// A system a release has a file for.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Platform {
        LinuxX64,
        MacosArm64,
        WindowsX64,
    }

    impl Platform {
        /// Every platform of a release, in the order of the names.
        pub const ALL: [Platform; 3] = [Platform::LinuxX64, Platform::MacosArm64, Platform::WindowsX64];

        /// The platform's part of a file name, as the core's updater reads it.
        pub fn token(self) -> &'static str {
            match self {
                Platform::LinuxX64 => "linux-x86_64",
                Platform::MacosArm64 => "macos-aarch64",
                Platform::WindowsX64 => "windows-x64",
            }
        }

        /// What follows the token in the file name: the kind of package.
        pub fn suffix(self) -> &'static str {
            match self {
                Platform::LinuxX64 => ".tar.gz",
                Platform::MacosArm64 => ".dmg",
                Platform::WindowsX64 => "-portable.zip",
            }
        }

        /// The platform a token names.
        pub fn from_token(token: &str) -> Option<Platform> {
            Platform::ALL.into_iter().find(|p| p.token() == token)
        }

        /// The platform of a system and processor, in Rust's names for them.
        pub fn of(os: &str, arch: &str) -> Option<Platform> {
            match (os, arch) {
                ("linux", "x86_64") => Some(Platform::LinuxX64),
                ("macos", "aarch64") => Some(Platform::MacosArm64),
                ("windows", "x86_64") => Some(Platform::WindowsX64),
                _ => None,
            }
        }

        /// The platform this program runs on, when a release has a file for it.
        pub fn host() -> Option<Platform> {
            Platform::of(std::env::consts::OS, std::env::consts::ARCH)
        }
    }

    /// The file that lists the checksum of every package of a release.
    pub const SUMS: &str = "SHA256SUMS.txt";
    /// Where an app's own license files and `THIRD-PARTY-LICENSES.txt` sit beside each other in
    /// a package: this folder of the Linux package (`share/<id>`), the top of the Windows one,
    /// `Contents/Resources` of the macOS app.
    pub const LICENSES_DIR: &str = "licenses";
    /// The legal files every package carries, beside the ones the app adds (`App::legal_files`).
    pub const LEGAL: &[&str] = &["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md"];
    /// The folder release files are written to.
    const DIST: &str = "dist";
    /// The oldest macOS the packages say they run on.
    const MACOS_MINIMUM: &str = "11.0";

    /// `<id>-<version>-<platform>`: a release file's name without its kind, and the name of the
    /// one folder inside it.
    pub fn stem(id: &str, version: &str, platform: Platform) -> String {
        format!("{id}-{version}-{}", platform.token())
    }

    /// The name of the release file for `platform`.
    pub fn file_name(id: &str, version: &str, platform: Platform) -> String {
        format!("{}{}", stem(id, version, platform), platform.suffix())
    }

    /// The tag of a release.
    pub fn tag(version: &str) -> String {
        format!("v{version}")
    }

    /// The package data of an app, or why it cannot be packaged.
    pub fn data(app: &App) -> Result<&Package, String> {
        app.package.as_ref().ok_or_else(|| {
            "this app has no package data: set `package: Some(Package { .. })` in xtask/src/main.rs (the core's docs/app-releases.md says what goes in it)"
                .to_owned()
        })
    }

    fn plain(text: &str, more: &[char]) -> bool {
        !text.is_empty() && text.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || more.contains(&c))
    }

    /// Whether text can stand in a menu entry, a property list and a Windows command file as it
    /// is: one line, with none of the characters those files read as their own.
    fn printable(text: &str) -> bool {
        !text.trim().is_empty()
            && !text.chars().any(|c| c.is_control() || matches!(c, '"' | '%' | '<' | '>' | '&' | '\\' | '/' | ':' | '|' | '*' | '?'))
    }

    fn file_type_problems(kind: &FileType) -> Vec<String> {
        let mut out = Vec::new();
        let named = if kind.name.is_empty() { kind.mime } else { kind.name };
        if !printable(kind.name) {
            out.push(format!("file type '{named}': the name must be one line without \" % < > & \\ / : | * ?"));
        }
        if kind.extensions.is_empty() || kind.extensions.iter().any(|e| !plain(e, &[])) {
            out.push(format!("file type '{named}': extensions are lower-case letters and digits, without the dot, and there is at least one"));
        }
        let media = |m: &str| m.split_once('/').is_some_and(|(a, b)| plain(a, &['-']) && plain(b, &['-', '.', '+']));
        if !media(kind.mime) || !(kind.parent.is_empty() || media(kind.parent)) {
            out.push(format!("file type '{named}': `mime` (and `parent`, when set) is a media type such as application/x-example"));
        }
        out
    }

    /// What is wrong with an app's package data, as written: nothing is read from disk.
    pub fn data_problems(app: &App) -> Vec<String> {
        let Some(package) = &app.package else { return Vec::new() };
        let mut out = Vec::new();
        if !plain(package.id, &['-']) || package.id.starts_with('-') || package.id.ends_with('-') {
            out.push(format!("package id '{}': lower-case letters, digits and `-`", package.id));
        }
        let parts: Vec<&str> = package.app_id.split('.').collect();
        if parts.len() < 3 || parts.iter().any(|p| !plain(p, &['-']) || p.starts_with(|c: char| c.is_ascii_digit())) {
            out.push(format!("package app_id '{}': a reverse-DNS name of three parts or more, such as org.example.app", package.app_id));
        }
        if !printable(app.product) {
            out.push(format!("product '{}': the name must be one line without \" % < > & \\ / : | * ?", app.product));
        }
        if !printable(package.summary) {
            out.push("package summary: one line that says what the app is for, without \" % < > & \\ / : | * ?".to_owned());
        }
        if package.categories.is_empty()
            || !package.categories.ends_with(';')
            || !package.categories.chars().all(|c| c.is_ascii_alphanumeric() || c == ';')
        {
            out.push(format!("package categories '{}': menu categories, each followed by `;`", package.categories));
        }
        if app.shipped.is_empty() {
            out.push("`shipped` names no program to package".to_owned());
        }
        if package.icons.is_empty() {
            out.push("package icons: at least one PNG, as (pixels a side, path)".to_owned());
        }
        for (size, path) in package.icons.iter().chain(package.macos_icons) {
            if !path.ends_with(".png") {
                out.push(format!("package icon {path}: a PNG file"));
            }
            if !(16..=1024).contains(size) {
                out.push(format!("package icon {path}: {size} pixels is outside 16 to 1024"));
            }
        }
        let mac = if package.macos_icons.is_empty() { package.icons } else { package.macos_icons };
        if !package.icons.is_empty() && !mac.iter().any(|(size, _)| icns_type(*size).is_some()) {
            out.push("package icons: none has a size a macOS icon holds (16, 32, 64, 128, 256, 512 or 1024)".to_owned());
        }
        for kind in package.file_types {
            out.extend(file_type_problems(kind));
        }
        out
    }

    /// One program a package carries: the workspace package it is built from, and its name.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Program {
        pub package: String,
        pub name: String,
    }

    /// The programs of the shipped packages, in the order `shipped` names them, read from
    /// `cargo metadata --no-deps`.
    pub fn programs_from(meta: &json::Value, shipped: &[&str]) -> Result<Vec<Program>, String> {
        let packages = meta.get("packages").ok_or("cargo metadata has no packages")?.items();
        let mut out = Vec::new();
        for name in shipped {
            let package = packages.iter().find(|p| p.str_of("name") == Some(name)).ok_or_else(|| {
                format!("`shipped` names {name}, which is no package of this workspace (it holds package names, as `cargo build -p` takes them)")
            })?;
            let is_bin = |t: &&json::Value| t.get("kind").map(json::Value::items).unwrap_or_default().iter().any(|k| k.as_str() == Some("bin"));
            let bins: Vec<&str> =
                package.get("targets").map(json::Value::items).unwrap_or_default().iter().filter(is_bin).filter_map(|t| t.str_of("name")).collect();
            match bins.as_slice() {
                [one] => out.push(Program { package: (*name).to_owned(), name: (*one).to_owned() }),
                [] => return Err(format!("the shipped package {name} builds no program")),
                _ => {
                    return Err(format!(
                        "the shipped package {name} builds {} programs ({}); a shipped package builds one",
                        bins.len(),
                        bins.join(", ")
                    ));
                }
            }
        }
        Ok(out)
    }

    /// What is wrong with the programs' names. The desktop program is the release id, which is
    /// what the updater looks for inside a package, and every other program starts with it, so
    /// that two apps installed side by side share no name.
    pub fn program_problems(id: &str, programs: &[Program]) -> Vec<String> {
        let mut out = Vec::new();
        for (i, program) in programs.iter().enumerate() {
            if i == 0 && program.name != id {
                out.push(format!(
                    "the first shipped package ({}) builds the program `{}`; the desktop program of this app is named `{id}`, its release id",
                    program.package, program.name
                ));
            } else if i > 0 && program.name.strip_prefix(id).is_none_or(|rest| !rest.starts_with('-') || rest.len() < 2 || rest.contains('.')) {
                out.push(format!("the program `{}` ({}) must be named `{id}-<name>`, such as `{id}-cli`", program.name, program.package));
            }
        }
        out
    }

    /// The width and height of a PNG file, from its first chunk.
    pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
        let number = |at: usize| bytes.get(at..at + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()).map(u32::from_be_bytes);
        (bytes.get(..8)? == b"\x89PNG\r\n\x1a\n" && bytes.get(12..16)? == b"IHDR").then_some(())?;
        Some((number(16)?, number(20)?))
    }

    fn is_license_name(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        ["license", "licence", "copying", "notice", "ofl"].iter().any(|p| lower.starts_with(p))
    }

    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> TaskResult {
        if depth > 16 {
            return Err(format!("{} is nested too deep", dir.display()));
        }
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let kind = entry.file_type().map_err(|e| format!("{}: {e}", entry.path().display()))?;
            if kind.is_dir() {
                walk(&entry.path(), depth + 1, out)?;
            } else if kind.is_file() {
                out.push(entry.path());
            }
        }
        Ok(())
    }

    /// Every file under `dir`, as paths from it with `/` between the parts, sorted. Links are
    /// left out.
    pub fn files_under(dir: &Path) -> Result<Vec<String>, String> {
        let mut found = Vec::new();
        walk(dir, 0, &mut found)?;
        let mut out: Vec<String> = found
            .iter()
            .filter_map(|p| p.strip_prefix(dir).ok())
            .map(|p| p.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
            .collect();
        out.sort();
        Ok(out)
    }

    /// The license texts `license_files` names, as paths from the root: a file as it is, a
    /// folder as every file in it that is named like a license.
    pub fn license_texts(root: &Path, license_files: &[&str]) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for entry in license_files {
            let entry = entry.trim_end_matches('/');
            if root.join(entry).is_file() {
                out.push(entry.to_owned());
            } else if root.join(entry).is_dir() {
                let found: Vec<String> = files_under(&root.join(entry))?
                    .into_iter()
                    .filter(|f| is_license_name(f.rsplit('/').next().unwrap_or_default()))
                    .map(|f| format!("{entry}/{f}"))
                    .collect();
                if found.is_empty() {
                    return Err(format!("package license_files: {entry}/ holds no file named like a license (LICENSE, OFL, NOTICE, COPYING)"));
                }
                out.extend(found);
            } else {
                return Err(format!("package license_files: {entry} is not there"));
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Where the license texts of fonts are kept, in an app and in a dependency: this folder at
    /// the top of the repository.
    const FONTS: &str = "assets/fonts";
    /// How far above a crate the top of its checkout may be.
    const MAX_WALK_UP: usize = 6;

    /// The license texts of the fonts that dependencies build into the programs. A dependency
    /// taken from a git repository that keeps fonts under `assets/fonts` at its top keeps
    /// their license texts there (the core's interface fonts come to an app this way); those
    /// are the files returned, one for each name, sorted. Every package carries them, so the
    /// fonts' licenses go with the programs without an app having to copy them.
    pub fn dependency_font_licenses(packages: &[licenses::Package]) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = Vec::new();
        for package in packages.iter().filter(|p| p.git) {
            // `.cargo-ok` marks the top of Cargo's checkout, `.git` the top of any other.
            let top = package.dir.ancestors().take(MAX_WALK_UP + 1).find(|dir| dir.join(".cargo-ok").exists() || dir.join(".git").exists());
            let Some(entries) = top.and_then(|top| std::fs::read_dir(top.join(FONTS)).ok()) else { continue };
            for file in entries.flatten().map(|e| e.path()) {
                let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if file.is_file() && is_license_name(&name) && !found.iter().any(|f| f.file_name() == file.file_name()) {
                    found.push(file);
                }
            }
        }
        found.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        found
    }

    /// What is wrong with the files the package data names.
    pub fn file_problems(root: &Path, app: &App, package: &Package) -> Vec<String> {
        let mut out = Vec::new();
        for (size, path) in package.icons.iter().chain(package.macos_icons) {
            match std::fs::read(root.join(path)).map(|bytes| png_size(&bytes)) {
                Ok(Some(found)) if found == (*size, *size) => {}
                Ok(Some((w, h))) => out.push(format!("package icon {path} is {w} by {h} pixels, not {size} by {size}")),
                Ok(None) => out.push(format!("package icon {path} is not a PNG file")),
                Err(e) => out.push(format!("package icon {path}: {e}")),
            }
        }
        if let Some(icon) = package.windows_icon
            && !root.join(icon).is_file()
        {
            out.push(format!("package windows_icon {icon} is not there"));
        }
        for file in LEGAL.iter().chain(app.legal_files) {
            if !root.join(file).is_file() {
                out.push(format!("{file} is missing: every package carries it"));
            }
        }
        if let Err(e) = license_texts(root, package.license_files) {
            out.push(e);
        }
        out
    }

    /// The programs an app ships, from Cargo.
    pub fn programs(root: &Path, app: &App) -> Result<Vec<Program>, String> {
        let out = cargo_output(root, &["metadata", "--format-version", "1", "--no-deps", "--locked"])?;
        programs_from(&json::parse(&out)?, app.shipped)
    }

    /// The `release-data` check of the gate, and the first step of every other task here: the
    /// package data is sound, the files it names are there, and the programs carry the names a
    /// release needs. An app with no package data passes: it publishes no release.
    pub fn check(root: &Path, app: &App) -> TaskResult {
        let Some(package) = &app.package else {
            println!("ok: no package data in xtask/src/main.rs: this app publishes no release");
            return Ok(());
        };
        let mut failures = data_problems(app);
        if failures.is_empty() {
            failures.extend(file_problems(root, app, package));
            match programs(root, app) {
                Ok(found) => failures.extend(program_problems(package.id, &found)),
                Err(e) => failures.push(e),
            }
            let changelog = release::read(root, "CHANGELOG.md").unwrap_or_default();
            let newest = release::sections(&changelog).into_iter().next().map(|s| s.heading.to_owned());
            match newest {
                Some(heading) if heading == release::UNRELEASED || release::released_heading(&heading).is_some() => {}
                Some(heading) => failures.push(format!(
                    "CHANGELOG.md: the newest section is `## {heading}`. A section is `## {}` until its release, then `## X.Y.Z (YYYY-MM-DD)`",
                    release::UNRELEASED
                )),
                None => failures.push("CHANGELOG.md has no `## ` section: the text of a release is its section there".to_owned()),
            }
            if let Err(e) = release::checkout_version(root) {
                failures.push(e);
            }
        }
        report(failures, &format!("the release data of {} is sound: its programs, icons, license files and changelog", package.id))
    }

    /// `cargo xtask package --describe`: what a workflow needs to know, one `name=value` a line.
    pub fn describe(app: &App, version: &str) -> TaskResult {
        let package = data(app)?;
        let files: Vec<String> = Platform::ALL.iter().map(|p| file_name(package.id, version, *p)).collect();
        println!("id={}", package.id);
        println!("product={}", app.product);
        println!("version={version}");
        println!("tag={}", tag(version));
        println!("files={}", files.join(" "));
        println!("sums={SUMS}");
        Ok(())
    }

    // ------------------------------------------------------------------ the texts a package carries

    fn xml(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    /// The media types the app is offered for, as a menu entry lists them.
    fn mime_list(package: &Package) -> String {
        package.file_types.iter().map(|k| format!("{};", k.mime)).collect()
    }

    /// The Linux menu entry.
    pub fn desktop_entry(product: &str, package: &Package, program: &str) -> String {
        let mut out = format!(
            "[Desktop Entry]\nType=Application\nName={product}\nComment={}\nExec={program} %f\nIcon={id}\nStartupWMClass={id}\nTerminal=false\nCategories={}\n",
            package.summary,
            package.categories,
            id = package.app_id,
        );
        if !package.file_types.is_empty() {
            out.push_str(&format!("MimeType={}\n", mime_list(package)));
        }
        out
    }

    /// The Linux description of the app's own file types, or nothing when it has none.
    pub fn mime_info(package: &Package) -> Option<String> {
        let own: Vec<&FileType> = package.file_types.iter().filter(|k| k.own).collect();
        if own.is_empty() {
            return None;
        }
        let mut out =
            String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n");
        for kind in own {
            out.push_str(&format!("  <mime-type type=\"{}\">\n    <comment>{}</comment>\n", kind.mime, xml(kind.name)));
            let parent = if kind.folder { "inode/directory" } else { kind.parent };
            if !parent.is_empty() {
                out.push_str(&format!("    <sub-class-of type=\"{parent}\"/>\n"));
            }
            for extension in kind.extensions {
                out.push_str(&format!("    <glob pattern=\"*.{extension}\"/>\n"));
            }
            out.push_str(&format!("    <icon name=\"{}\"/>\n  </mime-type>\n", package.app_id));
        }
        out.push_str("</mime-info>\n");
        Some(out)
    }

    /// The macOS name of a file type of the app's own.
    fn uniform_type(package: &Package, kind: &FileType) -> String {
        format!("{}.{}", package.app_id, kind.extensions.first().copied().unwrap_or("document"))
    }

    fn plist_strings(items: &[&str]) -> String {
        let list: String = items.iter().map(|i| format!("<string>{}</string>", xml(i))).collect();
        format!("<array>{list}</array>")
    }

    fn plist_document_type(package: &Package, kind: &FileType) -> String {
        let mut out = format!(
            "    <dict>\n      <key>CFBundleTypeName</key><string>{}</string>\n      <key>CFBundleTypeRole</key><string>Editor</string>\n      \
             <key>LSHandlerRank</key><string>{}</string>\n",
            xml(kind.name),
            if kind.own { "Owner" } else { "Alternate" }
        );
        if kind.folder {
            out.push_str("      <key>LSTypeIsPackage</key><true/>\n");
        }
        if kind.own {
            out.push_str(&format!("      <key>LSItemContentTypes</key>{}\n", plist_strings(&[&uniform_type(package, kind)])));
        } else {
            out.push_str(&format!("      <key>CFBundleTypeExtensions</key>{}\n", plist_strings(kind.extensions)));
        }
        out.push_str("    </dict>\n");
        out
    }

    fn plist_exported_type(package: &Package, kind: &FileType) -> String {
        let conforms: &[&str] = match (kind.folder, kind.parent) {
            (true, _) => &["com.apple.package", "public.composite-content"],
            (false, "application/zip") => &["public.zip-archive"],
            (false, "text/plain") => &["public.plain-text"],
            (false, _) => &["public.data"],
        };
        format!(
            "    <dict>\n      <key>UTTypeIdentifier</key><string>{}</string>\n      <key>UTTypeDescription</key><string>{}</string>\n      \
             <key>UTTypeConformsTo</key>{}\n      <key>UTTypeTagSpecification</key>\n      <dict>\n        \
             <key>public.filename-extension</key>{}\n        <key>public.mime-type</key><string>{}</string>\n      </dict>\n    </dict>\n",
            uniform_type(package, kind),
            xml(kind.name),
            plist_strings(conforms),
            plist_strings(kind.extensions),
            kind.mime
        )
    }

    /// The `Info.plist` of the macOS app: its name, id, program, version, icon and file types.
    pub fn info_plist(product: &str, package: &Package, program: &str, version: &str) -> String {
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  \
             <key>CFBundleName</key><string>{name}</string>\n  <key>CFBundleDisplayName</key><string>{name}</string>\n  \
             <key>CFBundleIdentifier</key><string>{id}</string>\n  <key>CFBundleExecutable</key><string>{program}</string>\n  \
             <key>CFBundlePackageType</key><string>APPL</string>\n  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\n  \
             <key>CFBundleShortVersionString</key><string>{version}</string>\n  <key>CFBundleVersion</key><string>{version}</string>\n  \
             <key>LSMinimumSystemVersion</key><string>{MACOS_MINIMUM}</string>\n  <key>NSHighResolutionCapable</key><true/>\n  \
             <key>CFBundleIconFile</key><string>{id}</string>\n",
            name = xml(product),
            id = package.app_id,
        );
        if !package.file_types.is_empty() {
            out.push_str("  <key>CFBundleDocumentTypes</key>\n  <array>\n");
            for kind in package.file_types {
                out.push_str(&plist_document_type(package, kind));
            }
            out.push_str("  </array>\n");
        }
        let own: Vec<&FileType> = package.file_types.iter().filter(|k| k.own).collect();
        if !own.is_empty() {
            out.push_str("  <key>UTExportedTypeDeclarations</key>\n  <array>\n");
            for kind in own {
                out.push_str(&plist_exported_type(package, kind));
            }
            out.push_str("  </array>\n");
        }
        out.push_str("</dict>\n</plist>\n");
        out
    }

    /// The macOS icon type that holds a PNG of `size` pixels a side.
    pub fn icns_type(size: u32) -> Option<&'static [u8; 4]> {
        match size {
            16 => Some(b"icp4"),
            32 => Some(b"icp5"),
            64 => Some(b"icp6"),
            128 => Some(b"ic07"),
            256 => Some(b"ic08"),
            512 => Some(b"ic09"),
            1024 => Some(b"ic10"),
            _ => None,
        }
    }

    /// A macOS icon file holding the PNGs as they are: the `icns` header, then one (type,
    /// length, data) entry for each. Lengths count the 8-byte headers.
    pub fn icns(images: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let total = 8 + images.iter().map(|(_, png)| 8 + png.len()).sum::<usize>();
        let length = |n: usize| u32::try_from(n).unwrap_or(u32::MAX).to_be_bytes();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"icns");
        out.extend_from_slice(&length(total));
        for (kind, png) in images {
            out.extend_from_slice(*kind);
            out.extend_from_slice(&length(8 + png.len()));
            out.extend_from_slice(png);
        }
        out
    }

    /// The Linux package's `install.sh`: the folder is laid out like `~/.local`, so installing
    /// is a copy.
    pub fn install_script(product: &str, package: &Package) -> String {
        format!(
            "#!/bin/sh\n# Install {product} for the current user from this unpacked folder: the programs, the menu\n\
             # entry, the icons, the file types and the license texts. PREFIX=<folder> installs elsewhere.\n\
             # The app updates itself in place from then on; uninstall.sh removes it.\nset -eu\n\
             here=$(cd \"$(dirname \"$0\")\" && pwd)\nprefix=${{PREFIX:-$HOME/.local}}\n\
             mkdir -p \"$prefix/bin\" \"$prefix/share\"\ncp \"$here\"/bin/* \"$prefix/bin/\"\ncp -R \"$here/share/.\" \"$prefix/share/\"\n\
             if command -v update-mime-database >/dev/null 2>&1; then update-mime-database \"$prefix/share/mime\" >/dev/null 2>&1 || true; fi\n\
             if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database \"$prefix/share/applications\" >/dev/null 2>&1 || true; fi\n\
             echo \"{product} is installed in $prefix/bin. Its license texts are in $prefix/share/{id}.\"\n\
             case \":$PATH:\" in\n  *\":$prefix/bin:\"*) ;;\n  *) echo \"$prefix/bin is not on PATH: add it, so that the menu entry and a terminal find the program.\" ;;\nesac\n",
            id = package.id
        )
    }

    /// The Linux package's `uninstall.sh`: removes the files `install.sh` copied, by name.
    /// `files` are the package's files under `bin/` and `share/`, as paths from its top.
    pub fn uninstall_script(product: &str, package: &Package, files: &[String]) -> String {
        let mut out = format!(
            "#!/bin/sh\n# Remove what install.sh installed of {product}. Settings and documents are left alone.\nset -eu\nprefix=${{PREFIX:-$HOME/.local}}\n"
        );
        for file in files.iter().filter(|f| !f.starts_with(&format!("share/{}/", package.id))) {
            out.push_str(&format!("rm -f \"$prefix/{file}\"\n"));
        }
        out.push_str(&format!(
            "rm -rf \"$prefix/share/{id}\"\n\
             if command -v update-mime-database >/dev/null 2>&1; then update-mime-database \"$prefix/share/mime\" >/dev/null 2>&1 || true; fi\n\
             if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database \"$prefix/share/applications\" >/dev/null 2>&1 || true; fi\n\
             echo \"{product} is removed from $prefix.\"\n",
            id = package.id
        ));
        out
    }

    /// The name a Windows file type is registered under.
    fn windows_class(package: &Package, extension: &str) -> String {
        format!("{}.{extension}", package.app_id)
    }

    /// The Windows package's `register-file-types.cmd`, or nothing when the app opens no file
    /// type. It registers for the current user and finds the program beside itself, so it is
    /// run again after the folder is moved.
    pub fn register_script(product: &str, package: &Package, program: &str) -> Option<String> {
        if package.file_types.is_empty() {
            return None;
        }
        let mut lines = vec![
            "@echo off".to_owned(),
            format!("rem Register {product}'s file types for the current user. The program is found beside this"),
            "rem file: run it again after moving the folder. unregister-file-types.cmd takes them away.".to_owned(),
            format!("set \"APP=%~dp0{program}.exe\""),
        ];
        let classes = "HKCU\\Software\\Classes";
        for kind in package.file_types {
            for extension in kind.extensions {
                let class = windows_class(package, extension);
                if kind.folder {
                    let key = format!("{classes}\\Directory\\shell\\{class}");
                    lines.push(format!("reg add \"{key}\" /ve /d \"Open with {product}\" /f >nul"));
                    lines.push(format!("reg add \"{key}\" /v \"AppliesTo\" /d \"System.FileName:\\\"*.{extension}\\\"\" /f >nul"));
                    lines.push(format!("reg add \"{key}\\command\" /ve /d \"\\\"%APP%\\\" \\\"%%1\\\"\" /f >nul"));
                    continue;
                }
                lines.push(format!("reg add \"{classes}\\{class}\" /ve /d \"{}\" /f >nul", kind.name));
                lines.push(format!("reg add \"{classes}\\{class}\\DefaultIcon\" /ve /d \"\\\"%APP%\\\",0\" /f >nul"));
                lines.push(format!("reg add \"{classes}\\{class}\\shell\\open\\command\" /ve /d \"\\\"%APP%\\\" \\\"%%1\\\"\" /f >nul"));
                if kind.own {
                    lines.push(format!("reg add \"{classes}\\.{extension}\" /ve /d \"{class}\" /f >nul"));
                } else {
                    lines.push(format!("reg add \"{classes}\\.{extension}\\OpenWithProgids\" /v \"{class}\" /t REG_NONE /f >nul"));
                }
            }
        }
        lines.push(format!("echo {product}'s file types are registered."));
        Some(lines.join("\r\n") + "\r\n")
    }

    /// The Windows package's `unregister-file-types.cmd`: removes what the other one added.
    pub fn unregister_script(product: &str, package: &Package) -> Option<String> {
        if package.file_types.is_empty() {
            return None;
        }
        let mut lines = vec!["@echo off".to_owned(), "rem Remove what register-file-types.cmd added.".to_owned()];
        let classes = "HKCU\\Software\\Classes";
        for kind in package.file_types {
            for extension in kind.extensions {
                let class = windows_class(package, extension);
                if kind.folder {
                    lines.push(format!("reg delete \"{classes}\\Directory\\shell\\{class}\" /f >nul 2>&1"));
                    continue;
                }
                lines.push(format!("reg delete \"{classes}\\{class}\" /f >nul 2>&1"));
                if kind.own {
                    lines.push(format!("reg delete \"{classes}\\.{extension}\" /f >nul 2>&1"));
                } else {
                    lines.push(format!("reg delete \"{classes}\\.{extension}\\OpenWithProgids\" /v \"{class}\" /f >nul 2>&1"));
                }
            }
        }
        lines.push(format!("echo {product}'s file types are removed."));
        Some(lines.join("\r\n") + "\r\n")
    }

    // ------------------------------------------------------------------ checksums

    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    /// SHA-256, a block at a time (FIPS 180-4). The gate has no dependencies, so it carries this
    /// instead of a crate; the tests hold the standard's own examples.
    pub struct Sha256 {
        state: [u32; 8],
        block: Vec<u8>,
        length: u64,
    }

    impl Default for Sha256 {
        fn default() -> Self {
            Sha256 {
                state: [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19],
                block: Vec::with_capacity(64),
                length: 0,
            }
        }
    }

    impl Sha256 {
        fn compress(state: &mut [u32; 8], block: &[u8]) {
            let mut w = [0u32; 64];
            for (word, bytes) in w.iter_mut().zip(block.as_chunks::<4>().0) {
                *word = u32::from_be_bytes(*bytes);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let t1 = h.wrapping_add(s1).wrapping_add((e & f) ^ (!e & g)).wrapping_add(K[i]).wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let t2 = s0.wrapping_add((a & b) ^ (a & c) ^ (b & c));
                (h, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
            }
            for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
                *word = word.wrapping_add(add);
            }
        }

        /// Take more of the data.
        pub fn update(&mut self, mut data: &[u8]) {
            self.length = self.length.wrapping_add(data.len() as u64);
            while !data.is_empty() {
                let take = (64 - self.block.len()).min(data.len());
                let (now, later) = data.split_at(take);
                self.block.extend_from_slice(now);
                data = later;
                if self.block.len() == 64 {
                    Self::compress(&mut self.state, &self.block);
                    self.block.clear();
                }
            }
        }

        /// The checksum of everything taken, as 64 lower-case hexadecimal digits.
        pub fn hex(mut self) -> String {
            let bits = self.length.wrapping_mul(8);
            let mut tail = vec![0x80u8];
            tail.resize(1 + (119 - self.block.len()) % 64, 0);
            tail.extend_from_slice(&bits.to_be_bytes());
            self.update(&tail);
            self.state.iter().map(|word| format!("{word:08x}")).collect()
        }
    }

    /// The SHA-256 checksum of a file, read a piece at a time.
    pub fn sha256_of(path: &Path) -> Result<String, String> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut sum = Sha256::default();
        let mut piece = vec![0u8; 1 << 16];
        loop {
            let read = file.read(&mut piece).map_err(|e| format!("{}: {e}", path.display()))?;
            if read == 0 {
                return Ok(sum.hex());
            }
            sum.update(piece.get(..read).unwrap_or_default());
        }
    }

    /// The text of `SHA256SUMS.txt` for (file name, checksum) pairs: for each file, in the
    /// order of the names, a line of the checksum, two spaces, the bare name and a line feed.
    /// This is what `sha256sum` writes and reads, and what the core's updater reads.
    pub fn sums_text(files: &[(String, String)]) -> String {
        let mut sorted: Vec<&(String, String)> = files.iter().collect();
        sorted.sort();
        sorted.iter().map(|(name, sum)| format!("{sum}  {name}\n")).collect()
    }

    /// The platform whose release file of `id` and `version` is named `name`.
    pub fn platform_of(name: &str, id: &str, version: &str) -> Option<Platform> {
        Platform::ALL.into_iter().find(|p| file_name(id, version, *p) == name)
    }

    /// What is wrong with the names of a folder of release files, for a release of `id` and
    /// `version`: a file that is none of the release's, and with `complete` a platform without
    /// its file. The checksum file itself is not a problem: it is written again.
    pub fn set_problems(names: &[String], id: &str, version: &str, complete: bool) -> Vec<String> {
        let mut out = Vec::new();
        for name in names.iter().filter(|n| *n != SUMS) {
            if platform_of(name, id, version).is_none() {
                out.push(format!("{name} is not a release file of {id} {version}"));
            }
        }
        for platform in Platform::ALL {
            let wanted = file_name(id, version, platform);
            if complete && !names.contains(&wanted) {
                out.push(format!("{wanted} is missing"));
            }
        }
        if !names.iter().any(|n| platform_of(n, id, version).is_some()) {
            out.push(format!("no release file of {id} {version} is here"));
        }
        out
    }

    /// `cargo xtask package --sums <folder> [--complete]`.
    fn sums_task(root: &Path, app: &App, folder: &str, complete: bool) -> TaskResult {
        let package = data(app)?;
        let version = release::checkout_version(root)?;
        let folder = root.join(folder);
        let entries = std::fs::read_dir(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let mut names: Vec<String> = entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        let problems = set_problems(&names, package.id, &version, complete);
        if !problems.is_empty() {
            return Err(format!("{} does not hold a release of {} {version}:\n  - {}", folder.display(), package.id, problems.join("\n  - ")));
        }
        let mut sums = Vec::new();
        for name in names.iter().filter(|n| *n != SUMS) {
            sums.push((name.clone(), sha256_of(&folder.join(name))?));
        }
        let text = sums_text(&sums);
        std::fs::write(folder.join(SUMS), &text).map_err(|e| format!("{SUMS}: {e}"))?;
        print!("{text}");
        println!("xtask: wrote {}", folder.join(SUMS).display());
        Ok(())
    }

    // ------------------------------------------------------------------ building a package

    fn copy(from: &Path, to: &Path) -> TaskResult {
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{} to {}: {e}", from.display(), to.display()))
    }

    fn write(path: &Path, bytes: &[u8]) -> TaskResult {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A script a person runs: readable and runnable by everyone, writable by its owner.
    fn write_script(path: &Path, text: &str) -> TaskResult {
        write(path, text.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    }

    /// Runs a command, showing its output. `shown` is what an error calls it: never the
    /// arguments themselves, one of which may be a password.
    fn run(command: &mut Command, shown: &str) -> TaskResult {
        let status = command.status().map_err(|e| format!("could not run {shown}: {e}"))?;
        if status.success() { Ok(()) } else { Err(format!("{shown} exited with {status}")) }
    }

    /// What one package is made from.
    pub struct Build<'a> {
        pub root: &'a Path,
        pub app: &'a App,
        pub package: &'a Package,
        pub version: &'a str,
        /// Each program's name and the file built for it.
        pub programs: &'a [(String, PathBuf)],
        /// The text of `THIRD-PARTY-LICENSES.txt`.
        pub third_party: &'a str,
        /// The license texts of fonts that dependencies build in ([`dependency_font_licenses`]).
        pub font_licenses: &'a [PathBuf],
    }

    impl Build<'_> {
        /// Puts the license files in `dir`: the app's own, `THIRD-PARTY-LICENSES.txt`, and under
        /// `licenses/` the texts of what the programs embed.
        fn legal_files(&self, dir: &Path) -> TaskResult {
            for file in LEGAL.iter().chain(self.app.legal_files) {
                copy(&self.root.join(file), &dir.join(file))?;
            }
            write(&dir.join(licenses::FILE_NAME), self.third_party.as_bytes())?;
            // A dependency's fonts first, where an app keeps its own: the app's file of the
            // same name, written after it, is the one that stays.
            for text in self.font_licenses {
                let name = text.file_name().ok_or_else(|| format!("{}: no file name", text.display()))?;
                copy(text, &dir.join(LICENSES_DIR).join(FONTS).join(name))?;
            }
            for text in license_texts(self.root, self.package.license_files)? {
                copy(&self.root.join(&text), &dir.join(LICENSES_DIR).join(&text))?;
            }
            Ok(())
        }

        fn desktop_program(&self) -> Result<&str, String> {
            self.programs.first().map(|(name, _)| name.as_str()).ok_or_else(|| "no program to package".to_owned())
        }

        /// The Linux package's folder: laid out like `~/.local`, with the two scripts on top.
        pub fn stage_linux(&self, stage: &Path) -> TaskResult {
            let (package, product) = (self.package, self.app.product);
            for (name, file) in self.programs {
                copy(file, &stage.join("bin").join(name))?;
            }
            let entry = desktop_entry(product, package, self.desktop_program()?);
            write(&stage.join(format!("share/applications/{}.desktop", package.app_id)), entry.as_bytes())?;
            if let Some(types) = mime_info(package) {
                write(&stage.join(format!("share/mime/packages/{}.xml", package.app_id)), types.as_bytes())?;
            }
            for (size, path) in package.icons {
                copy(&self.root.join(path), &stage.join(format!("share/icons/hicolor/{size}x{size}/apps/{}.png", package.app_id)))?;
            }
            self.legal_files(&stage.join("share").join(package.id))?;
            let files: Vec<String> = files_under(stage)?.into_iter().filter(|f| f.starts_with("bin/") || f.starts_with("share/")).collect();
            write_script(&stage.join("install.sh"), &install_script(product, package))?;
            write_script(&stage.join("uninstall.sh"), &uninstall_script(product, package, &files))
        }

        /// The Windows package's folder: the programs and the license files at its top.
        pub fn stage_windows(&self, stage: &Path) -> TaskResult {
            let (package, product) = (self.package, self.app.product);
            for (name, file) in self.programs {
                copy(file, &stage.join(format!("{name}.exe")))?;
            }
            self.legal_files(stage)?;
            let program = self.desktop_program()?;
            if let (Some(register), Some(unregister)) = (register_script(product, package, program), unregister_script(product, package)) {
                write(&stage.join("register-file-types.cmd"), register.as_bytes())?;
                write(&stage.join("unregister-file-types.cmd"), unregister.as_bytes())?;
            }
            Ok(())
        }

        /// The macOS package's folder: the app, with the programs and the license files inside
        /// it, and the license files again beside it. Returns the app's folder.
        pub fn stage_macos(&self, stage: &Path) -> Result<PathBuf, String> {
            let (package, product) = (self.package, self.app.product);
            let bundle = stage.join(format!("{product}.app"));
            let contents = bundle.join("Contents");
            for (name, file) in self.programs {
                copy(file, &contents.join("MacOS").join(name))?;
            }
            write(&contents.join("Info.plist"), info_plist(product, package, self.desktop_program()?, self.version).as_bytes())?;
            let pngs = if package.macos_icons.is_empty() { package.icons } else { package.macos_icons };
            let mut images = Vec::new();
            for (size, path) in pngs {
                if let Some(kind) = icns_type(*size) {
                    images.push((kind, std::fs::read(self.root.join(path)).map_err(|e| format!("{path}: {e}"))?));
                }
            }
            write(&contents.join(format!("Resources/{}.icns", package.app_id)), &icns(&images))?;
            // Inside the app, so the notices stay with it once it is copied off the disk image.
            self.legal_files(&contents.join("Resources"))?;
            self.legal_files(&stage.join("Legal"))?;
            Ok(bundle)
        }
    }

    /// What a staged package must hold before it is packed, as paths from its top.
    pub fn expected_files(platform: Platform, app: &App, package: &Package, programs: &[String]) -> Vec<String> {
        let (legal, program_dir, exe) = match platform {
            Platform::LinuxX64 => (format!("share/{}/", package.id), "bin/".to_owned(), ""),
            Platform::WindowsX64 => (String::new(), String::new(), ".exe"),
            Platform::MacosArm64 => (format!("{}.app/Contents/Resources/", app.product), format!("{}.app/Contents/MacOS/", app.product), ""),
        };
        let mut out: Vec<String> = programs.iter().map(|p| format!("{program_dir}{p}{exe}")).collect();
        out.extend(LEGAL.iter().chain(app.legal_files).map(|f| format!("{legal}{f}")));
        out.push(format!("{legal}{}", licenses::FILE_NAME));
        match platform {
            Platform::LinuxX64 => {
                out.push(format!("share/applications/{}.desktop", package.app_id));
                out.push("install.sh".to_owned());
                out.push("uninstall.sh".to_owned());
            }
            Platform::MacosArm64 => {
                out.push(format!("{}.app/Contents/Info.plist", app.product));
                out.push(format!("{}.app/Contents/Resources/{}.icns", app.product, package.app_id));
            }
            Platform::WindowsX64 => {}
        }
        out
    }

    /// Fails unless the staged package holds every file a package must: a package without its
    /// programs or its license texts is not published.
    pub fn check_stage(stage: &Path, expected: &[String]) -> TaskResult {
        let missing: Vec<&str> = expected.iter().filter(|f| !stage.join(f).is_file()).map(String::as_str).collect();
        let third_party = expected.iter().find(|f| f.ends_with(licenses::FILE_NAME));
        let empty = third_party.is_some_and(|f| std::fs::metadata(stage.join(f)).is_ok_and(|m| m.len() < 64));
        if missing.is_empty() && !empty {
            return Ok(());
        }
        let mut what = missing.join(", ");
        if empty {
            what.push_str(&format!("{}an empty {}", if what.is_empty() { "" } else { ", " }, licenses::FILE_NAME));
        }
        Err(format!("the package in {} lacks {what}", stage.display()))
    }

    /// Whether every image of an `.ico` file is in a Windows program byte for byte, which is
    /// how a build script's resource compiler embeds it. Returns how many images that is.
    pub fn icon_embedded(ico: &[u8], program: &[u8]) -> Result<usize, String> {
        let number = |at: usize, len: usize| ico.get(at..at + len).map(|b| b.iter().rev().fold(0usize, |n, b| (n << 8) | usize::from(*b)));
        let count = number(4, 2).filter(|n| *n > 0 && number(0, 4) == Some(0x0001_0000)).ok_or("not an .ico file")?;
        for i in 0..count {
            let entry = 6 + 16 * i;
            let (size, offset) = (number(entry + 8, 4).ok_or("a cut-off .ico file")?, number(entry + 12, 4).ok_or("a cut-off .ico file")?);
            let image = ico.get(offset..offset + size).filter(|image| !image.is_empty()).ok_or("an .ico entry that points outside the file")?;
            if !program.windows(image.len()).any(|window| window == image) {
                return Err(format!("image {} of {count} is not in the program", i + 1));
            }
        }
        Ok(count)
    }

    /// Signing, asked for through the environment and left out, with a line that says so, when
    /// it is not. The release workflow sets these from the repository's secrets:
    ///
    /// - `PACKAGE_MACOS_IDENTITY`: the name or hash of a Developer ID Application certificate in
    ///   a keychain on the search list. Without it the app is signed ad hoc, which lets it start
    ///   on Apple silicon and claims no identity.
    /// - `PACKAGE_WINDOWS_PFX` (a certificate file with its key) and
    ///   `PACKAGE_WINDOWS_PFX_PASSWORD`, with `PACKAGE_WINDOWS_SIGNTOOL` (the program, when it
    ///   is not on the path) and `PACKAGE_WINDOWS_TIMESTAMP` (another timestamp server).
    pub mod sign {
        use std::path::{Path, PathBuf};
        use std::process::Command;

        use super::{TaskResult, run};

        const TIMESTAMP: &str = "http://timestamp.digicert.com";

        fn asked(name: &str) -> Option<String> {
            std::env::var(name).ok().filter(|value| !value.trim().is_empty())
        }

        /// Sign the programs of a Windows package, before they are packed.
        pub fn windows(programs: &[PathBuf]) -> TaskResult {
            let Some(certificate) = asked("PACKAGE_WINDOWS_PFX") else {
                println!("not signed: no certificate was given (PACKAGE_WINDOWS_PFX)");
                return Ok(());
            };
            let tool = asked("PACKAGE_WINDOWS_SIGNTOOL").unwrap_or_else(|| "signtool".to_owned());
            let timestamp = asked("PACKAGE_WINDOWS_TIMESTAMP").unwrap_or_else(|| TIMESTAMP.to_owned());
            let mut command = Command::new(tool);
            command.args(["sign", "/fd", "SHA256", "/td", "SHA256", "/tr", &timestamp, "/f", &certificate]);
            if let Some(password) = asked("PACKAGE_WINDOWS_PFX_PASSWORD") {
                command.args(["/p", &password]);
            }
            run(command.args(programs), "signtool sign")?;
            println!("signed {} program(s) with the certificate given", programs.len());
            Ok(())
        }

        /// Sign the macOS app: with the identity given, under the hardened runtime and with a
        /// timestamp, as notarisation asks; else ad hoc.
        pub fn macos_app(bundle: &Path, other_programs: &[PathBuf]) -> Result<Option<String>, String> {
            let Some(identity) = asked("PACKAGE_MACOS_IDENTITY") else {
                println!("signed ad hoc: no identity was given (PACKAGE_MACOS_IDENTITY)");
                run(Command::new("codesign").args(["--force", "--deep", "--sign", "-"]).arg(bundle), "codesign")?;
                return Ok(None);
            };
            // Inside out: a program beside the main one first, then the app, which seals them.
            for file in other_programs.iter().map(PathBuf::as_path).chain([bundle]) {
                run(Command::new("codesign").args(["--force", "--options", "runtime", "--timestamp", "--sign", &identity]).arg(file), "codesign")?;
            }
            run(Command::new("codesign").args(["--verify", "--strict", "--deep"]).arg(bundle), "codesign --verify")?;
            Ok(Some(identity))
        }

        /// Sign the disk image with the identity the app was signed with.
        pub fn macos_image(image: &Path, identity: &str) -> TaskResult {
            run(Command::new("codesign").args(["--force", "--timestamp", "--sign", identity]).arg(image), "codesign")
        }
    }

    fn pack_linux(build: &Build, out_dir: &Path, name: &str, out: &Path) -> TaskResult {
        let stage = out_dir.join(name);
        build.stage_linux(&stage)?;
        let programs: Vec<String> = build.programs.iter().map(|(n, _)| n.clone()).collect();
        check_stage(&stage, &expected_files(Platform::LinuxX64, build.app, build.package, &programs))?;
        // No name of whoever built it goes into the archive, and no file in it can be written
        // by anyone but its owner, whatever the building system's own defaults are.
        let mut tar = Command::new("tar");
        tar.current_dir(out_dir).args(["--owner=0", "--group=0", "--numeric-owner", "--mode=go-w", "-czf"]).arg(out).arg(name);
        run(&mut tar, "tar")
    }

    fn pack_windows(build: &Build, out_dir: &Path, name: &str, out: &Path) -> TaskResult {
        let stage = out_dir.join(name);
        build.stage_windows(&stage)?;
        let programs: Vec<String> = build.programs.iter().map(|(n, _)| n.clone()).collect();
        check_stage(&stage, &expected_files(Platform::WindowsX64, build.app, build.package, &programs))?;
        if let Some(icon) = build.package.windows_icon {
            let ico = std::fs::read(build.root.join(icon)).map_err(|e| format!("{icon}: {e}"))?;
            let program = stage.join(format!("{}.exe", build.desktop_program()?));
            let bytes = std::fs::read(&program).map_err(|e| format!("{}: {e}", program.display()))?;
            let images = icon_embedded(&ico, &bytes).map_err(|e| format!("{icon}: {e} (the app's build script embeds it)"))?;
            println!("all {images} images of {icon} are in the program");
        }
        let staged: Vec<PathBuf> = programs.iter().map(|p| stage.join(format!("{p}.exe"))).collect();
        sign::windows(&staged)?;
        // Windows' own tar writes zip archives; the one a Unix shell brings does not.
        let system = std::env::var_os("SystemRoot").map(|dir| PathBuf::from(dir).join("System32").join("tar.exe")).filter(|tar| tar.is_file());
        let mut tar = Command::new(system.unwrap_or_else(|| PathBuf::from("tar")));
        tar.current_dir(out_dir).args(["-a", "-c", "-f"]).arg(out).arg(name);
        run(&mut tar, "tar")
    }

    fn pack_macos(build: &Build, out_dir: &Path, name: &str, out: &Path) -> TaskResult {
        let stage = out_dir.join(name);
        let bundle = build.stage_macos(&stage)?;
        let programs: Vec<String> = build.programs.iter().map(|(n, _)| n.clone()).collect();
        check_stage(&stage, &expected_files(Platform::MacosArm64, build.app, build.package, &programs))?;
        let others: Vec<PathBuf> = programs.iter().skip(1).map(|p| bundle.join("Contents/MacOS").join(p)).collect();
        let identity = sign::macos_app(&bundle, &others)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink("/Applications", stage.join("Applications")).map_err(|e| format!("the link to Applications: {e}"))?;
        // The disk image tool now and then finds the folder busy; a second try gets through.
        let mut made = Err(String::new());
        for _ in 0..3 {
            let mut hdiutil = Command::new("hdiutil");
            // The file system is left to the tool: the one it picks is one this system's own
            // version of it can still make.
            hdiutil.args(["create", "-volname", build.app.product, "-format", "UDZO", "-ov", "-srcfolder"]).arg(&stage).arg(out);
            made = run(&mut hdiutil, "hdiutil create");
            if made.is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
        made?;
        match identity {
            Some(identity) => sign::macos_image(out, &identity),
            None => Ok(()),
        }
    }

    /// What `cargo xtask package` was asked for.
    #[derive(Debug, PartialEq, Eq)]
    pub enum Asked {
        Describe,
        Sums { folder: String, complete: bool },
        Build { platform: Option<Platform>, out: Option<String>, build: bool },
    }

    const USAGE: &str = "usage: cargo xtask package [linux-x86_64 | macos-aarch64 | windows-x64] [--out <folder>] [--no-build]\n       \
                         cargo xtask package --sums <folder> [--complete]\n       cargo xtask package --describe";

    /// Read the arguments of `package`.
    pub fn asked(args: &[String]) -> Result<Asked, String> {
        let (mut platform, mut out, mut build, mut sums, mut complete, mut describe) = (None, None, true, None, false, false);
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--describe" => describe = true,
                "--complete" => complete = true,
                "--no-build" => build = false,
                "--sums" => sums = Some(it.next().ok_or("--sums needs a folder")?.clone()),
                "--out" => out = Some(it.next().ok_or("--out needs a folder")?.clone()),
                token if !token.starts_with('-') && platform.is_none() => {
                    platform = Some(Platform::from_token(token).ok_or_else(|| format!("'{token}' is no platform of a release. {USAGE}"))?);
                }
                other => return Err(format!("unknown argument '{other}'. {USAGE}")),
            }
        }
        match (describe, sums) {
            (true, None) if platform.is_none() && out.is_none() && !complete => Ok(Asked::Describe),
            (false, Some(folder)) if platform.is_none() && out.is_none() => Ok(Asked::Sums { folder, complete }),
            (false, None) if !complete => Ok(Asked::Build { platform, out, build }),
            _ => Err(USAGE.to_owned()),
        }
    }

    /// Where Cargo puts what it builds.
    fn target_dir(root: &Path) -> Result<PathBuf, String> {
        let out = cargo_output(root, &["metadata", "--format-version", "1", "--no-deps", "--locked"])?;
        let meta = json::parse(&out)?;
        meta.str_of("target_directory").map(PathBuf::from).ok_or_else(|| "cargo metadata names no target directory".to_owned())
    }

    fn build_task(root: &Path, app: &App, platform: Option<Platform>, out: Option<String>, build: bool) -> TaskResult {
        check(root, app)?;
        let package = data(app)?;
        let host = Platform::host();
        let platform = platform
            .or(host)
            .ok_or("a release has no file for this system: packages are made on Linux x86_64, macOS on Apple silicon and Windows x64")?;
        if host != Some(platform) {
            return Err(format!("a package is made on its own system: {} is not made here", platform.token()));
        }
        let version = release::checkout_version(root)?;
        let found = programs(root, app)?;
        if build {
            let mut args = vec!["build", "--release", "--locked"];
            for program in &found {
                args.extend(["-p", program.package.as_str()]);
            }
            cargo(root, &args)?;
        }
        let built = target_dir(root)?.join("release");
        let files: Vec<(String, PathBuf)> =
            found.iter().map(|p| (p.name.clone(), built.join(format!("{}{}", p.name, std::env::consts::EXE_SUFFIX)))).collect();
        if let Some((name, file)) = files.iter().find(|(_, file)| !file.is_file()) {
            return Err(format!("the program {name} is not built ({} is not there)", file.display()));
        }
        let third_party = licenses::third_party_licenses(root, app.shipped, app.product)?;
        let font_licenses = dependency_font_licenses(&licenses::shipped_packages(root, app.shipped)?);
        let out_dir = root.join(out.as_deref().unwrap_or(DIST));
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
        let name = stem(package.id, &version, platform);
        let out = out_dir.join(file_name(package.id, &version, platform));
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_dir_all(out_dir.join(&name));
        let build = Build { root, app, package, version: &version, programs: &files, third_party: &third_party, font_licenses: &font_licenses };
        match platform {
            Platform::LinuxX64 => pack_linux(&build, &out_dir, &name, &out),
            Platform::WindowsX64 => pack_windows(&build, &out_dir, &name, &out),
            Platform::MacosArm64 => pack_macos(&build, &out_dir, &name, &out),
        }?;
        // The folder the package was made from is not a release file.
        std::fs::remove_dir_all(out_dir.join(&name)).map_err(|e| format!("{}: {e}", out_dir.join(&name).display()))?;
        println!("\nxtask: package written to {}", out.display());
        Ok(())
    }

    /// `cargo xtask package ...` in an app.
    pub fn task(root: &Path, app: &App, args: &[String]) -> TaskResult {
        match asked(args)? {
            Asked::Describe => describe(app, &release::checkout_version(root)?),
            Asked::Sums { folder, complete } => sums_task(root, app, &folder, complete),
            Asked::Build { platform, out, build } => build_task(root, app, platform, out, build),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const TYPES: &[FileType] = &[
            FileType { name: "Sample document", extensions: &["smp"], mime: "application/x-sample", ..FileType::DEFAULT },
            FileType { name: "Sample project", extensions: &["smpd"], mime: "application/x-sample-project", folder: true, ..FileType::DEFAULT },
            FileType {
                name: "Plain notes",
                extensions: &["notes", "nts"],
                mime: "text/x-notes",
                parent: "text/plain",
                own: false,
                ..FileType::DEFAULT
            },
        ];
        const PACKAGE: Package = Package {
            id: "sample-app",
            app_id: "org.example.sample",
            summary: "Write samples",
            categories: "Office;WordProcessor;",
            icons: &[(48, "assets/icon-48.png"), (256, "assets/icon-256.png")],
            file_types: TYPES,
            license_files: &["assets/fonts"],
            ..Package::DEFAULT
        };
        const APP: App = App { product: "Sample App", shipped: &["sample-app", "sample-app-cli"], package: Some(PACKAGE), ..App::DEFAULT };

        /// A PNG file's first 24 bytes, which is all that is read of it here.
        fn png(size: u32) -> Vec<u8> {
            let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
            out.extend_from_slice(&size.to_be_bytes());
            out.extend_from_slice(&size.to_be_bytes());
            out
        }

        /// A scratch folder with what `APP` names, and two files standing in for its programs.
        fn scratch(name: &str) -> (PathBuf, Vec<(String, PathBuf)>) {
            let root = std::env::temp_dir().join(format!("xtask-package-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for file in LEGAL {
                write(&root.join(file), file.as_bytes()).unwrap();
            }
            write(&root.join("assets/icon-48.png"), &png(48)).unwrap();
            write(&root.join("assets/icon-256.png"), &png(256)).unwrap();
            write(&root.join("assets/fonts/serif/OFL.txt"), b"SIL Open Font License").unwrap();
            write(&root.join("assets/fonts/serif/regular.ttf"), b"").unwrap();
            let programs: Vec<(String, PathBuf)> = ["sample-app", "sample-app-cli"].map(|p| (p.to_owned(), root.join("built").join(p))).to_vec();
            for (name, file) in &programs {
                write(file, name.as_bytes()).unwrap();
            }
            (root, programs)
        }

        const THIRD_PARTY: &str =
            "Third-party licenses\n\nThe programs of Sample App are built from the packages below, each under its own license.\n";

        #[test]
        fn release_files_are_named_as_the_contract_says() {
            let names: Vec<String> = Platform::ALL.iter().map(|p| file_name("sample-app", "1.2.3", *p)).collect();
            assert_eq!(
                names,
                ["sample-app-1.2.3-linux-x86_64.tar.gz", "sample-app-1.2.3-macos-aarch64.dmg", "sample-app-1.2.3-windows-x64-portable.zip"]
            );
            assert_eq!(stem("sample-app", "1.2.3", Platform::WindowsX64), "sample-app-1.2.3-windows-x64");
            assert_eq!(tag("1.2.3"), "v1.2.3");
            for platform in Platform::ALL {
                assert_eq!(Platform::from_token(platform.token()), Some(platform));
                assert_eq!(platform_of(&file_name("sample-app", "1.2.3", platform), "sample-app", "1.2.3"), Some(platform));
            }
            assert_eq!(Platform::of("linux", "x86_64"), Some(Platform::LinuxX64));
            assert_eq!(Platform::of("macos", "aarch64"), Some(Platform::MacosArm64));
            assert_eq!(Platform::of("windows", "x86_64"), Some(Platform::WindowsX64));
            // No file for a system the releases do not cover.
            assert_eq!(Platform::of("macos", "x86_64"), None);
            assert_eq!(Platform::of("linux", "aarch64"), None);
            assert_eq!(platform_of("sample-app-1.2.4-linux-x86_64.tar.gz", "sample-app", "1.2.3"), None);
        }

        #[test]
        fn sha256_is_the_standards() {
            let hex = |data: &[u8]| {
                let mut sum = Sha256::default();
                sum.update(data);
                sum.hex()
            };
            assert_eq!(hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
            assert_eq!(hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
            assert_eq!(
                hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
            );
            // Every length around a block's end, taken in pieces, agrees with the whole.
            let data: Vec<u8> = (0..200u8).collect();
            for length in [55, 56, 57, 63, 64, 65, 119, 120, 128, 200] {
                let mut pieces = Sha256::default();
                for piece in data[..length].chunks(7) {
                    pieces.update(piece);
                }
                assert_eq!(pieces.hex(), hex(&data[..length]), "{length}");
            }
            let mut million = Sha256::default();
            for _ in 0..1000 {
                million.update(&[b'a'; 1000]);
            }
            assert_eq!(million.hex(), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
        }

        #[test]
        fn the_checksum_file_is_what_sha256sum_writes() {
            let files = [
                ("sample-app-1.2.3-windows-x64-portable.zip".to_owned(), "c".repeat(64)),
                ("sample-app-1.2.3-linux-x86_64.tar.gz".to_owned(), "a".repeat(64)),
            ];
            let text = sums_text(&files);
            assert_eq!(
                text,
                format!("{}  sample-app-1.2.3-linux-x86_64.tar.gz\n{}  sample-app-1.2.3-windows-x64-portable.zip\n", "a".repeat(64), "c".repeat(64))
            );
            // Bare names: no `./`, no `*`, no folder.
            assert!(!text.contains("./") && !text.contains('*') && !text.contains('\r'));
        }

        #[test]
        fn a_folder_of_release_files_holds_this_release_and_nothing_else() {
            let names = |list: &[&str]| list.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
            let all = names(&[
                "SHA256SUMS.txt",
                "sample-app-1.2.3-linux-x86_64.tar.gz",
                "sample-app-1.2.3-macos-aarch64.dmg",
                "sample-app-1.2.3-windows-x64-portable.zip",
            ]);
            assert!(set_problems(&all, "sample-app", "1.2.3", true).is_empty());
            let linux = names(&["sample-app-1.2.3-linux-x86_64.tar.gz"]);
            assert!(set_problems(&linux, "sample-app", "1.2.3", false).is_empty());
            assert_eq!(
                set_problems(&linux, "sample-app", "1.2.3", true),
                ["sample-app-1.2.3-macos-aarch64.dmg is missing", "sample-app-1.2.3-windows-x64-portable.zip is missing"]
            );
            let odd = names(&["sample-app-1.2.3-linux-x86_64.tar.gz", "sample-app-1.2.3-windows-x86_64.zip", "notes.md"]);
            assert_eq!(
                set_problems(&odd, "sample-app", "1.2.3", false),
                [
                    "sample-app-1.2.3-windows-x86_64.zip is not a release file of sample-app 1.2.3",
                    "notes.md is not a release file of sample-app 1.2.3"
                ]
            );
            assert_eq!(set_problems(&names(&["SHA256SUMS.txt"]), "sample-app", "1.2.3", false), ["no release file of sample-app 1.2.3 is here"]);
        }

        #[test]
        fn package_data_is_checked_as_written() {
            assert_eq!(data_problems(&APP), Vec::<String>::new());
            assert!(data_problems(&App::DEFAULT).is_empty(), "an app without package data publishes no release");
            assert!(data(&App::DEFAULT).unwrap_err().contains("no package data"));
            let with = |package: Package| data_problems(&App { package: Some(package), ..APP }).join("; ");
            assert!(with(Package { id: "Sample App", ..PACKAGE }).contains("package id 'Sample App'"));
            assert!(with(Package { app_id: "sample", ..PACKAGE }).contains("reverse-DNS"));
            assert!(with(Package { summary: "", ..PACKAGE }).contains("package summary"));
            assert!(with(Package { categories: "Office", ..PACKAGE }).contains("categories"));
            assert!(with(Package { icons: &[], ..PACKAGE }).contains("at least one PNG"));
            assert!(with(Package { icons: &[(48, "assets/icon.svg")], ..PACKAGE }).contains("a PNG file"));
            assert!(with(Package { icons: &[(48, "assets/icon-48.png")], ..PACKAGE }).contains("a macOS icon holds"));
            const BAD: &[FileType] = &[FileType { name: "Odd \"type\"", extensions: &[".Odd"], mime: "odd", ..FileType::DEFAULT }];
            let found = with(Package { file_types: BAD, ..PACKAGE });
            assert!(found.contains("the name must be one line") && found.contains("extensions are") && found.contains("media type"), "{found}");
            assert!(data_problems(&App { shipped: &[], ..APP }).join("; ").contains("names no program"));
            assert!(data_problems(&App { product: "A/B", ..APP }).join("; ").contains("product 'A/B'"));
        }

        #[test]
        fn programs_come_from_cargo_and_carry_the_release_id() {
            let meta = json::parse(
                r#"{"packages":[
                    {"name":"sample-app","targets":[{"name":"sample-app","kind":["bin"]},{"name":"build-script-build","kind":["custom-build"]}]},
                    {"name":"sample-app-cli","targets":[{"name":"sample-cli","kind":["bin"]}]},
                    {"name":"sample-lib","targets":[{"name":"sample_lib","kind":["lib"]}]},
                    {"name":"two","targets":[{"name":"a","kind":["bin"]},{"name":"b","kind":["bin"]}]}],
                  "target_directory":"/work/target"}"#,
            )
            .unwrap();
            let found = programs_from(&meta, &["sample-app", "sample-app-cli"]).unwrap();
            assert_eq!(found.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["sample-app", "sample-cli"]);
            // The second program does not start with the release id.
            assert_eq!(
                program_problems("sample-app", &found),
                ["the program `sample-cli` (sample-app-cli) must be named `sample-app-<name>`, such as `sample-app-cli`"]
            );
            let program = |name: &str| Program { package: name.to_owned(), name: name.to_owned() };
            assert!(program_problems("sample-app", &[program("sample-app"), program("sample-app-cli")]).is_empty());
            assert!(program_problems("sample-app", &[program("sample")]).join("").contains("is named `sample-app`, its release id"));
            assert!(programs_from(&meta, &["sample-cli"]).unwrap_err().contains("no package of this workspace"));
            assert!(programs_from(&meta, &["sample-lib"]).unwrap_err().contains("builds no program"));
            assert!(programs_from(&meta, &["two"]).unwrap_err().contains("builds 2 programs"));
        }

        #[test]
        fn the_files_the_data_names_are_checked() {
            let (root, _) = scratch("files");
            assert_eq!(file_problems(&root, &APP, &PACKAGE), Vec::<String>::new());
            assert_eq!(license_texts(&root, PACKAGE.license_files).unwrap(), ["assets/fonts/serif/OFL.txt"]);
            assert_eq!(png_size(&png(256)), Some((256, 256)));
            assert_eq!(png_size(b"GIF89a"), None);
            write(&root.join("assets/icon-48.png"), &png(64)).unwrap();
            std::fs::remove_file(root.join("NOTICE")).unwrap();
            std::fs::remove_file(root.join("assets/fonts/serif/OFL.txt")).unwrap();
            let found =
                file_problems(&root, &App { legal_files: &["UPSTREAM.md"], ..APP }, &Package { windows_icon: Some("assets/icon.ico"), ..PACKAGE });
            assert_eq!(
                found,
                [
                    "package icon assets/icon-48.png is 64 by 64 pixels, not 48 by 48",
                    "package windows_icon assets/icon.ico is not there",
                    "NOTICE is missing: every package carries it",
                    "UPSTREAM.md is missing: every package carries it",
                    "package license_files: assets/fonts/ holds no file named like a license (LICENSE, OFL, NOTICE, COPYING)",
                ]
            );
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn the_linux_package_is_laid_out_for_the_updater_and_carries_the_licenses() {
            let (root, programs) = scratch("linux");
            // A dependency's checkout with fonts and their licenses at its top, three folders up.
            let checkout = root.join("checkout");
            for file in [".cargo-ok", "assets/fonts/OFL-Sans.txt", "assets/fonts/Sans.ttf", "assets/fonts/readme.md", "crates/fonts/src/lib.rs"] {
                write(&checkout.join(file), b"SIL Open Font License").unwrap();
            }
            let dependency = |dir: &str, git: bool| licenses::Package {
                name: "fonts".to_owned(),
                version: "1.0.0".to_owned(),
                license: None,
                license_file: None,
                repository: None,
                authors: Vec::new(),
                dir: checkout.join(dir),
                git,
            };
            let font_licenses = dependency_font_licenses(&[dependency("crates/fonts", true), dependency("crates/other", true)]);
            assert_eq!(font_licenses, [checkout.join("assets/fonts/OFL-Sans.txt")]);
            // A package from a registry or the workspace has no checkout to look in.
            assert!(dependency_font_licenses(&[dependency("crates/fonts", false)]).is_empty());
            let build = Build {
                root: &root,
                app: &APP,
                package: &PACKAGE,
                version: "1.2.3",
                programs: &programs,
                third_party: THIRD_PARTY,
                font_licenses: &font_licenses,
            };
            let stage = root.join("dist/sample-app-1.2.3-linux-x86_64");
            build.stage_linux(&stage).unwrap();
            assert_eq!(
                files_under(&stage).unwrap(),
                [
                    // The programs sit in bin/, the desktop one under the release id: that is where the updater looks.
                    "bin/sample-app",
                    "bin/sample-app-cli",
                    "install.sh",
                    "share/applications/org.example.sample.desktop",
                    "share/icons/hicolor/256x256/apps/org.example.sample.png",
                    "share/icons/hicolor/48x48/apps/org.example.sample.png",
                    "share/mime/packages/org.example.sample.xml",
                    // share/<id>: the license files, and the folder that tells the updater to renew share/ too.
                    "share/sample-app/ATTRIBUTION.md",
                    "share/sample-app/LICENSE-APACHE",
                    "share/sample-app/LICENSE-MIT",
                    "share/sample-app/NOTICE",
                    "share/sample-app/THIRD-PARTY-LICENSES.txt",
                    // The license of a font a dependency builds in, beside the app's own fonts'.
                    "share/sample-app/licenses/assets/fonts/OFL-Sans.txt",
                    "share/sample-app/licenses/assets/fonts/serif/OFL.txt",
                    "uninstall.sh",
                ]
            );
            let names: Vec<String> = programs.iter().map(|(n, _)| n.clone()).collect();
            let expected = expected_files(Platform::LinuxX64, &APP, &PACKAGE, &names);
            check_stage(&stage, &expected).unwrap();
            let uninstall = std::fs::read_to_string(stage.join("uninstall.sh")).unwrap();
            assert!(uninstall.contains("rm -f \"$prefix/bin/sample-app-cli\"\n") && uninstall.contains("rm -rf \"$prefix/share/sample-app\"\n"));
            assert!(
                uninstall.contains("rm -f \"$prefix/share/applications/org.example.sample.desktop\"\n") && !uninstall.contains("$prefix/install.sh")
            );
            // A package without its license texts, or without a program, is refused.
            std::fs::write(stage.join("share/sample-app/THIRD-PARTY-LICENSES.txt"), "").unwrap();
            assert!(check_stage(&stage, &expected).unwrap_err().contains("an empty THIRD-PARTY-LICENSES.txt"));
            std::fs::remove_file(stage.join("share/sample-app/THIRD-PARTY-LICENSES.txt")).unwrap();
            std::fs::remove_file(stage.join("bin/sample-app")).unwrap();
            let refused = check_stage(&stage, &expected).unwrap_err();
            assert!(refused.contains("lacks bin/sample-app, share/sample-app/THIRD-PARTY-LICENSES.txt"), "{refused}");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn the_windows_and_macos_packages_carry_the_programs_and_the_licenses() {
            let (root, programs) = scratch("others");
            let build = Build {
                root: &root,
                app: &APP,
                package: &PACKAGE,
                version: "1.2.3",
                programs: &programs,
                third_party: THIRD_PARTY,
                font_licenses: &[],
            };
            let names: Vec<String> = programs.iter().map(|(n, _)| n.clone()).collect();

            let windows = root.join("dist/sample-app-1.2.3-windows-x64");
            build.stage_windows(&windows).unwrap();
            let found = files_under(&windows).unwrap();
            // The desktop program at the top, under the release id: that is where the updater looks.
            for file in [
                "sample-app.exe",
                "sample-app-cli.exe",
                "LICENSE-MIT",
                "THIRD-PARTY-LICENSES.txt",
                "licenses/assets/fonts/serif/OFL.txt",
                "register-file-types.cmd",
            ] {
                assert!(found.iter().any(|f| f == file), "{file} is not in {found:?}");
            }
            check_stage(&windows, &expected_files(Platform::WindowsX64, &APP, &PACKAGE, &names)).unwrap();

            let macos = root.join("dist/sample-app-1.2.3-macos-aarch64");
            let bundle = build.stage_macos(&macos).unwrap();
            assert_eq!(bundle, macos.join("Sample App.app"));
            let found = files_under(&macos).unwrap();
            for file in [
                "Sample App.app/Contents/MacOS/sample-app",
                "Sample App.app/Contents/MacOS/sample-app-cli",
                "Sample App.app/Contents/Info.plist",
                "Sample App.app/Contents/Resources/org.example.sample.icns",
                "Sample App.app/Contents/Resources/THIRD-PARTY-LICENSES.txt",
                "Sample App.app/Contents/Resources/licenses/assets/fonts/serif/OFL.txt",
                "Legal/THIRD-PARTY-LICENSES.txt",
            ] {
                assert!(found.iter().any(|f| f == file), "{file} is not in {found:?}");
            }
            check_stage(&macos, &expected_files(Platform::MacosArm64, &APP, &PACKAGE, &names)).unwrap();
            // The icon holds the one size a macOS icon has a place for.
            let icon = std::fs::read(bundle.join("Contents/Resources/org.example.sample.icns")).unwrap();
            assert_eq!((&icon[..4], &icon[8..12]), (&b"icns"[..], &b"ic08"[..]));
            assert_eq!(u32::from_be_bytes([icon[4], icon[5], icon[6], icon[7]]) as usize, icon.len());
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn a_desktop_is_told_the_apps_name_id_and_file_types() {
            let entry = desktop_entry("Sample App", &PACKAGE, "sample-app");
            assert_eq!(
                entry,
                "[Desktop Entry]\nType=Application\nName=Sample App\nComment=Write samples\nExec=sample-app %f\nIcon=org.example.sample\n\
                 StartupWMClass=org.example.sample\nTerminal=false\nCategories=Office;WordProcessor;\n\
                 MimeType=application/x-sample;application/x-sample-project;text/x-notes;\n"
            );
            assert!(!desktop_entry("Sample App", &Package { file_types: &[], ..PACKAGE }, "sample-app").contains("MimeType"));
            let types = mime_info(&PACKAGE).unwrap();
            assert!(
                types.contains("<mime-type type=\"application/x-sample\">\n    <comment>Sample document</comment>\n    <glob pattern=\"*.smp\"/>")
            );
            assert!(types.contains("<sub-class-of type=\"inode/directory\"/>\n    <glob pattern=\"*.smpd\"/>"));
            // Another program's format is opened, not described.
            assert!(!types.contains("text/x-notes"));
            assert_eq!(mime_info(&Package { file_types: &[], ..PACKAGE }), None);

            let plist = info_plist("Sample App", &PACKAGE, "sample-app", "1.2.3");
            for part in [
                "<key>CFBundleIdentifier</key><string>org.example.sample</string>",
                "<key>CFBundleExecutable</key><string>sample-app</string>",
                "<key>CFBundleShortVersionString</key><string>1.2.3</string>",
                "<key>CFBundleIconFile</key><string>org.example.sample</string>",
                "<key>LSItemContentTypes</key><array><string>org.example.sample.smp</string></array>",
                "<key>LSTypeIsPackage</key><true/>",
                "<key>UTTypeConformsTo</key><array><string>com.apple.package</string><string>public.composite-content</string></array>",
                "<key>CFBundleTypeExtensions</key><array><string>notes</string><string>nts</string></array>",
                "<key>LSHandlerRank</key><string>Alternate</string>",
            ] {
                assert!(plist.contains(part), "{part} is not in\n{plist}");
            }
            assert_eq!(plist.matches("<key>UTTypeIdentifier</key>").count(), 2);
            assert_eq!(plist.matches("<dict>").count(), plist.matches("</dict>").count());

            let register = register_script("Sample App", &PACKAGE, "sample-app").unwrap();
            assert!(register.contains("set \"APP=%~dp0sample-app.exe\"\r\n"));
            assert!(register.contains("reg add \"HKCU\\Software\\Classes\\.smp\" /ve /d \"org.example.sample.smp\" /f >nul\r\n"));
            assert!(register.contains("\\shell\\open\\command\" /ve /d \"\\\"%APP%\\\" \\\"%%1\\\"\" /f >nul\r\n"));
            assert!(register.contains("\\Directory\\shell\\org.example.sample.smpd\" /v \"AppliesTo\" /d \"System.FileName:\\\"*.smpd\\\"\""));
            assert!(register.contains("\\.nts\\OpenWithProgids\" /v \"org.example.sample.nts\" /t REG_NONE /f >nul"));
            // A format of another program's is never taken over.
            assert!(!register.contains("Classes\\.notes\" /ve"));
            let unregister = unregister_script("Sample App", &PACKAGE).unwrap();
            assert_eq!(unregister.matches("reg delete").count(), 2 + 1 + 4);
            assert!(register.lines().count() > 10 && !register.replace("\r\n", "").contains('\n'));
            assert_eq!(register_script("Sample App", &Package { file_types: &[], ..PACKAGE }, "sample-app"), None);
            let install = install_script("Sample App", &PACKAGE);
            assert!(
                install.starts_with("#!/bin/sh\n")
                    && install.contains("prefix=${PREFIX:-$HOME/.local}\n")
                    && install.contains("$prefix/share/sample-app.")
            );
        }

        #[test]
        fn an_icon_is_found_in_a_program_image_by_image() {
            // Two images, of 3 and 2 bytes, after a header of 6 bytes and two entries of 16.
            let mut ico = vec![0, 0, 1, 0, 2, 0];
            for (size, offset) in [(3u32, 38u32), (2, 41)] {
                ico.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0]);
                ico.extend_from_slice(&size.to_le_bytes());
                ico.extend_from_slice(&offset.to_le_bytes());
            }
            ico.extend_from_slice(&[1, 2, 3, 9, 8]);
            assert_eq!(icon_embedded(&ico, &[0, 9, 8, 0, 1, 2, 3, 0]), Ok(2));
            assert_eq!(icon_embedded(&ico, &[0, 1, 2, 3, 0]).unwrap_err(), "image 2 of 2 is not in the program");
            assert!(icon_embedded(b"not an icon", &[]).is_err());
            assert!(icon_embedded(&ico[..30], &[]).is_err());
        }

        #[test]
        fn reads_what_package_is_asked_for() {
            let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            assert_eq!(asked(&args(&[])), Ok(Asked::Build { platform: None, out: None, build: true }));
            assert_eq!(
                asked(&args(&["linux-x86_64", "--out", "out", "--no-build"])),
                Ok(Asked::Build { platform: Some(Platform::LinuxX64), out: Some("out".to_owned()), build: false })
            );
            assert_eq!(asked(&args(&["--sums", "dist", "--complete"])), Ok(Asked::Sums { folder: "dist".to_owned(), complete: true }));
            assert_eq!(asked(&args(&["--describe"])), Ok(Asked::Describe));
            for wrong in [
                &["linux-arm64"][..],
                &["--sums"],
                &["--describe", "--sums", "dist"],
                &["--complete"],
                &["--publish"],
                &["windows-x64", "macos-aarch64"],
            ] {
                assert!(asked(&args(wrong)).is_err(), "{wrong:?}");
            }
        }
    }
}
