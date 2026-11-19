// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script web build. `cargo xtask ci` compiles this crate for
//! `wasm32-unknown-unknown` so the interface stays portable.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

/// Text the web build shows in its About panel.
pub fn about_text() -> String {
    wordsmith_ui::about_text()
}

#[cfg(test)]
mod tests {
    #[test]
    fn about_matches_desktop() {
        assert_eq!(super::about_text(), wordsmith_ui::about_text());
    }
}
