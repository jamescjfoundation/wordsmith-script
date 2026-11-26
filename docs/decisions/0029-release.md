# 0029: The command line, the MCP server and the release

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0007](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0007-engine-agents-output.md))

Since decision 0044 (2026-11-24) the MCP server is the core's `jcj-agents`:
general tools, and each screenplay command as a tool of its own (the guide's
"Command line and agents" page maps the old tools). Still Script's: the command
line, the screenshots and the bundles, with file names as decision 0038 changed
them.

## Command line

`wordsmith-cli` reads `.wss` and Fountain scripts and imports plain text, RTF
and PDF as scripts:

- `convert <input> <output>`: write in the format the output's extension
  names (wss, fountain, txt, rtf, html, pdf).
- `export-pdf <input> <output.pdf> [--revised-only] [--no-marks]`: the
  printed script, with its title page; or only the revised pages; with or
  without revision marks.
- `report characters|scenes <input> [<output.csv>]`: the character report or
  the scenes table as CSV, to the file or to standard output.
- `info <input>`: title, pages, scenes, characters, paragraphs.
- `mcp [--read-root <folder>] [--write-root <folder>]`: the MCP server.

Exit codes: 0 done, 1 failed (a file could not be read or written, nothing
revised to export), 2 a usage error. Writes are atomic.

## MCP server

`wordsmith-cli mcp` serves the Model Context Protocol (JSON-RPC 2.0 over
standard input and output) on a headless session with every screenplay
command and the file commands. Besides the general tools (list commands, run
one or a batch, inspect the script, render a page), screenplay tools:
`open_script`, `save_script`, `export_script`, `type_text`, `set_element`,
`scenes`, `characters`, `page_count`. With `--read-root` and `--write-root`,
every file a command reads or writes must be inside those folders, for the
whole process. Desktop-only tools (screenshot, click, key) answer that they
need the app.

The file commands moved from the interface to `wordsmith-format::commands`,
so the window, the command line and the MCP server share one copy; the
window still answers their requests for file dialogs.

## Screenshots

The README's pictures come from the app itself: `wordsmith_ui::snapshot` runs
the editor headlessly for a few frames and rasterizes egui's output on the
CPU (each triangle's interpolated vertex color times its texture: the font
atlas and the page tiles, blended premultiplied), so no window or GPU is
needed. `cargo xtask screenshots` draws the page view, index cards, story
board, outline and the dark draft view of a public-domain sample (with a
title page, locked pages, a Blue revision, scene numbers, beats and lanes)
into `docs/images`. Looking at them found two layout bugs in the index cards
(cards growing with their text and not wrapping into rows), now fixed.

## Bundles

(File names, the macOS disk image and the release workflow changed in
decision 0038.)

`cargo xtask bundle` builds the app and the command-line tool in release mode
and packs them for the computer it runs on, in `dist/` (ignored by git):

- **macOS:** `Wordsmith Script.app` (both programs in `Contents/MacOS`, an
  Info.plist naming the app, its version and the `.wss` and Fountain document
  types), with the licenses, in a zip made by `ditto`. The app is not signed
  or notarized (that needs a paid developer account, which the maintainers decide on first, decision 0042).
- **Linux:** `bin/wordsmith`, `bin/wordsmith-cli`, a `.desktop` file and the
  licenses, as a `.tar.gz`.
- **Windows:** `wordsmith.exe`, `wordsmith-cli.exe` and the licenses, as a
  `.zip` made by Windows' own `tar`.

No installers: an installer per platform would need signing to be worth
having, and signing needs a paid account (decision 0042). The release workflow
(`.github/workflows/release.yml`) runs the gate and builds the three bundles
when started by hand, keeping them as workflow artifacts; it publishes
nothing. The app opens a file named on its command line; opening a document
from the Finder or Explorer (an Apple Event on macOS) is not handled in v0.1.

