// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script on the command line (decision 0029).
//!
//! ```text
//! wordsmith-cli convert <input> <output>
//! wordsmith-cli export-pdf <input> <output.pdf> [--revised-only] [--no-marks]
//! wordsmith-cli report characters|scenes <input> [<output.csv>]
//! wordsmith-cli info <input>
//! wordsmith-cli mcp [<file>] [--read-root <folder>] [--write-root <folder>]
//! wordsmith-cli --help | --version
//! ```
//!
//! Inputs are `.wss` and Fountain scripts; plain text, RTF and PDF are imported as scripts.
//! Outputs are written in the format their extension names.
//!
//! The command line follows the convention every app on the core shares
//! ([core decision 0017](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0017-command-lines-and-agent-tools.md)),
//! through the core's code for it (`jcj_agents::cli`). Results go to standard output and what
//! went wrong to standard error. Exit codes: 0 done, 1 failed, 2 a wrong command line. 3, "a
//! check of the content did not pass", is kept for a command that checks or compares: no command
//! here does, so this program never ends with it.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;
use std::process::ExitCode;

use jcj_agents::cli::{Exit, McpArgs, Standard, standard, version_line};
use jcj_doc::Document;
use wordsmith_format::files::{self, Kind};

const PROGRAM: &str = "wordsmith-cli";

fn usage() -> String {
    format!(
        "usage:
  {PROGRAM} convert <input> <output>
  {PROGRAM} export-pdf <input> <output.pdf> [--revised-only] [--no-marks]
  {PROGRAM} report characters|scenes <input> [<output.csv>]
  {PROGRAM} info <input>
  {PROGRAM} {}
  {PROGRAM} --help | --version",
        McpArgs::USAGE
    )
}

/// What went wrong, with the exit code that says which kind it is.
struct Error {
    exit: Exit,
    why: String,
}

/// The command line was wrong: nothing is done.
fn wrong(why: impl Into<String>) -> Error {
    Error { exit: Exit::Usage, why: why.into() }
}

/// The command could not do what was asked.
fn failed(why: impl Into<String>) -> Error {
    Error { exit: Exit::Failed, why: why.into() }
}

/// Whether `arg` is written as an option (`-x`, `--long`); a lone `-` is not.
fn is_option(arg: &str) -> bool {
    arg.starts_with('-') && arg.len() > 1
}

/// Read a script: open a native or Fountain file, import anything else as a script.
fn load(input: &Path) -> Result<Document, Error> {
    match Kind::of(input) {
        Some(Kind::Native | Kind::Fountain) => files::open(input).map_err(failed),
        _ => files::import_file(input, true).map_err(failed),
    }
}

fn convert(args: &[String]) -> Result<(), Error> {
    let [input, output] = args else { return Err(wrong("convert takes an input and an output")) };
    let kind = Kind::of(Path::new(output)).ok_or_else(|| wrong(format!("{output}: unknown output format (wss, fountain, txt, rtf, html, pdf)")))?;
    let doc = load(Path::new(input))?;
    files::export_to(Path::new(output), &doc, kind).map_err(failed)
}

fn export_pdf(args: &[String]) -> Result<(), Error> {
    let mut opts = wordsmith_script::export::PrintOptions::default();
    let mut paths = Vec::new();
    for a in args {
        match a.as_str() {
            "--revised-only" => opts.revised_only = true,
            "--no-marks" => opts.marks = false,
            flag if is_option(flag) => return Err(wrong(format!("unknown option {flag}"))),
            p => paths.push(p),
        }
    }
    let [input, output] = paths[..] else { return Err(wrong("export-pdf takes an input and an output")) };
    let doc = load(Path::new(input))?;
    let (bytes, pages) = wordsmith_script::export::pdf_with(&doc, opts).map_err(failed)?;
    files::write_atomic(Path::new(output), &bytes).map_err(failed)?;
    println!("{output}: {pages} page{}", if pages == 1 { "" } else { "s" });
    Ok(())
}

fn report(args: &[String]) -> Result<(), Error> {
    let (which, input, output) = match args {
        [w, i] => (w, i, None),
        [w, i, o] => (w, i, Some(o)),
        _ => return Err(wrong("report takes characters or scenes, an input, and an optional output")),
    };
    if !matches!(which.as_str(), "characters" | "scenes") {
        return Err(wrong(format!("unknown report {which} (characters, scenes)")));
    }
    let doc = load(Path::new(input))?;
    let csv = if which == "characters" {
        wordsmith_script::reports::character_report_csv(&doc)
    } else {
        wordsmith_script::commands::character::scenes_csv(&doc)
    };
    match output {
        Some(o) => files::write_atomic(Path::new(o), csv.as_bytes()).map_err(failed),
        None => {
            print!("{csv}");
            Ok(())
        }
    }
}

fn info(args: &[String]) -> Result<(), Error> {
    let [input] = args else { return Err(wrong("info takes an input")) };
    let doc = load(Path::new(input))?;
    let pag = wordsmith_script::paginate::paginate(&doc);
    let scenes = wordsmith_script::scenes(&doc).len();
    let characters = wordsmith_script::commands::character::characters(&doc).len();
    let title = if doc.core.title.is_empty() { "(untitled)" } else { doc.core.title.as_str() };
    println!("title: {title}\npages: {}\nscenes: {scenes}\ncharacters: {characters}\nparagraphs: {}", pag.pages.len(), doc.body.len());
    Ok(())
}

/// The short name every tool of Script's own starts with (`script_scene_list`).
const TOOL_PREFIX: &str = "script";

/// What the agent server tells a client about the app.
const INSTRUCTIONS: &str = "Wordsmith Script is a screenwriting app. Every action is a command: find ids and \
parameters with list_commands and run them with execute (or several with batch). The screenplay's own commands \
are also tools of their own, named script_<command> (script_scene_list, script_element_action, \
script_report_characters ...). Open a script with execute file.openPath {\"path\": ...} and save it with \
execute file.save or file.saveAsPath {\"path\": ...}. Positions are {story:\"body\", path:[block], off:byteOffset}. \
Undo with execute edit.undo. render_page shows a page as it prints.";

/// The command namespaces that are tools of their own: the screenplay's.
const EXPORTED: [&str; 16] = [
    "element.",
    "scene.",
    "character.",
    "dialogue.",
    "report.",
    "beat.",
    "beats.",
    "storyboard.",
    "outline.",
    "titlePage.",
    "sceneHeading.",
    "moresContinueds.",
    "autocomplete.",
    "check.",
    "tables.",
    "strip.",
];

/// `mcp [<file>] [--read-root <folder>] [--write-root <folder>]`: serve every command to an
/// agent over standard input and output, until standard input ends.
fn mcp(args: &[String]) -> Result<(), Error> {
    let args = McpArgs::parse(args).map_err(wrong)?;
    // A root that is no folder is a wrong command line: say so before anything is opened.
    args.roots().map_err(wrong)?;
    let mut session = wordsmith_format::commands::headless_session();
    let mut instructions = INSTRUCTIONS.to_owned();
    if let Some(file) = &args.file {
        // The file is the one the person named, not the agent: it is opened, as the app opens
        // it, before the agent's folders hold. Plain text and RTF are read as scripts, as the
        // other commands here read them.
        session.run("file.openPath", &serde_json::json!({"path": file, "asScript": true})).map_err(|e| failed(e.to_string()))?;
        instructions.push_str(&format!(" The script \"{}\" is already open.", wordsmith_format::commands::script_name(&session)));
    }
    let roots = args.confine(PROGRAM).map_err(wrong)?;
    let info = jcj_agents::AgentInfo::new("wordsmith-script", wordsmith_script::PRODUCT_NAME, wordsmith_script::VERSION, &instructions);
    let mut config = jcj_agents::AgentConfig::new(info).confined(roots);
    for prefix in EXPORTED {
        config = config.export(prefix);
    }
    let mut server = jcj_agents::AgentServer::headless(session, config).tool_prefix(TOOL_PREFIX);
    server.serve_stdio().map_err(|e| failed(format!("mcp: {e}")))
}

fn run(args: &[String]) -> Result<(), Error> {
    let Some((cmd, rest)) = args.split_first() else { return Err(wrong("no command")) };
    match cmd.as_str() {
        "convert" => convert(rest),
        "export-pdf" => export_pdf(rest),
        "report" => report(rest),
        "info" => info(rest),
        "mcp" => mcp(rest),
        option if is_option(option) => Err(wrong(format!("unknown option {option}"))),
        other => Err(wrong(format!("unknown command '{other}'"))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match standard(&args) {
        Some(Standard::Help) => {
            println!("{}", usage());
            Ok(())
        }
        Some(Standard::Version) => {
            // The first line is the same in every app; what the program is follows it.
            println!("{}", version_line(PROGRAM, wordsmith_script::VERSION));
            println!("{}, native extension .{}", wordsmith_script::PRODUCT_NAME, wordsmith_format::NATIVE_EXTENSION);
            Ok(())
        }
        // Nothing was asked, so nothing was done: a caller that lost its arguments must not
        // read success.
        Some(Standard::NoCommand) => Err(wrong("no command")),
        None => run(&args),
    };
    match outcome {
        Ok(()) => Exit::Done.into(),
        Err(Error { exit, why }) => {
            eprintln!("{PROGRAM}: {why}");
            if exit == Exit::Usage {
                eprintln!("{}", usage());
            }
            exit.into()
        }
    }
}
