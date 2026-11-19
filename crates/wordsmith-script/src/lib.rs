// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenplay layer for Wordsmith Script.
//!
//! This crate will hold every screenplay rule from `docs/SPEC.md`: the
//! element registry, Return and Tab behavior, autocomplete lists, the scene
//! index, revision sets and page locks. The skeleton only exposes the crate
//! identity so the workspace, lints and layering checks have something to run.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

/// Product name shown to people.
pub const PRODUCT_NAME: &str = "Wordsmith Script";

/// Version of this crate, from Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_plain() {
        assert_eq!(PRODUCT_NAME, "Wordsmith Script");
        assert!(!VERSION.is_empty());
    }
}
