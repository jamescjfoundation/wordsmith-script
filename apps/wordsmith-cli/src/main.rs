// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Script on the command line (decision 0029).
//!
//! ```text
//! wordsmith-cli convert <input> <output>
//! wordsmith-cli export-pdf <input> <output.pdf> [--revised-only] [--no-marks]
//! wordsmith-cli report characters|scenes <input> [<output.csv>]
//! wordsmith-cli info <input>
//! wordsmith-cli mcp [--read-root <folder>] [--write-root <folder>]
//! ```
//!
//! Inputs are `.wss` and Fountain scripts; plain text, RTF and PDF are imported as scripts.
//! Outputs are written in the format their extension names. Exit codes: 0 done, 1 failed, 2 a
//! usage error.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jcj_doc::Document;
use wordsmith_format::files::{self, Kind};

const USAGE: &str = "usage:
  wordsmith-cli convert <input> <output>
  wordsmith-cli export-pdf <input> <output.pdf> [--revised-only] [--no-marks]
  wordsmith-cli report characters|scenes <input> [<output.csv>]
  wordsmith-cli info <input>
  wordsmith-cli mcp [--read-root <folder>] [--write-root <folder>]";

/// What went wrong: a usage error (exit 2) or a failure (exit 1).
enum Error {
    Usage(String),
    Failed(String),
}

fn failed(e: impl Into<String>) -> Error {
    Error::Failed(e.into())
}

/// Read a script: open a native or Fountain file, import anything else as a script.
fn load(input: &Path) -> Result<Document, Error> {
    match Kind::of(input) {
        Some(Kind::Native | Kind::Fountain) => files::open(input).map_err(failed),
        _ => files::import_file(input, true).map_err(failed),
    }
}

fn convert(args: &[String]) -> Result<(), Error> {
    let [input, output] = args else { return Err(Error::Usage("convert takes an input and an output".into())) };
    let doc = load(Path::new(input))?;
    let kind =
        Kind::of(Path::new(output)).ok_or_else(|| Error::Usage(format!("{output}: unknown output format (wss, fountain, txt, rtf, html, pdf)")))?;
    files::export_to(Path::new(output), &doc, kind).map_err(failed)
}

fn export_pdf(args: &[String]) -> Result<(), Error> {
    let mut opts = wordsmith_script::export::PrintOptions::default();
    let mut paths = Vec::new();
    for a in args {
        match a.as_str() {
            "--revised-only" => opts.revised_only = true,
            "--no-marks" => opts.marks = false,
            flag if flag.starts_with("--") => return Err(Error::Usage(format!("unknown option {flag}"))),
            p => paths.push(p),
        }
    }
    let [input, output] = paths[..] else { return Err(Error::Usage("export-pdf takes an input and an output".into())) };
    let doc = load(Path::new(input))?;
    let (bytes, pages) = wordsmith_script::export::pdf_with(&doc, opts).map_err(failed)?;
    files::write_atomic(Path::new(output), &bytes).map_err(failed)?;
    eprintln!("{output}: {pages} page{}", if pages == 1 { "" } else { "s" });
    Ok(())
}

fn report(args: &[String]) -> Result<(), Error> {
    let (which, input, output) = match args {
        [w, i] => (w, i, None),
        [w, i, o] => (w, i, Some(o)),
        _ => return Err(Error::Usage("report takes characters or scenes, an input, and an optional output".into())),
    };
    if !matches!(which.as_str(), "characters" | "scenes") {
        return Err(Error::Usage(format!("unknown report {which} (characters, scenes)")));
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
    let [input] = args else { return Err(Error::Usage("info takes an input".into())) };
    let doc = load(Path::new(input))?;
    let pag = wordsmith_script::paginate::paginate(&doc);
    let scenes = wordsmith_script::scenes(&doc).len();
    let characters = wordsmith_script::commands::character::characters(&doc).len();
    let title = if doc.core.title.is_empty() { "(untitled)" } else { doc.core.title.as_str() };
    println!("title: {title}\npages: {}\nscenes: {scenes}\ncharacters: {characters}\nparagraphs: {}", pag.pages.len(), doc.body.len());
    Ok(())
}

/// What the agent server tells a client about the app.
const INSTRUCTIONS: &str = "Wordsmith Script is a screenwriting app. Every action is a command: find ids and \
parameters with list_commands and run them with execute (or several with batch); the screenplay's own \
commands are also tools of their own (scene_list, element_action, report_characters ...). Positions are \
{story:\"body\", path:[block], off:byteOffset}. Undo with execute edit.undo. render_page shows a page as it prints.";

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

fn mcp(args: &[String]) -> Result<(), Error> {
    let (mut read, mut write): (Option<PathBuf>, Option<PathBuf>) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let value = || it.clone().next().cloned().ok_or_else(|| Error::Usage(format!("{a} needs a folder")));
        match a.as_str() {
            "--read-root" => read = Some(PathBuf::from(value()?)),
            "--write-root" => write = Some(PathBuf::from(value()?)),
            other => return Err(Error::Usage(format!("unknown option {other}"))),
        }
        it.next();
    }
    let roots = jcj_engine::file_access::AutomationRoots::new(read.as_deref(), write.as_deref()).map_err(Error::Usage)?;
    match &roots {
        Some(r) => {
            jcj_engine::file_access::confine_process(r.clone());
        }
        // Standard error: standard output carries the protocol.
        None => eprintln!("wordsmith-cli mcp: no --read-root or --write-root given, so commands can read and write any file you can"),
    }
    let session = wordsmith_format::commands::headless_session();
    let info = jcj_agents::AgentInfo::new("wordsmith-script", wordsmith_script::PRODUCT_NAME, wordsmith_script::VERSION, INSTRUCTIONS);
    let mut config = jcj_agents::AgentConfig::new(info).confined(roots);
    for prefix in EXPORTED {
        config = config.export(prefix);
    }
    let mut server = jcj_agents::AgentServer::headless(session, config);
    let stdin = std::io::stdin();
    server.serve(stdin.lock(), std::io::stdout()).map_err(|e| failed(e.to_string()))
}

fn run(args: &[String]) -> Result<(), Error> {
    let Some((cmd, rest)) = args.split_first() else { return Err(Error::Usage("no command".into())) };
    match cmd.as_str() {
        "convert" => convert(rest),
        "export-pdf" => export_pdf(rest),
        "report" => report(rest),
        "info" => info(rest),
        "mcp" => mcp(rest),
        other => Err(Error::Usage(format!("unknown command '{other}'"))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some("--version" | "-V") => {
            println!(
                "wordsmith-cli {} ({}, native extension .{})",
                wordsmith_script::VERSION,
                wordsmith_script::PRODUCT_NAME,
                wordsmith_format::NATIVE_EXTENSION
            );
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Usage(e)) => {
            eprintln!("wordsmith-cli: {e}\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Error::Failed(e)) => {
            eprintln!("wordsmith-cli: {e}");
            ExitCode::FAILURE
        }
    }
}
