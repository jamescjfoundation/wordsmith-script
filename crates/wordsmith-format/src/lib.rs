// SPDX-License-Identifier: MIT OR Apache-2.0
//! File formats for Wordsmith Script: the native plain-text format and
//! Fountain ([`native`], [`fountain`]); imports and exports arrive with M4.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod fountain;
pub mod native;

/// File extension of the native format. The final choice is recorded in
/// `docs/format.md` when the format lands in M4.
pub const NATIVE_EXTENSION: &str = "wss";

/// Returns the product name, proving the dependency on the screenplay layer.
pub fn product_name() -> &'static str {
    wordsmith_script::PRODUCT_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_screenplay_layer() {
        assert_eq!(product_name(), "Wordsmith Script");
        assert_eq!(NATIVE_EXTENSION, "wss");
    }
}
