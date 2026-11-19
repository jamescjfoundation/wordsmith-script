// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface layer for Wordsmith Script. Only the app crates may depend on
//! it; `cargo xtask ci` enforces that with its layering check.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

/// Plain-text credit required by the reuse rules. Shown in the About window.
pub const UPSTREAM_CREDIT: &str = "based on WordCraft by the ArtCraft team";

/// Text for the About window.
pub fn about_text() -> String {
    format!(
        "{} {}\n{}\nNative files: .{}",
        wordsmith_script::PRODUCT_NAME,
        wordsmith_script::VERSION,
        UPSTREAM_CREDIT,
        wordsmith_format::NATIVE_EXTENSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_carries_credit() {
        let text = about_text();
        assert!(text.starts_with("Wordsmith Script "));
        assert!(text.contains(UPSTREAM_CREDIT));
    }
}
