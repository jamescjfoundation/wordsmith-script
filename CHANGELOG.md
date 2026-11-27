# Changelog

What changed in each release of Wordsmith Script, newest first. `## Unreleased`
holds what the next release will bring; when that release is cut, the section
takes its version and date (`## 0.9.0 (2026-12-01)`), and a new, empty
`## Unreleased` opens above it.

The first public release is 0.9.0, a preview; 1.0.0 follows it (decision
0047). The versions under "Before the first release", at the end, were builds
of the first version of the app and were never published.

## Unreleased

### Changed

- The app icon at 256 and 512 px, the window's and the packages', is the
  Wordsmith brand kit's own file, in place of one scaled down here, and the
  palette card is the kit's current one. The brand files in `assets/brand/`
  are held to the kit's by `cargo xtask brand`, in the gate.
- **Templates on the File page's New page** are the tiles every Wordsmith app
  draws: each still shows the template's first lines on a small page, in the
  script's fixed-width font, with its name and what it is for, under its kind
  (screenplays, television, stage, audio and prose, outlines). They now come
  right after New document, above the recent scripts. Picking one while a
  script has unsaved changes still opens it in a tab of its own.
- **The Writing Statistics window** is the one the Wordsmith apps share. It
  shows what it showed: your current and longest streaks, your best day and
  hours (UTC), and words, pages or minutes by days, weeks, months or years
  with your goal as a line, and your goals and settings to change. The bars
  take the theme's accent color, and the average reads "Average a day, the
  last 7 days". **Your statistics are kept**: the `stats.json` 0.9.0 wrote
  beside your preferences is read as it is, and every day, goal and setting in
  it is written back.
- **A sprint counts the words you write**, as the statistics do: deleting
  during a sprint takes none away. In 0.9.0 it gave the change in the script's
  length.
- **Starting and closing** are the steps every Wordsmith app shares. The
  command line, `--version` and the window's title are as before. A
  preference changed within a second of another is written without waiting
  for your next key or click, and with no settings folder at all a first start
  opens the Scenes panel as it does with one.

### Fixed

- **A statistics file that cannot be read is kept**, as `stats.json.damaged`
  beside it, and the window says so; 0.9.0 started again without a word and
  wrote over it.
- **On macOS, Option with the Left or Right arrow moves by word**, as on any
  Mac; it went to the end of the line in 0.9.0 (core issue 48, fixed
  in open-source-core 0.1.24). Command with the arrows still goes to the
  line's ends.

## 0.9.0 (2026-11-27)

The first public release of Wordsmith Script: a preview, to try the
downloads, the app and its updates before 1.0.0. The packages are for macOS on
Apple silicon, Windows x64 and Linux x86_64, and are not signed with a
developer's certificate yet: the guide's
[Installing and updating](https://github.com/jamescjfoundation/wordsmith-script/blob/main/docs/guide/installing.md) page says how to open the
app the first time on each system. What is still missing for 1.0.0 is the
[`v1` milestone](https://github.com/jamescjfoundation/wordsmith-script/milestone/1).

This is the app as rebuilt on open-source-core. Where an entry says "before",
or "the first build", it compares with 0.1.2, the last build before the
rebuild, which was never published.

### Added

- The app has its own mark, wordmark, icon and colors. The window and the
  macOS and Linux packages carry the icon (an S and a brass caret on ink);
  the quick-access row starts with the mark, the File page's column is ink
  with the wordmark at its foot and a brass caret at the current page, and
  About shows the mark. The three themes take their colors from the
  palette: Iron Gall Ink, Page White, Brad Brass, Margin Blue, Slate and
  Night.
- **Downloads.** A release has one file for each system: a disk image for
  macOS on Apple silicon (`wordsmith-script-<version>-macos-aarch64.dmg`), a
  portable zip for Windows x64 (`...-windows-x64-portable.zip`) and an archive
  for Linux x86_64 (`...-linux-x86_64.tar.gz`), with `SHA256SUMS.txt` beside
  them. Each holds the app, the command line and the license texts. There is
  no file for a Mac with an Intel processor. The programs are not signed with
  a developer's certificate yet, so macOS and Windows ask before the first
  start; the guide's new page, Installing and updating, says how to go on.
- **Scripts open with a double-click.** On Linux the package's `install.sh`
  installs the app for your account with its menu entry, icon and the `.wss`
  file type; on Windows `register-file-types.cmd` registers `.wss`; on macOS
  the app declares `.wss` as its own type. Fountain files (`.fountain`,
  `.spmd`) are offered Wordsmith Script and not taken from another program.
- Drop-downs (More, Highlight, All Macros, the revision set, folded groups)
  have key tips that open them; their items have letters too.
- Paste is on the Write tab beside Cut and Copy, and in the Edit menu.
- Spelling, the Script panel and Inspector switches, the Light theme, the
  menu bar, the zoom, Print Cards, the index card options, each report tile
  and the status bar's theme, focus and sprint switches are commands: they
  are in the command search box, can be given keys, and show their key in
  their tooltip. A report tile opens the Reports window on its report.
- The View tab's zoom shows a menu of sizes.
- File › About has an Acknowledgments section: WordCraft, the libraries
  and fonts the app is built on, and the word list spelling uses. The
  README has the same section.
- **Keys**: every key of the first build still works. The engine's general commands add
  more: Strikethrough (Mod+Shift+X), Superscript and Subscript (Mod+Shift+=,
  Mod+=), Extend Selection (F8), Change Case (Shift+F3) and others; the
  keyboard guide lists them.
- **`wordsmith-script-cli mcp <file>` opens a script before the agent connects**,
  and tells the agent it is open. The file is the person's choice, so it is
  opened before `--read-root` and `--write-root` hold; what the agent opens
  and saves afterwards must still be inside those folders, a save back to
  that file included. Before, `mcp` took no file.

### Changed

#### Rebuilt on open-source-core

Wordsmith Script is built on open-source-core, The James C. J. Foundation's
shared base, where the first build carried its own copy of WordCraft
(decision 0044). Scripts print exactly as before: the golden scripts' pages
and PDF text are unchanged, and every script 0.1.2 saved opens with the same
pages.

- **Scripts are saved in format 5.** A script saved by the first build
  (formats 1 to 4) opens as it was and is saved in format 5 from then on,
  which the first build cannot open. Notes, revisions,
  page locks, bookmarks, the watermark, headers and footers and the script's
  own words are kept under the shared features' names; nothing is lost
  (`docs/format.md`).
- **The window** is the shared shell: the same tabs, panels and File page,
  with general icons on the ribbon, "Left panel" and "Right panel" in the
  quick row, Compare as a File page of its own, and the shell's preferences
  before Script's. Preferences are kept and moved to the new file shape.
- **Bold, italic, underline and the other text styles** at a caret inside a
  word format the whole word, as word processors do.
- **The agent server** (`wordsmith-script-cli mcp`) is the shared one: general tools,
  and every screenplay command as a tool of its own (`script_scene_list`,
  `script_element_action`, ...). The first build's `scenes`, `characters`, `set_element`,
  `open_script`, `save_script`, `export_script` and `page_count` tools are
  replaced; the guide maps each.
- **RTF export** is written by the shared RTF writer (same layout, different
  file text). PDFs name WordCraft as their creator; the first build's
  named Wordsmith Script (`docs/gaps.md`).
- **Not yet**: the web build, and an update in place on Windows and macOS,
  where the app opens the release page (`docs/gaps.md`).
- **Known in this preview, on macOS**: Option with the Left or Right arrow
  goes to the end of the line instead of moving by word
  ([core issue 48](https://github.com/jamescjfoundation/open-source-core/issues/48)).
  Command with the arrows works as on any Mac. The guide's keys page lists
  the keys of Windows and Linux; a command's tooltip shows its key on your
  system.
- Script tabs and the split view are drawn by the shared shell. Every open
  script's tab has its own close button (closing a script with unsaved
  changes still asks first), and the new-tab button sits at the right end of
  the strip. A split view's two panes start at half the space each, and the
  bar between them is dragged to give one more room; before, the second pane
  opened at a fixed width or height.
- The file dialogs of Open, Import, Save As, the exports and Compare are
  shown by the shared shell. What you will notice:
  - The window is not stopped while a dialog is up: it keeps drawing, and
    where the system leaves it usable (the Open dialog on macOS does) you
    can go on working in it. Before, the window stood still until the
    dialog closed.
  - One dialog shows at a time; another asked for meanwhile waits its turn.
  - A dialog's answer goes to the script that asked for it. If another tab
    is in front by then, the app goes back to that script first; if the
    script was closed, nothing is done and the status bar says so.
  - When the system cannot show a dialog, the status bar says why. Before,
    nothing was said.

#### The programs' names

The programs are named for Script, so that `wordsmith` stays free for the
Wordsmith launcher and nobody takes one Wordsmith app's program for another's
(decision 0045):

- **The desktop program is `wordsmith-script`** (it was `wordsmith`) and
  **the command line is `wordsmith-script-cli`** (it was `wordsmith-cli`). A
  script, a shortcut or an agent's configuration that starts either by name
  must use the new one. The command line's messages and its `--version` line
  begin with the new name.
- **The app's id is `org.jcjfoundation.script`**, in the pattern of the other
  Wordsmith apps: the macOS bundle's id (it was `org.wordsmith-script.app`),
  and on Linux the window's id, the menu entry
  (`org.jcjfoundation.script.desktop`) and the icon's name (they were
  `wordsmith-script`).
- **An isolated settings folder is named by `WORDSMITH_SCRIPT_PROFILE_DIR`**
  (it was `WORDSMITH_PROFILE_DIR`).
- Nothing else moves: the settings folder (`wordsmith-script` on Linux,
  `Wordsmith Script` on Windows and macOS) with its preferences, recovery
  files, backups, dictionary and statistics, the release files' names, the
  `.wss` file type and the name the agent server gives are as they were.

#### The command line and agents

`wordsmith-script-cli` follows the convention every app on open-source-core shares
([core decision 0017](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0017-command-lines-and-agent-tools.md)),
so a script or an agent that knows one app's command line knows the next.
What changes for a script that calls it, and for an agent:

- **Run with no argument, it exits with 2** and prints the usage on standard
  error. Before, it printed the usage on standard output and exited with 0,
  so a caller that had lost its arguments read success. `--help`, `-h` and
  `help` still print the usage on standard output with 0.
- **`--version` prints `wordsmith-script-cli <version>` alone on its first line**,
  and "Wordsmith Script, native extension .wss" on the second. Before, both
  were on one line.
- **Every tool of Script's own is named `script_<command>`**:
  `script_scene_list`, `script_element_action`, `script_report_characters`.
  Before, a tool had its command's name alone (`scene_list`). The general
  tools (`execute`, `list_commands`, `type_text` and the rest) keep their
  names, and the commands' ids, which `execute` takes, are unchanged. The
  server's instructions to the agent say so.
- `--read-root` or `--write-root` given twice, and a second file, are usage
  errors. Before, the last of two folders won.
- `export-pdf` prints its page count on standard output, where results go.
  Before, it was on standard error.
- `export-pdf` treats any argument that starts with `-` as an option, so a
  mistyped one-dash option is a usage error; before, it was read as a file
  name. `convert` checks the output's format before it reads the input.
- A word that starts with `-` in the command's place is reported as an
  unknown option.
- Exit codes are as before: 0 done, 1 failed, 2 a wrong command line. 3, "a
  check ran and did not pass", is kept by the convention for commands that
  check or compare; `wordsmith-script-cli` has none yet and never exits with 3.

### Fixed

- Key tips (F10) reach every control on a tab. Before, from the 21st
  control on, typing the first letter ran another control. The first 19
  controls have one letter and the rest start with Z; a control keeps its
  letters while the app runs, even when its group folds to fit the window.
- A key tip runs its control as a click does, with the same settings (Start
  Up and Start Down on locked pages, a macro's slot, Strike Through).
- The find bar can be closed while the Find panel is open; the panel keeps
  its results. Escape closes the bar only from its own fields or when
  nothing else is open, so closing the palette or a suggestion list no
  longer closes it.
- File › Preferences: the auto-save fields (how many minutes, and how long
  typing must pause) are named for screen readers by the words beside them.
  Before, they were unnamed number fields.
- A shifted caret key that you turned off on the Keys page does nothing.
  Before, Shift with an arrow, Home or End still extended the selection
  when its own entry was turned off.

### Inside

For people who work on the code.

- The crates and folders follow the other Wordsmith apps (decision 0045, which
  has the whole table): `crates/script-model` (`wordsmith-script-model`, the
  screenplay rules; it was `wordsmith-script`), `crates/script-format`
  (`wordsmith-script-format`), `crates/script-ui` (`wordsmith-script-ui`), and
  the programs in `apps/script`, `apps/script-cli` and `apps/script-web`.
- The files the tests read are in `tests/fixtures/` and `tests/golden/` at
  the root, as in the other Wordsmith apps, and `docs/fixtures.md` lists each
  with where it came from. `UPDATE_GOLDEN=1` is the one switch that rewrites
  a reviewed file, the guide's page of keys included (it was
  `UPDATE_GUIDE=1`). The guide's pages `keyboard.md` and `cli-and-mcp.md` are
  `keys.md` and `command-line-and-agents.md`. No fixture or golden file
  changed (decision 0046, `docs/golden.md`).
- The version is 0.9.0, the first that will be published; it was 0.2.0,
  which never was (decision 0047).
- What is planned is the `v1` milestone of this repository's issues.
  `ROADMAP.md` links to it and holds no list. `docs/gaps.md` keeps only the
  limits a writer should know; every other line it had is an issue, or was
  out of date and is gone. The first build's log is
  `docs/history/progress-first-build.md` and `docs/progress.md` starts at the
  rebuild; `docs/REUSE.md`, `docs/provenance.md` and the timings in
  `docs/performance.md` say in their first lines that they are the first
  build's.
- Script uses the core's file dialogs (`jcj_shell::files`, core issue #28)
  and the core's rule for shifted caret keys (`KeyMap::command_for`, core
  issue #29), both new in core 0.1.20, and its own copies are deleted. The
  desktop app no longer depends on `rfd` or `raw-window-handle` itself, so
  the build holds one version of the dialog library instead of two.
- The command line's exit codes, its `--help`, `--version` and `mcp`
  arguments and the names of its tools are the core's code (`jcj_agents::cli`,
  `AgentServer::tool_prefix`), and a test runs the built program and holds
  it to the convention (`apps/script-cli/tests/command_line.rs`).
- The checks that every control runs a registered command, and that every
  menu command is on the ribbon or the File page, read what the window draws
  instead of lists kept by hand (decision 0043).
- Built on open-source-core 0.1.22 (before, 0.1.1), so new work starts from
  the newest shared base. The releases between add interfaces Script does not
  call yet and one fix it shows (the Preferences fields above); no code of
  Script's changed. The golden scripts' PDF text and the page plans of every
  file 0.1.2 saved are unchanged (`docs/golden.md`).
- The release packages and the release workflow are the core's, the same for
  every app built on it (core decision 0019). `cargo xtask package` makes this
  computer's package from data in `xtask/src/main.rs`, and
  `.github/workflows/release.yml` only calls the core's shared workflow: it
  runs the whole gate on Linux, macOS and Windows, builds and checks the
  three packages and `SHA256SUMS.txt`, and only then makes the tag and
  publishes, with this file's section as the release's text. Script's own
  `cargo xtask bundle`, its release workflow and the `release` branch are
  gone. The files' names are unchanged. A file's attestation is checked with
  `--signer-repo jamescjfoundation/open-source-core`.
- The desktop program finishes an update when Windows starts it as the
  updater's helper, and a file name that is not Unicode on the command line
  no longer stops it.
- `cargo xtask core-pin` joins the gate: every `jcj-` line names one core
  release and `Cargo.lock` holds one core. A weekly workflow opens an issue
  when the pin is more than three core releases behind.
- The rules every app shares with the core are no longer copied by hand.
  They are marked blocks in `AGENTS.md`, `SECURITY.md` and `CONTRIBUTING.md`
  that `cargo xtask sync` writes from the pinned core release, and the gate
  fails when one differs. `CODE_OF_CONDUCT.md` is the core's file. What is
  Script's own stays outside the blocks.

## Before the first release

Nothing in this section was published. 0.1.0, 0.1.1 and 0.1.2 were builds of
the first version of the app, made in November 2026 before it was rebuilt on
open-source-core. That code carried its own copy of WordCraft; it was written
before this repository's history starts, and its history is not published: no
commit, tag or release of it exists here (`UPSTREAM.md`, decision 0047). The
notes are kept as they were written, for what they say about how the app came
to be. They name the crates and programs as they were then (decision 0045),
and "v0.2" and "v0.3" in them are the names of plans, not of versions.

### 0.1.2 (built 2026-11-22, never released)

New names for the Script panel and the Draft view, script files that keep
what a later version wrote, and checked file writes for every command.

#### Automation

- Every command that writes a file, including report, storyboard and index
  card exports, goes through the same checked writer, so a write is complete
  or not made at all and `wordsmith-cli mcp --write-root` applies to it.
- While `wordsmith-cli mcp` is confined to folders, printing and the print
  preview are refused. The print preview writes a new, uniquely named file
  each time.
- `wordsmith-cli mcp` started without `--read-root` or `--write-root` warns
  that the agent can read and write any file your account can.

#### Inside

- The crates that started as a copy of WordCraft are now a fork, trimmed to
  what a screenplay uses (decision 0039). Nothing the app does changes.

#### Privacy and licenses (decision 0040)

- The README and the user guide (The window › Updates and privacy) say what
  the update check at start sends (one request to GitHub's API with the app's
  name and version; GitHub sees your IP address) and how to turn it off. The
  app has no telemetry and never sends script content anywhere. The first-run
  tour mentions the check and where to turn it off.
- Release bundles carry a `licenses` folder: the license texts of every Rust
  library the app is built from (`THIRD-PARTY-LICENSES.txt`) and of the
  bundled fonts. On macOS they are inside the app as well. The About page says
  where they are and names all five fonts.
- Each release file has a GitHub build provenance attestation; check one with
  `gh attestation verify <file> --repo jamescjfoundation/wordsmith-script`.
- Development: the Rust toolchain is pinned to an exact release, CI checks the
  minimum version (1.95), runs cargo-deny for licenses, advisories and
  sources, and uses commit-pinned actions; Dependabot proposes updates.

#### Names (decision 0041)

- The left side panel (scenes, outline, bookmarks, notes, characters, find) is
  now the **Script panel**, and the compact view beside Page and Normal is the
  **Draft view**. The old names belong to another product. Preference files
  that name the old view still open in the draft view, and the
  `view.speed` command id is unchanged, so keys, scripts and agents keep
  working.

#### Commands

- `lock.adjustBreak` refuses a `by` of 0, or one that is not a whole number,
  with a message saying what it takes. Before, 0 failed with an unrelated
  message and a value that is not a number moved the page start down.
- The header and footer `{date}` reads the clock the way the rest of the app
  does, which also works in the web build.

#### Speed

- Typing no longer reads the script's element list from its settings two
  dozen times a keystroke; it is read again only when it changes. A test in
  CI fails if a keystroke in a 150-page script paginates more than once or
  lays out more than the paragraph typed in (`docs/performance.md`).

#### Script files

- Paragraph attributes in a `.wss` file that this version does not know (from
  a later version) are kept and written back when the script is saved, instead
  of being dropped.
- A script that cannot be written in full is not saved, with a message saying
  why, instead of being saved with empty settings or without some of its
  tracked-change records. A file with more than 100,000 tracked-change records
  is refused as damaged when opened.
- Scene numbers: every number the app gives a scene is one the script file
  keeps, so locked numbers no longer change after saving and opening a script
  with a long run of added scenes. Fountain scene numbers the app cannot keep
  (longer than 12 characters, or with other characters than letters, digits,
  `-` and `.`) are left out on import instead of being lost on the next save.
- `docs/format.md` describes format version 4, including the tracked-change
  codes and the `revisions` settings key.

### 0.1.1 (built 2026-11-20, never released)

The first build that checks for its own updates. It also carries
everything built since 0.1.0: the v0.2 features from SPEC-2 and the v0.3
interface, listed below.

#### Releases and updates (decision 0038)

- Help ▸ Check for Updates, an Updates section on the About page, and a check
  at start (a preference, on by default). A newer release is offered in a
  Software Update window. On Linux it installs the update, verified against
  its SHA-256, and restarts; on macOS and Windows it opens the release page.
- Release files are named `wordsmith-script-<version>-<os>-<arch>`; macOS ships
  a disk image.
- The release workflow drafts a GitHub release with `SHA256SUMS.txt` and
  publishes it only when asked, refusing one with fewer files than the last.

#### The interface (v0.3 plan, M15 to M19)

- A ribbon of tabs named for screenplay work, every control a command with its
  key in the tooltip, groups that fold to fit the window, Story Board and Index
  Cards tabs with those views, and key tips (F10).
- A quick-access row with a command search box.
- The Navigator (scenes, outline, bookmarks, notes, characters, find) and the
  Inspector (element, scene, revisions, page, changes, checker, goals) in rails
  either side of the page.
- A File page for new (template cards), open, save, export, print, compare,
  preferences, keys and about.
- A status bar that says what Return and Tab make, with clickable page, words
  and scene and chips for revision mode and tracking.
- Light, dark and extra-dark themes from one set of tokens; icons drawn in code.
- A first-run tour; the window's layout kept between runs.

#### Features from SPEC-2 (M9 to M14)

Work toward v0.2 from `docs/SPEC-2.md`. Nothing built in 0.1.0 changes its
behavior; every addition below is new.

- **Editing (M9):** the Sequence element; the Return and Tab hint; reset to
  element; toggle case; smart quotes and sentence capitals as options; select
  scene, new scene, page break; Go To by page, scene or bookmark; bookmarks
  and labels; find within one element; the reformat tool; the format checker.
- **Macros and styles (M10):** twenty macros per script with shortcuts,
  aliases and a starter set; bold, italic and underline keys; double, word and
  dotted underlines, strike, raised, lowered, capitals, small capitals, small
  letters; highlighter and text colors; element color, highlight and strike;
  template formatting for all elements. The native format is now version 4
  (0.1.0 cannot open version-4 files).
- **Characters and reports (M11):** scene properties (title, color, cast,
  non-speaking characters, arc beats); character traits and speech
  highlighting; scene, location, cast, character, filtered-script, notes,
  statistics and representation reports, saved as text, CSV or PDF; tables
  that sort, filter, hide columns and keep tabs of your own; note titles,
  types, colors and dates; next and previous note.
- **Planning (M12):** several story boards per script; arrange beats on the
  grid, in a column, a row or a cascade; link styles and colors; list lines
  in beats; story board and index cards as PDF; cards per row and script-text
  cards; the structure strip above the script; nesting bars for outline
  paragraphs.
- **Production (M13):** dual dialogue (also from Fountain `^`); relock, move a
  locked page's start, lock or unlock one page, edit a page number; scene
  number options (start, restart, A2 letters, sides, font, one number by
  hand); a mark per revision set and current-set-only display; print ranges,
  one set, unrevised pages, sides, reverse, collated copies and preview; a
  watermark; header and footer start page and `{collated}`; change tracking;
  comparing two versions.
- **Workspace (M14):** twelve templates of our own and templates of your own;
  text documents for prose; the Welcome window, recent scripts and reopening
  the last ones; revert and merge; tabs; split views; zoom steps and an
  extra-dark theme; writing statistics, goals and sprints; your own keys.

### 0.1.0 (built 2026-11-19, never released)

The first numbered build: a starting point, not a finished app. Everything is
new; in brief:

- **Writing:** the screenplay elements with Return and Tab, element settings
  and custom elements, scene-heading separators, autocomplete over six lists,
  spelling (English), find and replace, notes, character rename.
- **Pages:** industry pagination with (MORE), (CONT'D) and CONTINUEDs, title
  page, headers and footers with fields, PDF export and printing matching the
  page view.
- **Views:** page, normal and speed views; index cards; scene view; tables
  (scenes, characters, notes) with CSV; dark theme, typewriter scrolling,
  focus mode.
- **Production:** revision mode and sets on the color cycle with margin
  marks and page colors; scene numbers with letters and OMITTED; locked pages
  with A-pages and placeholders; revised-pages-only export and printing;
  scene lengths in eighths; character report.
- **Outlining:** story board of beats with links and grid snap; outline
  lanes with structure templates; beats sent to the script; outline elements
  shown or hidden.
- **Files:** the `.wss` native format (version 3); Fountain both ways;
  import of plain text, RTF and text PDF; export of RTF, plain text, HTML
  and CSV; auto-save, timestamped backups, crash recovery; preferences.
- **Automation:** `wordsmith-cli` (convert, export-pdf, report, info, mcp)
  and an MCP server with screenplay tools.
- **Release:** bundles for macOS, Linux and Windows (`cargo xtask bundle`).

What v0.1 does not do is listed in `docs/gaps.md`.
