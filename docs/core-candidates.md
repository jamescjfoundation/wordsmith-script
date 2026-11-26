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
same change. Last checked against the issues and core v0.1.18 on 2026-11-26.

## Markers in the code

| What | `// core-candidate:` location | Core issue | State |
| --- | --- | --- | --- |
| File dialogs in the shell (open, save, choose a folder) | `apps/wordsmith/src/files.rs` | [#28](https://github.com/jamescjfoundation/open-source-core/issues/28) | app-first. Script shows the system's dialogs itself, modally, through the `rfd` crate |
| A shifted caret key extends the selection | `crates/wordsmith-ui/src/editor.rs` (`shifted`) | [#29](https://github.com/jamescjfoundation/open-source-core/issues/29) | app-first |
| Keys and typed text of one frame run in the order typed | `crates/wordsmith-ui/src/editor.rs` (`Editor::handle_keys`) | [#39](https://github.com/jamescjfoundation/open-source-core/issues/39) | app-first. The editor takes every key itself, with the shell's `shortcuts` option off |
| Pagination pass: room for a foot line, a run's page count, a forced start inside a paragraph, placeholder pages | `crates/wordsmith-script/src/paginate/plan.rs` | [#34](https://github.com/jamescjfoundation/open-source-core/issues/34) | app-first. Script's planner does all four itself; its screenplay rules stay here as a pass once the core has the hooks (decision 0044) |
| Story board lanes | `crates/wordsmith-script/src/storyboard.rs` (`Lane`; the commands are in `outline.rs`) | [#12](https://github.com/jamescjfoundation/open-source-core/issues/12) | app-first. A second app has lanes of its own, marked for the same issue |
| Template tiles with a preview | `crates/wordsmith-ui/src/backstage.rs` (`template_card`) | [#40](https://github.com/jamescjfoundation/open-source-core/issues/40) | copy kept. The core's chooser ([#2](https://github.com/jamescjfoundation/open-source-core/issues/2), core v0.1.2) draws a tile with a name and a line of text; Script's cards show each template's first lines |
| Writing statistics: pages and writing time per day, counting that only goes up, goals, the statistics window | `crates/wordsmith-ui/src/workspace.rs` | [#41](https://github.com/jamescjfoundation/open-source-core/issues/41) | copy kept. The core's module ([#3](https://github.com/jamescjfoundation/open-source-core/issues/3), core v0.1.2) keeps net words per day, a session target and a sprint; moving onto it would change what writers see and the shape of `stats.json` |

## Core changes with no code here

| What | Core issue | State |
| --- | --- | --- |
| Document tabs and a split view (`jcj_shell::split`) | [#4](https://github.com/jamescjfoundation/open-source-core/issues/4) | released in core v0.1.3, used: the tab strip and the two panes with their divider (`crates/wordsmith-ui/src/workspace.rs`). Which scripts are open, the new-tab button and what the second pane shows are Script's |
| The start window (`jcj_shell::start::start_window`) | [#2](https://github.com/jamescjfoundation/open-source-core/issues/2) | released in core v0.1.2, not used: a script is always open, so Script shows the File page's New page at start instead |
| The PDF names the app that made it | [#5](https://github.com/jamescjfoundation/open-source-core/issues/5) | open. PDFs name WordCraft as creator and producer; 0.1 wrote "Wordsmith Script" |
| A shell that builds for the web | [#35](https://github.com/jamescjfoundation/open-source-core/issues/35) | open. `apps/wordsmith-web` builds for the desktop only and `cargo xtask wasm` fails |
| The updater's finishing step on Windows | [#36](https://github.com/jamescjfoundation/open-source-core/issues/36) | open. An update cannot finish in place on Windows; the download is offered |
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
