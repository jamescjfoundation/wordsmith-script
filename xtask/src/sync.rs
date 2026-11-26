// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask sync`: keep an app repository's shared settings and rules identical to the core's.
//!
//! Every app carries the same toolchain, lint, cargo-deny and license settings, the same shared
//! xtask modules, the same workflows, an exact copy of the core's `[patch]` block, and the core's
//! rules as marked blocks in its `AGENTS.md`, `SECURITY.md` and `CONTRIBUTING.md`. This module
//! copies them from a core checkout into an app (`sync`) or reports any difference (`--check`, run
//! by the app's CI).
//!
//! A marked block is the text from the line `<!-- BEGIN open-source-core: NAME -->` to the line
//! `<!-- END open-source-core: NAME -->`. Its source is the same block in the core's `template/`
//! file. `sync` replaces a block where it finds the two lines and never guesses a place for one:
//! the text around a block is the app's own.
//!
//! In an app, the core checkout is the one Cargo already fetched for the pinned core release
//! (found through `cargo metadata`), so `cargo xtask sync --check` compares against exactly the
//! release the app builds with. From the core, `cargo xtask sync <app dir>` writes into an app.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::{Path, PathBuf};

use crate::json;
use crate::link::{BLOCK_BEGIN, BLOCK_END, extract_block, replace_block};
use crate::util::{TaskResult, cargo_output, report};

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
    ("template/.github/workflows/ci.yml", ".github/workflows/ci.yml"),
    ("template/.github/workflows/core-pin.yml", ".github/workflows/core-pin.yml"),
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
];

/// The first line of the marked block `name`.
pub fn block_begin(name: &str) -> String {
    format!("<!-- BEGIN open-source-core: {name} -->")
}

/// The last line of the marked block `name`.
pub fn block_end(name: &str) -> String {
    format!("<!-- END open-source-core: {name} -->")
}

/// Where the block `name` is in `text`: from the start of its BEGIN line to the end of its END
/// line, without that line's break. A marker counts only as a whole line, so a sentence that
/// quotes one is not taken for it.
fn block_span(text: &str, name: &str) -> Option<(usize, usize)> {
    let (begin, end) = (block_begin(name), block_end(name));
    let mut start = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim();
        match start {
            None if content == begin => start = Some(at),
            Some(s) if content == end => return Some((s, at + line.trim_end_matches(['\n', '\r']).len())),
            _ => {}
        }
        at += line.len();
    }
    None
}

/// The marked block `name` of `text`, its two marker lines included.
pub fn marked_block<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    block_span(text, name).and_then(|(s, e)| text.get(s..e))
}

/// `text` with its marked block `name` replaced by `block`; `None` when `text` has no such block.
pub fn replace_marked_block(text: &str, name: &str, block: &str) -> Option<String> {
    let (s, e) = block_span(text, name)?;
    Some(format!("{}{}{}", text.get(..s)?, block, text.get(e..)?))
}

/// The core's copy of the block `name` in its file `from`.
fn core_block(core: &Path, from: &str, name: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(core.join(from)).map_err(|e| format!("core file {from}: {e}"))?;
    marked_block(&text, name).map(str::to_owned).ok_or_else(|| format!("the core's {from} has no `{name}` block"))
}

/// What differs between the core's managed files and an app's.
pub fn differences(core: &Path, app: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (from, to) in MANAGED {
        let want = std::fs::read(core.join(from)).map_err(|e| format!("core file {from}: {e}"))?;
        match std::fs::read(app.join(to)) {
            Ok(have) if have == want => {}
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
        match marked_block(&have, name) {
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
/// where the block belongs and which hand copy it replaces.
pub fn write(core: &Path, app: &Path) -> TaskResult {
    for (from, to) in MANAGED {
        let bytes = std::fs::read(core.join(from)).map_err(|e| format!("core file {from}: {e}"))?;
        let dest = app.join(to);
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    let core_manifest = std::fs::read_to_string(core.join("Cargo.toml")).map_err(|e| format!("core Cargo.toml: {e}"))?;
    let block = extract_block(&core_manifest, BLOCK_BEGIN, BLOCK_END).ok_or("the core's Cargo.toml has no patch block")?;
    let path = app.join("Cargo.toml");
    let app_manifest = std::fs::read_to_string(&path).map_err(|e| format!("app Cargo.toml: {e}"))?;
    std::fs::write(&path, replace_block(&app_manifest, &block, BLOCK_BEGIN, BLOCK_END)).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut unplaced = Vec::new();
    for (from, to, name) in BLOCKS {
        let dest = app.join(to);
        let Ok(have) = std::fs::read_to_string(&dest) else {
            std::fs::copy(core.join(from), &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            continue;
        };
        match replace_marked_block(&have, name, &core_block(core, from, name)?) {
            Some(new) if new == have => {}
            Some(new) => std::fs::write(&dest, new).map_err(|e| format!("{}: {e}", dest.display()))?,
            None => unplaced.push(format!("{to}: no `{name}` block")),
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
    Err(format!(
        "{} marked block(s) have no place yet:\n  - {}\nEverything else is written. For each block, put its two marker lines where \
         the block belongs (`{}` and `{}`, each a line of its own, with the block's name), remove the hand copy of the same text, \
         and run sync again. The core's files under template/ show the place.",
        unplaced.len(),
        unplaced.join("\n  - "),
        block_begin("NAME"),
        block_end("NAME"),
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

/// What the template's `main.rs` runs that an app's own `main.rs`, started from an older template,
/// may not run yet: the call to look for, and what it is. A managed module that nothing calls
/// checks nothing, and `main.rs` is the app's own, so `sync` can only say so.
const RUN_FROM_MAIN: &[(&str, &str)] = &[("corepin::task", "the core-pin check")];

/// The entries of [`RUN_FROM_MAIN`] an app's `xtask/src/main.rs` does not call.
pub fn not_run(main: &str) -> Vec<String> {
    let missing = RUN_FROM_MAIN.iter().filter(|(call, _)| !main.contains(call));
    missing.map(|(call, what)| format!("xtask/src/main.rs does not run {what} (`{call}`); the core's CHANGELOG lists the lines to add")).collect()
}

/// `cargo xtask sync [--check]` in an app: against the pinned core release.
pub fn app_task(app: &Path, check: bool) -> TaskResult {
    let core = locate_core(app)?;
    for note in not_run(&std::fs::read_to_string(app.join("xtask/src/main.rs")).unwrap_or_default()) {
        println!("note: {note}");
    }
    if check {
        return report(differences(&core, app)?, "shared files, marked blocks and the patch block match the pinned core release");
    }
    let written = write(&core, app);
    // This program was built from the app's sync.rs as it was, so a file or block the new lists
    // add is not written until the next run.
    if std::fs::read(core.join("template/xtask/src/sync.rs")).is_ok_and(|new| new != include_bytes!("sync.rs")) {
        println!("note: sync.rs itself changed, and its lists of managed files and blocks may have grown: run `cargo xtask sync` again");
    }
    written
}

/// `cargo xtask sync <app dir> [--check]` in the core.
pub fn core_task(core: &Path, app: &str, check: bool) -> TaskResult {
    let app = PathBuf::from(app);
    if check { report(differences(core, &app)?, "the app's shared files, marked blocks and patch block match this core") } else { write(core, &app) }
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
        let text =
            format!("# Title\n\nThe line `{}` is quoted here.\n\n{}\nrule\n{}\n\nown text\n", block_begin("a"), block_begin("a"), block_end("a"));
        let block = marked_block(&text, "a").unwrap();
        assert_eq!(block, format!("{}\nrule\n{}", block_begin("a"), block_end("a")));
        assert!(marked_block(&text, "b").is_none());
        // An END line with no BEGIN line before it, or a BEGIN line with no END line: no block.
        assert!(marked_block(&format!("{}\nrule\n{}\n", block_end("a"), block_begin("a")), "a").is_none());
        assert!(marked_block(&format!("{}\nrule\n", block_begin("a")), "a").is_none());
        // Windows line ends: the block stops before the END line's break.
        let crlf = format!("{}\r\nrule\r\n{}\r\nown\r\n", block_begin("a"), block_end("a"));
        assert!(marked_block(&crlf, "a").unwrap().ends_with("-->"));
    }

    #[test]
    fn replacing_a_block_leaves_the_text_around_it() {
        let old = format!("intro\n\n{}\nold rule\n{}\n\nown rules\n", block_begin("a"), block_end("a"));
        let block = format!("{}\nnew rule\n{}", block_begin("a"), block_end("a"));
        assert_eq!(replace_marked_block(&old, "a", &block).unwrap(), format!("intro\n\n{block}\n\nown rules\n"));
        assert!(replace_marked_block("intro\n\nown rules\n", "a", &block).is_none());
    }

    /// A core and an app folder with the files `write` and `differences` read.
    fn folders(name: &str) -> (PathBuf, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("jcj-sync-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let (core, app) = (tmp.join("core"), tmp.join("app"));
        let put = |root: &Path, file: &str, text: &str| {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        for (from, _) in MANAGED {
            put(&core, from, &format!("managed {from}\n"));
        }
        for (from, _, _) in BLOCKS {
            let mut text = String::from("# The core's template\n");
            for (_, _, block) in BLOCKS.iter().filter(|(f, _, _)| f == from) {
                text.push_str(&format!("\n{}\nthe core's {block} rules\n{}\n", block_begin(block), block_end(block)));
            }
            put(&core, from, &text);
        }
        let patch = format!("{BLOCK_BEGIN}\n[patch.x]\n{BLOCK_END}\n");
        put(&core, "Cargo.toml", &patch);
        put(&app, "Cargo.toml", "[workspace]\n");
        (core, app)
    }

    #[test]
    fn writes_blocks_where_the_markers_are_and_never_guesses() {
        let (core, app) = folders("write");
        // AGENTS.md has two of its three blocks marked, around the app's own text; SECURITY.md is a
        // hand copy with no markers; CONTRIBUTING.md does not exist.
        let agents = format!(
            "# The app\n\n{}\nan old hand copy\n{}\n\nown text between\n\n{}\n{}\n\n## The app's own rules\n",
            block_begin("public"),
            block_end("public"),
            block_begin("app-and-core"),
            block_end("app-and-core")
        );
        std::fs::write(app.join("AGENTS.md"), &agents).unwrap();
        std::fs::write(app.join("SECURITY.md"), "# Security\n\na hand copy\n").unwrap();

        let err = write(&core, &app).unwrap_err();
        assert!(err.contains("AGENTS.md: no `app` block") && err.contains("SECURITY.md: no `security` block"), "{err}");
        assert!(!err.contains("public") && !err.contains("CONTRIBUTING.md"), "{err}");
        // Everything else was written all the same.
        let read = |file: &str| std::fs::read_to_string(app.join(file)).unwrap();
        assert_eq!(read("CODE_OF_CONDUCT.md"), "managed template/CODE_OF_CONDUCT.md\n");
        assert!(read("Cargo.toml").contains("[patch.x]"));
        assert_eq!(read("CONTRIBUTING.md"), std::fs::read_to_string(core.join("template/CONTRIBUTING.md")).unwrap());
        assert_eq!(read("SECURITY.md"), "# Security\n\na hand copy\n");
        let new = read("AGENTS.md");
        assert!(new.starts_with("# The app\n\n") && new.ends_with("\n\n## The app's own rules\n") && new.contains("\n\nown text between\n\n"));
        assert!(new.contains("the core's public rules") && new.contains("the core's app-and-core rules") && !new.contains("an old hand copy"));

        let found = differences(&core, &app).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().any(|d| d.starts_with("AGENTS.md: the `app` block is missing")));
        assert!(found.iter().any(|d| d.starts_with("SECURITY.md: the `security` block is missing")));

        // With the marker lines in place the next run fills them, and nothing differs.
        std::fs::write(app.join("AGENTS.md"), format!("{new}\n{}\n{}\n", block_begin("app"), block_end("app"))).unwrap();
        std::fs::write(app.join("SECURITY.md"), format!("# Security\n\n{}\n{}\n", block_begin("security"), block_end("security"))).unwrap();
        write(&core, &app).unwrap();
        assert!(differences(&core, &app).unwrap().is_empty());

        // A block edited in the app, and a missing file, are both reported.
        std::fs::write(app.join("SECURITY.md"), read("SECURITY.md").replace("the core's security rules", "our own rules")).unwrap();
        std::fs::remove_file(app.join("AGENTS.md")).unwrap();
        let found = differences(&core, &app).unwrap();
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
    fn says_what_an_older_main_does_not_run() {
        assert!(not_run("\"core-pin\" => corepin::task(&root, rest),").is_empty());
        let notes = not_run("mod sync;");
        assert_eq!(notes.len(), RUN_FROM_MAIN.len());
        assert!(notes[0].contains("the core-pin check"));
    }
}
