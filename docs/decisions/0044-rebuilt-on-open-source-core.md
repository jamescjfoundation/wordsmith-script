# 0044: Rebuilt on open-source-core

Date: 2026-11-24 · Status: accepted

## Context

Up to 0.1.2 Script carried its own copy of WordCraft: about 83,000 lines of
model, layout, fonts, rendering, PDF, engine and agent-server code, trimmed and
changed for screenplays (decisions 0005 to 0008 and 0039). The James C. J.
Foundation's apps now share one base, open-source-core, which uses WordCraft
and craft-libs through thin adapters and holds what every app needs. Its
review of Script at `29ed4bf` decided module by module what is general (moved
to the core) and what is about screenplays (stays here), and its
`docs/porting-script.md` is the plan this rebuild followed.

## Decision

Script 0.2 is built on open-source-core's `jcj-` crates and names no WordCraft
or craft-libs code (`cargo xtask layers`). The repository started from the
core's `template/`; `cargo xtask sync` keeps the toolchain, lint, cargo-deny and
license settings, the shared xtask modules, the CI workflow, the hooks and the
`[patch]` block identical to the pinned core release.

### What the core now does for Script

| Script's own before | Now |
| --- | --- |
| `wordsmith-{geom,doc,fonts,layout,render,pdf}` (copies) | `jcj-doc`, `jcj-layout`, `jcj-output` |
| `wordsmith-engine` (trimmed copy) | `jcj-engine`: the full engine, with Script's commands registered beside it (core decision 0007) |
| `wordsmith-mcp`, `wordsmith-control-key` | `jcj-agents`; the settings folder from `jcj-shell` |
| Bookmarks, notes, revisions, change tracking, page locks, watermark, headers and footers, dates, settings groups, comparison | `jcj-features` (core decision 0010); Script keeps the `{scene}` header field, the scene grouping of comparison, its own tracked typing, and its pagination |
| Caret, selection, formatting, find and spelling commands | The engine's and `jcj_engine::commands` (core decision 0007); Script's old ids stay as aliases (`commands::aliases`) |
| Saving backups, atomic writes | `jcj-project::files` |
| RTF import and export, PDF import's reader | `jcj-interchange` (with a new layout-keeping RTF writer), `jcj-output::read` (new) |
| Spelling data, user dictionary | `jcj-proof`; the script's own words are the core's document words (`core.spelling`) |
| The window around the script: ribbon frame, panels, File page, preferences, keys, auto-save, updates, icons, theme | `jcj-shell`, `jcj-theme`, `jcj-board` (see "The interface" below) |

### What stays Script's

Elements, typing rules, autocomplete, templates, macros; pagination with
(MORE), (CONT'D), CONTINUED, dual dialogue, margin marks and scene numbers;
scenes, scene data, characters, reports, the structure strip, the checker;
the title page; Fountain, the import classification rules and the screenplay
exports; the `.wss` format and its text markers; the story board's data
(beats, lanes, several boards); the screenplay's interface.

### Data: extension keys and `.wss` format 5

Script's nine fields on its copy of the model are typed extension data now
(core decision 0006), read and written through `keys::{ScriptPara, ScriptRun}`
as the fields were:

| 0.1 field | 0.2 key |
| --- | --- |
| `ParaProps::scene_number`, `omitted`, `beat`, `scene_id`, `dual` | `script.scene_number`, `script.omitted`, `script.beat`, `script.scene_id`, `script.dual` |
| `ParaProps::extra_attrs` | `script.extra_attrs` |
| `ParaProps::bookmark` | `core.bookmark` |
| `CharProps::note`, `revised`, `page_lock` | `core.note`, `core.revised`, `core.page_lock` |

`Document::app_data` became the document's `ext`, Script's own keys
unchanged (`wordsmith.elements` and the rest). `.wss` stays Script's readable
text format rather than becoming a `jcj-project` container, so scripts keep
their small, readable diffs. Format 5 writes `ext` in place of `appData`, moves
the shared features' groups to the core's keys and change tracking to
`trackChanges`; the text, attributes and marks are as in format 4
(`docs/format.md`). A test opens every file 0.1.2 saved for the golden scripts
and fixtures, saves it in format 5, reopens it and checks it paginates exactly
as 0.1.2 did.

### Behavior that changed, and why

- **Pages and PDF text: none.** The three golden scripts print the same text
  on the same pages, and every 0.1.2 page plan is reproduced.
- **PDF metadata** names WordCraft as creator and producer (the core's PDF
  writer fixes it; `docs/gaps.md`). The files are otherwise written by the
  core's writer at WordCraft v0.18.1.
- **RTF export** is written by the core's direct RTF writer: the same
  paragraphs, indents, spacing and emphasis, different RTF text (it reads back
  as the same elements, as the golden test checks).
- **The command registry** holds the whole engine catalog besides Script's
  commands. Script's keys win: an engine command whose key Script uses loses
  that key. `format.bold` and the other engine text styles format the whole
  word at a caret inside a word (core decision 0007), where Script's formatted
  only what was typed next; turning a style off stores it as off rather than
  removing it.
- **The agent server** is the shared one (`jcj-agents`): general tools plus
  each screenplay command as a tool of its own. 0.1's `scenes`,
  `characters`, `set_element`, `open_script`, `save_script`,
  `export_script` and `page_count` tools are gone (the guide's "Command line
  and agents" page maps each to its replacement).
- **Headers and footers** use the output's Header and Footer paragraph styles
  (Courier, as before) instead of Script's own band style.
- **Hostile PDF imports** with more than 5,000 pages, or more than 200,000
  characters on one page, are refused where 0.1 read the first ones.
- **Session time.** The core's session has no clock of its own; tracked
  changes and file names read `clock::now()`.

### Pagination stays Script's planner, for now

The plan asks for Script's pagination as a `PaginationPass` on the core's
planner. Script keeps its own planner, measured with the core's primitives
and wired to the core's page labels and page-aware commands
(`paginate::core_pages`, `jcj_features::pages::set_pages`). Tried on the
golden and fixture scripts, the core's planner with a screenplay pass makes
the same page starts on six of seven files and differs on Everybody's Girl
from page 6, because it keeps no room for the (MORE) line when it keeps a
speaker with the start of the speech. It also lacks three hooks Script uses:
a scene's page count for "CONTINUED: (2)", a forced page start at a lock
inside a paragraph, and placeholder pages. Golden files decide, so the move
waits for those hooks in the core.

### The interface

The window around the script is the core's shell (`jcj-shell`): the quick
access row, the ribbon frame with key tips, side panels with rails, the File
page, the command search, preferences, keys, auto-save's schedule, crash
recovery, About and updates. Script implements the shell's `App` for its editor
and supplies its ribbon tabs, its panels' contents, its status bar part, its
File page sections (templates on New, its export formats, its print choices,
Compare, its own preferences) and its brand: `assets/brand/script/tokens.json`
in the shape of `jcj-theme`'s tokens, Inter and JetBrains Mono for the
interface, its mark. Index cards and the story board draw with `jcj-board`'s
corkboard, canvas and card window; the story board's data stays Script's. The
editor, the screenplay's tabs, panels and dialogs, reports, tables, the
structure strip, tabs, split view, statistics and sprints stay Script's.

What users see differently:

- The shell's general icons on ribbon buttons, rails and the File page (it
  draws no app icons there yet); element tiles show their letter.
- "Left panel" and "Right panel" in the quick row; the menu bar sits at the top
  of the document area; the shell's message and light/dark switch at the far
  right of the status bar.
- The File page: the mark at the column's foot, Compare as a page of its own,
  the shell's preferences first, then Script's; New asks about unsaved changes
  (a new tab is still a button there). The unsaved-changes and recovery
  questions use the shell's words.
- The editor takes every key itself, in order, with the shell's key map
  (with the shell's own handling a key and text typed in the same frame ran out
  of order); Shift with a caret key extends the selection.
- Preferences are in the same file and folder; a 0.1 file is read into the new
  shape and written back in it (going back to 0.1 would read default
  preferences). The update check reads the same GitHub releases with the same
  file names; its work folder is named after the app's id.
- File dialogs are the system's own. Until core 0.1.20 Script showed them
  itself and the window waited for each; since then the shell shows them
  (`jcj_shell::files`) and the window goes on, so each dialog's answer
  carries the tab, the options and the waiting step it was asked with.

## Consequences

- Script follows the core's releases: all `jcj-` crates move together to a
  tag, then `cargo xtask sync`.
- A fix that every app wants is made in the core, with its own tests and a use
  in the core's test bed, then taken here with the next core release.
- The decisions this one replaces say so at their top: 0001 to 0008, 0039 and
  0040.

## Revisit when

The core has the pagination hooks above (move Script's planner onto a
`PaginationPass`), lets an app name itself in PDF metadata, or a second app
wants the story board's lanes and boards.
