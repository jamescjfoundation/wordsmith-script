# Core changes and core-candidates

Every change Wordsmith Script waits for in open-source-core, and every
`// core-candidate:` marker in this repository, with its core issue and its
state. Issue numbers are open-source-core's.

States:

- **app-first**: built here, marked `// core-candidate: open-source-core#N`,
  and general enough for the core. Nothing in the core does this yet.
- **copy kept**: the core has released its version, but this repository still
  holds its own, for the reason given, marked with the open issue that would
  free it.
- **released in core vX, used**: the core has it and Script uses the core's.
- **open**: the core's own work; nothing here is a prototype of it.

The rows, the markers in the code and the core's issues are kept in step. Each
app-first and copy-kept row's issue is open, labelled `core-candidate`, and
names the file here that holds the code. A marker is written
`core-candidate: open-source-core#N`, and `cargo xtask docs` fails on one that
names no issue. A new marker, or a new need, gets its issue and its row in the
same change. Last checked against the issues and core v0.1.24 on 2026-11-27.
Which of these 1.0.0 needs is decided in Script's own plan:
[issue 9](https://github.com/jamescjfoundation/wordsmith-script/issues/9).

## Markers in the code

| What | `// core-candidate:` location | Core issue | State |
| --- | --- | --- | --- |
| Keys and typed text of one frame run in the order typed | `crates/script-ui/src/editor.rs` (`Editor::handle_keys`) | [#39](https://github.com/jamescjfoundation/open-source-core/issues/39) | app-first. The editor takes every key itself, with the shell's `shortcuts` option off |
| Pagination pass: room for a foot line, a run's page count, a forced start inside a paragraph, placeholder pages | `crates/script-model/src/paginate/plan.rs` | [#34](https://github.com/jamescjfoundation/open-source-core/issues/34) | app-first. Script's planner does all four itself; its screenplay rules stay here as a pass once the core has the hooks (decision 0044) |
| Story board lanes | `crates/script-model/src/storyboard.rs` (`Lane`; the commands are in `outline.rs`) | [#12](https://github.com/jamescjfoundation/open-source-core/issues/12) | app-first. A second app has lanes of its own, marked for the same issue |
| A check for files an app carries from elsewhere, against a record with checksums (the brand kit's files in `assets/brand/`) | `xtask/src/brand.rs` | [#47](https://github.com/jamescjfoundation/open-source-core/issues/47) | app-first: `cargo xtask brand`, in the gate since 2026-11-27. Wordsmith Manuscript and Wordsmith Book hold the same check for their own folders |

## Core changes with no code here

| What | Core issue | State |
| --- | --- | --- |
| File dialogs in the shell (`Shell::ask_file`, `jcj_shell::files`) | [#28](https://github.com/jamescjfoundation/open-source-core/issues/28) | released in core v0.1.20, used: the editor asks the shell for each dialog and the window shows them (`Editor::flush`, `apps/script/src/main.rs`). Script's own dialog code and its `rfd` dependency are gone. Which tab a dialog is for, and the options and the waiting step its answer needs, are Script's (`editor::FileAsk`) |
| A shifted caret key extends the selection (`KeyMap::command_for`) | [#29](https://github.com/jamescjfoundation/open-source-core/issues/29) | released in core v0.1.20, used: the editor asks the key map what a key runs (`Editor::handle_keys`). Script's own `shifted` is gone |
| A file dialog that cannot be shown gives up the step that waited on it | [#43](https://github.com/jamescjfoundation/open-source-core/issues/43) | released in core v0.1.24, used with no change here: the shell says why in the status bar and runs the command Script gave for a dialog closed without choosing (`FILE_CLOSED`), so the editor drops what the ask carried, as when the writer cancels |
| An app's start-up and its brand files (`jcj_shell::desktop`, `jcj_theme::brand`) | [#46](https://github.com/jamescjfoundation/open-source-core/issues/46) | released in core v0.1.21, used since core v0.1.24: `apps/script/src/main.rs` is Script's `Desktop` data and `--version`; what only Script does (its shell, crash recovery, the personal dictionary, the statistics, the scripts to open, the question before closing) is its `DesktopApp` in `crates/script-ui/src/desktop.rs`. `brand.rs` is a `Brand` and two `Mask`s; its colors and the marks' geometry stay. Script has no `build.rs`, so its Windows program carries no icon or name of its own; the template's `build.rs` would add them, a change of its own. The folder-project layer (`jcj_project::folder`) does not apply: a script is one `.wss` file |
| Template tiles with a preview (`TemplateSpec::with_preview`) | [#40](https://github.com/jamescjfoundation/open-source-core/issues/40) | released in core v0.1.24, used: Script lists its templates with their first lines in a fixed-width font through `App::templates` (`crates/script-ui/src/backstage.rs`, `templates`), and the shell's chooser draws them on the New page. Script's own cards are gone |
| Writing statistics (`jcj_shell::writing::stats`) | [#41](https://github.com/jamescjfoundation/open-source-core/issues/41) | released in core v0.1.24, used: the days, goals, counting and the statistics window are the core's `Statistics`, `Tracker` and `statistics_window` with `Unit::Pages`, in the same `stats.json`, read as 0.9.0 wrote it (`crates/script-ui/tests/stats_file.rs`). The sprint is the core's `Writing` with `Counting::UpOnly` and its pages. Script's own are what it counts (`workspace::track`), its starting goals, its UTC days and the sprint's window |
| Document tabs and a split view (`jcj_shell::split`) | [#4](https://github.com/jamescjfoundation/open-source-core/issues/4) | released in core v0.1.3, used: the tab strip and the two panes with their divider (`crates/script-ui/src/workspace.rs`). Which scripts are open, the new-tab button and what the second pane shows are Script's |
| The start window (`jcj_shell::start::start_window`) | [#2](https://github.com/jamescjfoundation/open-source-core/issues/2) | released in core v0.1.2, not used: a script is always open, so Script shows the File page's New page at start instead |
| The PDF names the app that made it | [#5](https://github.com/jamescjfoundation/open-source-core/issues/5) | open. PDFs name WordCraft as creator and producer; 0.1 wrote "Wordsmith Script" |
| A shell that builds for the web | [#35](https://github.com/jamescjfoundation/open-source-core/issues/35) | open. `apps/script-web` builds for the desktop only and `cargo xtask wasm` fails |
| The updater's finishing step on Windows | [#36](https://github.com/jamescjfoundation/open-source-core/issues/36) | released in core v0.1.21, used: the core's start-up makes the call first in `main` (`DESKTOP.finish_update`, `apps/script/src/main.rs`), before `--version` is read, so a copy started as the updater's helper finishes the update. An unsigned copy is still offered the download: the updater replaces only a signed one |
| On macOS, Option with Left or Right moves by word | [#48](https://github.com/jamescjfoundation/open-source-core/issues/48) | released in core v0.1.24, used: the shell keeps Option+Arrow as `Alt+Arrow`, which the engine binds to moving by word. Script's test (`shift_extends_caret_movement_and_shift_backspace_deletes`) presses Option+Shift+Left on macOS |
| Engine commands in groups an app can choose from | [#37](https://github.com/jamescjfoundation/open-source-core/issues/37) | open. Every registry holds the engine's whole catalog; agents see it through `list_commands` |
| An app's own icons on the shell's controls | [#38](https://github.com/jamescjfoundation/open-source-core/issues/38) | open. The ribbon, the rails and the File page show the shell's general icons; Script's own icons show in its panels and status bar |

## Kept here on purpose

Not waiting for the core:

- **The text markers in `.wss`** (`**`, `*`, `_` and the note, revision, lock
  and style marks) are read and written by Script's own codec, not by
  `jcj_interchange::emphasis`: the format's markers are one grammar
  (`docs/format.md`).
- **The story board's data** (beats, links, several boards) is Script's. Only
  the lanes are general ([#12](https://github.com/jamescjfoundation/open-source-core/issues/12)).
- **The word count** in the status bar and the statistics is Script's own
  loop. The core's word rule (`jcj_features::verify::words`) is the same rule
  but builds a list of every word, and the count runs after each edit.
