// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command line, held to the convention every app on the core shares
//! ([core decision 0017](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0017-command-lines-and-agent-tools.md)):
//! the built program is run and its exit code, its standard output and its standard error are
//! read. `cli.rs` tests what the commands do; this file tests how the program answers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use jcj_agents::cli::{Exit, McpArgs};
use serde_json::{Value, json};

fn run(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wordsmith-script-cli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn code(o: &Output) -> Option<i32> {
    o.status.code()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn exit(e: Exit) -> Option<i32> {
    Some(i32::from(e.code()))
}

/// A folder of this test's own, with its links followed (a root is compared that way).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wordsmith-script-cli-line-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

fn golden(name: &str) -> String {
    format!("{}/../../tests/golden/{name}.fountain", env!("CARGO_MANIFEST_DIR"))
}

fn s(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// One JSON-RPC request, as a line.
fn request(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string() + "\n"
}

/// A call of the general `execute` tool.
fn execute(id: u64, command: &str, params: Value) -> String {
    request(id, "tools/call", json!({"name": "execute", "arguments": {"command": command, "params": params}}))
}

/// The server's answers: every line of standard output must be one.
fn replies(o: &Output) -> Vec<Value> {
    text(&o.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn is_error(reply: &Value) -> Option<bool> {
    reply["result"]["isError"].as_bool()
}

#[test]
fn help_and_version_are_answered_on_standard_output() {
    for help in ["--help", "-h", "help"] {
        let o = run(&[help], "");
        assert_eq!(code(&o), exit(Exit::Done), "{help}");
        assert!(text(&o.stdout).starts_with("usage:") && o.stderr.is_empty(), "{help}");
        assert!(text(&o.stdout).contains(McpArgs::USAGE), "{help}");
    }
    for version in ["--version", "-V"] {
        let o = run(&[version], "");
        assert_eq!(code(&o), exit(Exit::Done), "{version}");
        // The first line is the program and its version and nothing else; more may follow.
        assert_eq!(text(&o.stdout).lines().next(), Some(concat!("wordsmith-script-cli ", env!("CARGO_PKG_VERSION"))));
        assert!(text(&o.stdout).contains("Wordsmith Script") && o.stderr.is_empty(), "{version}");
    }
}

#[test]
fn no_argument_is_a_usage_error_not_a_success() {
    let o = run(&[], "");
    assert_eq!(code(&o), exit(Exit::Usage));
    assert!(o.stdout.is_empty(), "nothing on standard output");
    assert!(text(&o.stderr).contains("usage:"), "{}", text(&o.stderr));
}

#[test]
fn a_wrong_command_line_is_a_usage_error_on_standard_error() {
    let dir = temp_dir("usage");
    let never = s(&dir.join("never.wss"));
    let input = golden("timid-teddy");
    for bad in [
        &["--bogus"][..],
        &["frobnicate"],
        &["convert", "only-one"],
        &["convert", &input, &never, "extra"],
        &["convert", &input, "out.unknown"],
        &["export-pdf", &input, &never, "--bogus"],
        &["export-pdf", &input],
        &["report", "costumes", &input],
        &["info"],
        &["info", &input, "extra"],
        &["mcp", "--bogus"],
        &["mcp", "--read-root"],
        &["mcp", "--read-root", &s(&dir), "--read-root", &s(&dir)],
        &["mcp", &input, "second.wss"],
        // A root that is no folder, even beside a file that could be opened.
        &["mcp", "--read-root", &s(&dir.join("no-such-folder"))],
        &["mcp", &input, "--write-root", &s(&dir.join("no-such-folder"))],
    ] {
        let o = run(bad, "");
        assert_eq!(code(&o), exit(Exit::Usage), "{bad:?}");
        assert!(o.stdout.is_empty(), "{bad:?}: nothing on standard output");
        let said = text(&o.stderr);
        assert!(said.starts_with("wordsmith-script-cli: ") && said.contains("usage:"), "{bad:?}: {said}");
    }
    assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing was written");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failure_is_1_with_its_reason_on_standard_error() {
    let dir = temp_dir("failed");
    let missing = s(&dir.join("missing.fountain"));
    let out = s(&dir.join("out.pdf"));
    let input = golden("timid-teddy");
    for failing in [
        &["convert", &missing, &out][..],
        &["info", &missing],
        &["report", "scenes", &missing],
        // Nothing is revised, so there are no revised pages to write.
        &["export-pdf", &input, &out, "--revised-only"],
        // The file named to `mcp` could not be opened: the server does not start.
        &["mcp", &missing],
        &["mcp", &s(&dir.join("page.html"))],
    ] {
        let o = run(failing, "");
        assert_eq!(code(&o), exit(Exit::Failed), "{failing:?}");
        assert!(o.stdout.is_empty(), "{failing:?}: nothing on standard output");
        let said = text(&o.stderr);
        assert!(said.starts_with("wordsmith-script-cli: ") && !said.contains("usage:"), "{failing:?}: {said}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn results_go_to_standard_output() {
    let dir = temp_dir("results");
    let input = golden("timid-teddy");
    let pdf = s(&dir.join("teddy.pdf"));
    let o = run(&["export-pdf", &input, &pdf], "");
    assert_eq!(code(&o), exit(Exit::Done));
    let said = text(&o.stdout);
    assert!(said.starts_with(&pdf) && said.trim_end().ends_with("pages") && o.stderr.is_empty(), "{said}");
    for quiet in [&["convert", &input, &s(&dir.join("teddy.wss"))][..], &["report", "scenes", &input, &s(&dir.join("scenes.csv"))]] {
        let o = run(quiet, "");
        assert_eq!(code(&o), exit(Exit::Done), "{quiet:?}");
        assert!(o.stdout.is_empty() && o.stderr.is_empty(), "{quiet:?}");
    }
    for printed in [&["info", &input][..], &["report", "characters", &input]] {
        let o = run(printed, "");
        assert_eq!(code(&o), exit(Exit::Done), "{printed:?}");
        assert!(!o.stdout.is_empty() && o.stderr.is_empty(), "{printed:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mcp_names_its_own_tools_after_the_app() {
    // Unconfined: said on standard error; standard output carries only the protocol.
    let o = run(&["mcp"], &(request(1, "initialize", json!({})) + &request(2, "tools/list", json!({}))));
    assert_eq!(code(&o), exit(Exit::Done), "it ends when its input ends");
    let said = text(&o.stderr);
    assert!(said.starts_with("wordsmith-script-cli mcp: ") && said.contains("no --read-root or --write-root given"), "{said}");
    let replies = replies(&o);
    assert_eq!(replies.len(), 2, "one line of JSON for each request, nothing else");
    let about = &replies[0]["result"];
    assert_eq!(about["serverInfo"]["name"], "wordsmith-script");
    assert_eq!(about["serverInfo"]["title"], "Wordsmith Script");
    assert_eq!(about["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    let instructions = about["instructions"].as_str().unwrap();
    assert!(instructions.contains("script_scene_list") && !instructions.contains("already open"), "{instructions}");

    let tools = replies[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for own in ["script_scene_list", "script_element_action", "script_character_list", "script_report_characters", "script_titlePage_set"] {
        assert!(names.contains(&own), "{own}: {names:?}");
    }
    // The general tools keep the names they have in every app.
    for general in ["execute", "list_commands", "batch", "type_text", "get_text", "render_page"] {
        assert!(names.contains(&general), "{general}: {names:?}");
    }
    // A tool is the app's own exactly when its description names the command it runs, and
    // then its name is `script_` and that command's id.
    let mut own = 0;
    for tool in tools {
        let (name, description) = (tool["name"].as_str().unwrap(), tool["description"].as_str().unwrap_or_default());
        match description.split_once("Command `").and_then(|(_, rest)| rest.split_once('`')) {
            Some((id, _)) => {
                assert_eq!(name, jcj_agents::app_tool_name("script", id), "{description}");
                assert!(tool["annotations"]["readOnlyHint"].is_boolean(), "{name}");
                own += 1;
            }
            None => assert!(!name.starts_with("script_"), "{name}: {description}"),
        }
    }
    assert!(own > 100, "{own} tools of the app's own");
    assert_eq!(names.len(), names.iter().collect::<std::collections::BTreeSet<_>>().len(), "no name twice");
}

#[test]
fn mcp_opens_the_file_it_is_given_before_the_folders_hold() {
    // The person's script is in one folder; the agent is given another.
    let dir = temp_dir("file");
    let (mine, agents) = (dir.join("mine"), dir.join("agents"));
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::create_dir_all(&agents).unwrap();
    let script = mine.join("teddy.wss");
    assert_eq!(code(&run(&["convert", &golden("timid-teddy"), &s(&script)], "")), exit(Exit::Done));
    let before = std::fs::read(&script).unwrap();
    let (inside, outside) = (agents.join("copy.wss"), mine.join("copy.wss"));

    let input = request(1, "initialize", json!({}))
        + &request(2, "tools/call", json!({"name": "script_scene_list", "arguments": {}}))
        // The agent cannot open the same file itself: it is outside the folder it may read.
        + &execute(3, "file.openPath", json!({"path": s(&script), "discard": true}))
        // It can write inside its folder and nowhere else, the script's own file included.
        + &execute(4, "file.saveAsPath", json!({"path": s(&outside)}))
        + &execute(5, "text.insert", json!({"text": "A"}))
        + &execute(6, "file.save", json!({}))
        + &execute(7, "file.saveAsPath", json!({"path": s(&inside)}));
    let o = run(&["mcp", &s(&script), "--read-root", &s(&agents), "--write-root", &s(&agents)], &input);
    assert_eq!(code(&o), exit(Exit::Done));
    assert!(!text(&o.stderr).contains("any file"), "{}", text(&o.stderr));
    let replies = replies(&o);
    assert_eq!(replies.len(), 7);
    assert!(replies[0]["result"]["instructions"].as_str().unwrap().contains("The script \"teddy\" is already open."));
    assert_eq!(is_error(&replies[1]), Some(false));
    assert!(replies[1].to_string().contains("TEDDY'S ROOMS"), "{}", replies[1]);
    assert_eq!(is_error(&replies[2]), Some(true), "{}", replies[2]);
    assert_eq!(is_error(&replies[3]), Some(true), "{}", replies[3]);
    assert_eq!(is_error(&replies[4]), Some(false), "{}", replies[4]);
    assert_eq!(is_error(&replies[5]), Some(true), "{}", replies[5]);
    assert_eq!(is_error(&replies[6]), Some(false), "{}", replies[6]);
    assert!(inside.is_file() && !outside.exists());
    assert_eq!(std::fs::read(&script).unwrap(), before, "the script's own file is outside the write folder, so it is not written");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mcp_with_one_flag_grants_only_that_access() {
    let dir = temp_dir("one-flag");
    let script = dir.join("teddy.wss");
    assert_eq!(code(&run(&["convert", &golden("timid-teddy"), &s(&script)], "")), exit(Exit::Done));
    let copy = dir.join("copy.wss");
    // Reading only: the script in the folder opens, and nothing can be written, even there.
    let input = execute(1, "file.openPath", json!({"path": s(&script)})) + &execute(2, "file.saveAsPath", json!({"path": s(&copy)}));
    let o = run(&["mcp", "--read-root", &s(&dir)], &input);
    let answers = replies(&o);
    assert_eq!((is_error(&answers[0]), is_error(&answers[1])), (Some(false), Some(true)), "{answers:?}");
    assert!(!copy.exists());
    // Writing only: nothing can be read, and a new script can be saved in the folder.
    let input = execute(1, "file.openPath", json!({"path": s(&script)})) + &execute(2, "file.saveAsPath", json!({"path": s(&copy)}));
    let o = run(&["mcp", "--write-root", &s(&dir)], &input);
    let answers = replies(&o);
    assert_eq!((is_error(&answers[0]), is_error(&answers[1])), (Some(true), Some(false)), "{answers:?}");
    assert!(copy.is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
