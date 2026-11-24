// SPDX-License-Identifier: MIT OR Apache-2.0
//! The core's patch block, and dev-linking an app to a local core checkout.
//!
//! - **Patch block.** Cargo honours `[patch]` only in the top-level project, so every app carries
//!   an exact copy of the core's block (between the `BEGIN`/`END` marker lines of the core's
//!   `Cargo.toml`). [`extract_block`] reads it; [`replace_block`] writes it into an app manifest.
//! - **Dev-link.** While building a core change and an app change together, the app points at a
//!   local core checkout: `cargo xtask dev-link ../open-source-core` writes a `[patch]` section
//!   into the app's `.cargo/config.toml` between marker lines, and `cargo xtask dev-unlink`
//!   removes it. The section holds an absolute local path, so it must never be committed:
//!   `cargo xtask dev-link --check` (part of every app's `ci`) fails while it is there.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::{Path, PathBuf};

use crate::util::TaskResult;

pub const BLOCK_BEGIN: &str = "# BEGIN jcj patch block";
pub const BLOCK_END: &str = "# END jcj patch block";
pub const LINK_BEGIN: &str = "# BEGIN jcj dev-link (local only: never commit; `cargo xtask dev-unlink` removes it)";
pub const LINK_END: &str = "# END jcj dev-link";
/// The core's address, as apps name it in their dependencies.
pub const CORE_GIT: &str = "https://github.com/jamescjfoundation/open-source-core";

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

/// `manifest` without the marked block (and the blank line before it).
pub fn remove_block(manifest: &str, begin: &str, end: &str) -> String {
    match (manifest.find(begin), manifest.find(end)) {
        (Some(s), Some(e)) if e > s => {
            let before = manifest[..s].trim_end_matches('\n');
            let after = manifest[e + end.len()..].trim_start_matches('\n');
            if before.is_empty() {
                after.to_owned()
            } else if after.is_empty() {
                format!("{before}\n")
            } else {
                format!("{before}\n\n{after}")
            }
        }
        _ => manifest.to_owned(),
    }
}

/// The `jcj-*` crates of a core checkout (every directory under `crates/` with a manifest).
pub fn core_crates(core: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let dir = core.join("crates");
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{} is not an open-source-core checkout: {e}", core.display()))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("jcj-") && path.join("Cargo.toml").is_file() {
            out.push((name, path));
        }
    }
    out.sort();
    if out.is_empty() {
        return Err(format!("no jcj- crates under {}", dir.display()));
    }
    Ok(out)
}

/// The dev-link section for a core checkout at `core` (absolute).
pub fn link_section(core: &Path) -> Result<String, String> {
    let mut s = format!("{LINK_BEGIN}\n[patch.\"{CORE_GIT}\"]\n");
    for (name, path) in core_crates(core)? {
        let p = path.to_string_lossy().replace('\\', "/");
        s.push_str(&format!("{name} = {{ path = \"{p}\" }}\n"));
    }
    s.push_str(LINK_END);
    Ok(s)
}

fn config_path(app: &Path) -> PathBuf {
    app.join(".cargo").join("config.toml")
}

/// `cargo xtask dev-link <core dir>`.
pub fn dev_link(app: &Path, core: &str) -> TaskResult {
    let core = std::fs::canonicalize(core).map_err(|e| format!("cannot find the core checkout {core}: {e}"))?;
    let section = link_section(&core)?;
    let path = config_path(app);
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let new = replace_block(&old, &section, LINK_BEGIN, LINK_END);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, new).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!("linked to the core at {} (never commit .cargo/config.toml while linked)", core.display());
    Ok(())
}

/// `cargo xtask dev-unlink`.
pub fn dev_unlink(app: &Path) -> TaskResult {
    let path = config_path(app);
    let Ok(old) = std::fs::read_to_string(&path) else { return Ok(()) };
    let new = remove_block(&old, LINK_BEGIN, LINK_END);
    if new.trim().is_empty() {
        std::fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
    } else if new != old {
        std::fs::write(&path, new).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    println!("unlinked: the app builds against its pinned core release again");
    Ok(())
}

/// `cargo xtask dev-link --check`: fails while a dev-link is in place.
pub fn check_unlinked(app: &Path) -> TaskResult {
    let text = std::fs::read_to_string(config_path(app)).unwrap_or_default();
    if text.contains(LINK_BEGIN) || text.contains("open-source-core/crates/") {
        return Err("a local core dev-link is in .cargo/config.toml; run `cargo xtask dev-unlink` before committing".to_owned());
    }
    println!("ok: no local core dev-link");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "[workspace]\n\n# BEGIN jcj patch block\n[patch.x]\na = 1\n# END jcj patch block\n";

    #[test]
    fn blocks_extract_replace_and_remove() {
        let b = extract_block(M, BLOCK_BEGIN, BLOCK_END).unwrap();
        assert!(b.starts_with(BLOCK_BEGIN) && b.ends_with(BLOCK_END));
        let new = "# BEGIN jcj patch block\n[patch.y]\n# END jcj patch block";
        let r = replace_block(M, new, BLOCK_BEGIN, BLOCK_END);
        assert!(r.contains("[patch.y]") && !r.contains("[patch.x]"));
        let appended = replace_block("[workspace]\n", new, BLOCK_BEGIN, BLOCK_END);
        assert_eq!(appended, format!("[workspace]\n\n{new}\n"));
        assert_eq!(remove_block(&appended, BLOCK_BEGIN, BLOCK_END), "[workspace]\n");
        assert_eq!(remove_block("x\n", BLOCK_BEGIN, BLOCK_END), "x\n");
    }

    #[test]
    fn linking_writes_and_unlinking_removes() {
        let tmp = std::env::temp_dir().join(format!("jcj-link-test-{}", std::process::id()));
        let core = tmp.join("core");
        let app = tmp.join("app");
        std::fs::create_dir_all(core.join("crates/jcj-doc")).unwrap();
        std::fs::write(core.join("crates/jcj-doc/Cargo.toml"), "").unwrap();
        std::fs::create_dir_all(app.join(".cargo")).unwrap();
        std::fs::write(app.join(".cargo/config.toml"), "[alias]\nxtask = \"run -p xtask --\"\n").unwrap();
        dev_link(&app, core.to_str().unwrap()).unwrap();
        let text = std::fs::read_to_string(app.join(".cargo/config.toml")).unwrap();
        assert!(text.contains("jcj-doc = { path = ") && text.contains("[alias]"));
        assert!(check_unlinked(&app).is_err());
        dev_unlink(&app).unwrap();
        assert_eq!(std::fs::read_to_string(app.join(".cargo/config.toml")).unwrap(), "[alias]\nxtask = \"run -p xtask --\"\n");
        assert!(check_unlinked(&app).is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
