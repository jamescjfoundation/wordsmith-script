# Progress

One line per piece of work landed on `main` since the app was rebuilt on
open-source-core, newest last. A change adds its row here; what a writer will
notice also goes in `CHANGELOG.md` and the guide. "Tests" is the number the
gate ran when the row was written, where it was recorded.

The log of the first build (2026-11-19 to 2026-11-22) is history:
`docs/history/progress-first-build.md`.

| Date | Step | What changed | Tests |
| --- | --- | --- | --- |
| 2026-11-24 | rebuild/template | The repository starts from open-source-core's template (core v0.1.0); the first build's own screenplay code is brought in as it stood at 0.1.2, without its copies of WordCraft's crates | |
| 2026-11-24 | rebuild/port | The screenplay rules and the file formats ported onto the core's `jcj-` crates, then the interface onto `jcj-shell`; `.wss` format 5; Script's own xtask tasks and the release workflow; decision 0044; the golden files and every page plan 0.1.2 made unchanged | |
| 2026-11-24 | core/v0.1.1 | Built on core v0.1.1 | |
| 2026-11-24 | brand/kit | The brand kit's logos, icons and palette in `assets/brand/` | |
| 2026-11-26 | core/v0.1.16 | Core v0.1.16; `cargo xtask core-pin` joins the gate | |
| 2026-11-26 | core/v0.1.17 | Core v0.1.17: the core's rules are marked blocks written by `cargo xtask sync` | |
| 2026-11-26 | rules/own | Script's own rules rewritten outside the blocks: the layer table, the brand rule, naming, the `.wss` rule that names Wordsmith Manuscript's reader | |
| 2026-11-26 | core/v0.1.18 | Core v0.1.18: `cargo xtask docs` joins the gate | |
| 2026-11-26 | shell/tabs | The core's tabs and split view; a status on every decision record; `docs/core-candidates.md` | |
| 2026-11-26 | core/v0.1.19 | Core v0.1.19: CI is the core's shared workflow | |
| 2026-11-26 | core/v0.1.20 | Core v0.1.20: the gate is the core's, and `xtask/src/main.rs` holds only Script's data and its own tasks; the layer table in `AGENTS.md` is what `app-layers` reads | |
| 2026-11-26 | cli/convention | The command line on the core's convention (exit codes, `--version`, `mcp <file>`, tools named `script_<command>`) | |
| 2026-11-26 | shell/keys-and-dialogs | The core's rule for shifted caret keys and the core's file dialogs; Script's own copies and its `rfd` dependency deleted | |
| 2026-11-26 | names/programs | Crates, programs and the app id named for Script (decision 0045): `script-model`, `script-format`, `script-ui`, `wordsmith-script`, `wordsmith-script-cli`, `org.jcjfoundation.script` | 426 |
| 2026-11-26 | shape/tests-and-guide | Fixtures and golden files in `tests/` at the root with `docs/fixtures.md`, one `UPDATE_GOLDEN` switch, the guide's `keys.md` and `command-line-and-agents.md`, rules for the writer's words, golden files and fixtures (decision 0046); no fixture or golden file changed | 426 |
| 2026-11-26 | version/0.9.0 | The version is 0.9.0, the first that will be published; the changelog has `## Unreleased` first and marks 0.1.0 to 0.1.2 as never released; the records say the first build's history was not kept (decision 0047) | 426 |
| 2026-11-26 | plan/issues | The plan is the `v1` milestone: 13 issues for 1.0.0 and 7 for later made from `docs/gaps.md`, the roadmap and the SPEC-2 questions; `docs/gaps.md` keeps standing limits only; `ROADMAP.md` links to the milestone; the first build's log moved to `docs/history/` | 426 |
| 2026-11-27 | core/v0.1.22 | Core v0.1.22: the release packages and the release workflow are the core's (core decision 0019). Package data in `xtask/src/main.rs`; `xtask/src/bundle.rs` and Script's own release workflow deleted; the desktop program finishes an update as the updater's helper; the guide's Installing and updating page | |
| 2026-11-27 | release/0.9.0 | The release commit of 0.9.0, the first public release: `## Unreleased` of the changelog becomes `## 0.9.0 (2026-11-27)`, with what the preview is for at its head | |
| 2026-11-27 | tests/macos | The first gate on macOS (the release's) failed in Script's tests: they made a custom element with a key macOS gives to Spelling, and compared the guide's keys page, whose keys are those of Windows and Linux. The tests use a key free on every system, the page says whose keys it holds and is not compared on macOS, and cargo-nextest runs every test after a failure (`.config/nextest.toml`) | |
