// SPDX-License-Identifier: MIT OR Apache-2.0
//! The gate of an app built on open-source-core: `cargo xtask ci` and every task all apps share.
//!
//! An app's `xtask/src/main.rs` holds what is the app's and nothing else: its data ([`App`]: the
//! product's name, the programs it ships, the repositories it builds on besides the core) and
//! its own tasks ([`Task`]: a name, a line of help, a function). It ends in
//! `gate::main(&APP)`. The list of shared tasks, their order and how each is run are here, so a
//! core release that adds a check to the gate, or a field to [`App`], needs no edit in an app:
//! `cargo xtask sync` brings this file, and `..App::DEFAULT` in the app fills what is new.
//!
//! The gate runs the shared checks ([`CHECKS`]), then the app's own tasks marked for the gate,
//! then clippy and the tests ([`BUILD`]): cheap checks first, so a mistake fails fast.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Change it in the core, never in an app.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::changed::{self, Affected};
use super::link::{self, Provider};
use super::util::{self, TaskResult, cargo, cargo_output, report};
use super::{applayers, assets, corepin, docs, licenses, lintroots, privacy, spdx, sync};

/// What the shared tasks need to know about one app. An app writes the fields it has and takes
/// the rest from [`App::DEFAULT`], so a field added later breaks no app.
#[derive(Clone, Copy)]
pub struct App {
    /// The product's name, for generated files.
    pub product: &'static str,
    /// The programs the app ships: the packages `THIRD-PARTY-LICENSES.txt` is written for.
    pub shipped: &'static [&'static str],
    /// The repositories the app builds on besides the core (core decision 0015). `dev-link` links
    /// one by its name, `sync` writes their addresses into `deny.toml` as the app's own sources,
    /// and `app-layers` watches where their crates are used.
    pub providers: &'static [Provider],
    /// Packages whose roots must also deny `clippy::indexing_slicing` (`lint-roots`).
    pub no_indexing: &'static [&'static str],
    /// Legal files this app carries beyond the ones every app does (`assets`).
    pub legal_files: &'static [&'static str],
    /// The package whose `screenshots` example draws the app's pictures into `docs/images/`.
    /// `None`: the app has no `screenshots` task.
    pub screenshots: Option<&'static str>,
    /// The app's own tasks.
    pub tasks: &'static [Task],
}

impl App {
    /// An app with nothing of its own. Write `..App::DEFAULT` after the fields an app sets.
    pub const DEFAULT: App =
        App { product: "This app", shipped: &[], providers: &[], no_indexing: &[], legal_files: &[], screenshots: None, tasks: &[] };
}

/// One task of an app's own.
#[derive(Clone, Copy)]
pub struct Task {
    pub name: &'static str,
    /// One line for `cargo xtask help`.
    pub about: &'static str,
    /// Whether `cargo xtask ci` runs it. It then runs after the shared checks and before clippy
    /// and the tests, so it must need no product build.
    pub in_gate: bool,
    /// The folder of the workspace, and the arguments after the task's name.
    pub run: fn(&Path, &[String]) -> TaskResult,
}

impl Task {
    /// A check the gate runs.
    pub const fn check(name: &'static str, about: &'static str, run: fn(&Path, &[String]) -> TaskResult) -> Task {
        Task { name, about, in_gate: true, run }
    }

    /// A task run by name only: a release bundle, a slow build, a generator.
    pub const fn other(name: &'static str, about: &'static str, run: fn(&Path, &[String]) -> TaskResult) -> Task {
        Task { name, about, in_gate: false, run }
    }
}

/// The shared checks, in the order the gate runs them. None needs a product build.
pub const CHECKS: &[(&str, &str)] = &[
    ("fmt", "rustfmt in check mode"),
    ("lint-roots", "every crate root forbids unsafe and denies the panicking lints"),
    ("unlinked", "no dev-link in a tracked file, and none in place (`ci --linked` runs against one)"),
    ("core-pin", "every jcj- dependency names one core release tag, and Cargo.lock holds one core"),
    ("sync", "shared files, marked blocks and the patch block match the pinned core release"),
    ("layers", "only jcj- interfaces of the core are used, never upstream crates or jcj-upstream"),
    ("app-layers", "each package uses only what the layer table in AGENTS.md puts under it"),
    ("docs", "links in Markdown lead somewhere, decision references name a record, core-candidate markers name an issue"),
    ("assets", "every file under assets/ is listed in ATTRIBUTION.md; the legal files exist"),
    ("privacy", "no personal details or secrets in files or commit identities"),
    ("licenses", "every dependency's license is on the allowlist"),
];

/// The tasks that build the product, last in the gate. CI runs these two in a job of their own.
pub const BUILD: &[(&str, &str)] =
    &[("clippy", "clippy on all targets, warnings are errors"), ("test", "all tests: by cargo-nextest when it is installed, else by `cargo test`")];

/// The shared tasks that are run by name and are no part of the gate.
const OTHER: &[(&str, &str)] = &[
    ("affected [--base <ref>]", "the packages a change affects: `all`, names, or an empty line; why, on standard error"),
    ("dev-link [<core>] [<name>=<folder>]", "build against a local checkout of the core, or of a provider; alone, it says what is linked"),
    ("dev-unlink", "build on the pinned releases again"),
    ("sync [--check]", "write (or compare) the shared files, marked blocks and patch block of the pinned core release"),
    ("core-pin [--behind] [--max-behind <n>]", "--behind asks the core for its releases (network) and says how far behind the pin is"),
    ("deny", "cargo-deny: licenses, advisories, sources"),
    ("install-hooks", "use the repository's git hooks (pre-commit and pre-push)"),
    ("third-party-licenses [file]", "write the dependency license texts (default dist/THIRD-PARTY-LICENSES.txt)"),
    ("privacy [--staged] [--range <git range>]", "the privacy check on staged lines, or on a range of commits"),
    ("prune-target <older lock file> [--target <folder>]", "remove the build output that a move from that lock file made stale (CI's cache)"),
];

/// Names an app's own task may not take: the shared tasks, and the words `main` reads itself.
fn reserved() -> Vec<&'static str> {
    let shared = CHECKS.iter().chain(BUILD).chain(OTHER).map(|(name, _)| name.split(' ').next().unwrap_or(name));
    shared.chain(["ci", "help", "screenshots"]).collect()
}

/// What is wrong with an app's own task table, or with the names of its providers.
pub fn table_problems(app: &App) -> Vec<String> {
    let reserved = reserved();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (i, provider) in app.providers.iter().enumerate() {
        if provider.name.is_empty() || provider.name == link::CORE_NAME || provider.name.contains(['=', ' ']) {
            out.push(format!("'{}' cannot be a provider's name", provider.name));
        } else if app.providers.iter().take(i).any(|p| p.name == provider.name) {
            out.push(format!("two providers are named '{}'", provider.name));
        }
    }
    for task in app.tasks {
        if task.name.is_empty() || task.name.starts_with('-') || task.name.contains(char::is_whitespace) {
            out.push(format!("'{}' cannot be a task's name", task.name));
        } else if reserved.contains(&task.name) {
            out.push(format!("'{}' is a shared task's name; an app's own task needs another", task.name));
        } else if !seen.insert(task.name) {
            out.push(format!("two tasks are named '{}'", task.name));
        }
    }
    out
}

/// The gate's tasks in order: the shared checks, the app's own, then clippy and the tests.
pub fn gate_names(app: &App) -> Vec<&'static str> {
    let own = app.tasks.iter().filter(|t| t.in_gate).map(|t| t.name);
    CHECKS.iter().map(|(name, _)| *name).chain(own).chain(BUILD.iter().map(|(name, _)| *name)).collect()
}

/// `cargo xtask <task>` for an app: call it from `main` with the app's data.
pub fn main(app: &App) -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    let rest = args.get(1..).unwrap_or_default();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root: PathBuf = manifest.parent().map_or_else(|| manifest.to_path_buf(), Path::to_path_buf);
    match run(&root, app, task, rest) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("\nxtask: FAILED: {msg}");
            ExitCode::FAILURE
        }
    }
}

/// The git addresses an app's `deny.toml` allows beyond the core's: its providers'.
fn sources(app: &App) -> Vec<String> {
    app.providers.iter().map(|p| p.git.to_owned()).collect()
}

fn run(root: &Path, app: &App, task: &str, rest: &[String]) -> TaskResult {
    let wrong = table_problems(app);
    if !wrong.is_empty() {
        return Err(format!("the app's data in xtask/src/main.rs: {}", wrong.join("; ")));
    }
    let has = |flag: &str| rest.iter().any(|a| a == flag);
    match task {
        "ci" => ci(root, app, rest),
        "help" | "--help" | "-h" => {
            help(app);
            Ok(())
        }
        "affected" => changed::task(root, rest),
        "dev-link" if has("--check") => link::check_unlinked(root),
        "dev-link" => link::link_task(root, rest, app.providers),
        "dev-unlink" => link::dev_unlink(root),
        "unlinked" if has("--tracked") => link::check_tracked(root),
        "sync" => sync::app_sync(root, has("--check"), Some(&sources(app))),
        "core-pin" => corepin::task(root, rest),
        "deny" => cargo(root, &["deny", "--locked", "check"]).map_err(|e| format!("{e} (install: cargo install --locked cargo-deny)")),
        "install-hooks" => util::git_output(root, &["config", "core.hooksPath", "hooks"]).map(|_| {
            println!("git hooks on: pre-commit runs fmt, the privacy check on staged lines and the dev-link check; pre-push runs `cargo xtask ci --changed`");
        }),
        "third-party-licenses" => licenses::task(root, app.shipped, app.product, rest.first().map(String::as_str)),
        "privacy" => privacy::task(root, rest),
        "prune-target" => changed::prune_task(root, rest),
        "screenshots" if app.screenshots.is_some() => screenshots(root, app),
        name => {
            if let Some(own) = app.tasks.iter().find(|t| t.name == name) {
                return (own.run)(root, rest).map_err(|e| format!("{name}: {e}"));
            }
            if gate_names(app).contains(&name) {
                return run_task(name, root, app, &Affected::All);
            }
            help(app);
            Err(format!("unknown task '{name}'"))
        }
    }
}

fn help(app: &App) {
    println!("usage: cargo xtask <task>\n");
    println!("  ci [--changed] [--base <ref>] [--skip <tasks>] [--only <tasks>] [--linked]");
    println!("      the gate: every check below, in this order. --changed limits clippy and the tests to the");
    println!("      packages changed since --base (default: origin/main) and their dependents; --skip a,b leaves");
    println!("      tasks out and --only a,b runs those alone; --linked runs it against a dev-link\n");
    let own: Vec<(&str, &str)> = app.tasks.iter().filter(|t| t.in_gate).map(|t| (t.name, t.about)).collect();
    for (name, about) in CHECKS.iter().chain(&own).chain(BUILD) {
        println!("  {name:<12} {about}");
    }
    println!("\nother tasks:");
    for (name, about) in OTHER {
        println!("  {name}\n      {about}");
    }
    if let Some(package) = app.screenshots {
        println!("  screenshots\n      redraw docs/images/ with the app's own offscreen renderer (the `screenshots` example of {package})");
    }
    for task in app.tasks.iter().filter(|t| !t.in_gate) {
        println!("  {}\n      {}", task.name, task.about);
    }
}

/// What a `ci` run was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// The tasks to leave out.
    pub skip: Vec<String>,
    /// `--changed`: only the packages a change affects, since this ref.
    pub base: Option<String>,
    /// `--linked`: run against a dev-link on purpose.
    pub linked: bool,
}

/// Read the arguments of `ci`: `--changed`, `--base <ref>`, `--linked`, and `--skip`/`--only`
/// with the names in `tasks`. `default_base` is asked only when `--changed` names no base.
pub fn asked(args: &[String], tasks: &[&str], default_base: &dyn Fn() -> String) -> Result<Asked, String> {
    let mut rest = Vec::new();
    let (mut changed, mut base, mut linked) = (false, None, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--changed" => changed = true,
            "--linked" => linked = true,
            "--base" => base = Some(it.next().ok_or("--base needs a git ref")?.clone()),
            _ => rest.push(a.clone()),
        }
    }
    let skip = util::skipped(&rest, tasks)?;
    let base = changed.then(|| base.unwrap_or_else(default_base));
    Ok(Asked { skip, base, linked })
}

/// The packages a `ci` run covers, said on the way.
pub fn scope(root: &Path, asked: &Asked) -> Result<Affected, String> {
    let Some(base) = &asked.base else { return Ok(Affected::All) };
    let affected = changed::affected(root, base)?;
    match &affected {
        Affected::All => println!("changes since {base} affect every package"),
        Affected::Some(p) if p.is_empty() => println!("changes since {base} affect no package"),
        Affected::Some(p) => println!("changes since {base} affect: {}", p.join(", ")),
    }
    Ok(affected)
}

fn ci(root: &Path, app: &App, args: &[String]) -> TaskResult {
    let names = gate_names(app);
    let asked = asked(args, &names, &|| changed::base_ref(root, &[]))?;
    util::warn_if_hooks_off(root, "hooks");
    let links = link::links(root);
    if asked.linked && links.is_empty() {
        println!("note: --linked, and nothing is linked: this run builds the pinned releases");
    }
    let against_a_link = asked.linked && !links.is_empty();
    if against_a_link {
        println!("\nlinked ({}): this run checks the app against those checkouts. CI builds the pinned releases.", link::describe(&links));
    }
    let scope = scope(root, &asked)?;
    for name in &names {
        if asked.skip.iter().any(|s| s == name) {
            println!("\n==> {name} (skipped)");
        } else if *name == "unlinked" && against_a_link {
            println!("\n==> {name}");
            link::check_tracked(root).map_err(|e| format!("{name}: {e}"))?;
        } else {
            run_task(name, root, app, &scope)?;
        }
    }
    if against_a_link {
        println!("\nxtask: all checks passed against the linked checkouts ({}), not the pinned releases", link::describe(&links));
    } else {
        println!("\nxtask: all checks passed");
    }
    Ok(())
}

fn run_task(name: &str, root: &Path, app: &App, scope: &Affected) -> TaskResult {
    println!("\n==> {name}");
    let result = match name {
        "fmt" => cargo(root, &["fmt", "--all", "--", "--check"]),
        "lint-roots" => lintroots::check_lint_roots(root, app.no_indexing, &[]),
        "unlinked" => link::check_unlinked(root),
        "core-pin" => corepin::task(root, &[]),
        "sync" => sync::app_sync(root, true, Some(&sources(app))),
        "layers" => applayers::upstream_free(root),
        "app-layers" => applayers::task(root, app.providers),
        "docs" => docs::app_task(root),
        "assets" => assets::task(root, app.legal_files),
        "privacy" => privacy::task(root, &[]),
        "licenses" => check_licenses(root),
        "clippy" => scoped(root, scope, &["clippy", "--locked", "--all-targets"], &["--", "-D", "warnings"]),
        "test" => test(root, scope, &[]),
        own => match app.tasks.iter().find(|t| t.name == own) {
            Some(task) => (task.run)(root, &[]),
            None => Err(format!("no runner for task '{own}'")),
        },
    };
    result.map_err(|e| format!("{name}: {e}"))
}

/// Run a cargo command over the affected packages, or not at all when none are.
pub fn scoped(root: &Path, scope: &Affected, head: &[&str], tail: &[&str]) -> TaskResult {
    if scope.is_empty() {
        println!("nothing to do: no package affected");
        return Ok(());
    }
    let pkgs = scope.cargo_args();
    let mut args: Vec<&str> = head.to_vec();
    args.extend(pkgs.iter().map(String::as_str));
    args.extend(tail);
    cargo(root, &args)
}

/// The first line `cargo nextest --version` prints, when cargo-nextest is installed.
pub fn nextest(root: &Path) -> Option<String> {
    cargo_output(root, &["nextest", "--version"]).ok().and_then(|out| out.lines().next().map(|line| line.trim().to_owned()))
}

/// The cargo commands that run the tests, each as its arguments before the packages. With
/// cargo-nextest: the tests, then the tests in documentation, which it does not run. Without
/// it, `cargo test` runs both. `flags` are added to each (`--all-features`).
pub fn test_commands(with_nextest: bool, flags: &[&str]) -> Vec<Vec<String>> {
    let command = |head: &[&str]| head.iter().chain(flags).map(|s| (*s).to_owned()).collect::<Vec<String>>();
    if with_nextest {
        vec![command(&["nextest", "run", "--locked", "--no-tests=pass"]), command(&["test", "--locked", "--doc"])]
    } else {
        vec![command(&["test", "--locked"])]
    }
}

/// The `test` task: every test of the packages in `scope`. cargo-nextest runs them when it is
/// installed: each test in a process of its own, all test programs at once, where `cargo test`
/// runs one program after another. Both run the same tests, and the task says which ran, so a
/// result here and one in CI can be told apart when they differ.
pub fn test(root: &Path, scope: &Affected, flags: &[&str]) -> TaskResult {
    if scope.is_empty() {
        println!("nothing to do: no package affected");
        return Ok(());
    }
    let version = nextest(root);
    match &version {
        Some(version) => println!("tests run by {version}; the tests in documentation by `cargo test --doc`"),
        None => println!(
            "tests run by `cargo test`: cargo-nextest is not installed. CI uses it, and it is faster \
             (https://nexte.st/docs/installation/pre-built-binaries/)"
        ),
    }
    for command in test_commands(version.is_some(), flags) {
        let head: Vec<&str> = command.iter().map(String::as_str).collect();
        // `cargo test --doc` fails for a package with no library, so it is asked only for the others.
        let scope = if head.contains(&"--doc") { changed::doc_test_scope(root, scope)? } else { scope.clone() };
        if scope.is_empty() {
            println!("no package here has tests in its documentation");
            continue;
        }
        scoped(root, &scope, &head, &[])?;
    }
    Ok(())
}

/// The lines of `cargo tree --format "{p}\t{l}"` whose license is not on the allowlist (core
/// decision 0001), each package once.
pub fn license_problems(tree: &str) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut checked = BTreeSet::new();
    for line in tree.lines().filter(|l| !l.trim().is_empty()) {
        let line = line.trim().trim_end_matches("(*)").trim_end();
        let Some((pkg, license)) = line.split_once('\t') else { continue };
        if !checked.insert(pkg.to_owned()) {
            continue;
        }
        match spdx::is_allowed(license.trim()) {
            Ok(true) => {}
            Ok(false) => failures.push(format!("{pkg}: license '{license}' is not on the allowlist")),
            Err(e) => failures.push(format!("{pkg}: {e}")),
        }
    }
    (failures, checked.len())
}

fn check_licenses(root: &Path) -> TaskResult {
    let tree = cargo_output(
        root,
        &["tree", "--workspace", "--locked", "--target", "all", "-e", "normal,build,dev", "--prefix", "none", "--format", "{p}\t{l}"],
    )?;
    let (failures, checked) = license_problems(&tree);
    report(failures, &format!("the licenses of {checked} packages are on the allowlist"))
}

/// `cargo xtask screenshots`: the app draws its own pictures, offscreen, into `docs/images/`.
/// Never a screenshot of a real desktop. Not part of the gate.
fn screenshots(root: &Path, app: &App) -> TaskResult {
    let Some(package) = app.screenshots else { return Err("this app has no screenshots task".to_owned()) };
    let out = root.join("docs/images");
    let out = out.to_str().ok_or_else(|| format!("{} is not UTF-8", out.display()))?;
    cargo(root, &["run", "--release", "--locked", "-p", package, "--example", "screenshots", "--", out])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing(_: &Path, _: &[String]) -> TaskResult {
        Ok(())
    }

    const OWN: &[Task] = &[Task::check("names", "an app's own check", nothing), Task::other("bundle", "a release bundle", nothing)];

    #[test]
    fn the_gate_is_the_shared_checks_then_the_apps_own_then_the_build() {
        let app = App { tasks: OWN, ..App::DEFAULT };
        let names = gate_names(&app);
        assert_eq!(names.first(), Some(&"fmt"));
        assert_eq!(names[names.len() - 3..], ["names", "clippy", "test"]);
        // A task that is run by name only is no part of the gate.
        assert!(!names.contains(&"bundle") && !names.contains(&"ci"));
        let unique: BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len());
        assert!(table_problems(&app).is_empty());
        // An app with no task of its own has the shared gate and nothing else.
        assert_eq!(gate_names(&App::DEFAULT).len(), CHECKS.len() + BUILD.len());
    }

    #[test]
    fn every_shared_task_of_the_gate_has_a_runner() {
        // Run against a folder that does not exist: each task fails on its own, and none for want of a runner.
        let root = Path::new("/nonexistent/app");
        for (name, _) in CHECKS.iter().chain(BUILD) {
            if matches!(*name, "fmt" | "clippy" | "test") {
                continue;
            }
            let outcome = run_task(name, root, &App::DEFAULT, &Affected::All);
            assert!(!outcome.is_err_and(|e| e.contains("no runner for task")), "{name}");
        }
        assert!(run_task("nothing", root, &App::DEFAULT, &Affected::All).unwrap_err().contains("no runner for task"));
        let app = App { tasks: OWN, ..App::DEFAULT };
        assert!(run_task("names", root, &app, &Affected::All).is_ok());
    }

    #[test]
    fn an_apps_task_may_not_take_a_shared_name() {
        for name in ["fmt", "clippy", "ci", "sync", "dev-link", "affected", "screenshots", "privacy", "third-party-licenses", "prune-target"] {
            let tasks = [Task::other(name, "", nothing)];
            let found = table_problems(&App { tasks: Box::leak(Box::new(tasks)), ..App::DEFAULT });
            assert!(found.iter().any(|p| p.contains("shared task's name")), "{name}: {found:?}");
        }
        let twice = [Task::check("names", "", nothing), Task::other("names", "", nothing), Task::other("two words", "", nothing)];
        let found = table_problems(&App { tasks: Box::leak(Box::new(twice)), ..App::DEFAULT });
        assert_eq!(found, ["two tasks are named 'names'", "'two words' cannot be a task's name"]);
        // A provider is linked by its name, so the name is one word, not the core's, and its own.
        let other = |name: &'static str| Provider { name, git: "https://example.invalid/other-app" };
        let providers = [other("other"), other("core"), other("other"), other("a=b")];
        let found = table_problems(&App { providers: Box::leak(Box::new(providers)), ..App::DEFAULT });
        assert_eq!(found, ["'core' cannot be a provider's name", "two providers are named 'other'", "'a=b' cannot be a provider's name"]);
    }

    #[test]
    fn reads_what_a_ci_run_is_asked_for() {
        let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let tasks = ["fmt", "names", "clippy", "test"];
        let main = || "origin/main".to_owned();
        assert_eq!(asked(&args(&[]), &tasks, &main), Ok(Asked { skip: vec![], base: None, linked: false }));
        assert_eq!(asked(&args(&["--changed"]), &tasks, &main).unwrap().base.as_deref(), Some("origin/main"));
        let both = asked(&args(&["--changed", "--base", "abc", "--only", "clippy,test", "--linked"]), &tasks, &main).unwrap();
        assert_eq!(both, Asked { skip: vec!["fmt".to_owned(), "names".to_owned()], base: Some("abc".to_owned()), linked: true });
        // A base without --changed limits nothing, as before.
        assert_eq!(asked(&args(&["--base", "abc"]), &tasks, &main).unwrap().base, None);
        assert!(asked(&args(&["--base"]), &tasks, &main).is_err());
        assert!(asked(&args(&["--skip", "nothing"]), &tasks, &main).is_err());
        assert!(asked(&args(&["--fast"]), &tasks, &main).is_err());
    }

    #[test]
    fn the_tests_run_the_same_with_and_without_nextest() {
        let words = |c: &[Vec<String>]| c.iter().map(|c| c.join(" ")).collect::<Vec<_>>();
        assert_eq!(words(&test_commands(false, &[])), ["test --locked"]);
        // cargo-nextest does not run the tests in documentation: a second command does.
        assert_eq!(words(&test_commands(true, &[])), ["nextest run --locked --no-tests=pass", "test --locked --doc"]);
        assert_eq!(
            words(&test_commands(true, &["--all-features"])),
            ["nextest run --locked --no-tests=pass --all-features", "test --locked --doc --all-features"]
        );
        assert_eq!(words(&test_commands(false, &["--all-features"])), ["test --locked --all-features"]);
    }

    #[test]
    fn licenses_are_checked_once_per_package() {
        let tree = "app v1.0.0 (/repo/app)\tMIT OR Apache-2.0\nserde v1.0.0\tMIT OR Apache-2.0\nserde v1.0.0\tMIT OR Apache-2.0 (*)\n\
                    odd v0.1.0\tGPL-3.0-only\n\nno tab here\n";
        let (failures, checked) = license_problems(tree);
        assert_eq!(checked, 3);
        assert_eq!(failures, ["odd v0.1.0: license 'GPL-3.0-only' is not on the allowlist"]);
    }
}
