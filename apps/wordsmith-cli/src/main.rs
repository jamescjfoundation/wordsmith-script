// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script command-line tool. `convert <input> <output>` reads a script (`.wss`,
//! Fountain; plain text, RTF and PDF are imported as scripts) and writes it in the format the
//! output's extension names (`.wss`, `.fountain`, `.txt`, `.rtf`, `.html`, `.pdf`). More
//! commands arrive with milestone M8.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;
use std::process::ExitCode;

use wordsmith_format::files::{self, Kind};

/// Read `input` and write it as the kind `output` names.
fn convert(input: &Path, output: &Path) -> Result<(), String> {
    let doc = match Kind::of(input) {
        Some(Kind::Native | Kind::Fountain) => files::open(input)?,
        _ => files::import_file(input, true)?,
    };
    let kind = Kind::of(output).ok_or_else(|| format!("{}: unknown output format", output.display()))?;
    files::export_to(output, &doc, kind)
}

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
        Some("convert") => {
            let (Some(input), Some(output), None) = (args.next(), args.next(), args.next()) else {
                eprintln!("usage: wordsmith-cli convert <input> <output>");
                return ExitCode::from(2);
            };
            match convert(Path::new(&input), Path::new(&output)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("wordsmith-cli: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some(other) => {
            eprintln!("wordsmith-cli: unknown command '{other}'");
            ExitCode::from(2)
        }
    }
}
