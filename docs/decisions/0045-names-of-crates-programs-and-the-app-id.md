# 0045: The names of the crates, the programs and the app id

Date: 2026-11-26 · Status: accepted

## Context

Script was the first Wordsmith app, and its code took the family's name: the
desktop program was `wordsmith`, the command line `wordsmith-cli`, the crates
`wordsmith-script`, `wordsmith-format` and `wordsmith-ui`, and the macOS
bundle's id `org.wordsmith-script.app`. Wordsmith is now three apps and a
planned launcher, and those names read as the whole family's:

- the launcher is the program that should be called `wordsmith`;
- `wordsmith-format` and `wordsmith-ui` say nothing about screenplays, beside
  Manuscript's `wordsmith-manuscript-format` and Book's `wordsmith-book-ui`;
- once the desktop program is `wordsmith-script`, the crate of that name is in
  its way: two packages of a workspace cannot share a name;
- someone who learned where things are in Manuscript or Book (`crates/<app>-<part>`,
  `apps/<app>`, packages named `wordsmith-<app>-<part>`, the app id
  `org.jcjfoundation.<app>`) found none of it here.

No version of Script has been published, so no installed program, menu entry
or file association carries the old names.

## Decision

Script takes the pattern of the other two apps.

| What | Before | Now |
| --- | --- | --- |
| The screenplay rules | folder `crates/wordsmith-script`, package `wordsmith-script` (`wordsmith_script` in code) | `crates/script-model`, `wordsmith-script-model` (`wordsmith_script_model`) |
| The `.wss` format, Fountain, imports and exports | `crates/wordsmith-format`, `wordsmith-format` | `crates/script-format`, `wordsmith-script-format` |
| The interface | `crates/wordsmith-ui`, `wordsmith-ui` | `crates/script-ui`, `wordsmith-script-ui` |
| The desktop app | `apps/wordsmith`, package and program `wordsmith` | `apps/script`, package and program `wordsmith-script` |
| The command line and MCP server | `apps/wordsmith-cli`, `wordsmith-cli` | `apps/script-cli`, `wordsmith-script-cli` |
| The web build | `apps/wordsmith-web`, `wordsmith-web` | `apps/script-web`, `wordsmith-script-web` |
| The window's app id | `wordsmith-script` | `org.jcjfoundation.script` |
| The macOS bundle's id | `org.wordsmith-script.app` | `org.jcjfoundation.script` |
| The Linux menu entry, its `StartupWMClass` and its icon's name | `wordsmith-script.desktop`, `wordsmith-script` | `org.jcjfoundation.script.desktop`, `org.jcjfoundation.script` |
| The variable that names an isolated settings folder | `WORDSMITH_PROFILE_DIR` | `WORDSMITH_SCRIPT_PROFILE_DIR` |

- **`script-model`** is named for its place, as `manuscript-model` and
  `book-model` are: the lowest layer, with no files and no interface. It holds
  more than data: the typing rules, pagination, reports and the screenplay
  commands work on the model and live with it, as before.
- **One app id, written twice.** `org.jcjfoundation.script` is
  `wordsmith_script_ui::app::APP_ID` for the window and `APP_ID` in
  `xtask/src/bundle.rs` for the bundle (the gate has no dependencies). A Linux
  desktop joins a window to its menu entry by this id, so a test in the bundle
  task fails when the two differ.
- **The profile variable** carries the app's name because a variable called
  after the family would send every Wordsmith program started with it to one
  folder, where each would overwrite the others' preferences.

What keeps its name, and why:

| What | Name | Why it stays |
| --- | --- | --- |
| The product | Wordsmith Script | |
| The settings folder | `wordsmith-script` on Linux, `Wordsmith Script` on Windows and macOS (`app::ID`, `app::NAME`) | Already the app's own, in the pattern the other two use (`wordsmith-manuscript`, `wordsmith-book`). Preferences, recovery files, backups, the dictionary and statistics stay where they are |
| Release files | `wordsmith-script-<version>-<os>-<arch>…` | The same |
| The name the agent server gives | `wordsmith-script`; tools `script_<command>` | The same; core decision 0017 |
| The file type | `.wss`, `application/x-wordsmith-script` | Decision 0017 and `docs/format.md`; the name is already Script's own |
| Keys inside a `.wss` file | `wordsmith.elements`, `wordsmith.sceneNumbers` and the rest | They are the file format. Changing them needs a new format version and a change to Wordsmith Manuscript's reader, and gives a writer nothing |
| The repository | `wordsmith-script` | |

## Consequences

- `cargo run -p wordsmith-script` is the app; `cargo test -p wordsmith-script-model`
  the rules. The layer table in `AGENTS.md` names the folders.
- The website and the wiki name the programs; they follow from the guide and
  the changelog ([Route C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c)).
- Records from before this one, the first build's plans in `docs/history/` and
  the changelog's sections for 0.1.x name the crates and programs as they were.
  They are not rewritten; the first table above is the key to them.

## Revisit when

- The launcher needs to find or start Script by another name or id.
- The core gives apps one packaging routine with its own names for menu
  entries and icons.
