// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script desktop app. The window arrives with milestone M1.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

fn main() {
    println!("{}", wordsmith_ui::about_text());
}
