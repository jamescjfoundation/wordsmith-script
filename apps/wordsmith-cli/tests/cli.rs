// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command-line tool, run as a program.

use std::path::PathBuf;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wordsmith-cli")).args(args).output().expect("runs")
}

fn temp_dir(name: &str) -> PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("wordsmith-cli-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn golden(name: &str) -> String {
    format!("{}/../../crates/wordsmith-format/tests/golden/{name}.fountain", env!("CARGO_MANIFEST_DIR"))
}

fn s(p: &std::path::Path) -> &str {
    p.to_str().expect("utf-8 path")
}

#[test]
fn convert_export_report_and_info() {
    let dir = temp_dir("all");
    let input = golden("timid-teddy");
    let wss = dir.join("teddy.wss");
    assert!(cli(&["convert", &input, s(&wss)]).status.success());
    assert!(std::fs::read_to_string(&wss).expect("written").starts_with("Wordsmith Script "));
    let pdf = dir.join("teddy.pdf");
    let out = cli(&["export-pdf", s(&wss), s(&pdf)]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(std::fs::read(&pdf).expect("pdf").starts_with(b"%PDF-"));
    let out = cli(&["report", "characters", &input]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("Character,Speeches,Words,Scenes"));
    let csv = dir.join("scenes.csv");
    assert!(cli(&["report", "scenes", &input, s(&csv)]).status.success());
    assert!(std::fs::read_to_string(&csv).expect("csv").starts_with("Scene,Heading,Page,Length"));
    let out = cli(&["info", &input]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("title: Timid Teddy") && text.contains("scenes: 52"), "{text}");
    // Nothing revised: a revised-pages export says so and fails.
    let out = cli(&["export-pdf", &input, s(&dir.join("r.pdf")), "--revised-only"]);
    assert_eq!(out.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_errors_and_failures_have_their_exit_codes() {
    assert_eq!(cli(&["frobnicate"]).status.code(), Some(2));
    assert_eq!(cli(&["convert", "only-one"]).status.code(), Some(2));
    assert_eq!(cli(&["report", "costumes", "x.fountain"]).status.code(), Some(2));
    assert_eq!(cli(&["convert", "/no/such/file.fountain", "/tmp/x.wss"]).status.code(), Some(1));
    assert!(cli(&["--help"]).status.success());
    assert!(String::from_utf8_lossy(&cli(&["--version"]).stdout).starts_with("wordsmith-cli "));
}

#[test]
fn the_mcp_server_answers_over_standard_input() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_wordsmith-cli"))
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("starts");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"test","version":"1"}}}}}}"#).expect("write");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).expect("write");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"type_text","arguments":{{"text":"INT. KITCHEN - NIGHT"}}}}}}"#
        )
        .expect("write");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"scenes","arguments":{{}}}}}}"#).expect("write");
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("ends");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains(r#""id":3"#) && text.contains("INT. KITCHEN - NIGHT"), "{text}");
}
