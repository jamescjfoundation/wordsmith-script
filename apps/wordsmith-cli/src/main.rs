// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script command-line tool. Commands arrive with milestone M8.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("--version") | Some("-V") => {
            println!(
                "wordsmith-cli {} ({}, native extension .{})",
                wordsmith_script::VERSION,
                wordsmith_script::PRODUCT_NAME,
                wordsmith_format::NATIVE_EXTENSION
            );
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("wordsmith-cli: unknown command '{other}'");
            ExitCode::from(2)
        }
    }
}
