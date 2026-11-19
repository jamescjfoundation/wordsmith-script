// SPDX-License-Identifier: MIT OR Apache-2.0
//! The workspace layering rule.
//!
//! Every workspace crate has a layer number. A crate may depend on crates in
//! its own layer or below, never above. The interface layer sits directly
//! under the apps, so nothing below `wordsmith-ui` can depend on it. The
//! order is explained in `docs/decisions/0002-workspace-layers.md`.

/// Crates that are tooling, not product, and are not layered.
pub const UNLAYERED: &[&str] = &["xtask"];

/// (crate, layer). Higher numbers sit higher in the stack.
pub const LAYERS: &[(&str, u8)] = &[
    // Model
    ("wordsmith-geom", 1),
    ("wordsmith-doc", 1),
    // Layout, fonts and rendering
    ("wordsmith-fonts", 2),
    ("wordsmith-layout", 2),
    ("wordsmith-render", 2),
    // Output that only needs layout
    ("wordsmith-pdf", 3),
    // Engine: session, undo, command registry, recovery
    ("wordsmith-engine", 4),
    // Screenplay rules
    ("wordsmith-script", 5),
    // Files that need screenplay elements (native format, Fountain)
    ("wordsmith-format", 6),
    // Agent control over screenplay commands
    ("wordsmith-mcp", 7),
    // Interface
    ("wordsmith-ui", 8),
    // Apps
    ("wordsmith", 9),
    ("wordsmith-cli", 9),
    ("wordsmith-web", 9),
];

/// Layer of a workspace crate, if it has one.
pub fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

/// Checks one dependency edge between workspace crates.
pub fn check_edge(from: &str, to: &str) -> Result<(), String> {
    if UNLAYERED.contains(&from) {
        return Ok(());
    }
    if UNLAYERED.contains(&to) {
        return Err(format!("{from} depends on tooling crate {to}"));
    }
    let lf = layer_of(from).ok_or_else(|| format!("{from} has no layer in xtask/src/layers.rs"))?;
    let lt = layer_of(to).ok_or_else(|| format!("{to} has no layer in xtask/src/layers.rs"))?;
    if lt > lf {
        return Err(format!(
            "{from} (layer {lf}) depends on {to} (layer {lt}); dependencies must point down"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_apps_sit_above_ui() {
        let ui = layer_of("wordsmith-ui").unwrap_or(0);
        for (name, layer) in LAYERS {
            if *layer > ui {
                assert!(
                    ["wordsmith", "wordsmith-cli", "wordsmith-web"].contains(name),
                    "{name} sits above the interface layer"
                );
            }
        }
    }

    #[test]
    fn edges() {
        assert!(check_edge("wordsmith", "wordsmith-ui").is_ok());
        assert!(check_edge("wordsmith-format", "wordsmith-script").is_ok());
        assert!(check_edge("wordsmith-layout", "wordsmith-fonts").is_ok());
        assert!(check_edge("wordsmith-script", "wordsmith-ui").is_err());
        assert!(check_edge("wordsmith-mcp", "wordsmith-ui").is_err());
        assert!(check_edge("wordsmith-doc", "wordsmith-layout").is_err());
        assert!(check_edge("wordsmith-new", "wordsmith-doc").is_err());
        assert!(check_edge("wordsmith-ui", "xtask").is_err());
        assert!(check_edge("xtask", "wordsmith-ui").is_ok());
    }
}
