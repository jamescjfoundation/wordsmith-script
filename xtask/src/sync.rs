// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask sync`: keep an app repository's shared settings and rules identical to the core's.
//!
//! Every app carries the same toolchain, lint, cargo-deny and license settings, the same shared
//! xtask modules, the same workflow files (each calls a shared workflow kept in the core), the
//! same Dependabot settings, an exact copy of the core's `[patch]` block, and the core's rules
//! and settings as marked blocks in files that are otherwise the app's own. This module copies
//! them from a core checkout into an app (`sync`) or reports any difference (`--check`, run by
//! the app's CI).
//!
//! A marked block is the text from its BEGIN line to its END line: in a Markdown file
//! `<!-- BEGIN open-source-core: NAME -->` and `<!-- END open-source-core: NAME -->`, in any
//! other file `# BEGIN open-source-core: NAME` and `# END open-source-core: NAME`. Its source is
//! the same block in the core's `template/` file. `sync` replaces a block where it finds the two
//! lines and never guesses a place for one: the text around a block is the app's own.
//!
//! One managed file has a part that is the app's: the git addresses between the two
//! `app sources` lines of `deny.toml`. In an app they are written from the providers its
//! `xtask/src/main.rs` declares; everything else in that file is the core's.
//!
//! In an app, the core checkout is the one Cargo already fetched for the pinned core release
//! (found through `cargo metadata`), so `cargo xtask sync --check` compares against exactly the
//! release the app builds with. From the core, `cargo xtask sync <app dir>` writes into an app.
//!
//! One run is enough, even when the release changed this module or its lists: after writing, a
//! run that changed any shared xtask source builds the new xtask in a folder of its own and runs
//! it once more. The folder is what makes that work on Windows, where the program still running
//! cannot be replaced.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::{Path, PathBuf};

use super::json;
use super::link::{BLOCK_BEGIN, BLOCK_END, extract_block, links, replace_block};
use super::util::{TaskResult, cargo_bin, cargo_output, report};

/// (path in the core, path in an app). Apps must not edit these files; change them in the core.
pub const MANAGED: &[(&str, &str)] = &[
    ("rust-toolchain.toml", "rust-toolchain.toml"),
    ("rustfmt.toml", "rustfmt.toml"),
    ("clippy.toml", "clippy.toml"),
    ("deny.toml", "deny.toml"),
    ("template/xtask/src/util.rs", "xtask/src/util.rs"),
    ("template/xtask/src/lintroots.rs", "xtask/src/lintroots.rs"),
    ("template/xtask/src/privacy.rs", "xtask/src/privacy.rs"),
    ("template/xtask/src/link.rs", "xtask/src/link.rs"),
    ("template/xtask/src/sync.rs", "xtask/src/sync.rs"),
    ("template/xtask/src/spdx.rs", "xtask/src/spdx.rs"),
    ("template/xtask/src/json.rs", "xtask/src/json.rs"),
    ("template/xtask/src/licenses.rs", "xtask/src/licenses.rs"),
    ("template/xtask/src/changed.rs", "xtask/src/changed.rs"),
    ("template/xtask/src/corepin.rs", "xtask/src/corepin.rs"),
    ("template/xtask/src/docs.rs", "xtask/src/docs.rs"),
    ("template/xtask/src/applayers.rs", "xtask/src/applayers.rs"),
    ("template/xtask/src/assets.rs", "xtask/src/assets.rs"),
    ("template/xtask/src/gate.rs", "xtask/src/gate.rs"),
    ("template/xtask/src/shared.rs", "xtask/src/shared.rs"),
    ("template/xtask/Cargo.toml", "xtask/Cargo.toml"),
    ("template/.github/workflows/ci.yml", ".github/workflows/ci.yml"),
    ("template/.github/workflows/core-pin.yml", ".github/workflows/core-pin.yml"),
    ("template/.github/workflows/release.yml", ".github/workflows/release.yml"),
    ("template/.github/pull_request_template.md", ".github/pull_request_template.md"),
    ("template/.github/dependabot.yml", ".github/dependabot.yml"),
    ("template/hooks/pre-commit", "hooks/pre-commit"),
    ("template/hooks/pre-push", "hooks/pre-push"),
    ("template/CODE_OF_CONDUCT.md", "CODE_OF_CONDUCT.md"),
];

/// (file in the core, file in an app, block name): the marked blocks an app carries. Apps change
/// the text around a block, never the block; change it in the core's file.
pub const BLOCKS: &[(&str, &str, &str)] = &[
    ("template/AGENTS.md", "AGENTS.md", "app"),
    ("template/AGENTS.md", "AGENTS.md", "public"),
    ("template/AGENTS.md", "AGENTS.md", "app-and-core"),
    ("template/SECURITY.md", "SECURITY.md", "security"),
    ("template/CONTRIBUTING.md", "CONTRIBUTING.md", "contributing"),
    ("template/Cargo.toml", "Cargo.toml", "lints-and-profiles"),
    ("template/.cargo/config.toml", ".cargo/config.toml", "cargo"),
    ("template/.gitignore", ".gitignore", "ignore"),
    ("template/.gitattributes", ".gitattributes", "attributes"),
];

/// The managed file with a part that is the app's, and the two lines that part sits between.
pub const DENY: &str = "deny.toml";
pub const SOURCES_BEGIN: &str = "# BEGIN app sources";
pub const SOURCES_END: &str = "# END app sources";

/// Set for the second run of [`app_task`], so that it does not start a third.
const AGAIN: &str = "JCJ_XTASK_SYNC_AGAIN";

/// The first and last line of a marked block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marks {
    pub begin: String,
    pub end: String,
}

/// The marker lines of the block `name` in `file`: HTML comments in Markdown, `#` comments in
/// everything else an app carries a block in (TOML, `.gitignore`, `.gitattributes`).
pub fn marks(file: &str, name: &str) -> Marks {
    if file.ends_with(".md") {
        Marks { begin: format!("<!-- BEGIN open-source-core: {name} -->"), end: format!("<!-- END open-source-core: {name} -->") }
    } else {
        Marks { begin: format!("# BEGIN open-source-core: {name}"), end: format!("# END open-source-core: {name}") }
    }
}

/// Where a block is in `text`: from the start of its BEGIN line to the end of its END line,
/// without that line's break. A marker counts only as a whole line, so a sentence that quotes
/// one is not taken for it.
fn block_span(text: &str, marks: &Marks) -> Option<(usize, usize)> {
    let mut start = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim();
        match start {
            None if content == marks.begin => start = Some(at),
            Some(s) if content == marks.end => return Some((s, at + line.trim_end_matches(['\n', '\r']).len())),
            _ => {}
        }
        at += line.len();
    }
    None
}

/// A marked block of `text`, its two marker lines included.
pub fn marked_block<'a>(text: &'a str, marks: &Marks) -> Option<&'a str> {
    block_span(text, marks).and_then(|(s, e)| text.get(s..e))
}

/// `text` with a marked block replaced by `block`; `None` when `text` has no such block.
pub fn replace_marked_block(text: &str, marks: &Marks, block: &str) -> Option<String> {
    let (s, e) = block_span(text, marks)?;
    Some(format!("{}{}{}", text.get(..s)?, block, text.get(e..)?))
}

/// The core's copy of the block `name` in its file `from`.
fn core_block(core: &Path, from: &str, name: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(core.join(from)).map_err(|e| missing_in_core(core, from, &e))?;
    marked_block(&text, &marks(from, name)).map(str::to_owned).ok_or_else(|| {
        format!(
            "the core at {} has no `{name}` block in its {from}: it is an older release than the one this app's shared files are from. {OLDER_CORE}",
            core.display()
        )
    })
}

/// Where the app's own sources are in a `deny.toml`: from the end of the BEGIN line to the start
/// of the END line. `None` when the file has no such part.
fn sources_span(text: &str) -> Option<(usize, usize)> {
    let mut start = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        match start {
            None if line.trim().starts_with(SOURCES_BEGIN) => start = Some(at + line.len()),
            Some(s) if line.trim().starts_with(SOURCES_END) => return Some((s, at)),
            _ => {}
        }
        at += line.len();
    }
    None
}

/// The git addresses an app's `deny.toml` allows beyond the core's.
pub fn app_sources(deny: &str) -> Vec<String> {
    let Some((s, e)) = sources_span(deny) else { return Vec::new() };
    let lines = deny.get(s..e).unwrap_or_default().lines();
    lines.filter_map(|l| l.trim().strip_prefix('"')?.split('"').next().map(str::to_owned)).collect()
}

/// The core's `deny.toml` with `sources` as the app's own; unchanged when it has no place for them.
pub fn with_sources(core_deny: &str, sources: &[String]) -> String {
    let Some((s, e)) = sources_span(core_deny) else { return core_deny.to_owned() };
    let indent: String = core_deny.get(e..).unwrap_or_default().chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let lines: String = sources.iter().map(|address| format!("{indent}\"{address}\",\n")).collect();
    format!("{}{}{}", core_deny.get(..s).unwrap_or_default(), lines, core_deny.get(e..).unwrap_or_default())
}

/// Why a file this module manages cannot be read from the core checkout: when the file is not
/// there, this program is newer than the core it was pointed at.
fn missing_in_core(core: &Path, from: &str, error: &std::io::Error) -> String {
    if error.kind() != std::io::ErrorKind::NotFound {
        return format!("core file {from}: {error}");
    }
    format!("the core at {} has no {from}: it is an older release than the one this app's shared files are from. {OLDER_CORE}", core.display())
}

/// What to do when the core `sync` reads is older than the app's shared files.
const OLDER_CORE: &str = "Nothing was written. Move the pin to that release, or link a checkout of it (`cargo xtask dev-link`)";

/// What a managed file must hold in an app: the core's bytes, and for `deny.toml` the core's
/// text with the app's own sources. `sources` are the addresses the app declares; `None` (the
/// core writing into an app, whose declaration it cannot read) keeps the ones the app's file has.
fn wanted(core: &Path, app: &Path, from: &str, to: &str, sources: Option<&[String]>) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(core.join(from)).map_err(|e| missing_in_core(core, from, &e))?;
    if to != DENY {
        return Ok(bytes);
    }
    let Ok(text) = String::from_utf8(bytes.clone()) else { return Ok(bytes) };
    let kept;
    let sources = match sources {
        Some(declared) => declared,
        None => {
            kept = app_sources(&std::fs::read_to_string(app.join(to)).unwrap_or_default());
            &kept
        }
    };
    Ok(with_sources(&text, sources).into_bytes())
}

/// What differs between the core's managed files and an app's. `sources`: see [`wanted`].
pub fn differences(core: &Path, app: &Path, sources: Option<&[String]>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (from, to) in MANAGED {
        let want = wanted(core, app, from, to, sources)?;
        match std::fs::read(app.join(to)) {
            Ok(have) if have == want => {}
            Ok(_) if *to == DENY => {
                out.push(format!("{to} differs from the core's {from} with this app's own sources (`cargo xtask sync` writes it)"))
            }
            Ok(_) => out.push(format!("{to} differs from the core's {from}")),
            Err(_) => out.push(format!("{to} is missing (the core's {from})")),
        }
    }
    let mut missing_files: Vec<&str> = Vec::new();
    for (from, to, name) in BLOCKS {
        let want = core_block(core, from, name)?;
        let Ok(have) = std::fs::read_to_string(app.join(to)) else {
            if !missing_files.contains(to) {
                missing_files.push(to);
                out.push(format!("{to} is missing (the core's {from})"));
            }
            continue;
        };
        match marked_block(&have, &marks(to, name)) {
            Some(b) if b == want => {}
            Some(_) => out.push(format!("{to}: the `{name}` block differs from the core's {from}")),
            None => out.push(format!("{to}: the `{name}` block is missing (the core's {from} shows where it goes)")),
        }
    }
    let core_manifest = std::fs::read_to_string(core.join("Cargo.toml")).map_err(|e| format!("core Cargo.toml: {e}"))?;
    let block = extract_block(&core_manifest, BLOCK_BEGIN, BLOCK_END).ok_or("the core's Cargo.toml has no patch block")?;
    let app_manifest = std::fs::read_to_string(app.join("Cargo.toml")).map_err(|e| format!("app Cargo.toml: {e}"))?;
    match extract_block(&app_manifest, BLOCK_BEGIN, BLOCK_END) {
        Some(b) if b == block => {}
        Some(_) => out.push("Cargo.toml: the patch block differs from the core's".to_owned()),
        None => out.push("Cargo.toml: the core's patch block is missing".to_owned()),
    }
    Ok(out)
}

/// Write every managed file, marked block and the patch block from `core` into `app`.
///
/// A block goes where its two marker lines are. A file the app does not have yet is the core's
/// whole file. A file the app has without the marker lines is left as it is and named in the
/// error, after everything else is written: only a person or an agent reading the file knows
/// where the block belongs and which hand copy it replaces. `sources`: see [`wanted`].
pub fn write(core: &Path, app: &Path, sources: Option<&[String]>) -> TaskResult {
    // Everything is read from the core before anything is written. A core that lacks a file or
    // a block this program manages is an older release than the app's shared files are from, and
    // writing half of it would leave an xtask that does not build.
    let mut files = Vec::new();
    for (from, to) in MANAGED {
        files.push((app.join(to), wanted(core, app, from, to, sources)?));
    }
    let mut blocks = Vec::new();
    for (from, to, name) in BLOCKS {
        blocks.push((*from, *to, *name, core_block(core, from, name)?));
    }
    let core_manifest = std::fs::read_to_string(core.join("Cargo.toml")).map_err(|e| format!("core Cargo.toml: {e}"))?;
    let block = extract_block(&core_manifest, BLOCK_BEGIN, BLOCK_END).ok_or("the core's Cargo.toml has no patch block")?;

    for (dest, bytes) in files {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        // A file that is already right is left alone, so a run that changes nothing touches nothing.
        if std::fs::read(&dest).ok().as_ref() != Some(&bytes) {
            std::fs::write(&dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
    }
    let path = app.join("Cargo.toml");
    let app_manifest = std::fs::read_to_string(&path).map_err(|e| format!("app Cargo.toml: {e}"))?;
    let new_manifest = replace_block(&app_manifest, &block, BLOCK_BEGIN, BLOCK_END);
    if new_manifest != app_manifest {
        std::fs::write(&path, new_manifest).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut unplaced = Vec::new();
    for (from, to, name, block) in blocks {
        let dest = app.join(to);
        let Ok(have) = std::fs::read_to_string(&dest) else {
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            std::fs::copy(core.join(from), &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            continue;
        };
        match replace_marked_block(&have, &marks(to, name), &block) {
            Some(new) if new == have => {}
            Some(new) => std::fs::write(&dest, new).map_err(|e| format!("{}: {e}", dest.display()))?,
            None => unplaced.push((to, name)),
        }
    }
    println!(
        "synced {} managed files, {} of {} marked blocks and the patch block into {}",
        MANAGED.len(),
        BLOCKS.len() - unplaced.len(),
        BLOCKS.len(),
        app.display()
    );
    if unplaced.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = unplaced
        .iter()
        .map(|(to, name)| format!("{to}: no `{name}` block (its lines: `{}` and `{}`)", marks(to, name).begin, marks(to, name).end))
        .collect();
    Err(format!(
        "{} marked block(s) have no place yet:\n  - {}\nEverything else is written. For each block, put its two marker lines where \
         the block belongs, each a line of its own, remove the hand copy of the same text or settings, and run sync again. The core's \
         files under template/ show the place.",
        unplaced.len(),
        list.join("\n  - "),
    ))
}

/// The core checkout an app builds with: the folder Cargo fetched the pinned `jcj-` crates from.
pub fn locate_core(app: &Path) -> Result<PathBuf, String> {
    let out = cargo_output(app, &["metadata", "--format-version", "1", "--locked"])?;
    choose_core(&json::parse(&out)?)
}

/// The core checkout in `cargo metadata` output. The graph may hold more than one core release
/// when an app depends on another app's crates that pin an older core; the app's own pin is the
/// release its workspace members depend on directly (the newest, should they name several).
pub fn choose_core(meta: &json::Value) -> Result<PathBuf, String> {
    let members: Vec<&str> =
        meta.get("workspace_members").map(json::Value::items).unwrap_or_default().iter().filter_map(json::Value::as_str).collect();
    let mut direct: Vec<&str> = Vec::new();
    for node in meta.get("resolve").and_then(|r| r.get("nodes")).map(json::Value::items).unwrap_or_default() {
        if node.str_of("id").is_some_and(|id| members.contains(&id)) {
            direct.extend(node.get("deps").map(json::Value::items).unwrap_or_default().iter().filter_map(|d| d.str_of("pkg")));
        }
    }
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for pass_direct_only in [true, false] {
        for p in meta.get("packages").map(json::Value::items).unwrap_or_default() {
            let Some(name) = p.str_of("name") else { continue };
            if !name.starts_with("jcj-") || name == "jcj-upstream" {
                continue;
            }
            if pass_direct_only && !p.str_of("id").is_some_and(|id| direct.contains(&id)) {
                continue;
            }
            let manifest = PathBuf::from(p.str_of("manifest_path").ok_or_else(|| format!("{name} has no manifest_path"))?);
            // <core>/crates/<name>/Cargo.toml
            let Some(core) = manifest.ancestors().nth(3).map(Path::to_path_buf) else { continue };
            let version: Vec<u64> = p.str_of("version").unwrap_or("0").split(['.', '-', '+']).map(|n| n.parse().unwrap_or(0)).collect();
            if best.as_ref().is_none_or(|(v, _)| version > *v) {
                best = Some((version, core));
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, core)| core).ok_or_else(|| "this workspace does not depend on open-source-core (no jcj- package)".to_owned())
}

/// What to tell an app whose `xtask/src/main.rs` still holds its own copy of the gate. The shared
/// modules are written either way, but a check that `main.rs` does not run checks nothing, and
/// `main.rs` is the app's own, so `sync` can only say so.
pub fn not_on_the_shared_gate(main: &str) -> Option<String> {
    if main.contains("gate::main") {
        return None;
    }
    Some(
        "xtask/src/main.rs holds its own copy of the gate. The shared one is xtask/src/gate.rs: main.rs keeps the app's data and its \
         own tasks and calls `gate::main`. The core's CHANGELOG has the steps, and the core's template/xtask/src/main.rs shows the file"
            .to_owned(),
    )
}

/// The bytes of every managed file under `xtask/` in an app, in the order of [`MANAGED`].
fn xtask_sources(app: &Path) -> Vec<Option<Vec<u8>>> {
    MANAGED.iter().filter(|(_, to)| to.starts_with("xtask/")).map(|(_, to)| std::fs::read(app.join(to)).ok()).collect()
}

/// Whether a run must be followed by a second one: it changed the sources this program was built
/// from, so the lists and the rules it ran with may be older than the ones now on disk. Never
/// from a second run.
pub fn run_again(before: &[Option<Vec<u8>>], after: &[Option<Vec<u8>>], is_second_run: bool) -> bool {
    !is_second_run && before != after
}

/// Build the xtask from the sources now on disk and run `sync` with it. It is built in a folder
/// of its own, because the program running now cannot be replaced while it runs (on Windows a
/// file in use is locked).
fn sync_again(app: &Path) -> TaskResult {
    println!("\nthe shared xtask modules changed: running sync once more, built from the new ones");
    let target = app.join("target").join("xtask-sync");
    let status = std::process::Command::new(cargo_bin())
        .args(["run", "--quiet", "--package", "xtask", "--target-dir"])
        .arg(&target)
        .args(["--", "sync"])
        .current_dir(app)
        .env(AGAIN, "1")
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() { Ok(()) } else { Err("sync, run once more with the new modules, did not finish: see above".to_owned()) }
}

/// `cargo xtask sync [--check]` in an app: against the pinned core release. `sources` are the git
/// addresses the app's xtask declares for `deny.toml`; `None` keeps the ones the file has.
pub fn app_sync(app: &Path, check: bool, sources: Option<&[String]>) -> TaskResult {
    let core = locate_core(app)?;
    if let Some(note) = not_on_the_shared_gate(&std::fs::read_to_string(app.join("xtask/src/main.rs")).unwrap_or_default()) {
        println!("note: {note}");
    }
    if check {
        // Linked, the core Cargo builds with is the linked checkout, and that is what is compared.
        let against = if links(app).is_empty() { "the pinned core release" } else { "the linked core checkout" };
        return report(differences(&core, app, sources)?, &format!("shared files, marked blocks and the patch block match {against}"));
    }
    let before = xtask_sources(app);
    let written = write(&core, app, sources);
    if run_again(&before, &xtask_sources(app), std::env::var_os(AGAIN).is_some()) {
        // This program was built from the app's shared modules as they were. The second run is
        // built from the new ones and reports for both.
        return sync_again(app);
    }
    written
}

/// `cargo xtask sync [--check]` as an app's `main.rs` from before the shared gate calls it.
pub fn app_task(app: &Path, check: bool) -> TaskResult {
    app_sync(app, check, None)
}

/// `cargo xtask sync <app dir> [--check]` in the core.
pub fn core_task(core: &Path, app: &str, check: bool) -> TaskResult {
    let app = PathBuf::from(app);
    if let Some(note) = not_on_the_shared_gate(&std::fs::read_to_string(app.join("xtask/src/main.rs")).unwrap_or_default()) {
        println!("note: {note}");
    }
    if check {
        report(differences(core, &app, None)?, "the app's shared files, marked blocks and patch block match this core")
    } else {
        write(core, &app, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(packages: &[(&str, &str, &str, &str)], direct: &[&str]) -> json::Value {
        let pkgs: Vec<String> = packages
            .iter()
            .map(|(id, name, version, path)| format!(r#"{{"id":"{id}","name":"{name}","version":"{version}","manifest_path":"{path}"}}"#))
            .collect();
        let deps: Vec<String> = direct.iter().map(|d| format!(r#"{{"pkg":"{d}"}}"#)).collect();
        let text = format!(
            r#"{{"packages":[{}],"workspace_members":["app"],"resolve":{{"nodes":[{{"id":"app","deps":[{}]}},{{"id":"other","deps":[]}}]}}}}"#,
            pkgs.join(","),
            deps.join(",")
        );
        json::parse(&text).unwrap()
    }

    #[test]
    fn the_apps_own_core_wins_over_an_older_one_in_the_graph() {
        let packages = [
            ("old-doc", "jcj-doc", "0.1.4", "/c/old/crates/jcj-doc/Cargo.toml"),
            ("new-doc", "jcj-doc", "0.1.5", "/c/new/crates/jcj-doc/Cargo.toml"),
            ("new-shell", "jcj-shell", "0.1.5", "/c/new/crates/jcj-shell/Cargo.toml"),
        ];
        assert_eq!(choose_core(&meta(&packages, &["new-shell"])).unwrap(), PathBuf::from("/c/new"));
        // Only the older core is a direct dependency: it is the app's pin.
        assert_eq!(choose_core(&meta(&packages, &["old-doc"])).unwrap(), PathBuf::from("/c/old"));
        // No direct jcj- dependency: the newest in the graph.
        assert_eq!(choose_core(&meta(&packages, &[])).unwrap(), PathBuf::from("/c/new"));
        assert!(choose_core(&meta(&[], &[])).is_err());
    }

    #[test]
    fn a_marked_block_is_found_by_whole_lines() {
        let m = marks("AGENTS.md", "a");
        assert_eq!(m.begin, "<!-- BEGIN open-source-core: a -->");
        let text = format!("# Title\n\nThe line `{}` is quoted here.\n\n{}\nrule\n{}\n\nown text\n", m.begin, m.begin, m.end);
        let block = marked_block(&text, &m).unwrap();
        assert_eq!(block, format!("{}\nrule\n{}", m.begin, m.end));
        assert!(marked_block(&text, &marks("AGENTS.md", "b")).is_none());
        // An END line with no BEGIN line before it, or a BEGIN line with no END line: no block.
        assert!(marked_block(&format!("{}\nrule\n{}\n", m.end, m.begin), &m).is_none());
        assert!(marked_block(&format!("{}\nrule\n", m.begin), &m).is_none());
        // Windows line ends: the block stops before the END line's break.
        let crlf = format!("{}\r\nrule\r\n{}\r\nown\r\n", m.begin, m.end);
        assert!(marked_block(&crlf, &m).unwrap().ends_with("-->"));
    }

    #[test]
    fn a_block_outside_markdown_is_marked_with_comment_lines() {
        for file in ["Cargo.toml", ".cargo/config.toml", ".gitignore", ".gitattributes"] {
            let m = marks(file, "x");
            assert_eq!((m.begin.as_str(), m.end.as_str()), ("# BEGIN open-source-core: x", "# END open-source-core: x"), "{file}");
        }
        let m = marks(".gitignore", "ignore");
        let own = format!("*.swp\n\n{}\n/target\n{}\n\n/notes\n", m.begin, m.end);
        let block = format!("{}\n/target\n/dist\n{}", m.begin, m.end);
        assert_eq!(replace_marked_block(&own, &m, &block).unwrap(), format!("*.swp\n\n{block}\n\n/notes\n"));
        // A Markdown marker in such a file is not this block's.
        assert!(marked_block(&format!("{}\nx\n{}\n", marks("a.md", "ignore").begin, marks("a.md", "ignore").end), &m).is_none());
    }

    #[test]
    fn replacing_a_block_leaves_the_text_around_it() {
        let m = marks("AGENTS.md", "a");
        let old = format!("intro\n\n{}\nold rule\n{}\n\nown rules\n", m.begin, m.end);
        let block = format!("{}\nnew rule\n{}", m.begin, m.end);
        assert_eq!(replace_marked_block(&old, &m, &block).unwrap(), format!("intro\n\n{block}\n\nown rules\n"));
        assert!(replace_marked_block("intro\n\nown rules\n", &m, &block).is_none());
    }

    const CORE_DENY: &str = "[sources]\nallow-git = [\n    \"https://example.invalid/core\",\n    # BEGIN app sources (written by sync)\n    # END app sources\n]\nrequired-git-spec = \"tag\"\n";

    #[test]
    fn an_apps_own_sources_sit_between_the_two_lines_of_deny_toml() {
        assert!(app_sources(CORE_DENY).is_empty());
        let one = with_sources(CORE_DENY, &["https://example.invalid/other-app".to_owned()]);
        assert_eq!(
            one,
            "[sources]\nallow-git = [\n    \"https://example.invalid/core\",\n    # BEGIN app sources (written by sync)\n    \
             \"https://example.invalid/other-app\",\n    # END app sources\n]\nrequired-git-spec = \"tag\"\n"
        );
        assert_eq!(app_sources(&one), ["https://example.invalid/other-app"]);
        // Writing them again changes nothing, and writing none takes them out.
        assert_eq!(with_sources(&one, &app_sources(&one)), one);
        assert_eq!(with_sources(&one, &[]), CORE_DENY);
        // A file with no place for them is the core's as it is.
        assert_eq!(with_sources("[sources]\n", &["x".to_owned()]), "[sources]\n");
    }

    /// A core and an app folder with the files `write` and `differences` read.
    fn folders(name: &str) -> (PathBuf, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("jcj-sync-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let (core, app) = (tmp.join("core"), tmp.join("app"));
        for (from, to) in MANAGED {
            put(&core, from, &if *to == DENY { CORE_DENY.to_owned() } else { format!("managed {from}\n") });
        }
        let mut files: Vec<&str> = BLOCKS.iter().map(|(from, _, _)| *from).collect();
        files.dedup();
        for from in files {
            let mut text = String::from("# The core's template\n");
            for (_, _, block) in BLOCKS.iter().filter(|(f, _, _)| *f == from) {
                let m = marks(from, block);
                text.push_str(&format!("\n{}\nthe core's {block} rules\n{}\n", m.begin, m.end));
            }
            put(&core, from, &text);
        }
        let patch = format!("{BLOCK_BEGIN}\n[patch.x]\n{BLOCK_END}\n");
        std::fs::write(core.join("Cargo.toml"), patch).unwrap();
        put(&app, "Cargo.toml", "[workspace]\n");
        (core, app)
    }

    fn put(root: &Path, file: &str, text: &str) {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// The two marker lines of every block of `file`, with nothing between them.
    fn empty_blocks(file: &str) -> String {
        let names = BLOCKS.iter().filter(|(_, to, _)| *to == file).map(|(_, to, name)| marks(to, name));
        names.map(|m| format!("{}\n{}\n", m.begin, m.end)).collect()
    }

    #[test]
    fn writes_blocks_where_the_markers_are_and_never_guesses() {
        let (core, app) = folders("write");
        // AGENTS.md has two of its three blocks marked, around the app's own text; SECURITY.md is a
        // hand copy with no markers; CONTRIBUTING.md does not exist. The files that are not
        // Markdown have their marker lines, but for Cargo.toml.
        let (public, both) = (marks("AGENTS.md", "public"), marks("AGENTS.md", "app-and-core"));
        let agents = format!(
            "# The app\n\n{}\nan old hand copy\n{}\n\nown text between\n\n{}\n{}\n\n## The app's own rules\n",
            public.begin, public.end, both.begin, both.end
        );
        put(&app, "AGENTS.md", &agents);
        put(&app, "SECURITY.md", "# Security\n\na hand copy\n");
        put(&app, ".gitignore", &format!("*.swp\n{}", empty_blocks(".gitignore")));
        put(&app, ".cargo/config.toml", &empty_blocks(".cargo/config.toml"));

        let err = write(&core, &app, None).unwrap_err();
        assert!(err.contains("AGENTS.md: no `app` block") && err.contains("SECURITY.md: no `security` block"), "{err}");
        assert!(err.contains("Cargo.toml: no `lints-and-profiles` block (its lines: `# BEGIN open-source-core: lints-and-profiles`"), "{err}");
        assert!(!err.contains("public") && !err.contains("CONTRIBUTING.md") && !err.contains(".gitignore") && !err.contains("attributes"), "{err}");
        // Everything else was written all the same.
        let read = |file: &str| std::fs::read_to_string(app.join(file)).unwrap();
        assert_eq!(read("CODE_OF_CONDUCT.md"), "managed template/CODE_OF_CONDUCT.md\n");
        assert_eq!(read("xtask/src/gate.rs"), "managed template/xtask/src/gate.rs\n");
        assert!(read("Cargo.toml").contains("[patch.x]"));
        assert_eq!(read("CONTRIBUTING.md"), std::fs::read_to_string(core.join("template/CONTRIBUTING.md")).unwrap());
        // A file the app did not have is the core's whole file, in a folder that may be new too.
        assert_eq!(read(".gitattributes"), std::fs::read_to_string(core.join("template/.gitattributes")).unwrap());
        assert_eq!(read("SECURITY.md"), "# Security\n\na hand copy\n");
        assert!(read(".gitignore").starts_with("*.swp\n") && read(".gitignore").contains("the core's ignore rules"));
        let new = read("AGENTS.md");
        assert!(new.starts_with("# The app\n\n") && new.ends_with("\n\n## The app's own rules\n") && new.contains("\n\nown text between\n\n"));
        assert!(new.contains("the core's public rules") && new.contains("the core's app-and-core rules") && !new.contains("an old hand copy"));

        let found = differences(&core, &app, None).unwrap();
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found.iter().any(|d| d.starts_with("AGENTS.md: the `app` block is missing")));
        assert!(found.iter().any(|d| d.starts_with("SECURITY.md: the `security` block is missing")));
        assert!(found.iter().any(|d| d.starts_with("Cargo.toml: the `lints-and-profiles` block is missing")));

        // With the marker lines in place the next run fills them, and nothing differs.
        let app_block = marks("AGENTS.md", "app");
        put(&app, "AGENTS.md", &format!("{new}\n{}\n{}\n", app_block.begin, app_block.end));
        put(&app, "SECURITY.md", &format!("# Security\n\n{}", empty_blocks("SECURITY.md")));
        put(
            &app,
            "Cargo.toml",
            &format!("[workspace]\n\n{}\n{}", empty_blocks("Cargo.toml"), read("Cargo.toml").trim_start_matches("[workspace]\n")),
        );
        write(&core, &app, None).unwrap();
        assert!(differences(&core, &app, None).unwrap().is_empty());
        assert!(read("Cargo.toml").contains("the core's lints-and-profiles rules") && read("Cargo.toml").contains("[patch.x]"));

        // A block edited in the app, and a missing file, are both reported.
        put(&app, "SECURITY.md", &read("SECURITY.md").replace("the core's security rules", "our own rules"));
        std::fs::remove_file(app.join("AGENTS.md")).unwrap();
        let found = differences(&core, &app, None).unwrap();
        assert_eq!(
            found,
            [
                "AGENTS.md is missing (the core's template/AGENTS.md)",
                "SECURITY.md: the `security` block differs from the core's template/SECURITY.md"
            ]
        );
        let _ = std::fs::remove_dir_all(core.parent().unwrap());
    }

    #[test]
    fn a_core_older_than_the_apps_shared_files_writes_nothing() {
        let (core, app) = folders("older");
        put(&app, "xtask/src/util.rs", "the app's newer util.rs\n");
        // A core release from before a module was shared, and one from before a block existed.
        std::fs::remove_file(core.join("template/xtask/src/gate.rs")).unwrap();
        let err = write(&core, &app, None).unwrap_err();
        assert!(
            err.contains("has no template/xtask/src/gate.rs") && err.contains("an older release") && err.contains("Nothing was written"),
            "{err}"
        );
        assert_eq!(std::fs::read_to_string(app.join("xtask/src/util.rs")).unwrap(), "the app's newer util.rs\n");
        assert!(!app.join("deny.toml").exists() && !app.join("CODE_OF_CONDUCT.md").exists());
        assert!(differences(&core, &app, None).unwrap_err().contains("an older release"));

        put(&core, "template/xtask/src/gate.rs", "managed\n");
        put(&core, "template/.gitignore", "/target\n");
        let err = write(&core, &app, None).unwrap_err();
        assert!(err.contains("has no `ignore` block in its template/.gitignore") && err.contains("Nothing was written"), "{err}");
        assert_eq!(std::fs::read_to_string(app.join("xtask/src/util.rs")).unwrap(), "the app's newer util.rs\n");
        let _ = std::fs::remove_dir_all(core.parent().unwrap());
    }

    #[test]
    fn deny_toml_is_the_cores_but_for_the_apps_own_sources() {
        let (core, app) = folders("deny");
        for file in ["AGENTS.md", "SECURITY.md", "CONTRIBUTING.md", "Cargo.toml", ".cargo/config.toml", ".gitignore", ".gitattributes"] {
            let rest = if file == "Cargo.toml" { "[workspace]\n" } else { "" };
            put(&app, file, &format!("{rest}{}", empty_blocks(file)));
        }
        let declared = ["https://example.invalid/other-app".to_owned()];
        write(&core, &app, Some(&declared)).unwrap();
        let deny = std::fs::read_to_string(app.join(DENY)).unwrap();
        assert_eq!(app_sources(&deny), declared);
        assert!(differences(&core, &app, Some(&declared)).unwrap().is_empty());
        // The core, writing into the app, cannot read what the app declares: it keeps what is there.
        write(&core, &app, None).unwrap();
        assert_eq!(std::fs::read_to_string(app.join(DENY)).unwrap(), deny);
        assert!(differences(&core, &app, None).unwrap().is_empty());
        // The file allows a source the app does not declare, or the core's part was edited: both differ.
        assert_eq!(differences(&core, &app, Some(&[])).unwrap().len(), 1);
        put(&app, DENY, &deny.replace("required-git-spec = \"tag\"", "required-git-spec = \"branch\""));
        let found = differences(&core, &app, Some(&declared)).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("deny.toml differs from the core's deny.toml with this app's own sources"));
        let _ = std::fs::remove_dir_all(core.parent().unwrap());
    }

    #[test]
    fn a_run_that_changed_its_own_sources_runs_once_more() {
        let old = vec![Some(b"old sync.rs".to_vec()), None];
        let new = vec![Some(b"new sync.rs".to_vec()), Some(b"a new module".to_vec())];
        assert!(!run_again(&old, &old, false));
        assert!(run_again(&old, &new, false));
        // A module the release adds counts, and a second run never starts a third.
        assert!(run_again(&[Some(b"same".to_vec()), None], &[Some(b"same".to_vec()), Some(b"added".to_vec())], false));
        assert!(!run_again(&old, &new, true));
        // A core that lacks a file this program manages is an older release, and is said to be.
        let gone = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert!(missing_in_core(Path::new("/c/old"), "template/xtask/src/gate.rs", &gone).contains("an older release"));
        // The sources it compares are the managed files of the xtask, this module among them.
        let (core, app) = folders("again");
        let before = xtask_sources(&app);
        assert!(before.iter().all(Option::is_none) && before.len() > 10);
        let _ = write(&core, &app, None);
        assert!(run_again(&before, &xtask_sources(&app), false));
        assert!(!run_again(&xtask_sources(&app), &xtask_sources(&app), false));
        let _ = std::fs::remove_dir_all(core.parent().unwrap());
    }

    /// The second run is a real `cargo run` of the workspace's xtask, built in a folder of its
    /// own. A workspace with a one-line xtask stands in for an app here, so this runs on every
    /// platform the gate runs on.
    #[test]
    fn the_second_run_is_built_apart_and_knows_it_is_the_second() {
        let tmp = std::env::temp_dir().join(format!("jcj-sync-test-second-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        put(&tmp, "Cargo.toml", "[workspace]\nresolver = \"3\"\nmembers = [\"xtask\"]\n");
        put(&tmp, "xtask/Cargo.toml", "[package]\nname = \"xtask\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n");
        let main = format!(
            "fn main() {{\n    let asked = std::env::args().nth(1);\n    let ok = asked.as_deref() == Some(\"sync\") && std::env::var_os(\"{AGAIN}\").is_some();\n    \
             std::process::exit(if ok {{ 0 }} else {{ 1 }});\n}}\n"
        );
        put(&tmp, "xtask/src/main.rs", &main);
        assert_eq!(sync_again(&tmp), Ok(()));
        assert!(tmp.join("target/xtask-sync").is_dir());
        // A second run that fails is an error here too.
        put(&tmp, "xtask/src/main.rs", "fn main() {\n    std::process::exit(1);\n}\n");
        assert!(sync_again(&tmp).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn says_when_main_still_holds_its_own_gate() {
        assert!(not_on_the_shared_gate("fn main() -> ExitCode {\n    gate::main(&APP)\n}\n").is_none());
        let note = not_on_the_shared_gate("mod sync;\nfn ci(root: &Path, args: &[String]) -> TaskResult {").unwrap();
        assert!(note.contains("gate::main") && note.contains("CHANGELOG"));
    }
}
