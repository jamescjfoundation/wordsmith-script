# Gaps

What the app does not do, and anything else we know is missing, as of version
0.1.2 (2026-11-22). Keep it current: a change that adds or removes a gap updates
this page. The open questions about `docs/SPEC-2.md` are in
`docs/history/spec2-questions.md`; until they are answered, the values v0.1
built stay (decision 0030).

## Writing and formatting

- Copying element settings from another script's template (the built-in templates
  set their own; "Template Formatting for All" restores the screenplay's).
- Global line-spacing presets (tighter or looser pages) and A4 paper.
- More and continued texts cannot have their own font.
- Dark colors are not carried into the PDF.
- No emoji font is bundled: emoji show only where the interface or system fonts
  have them.
- Input methods: composing text is not specially protected (decision 0006).
- Shift+Tab, Page Up and Page Down, and word deletion are not assigned.
- Spelling is English only (decision 0014); no thesaurus or names lookup (no
  allowlisted data, question 10).
- The clipboard carries plain text, so pasting cannot bring element types in.
- Indents keep v0.1's range rule (0 to 8.5 in, half an inch apart), not SPEC-2's.
- Alternate lines, pictures in the script and in beats, and labels in headers are
  not built. The word processor's picture code was cut from the copied crates
  (decision 0039), so pictures are new work.

## Views

- The second speech of dual dialogue is edited in the normal and draft views; the
  page view shows it but the caret cannot go into it there.
- Hidden outline elements still show in the normal and draft views (decision
  0028).
- The ribbon's tabs and groups cannot be changed or rearranged (decision 0037).
- Notes are not printed; threaded replies are not built; Fountain import leaves
  notes inside text out (export writes them).

## Production

- Clearing revised text clears the marks of the text selected; the rule that a
  mark goes only when its whole line is cleared is not followed.
- Deleting across a locked page break is allowed (decision 0035).
- Numbering shots, characters or dialogue.
- Breakdown tagging, scheduling export, the tags report and the tagging-mode
  indicator: out of scope (decision 0042; question 9).
- Tracked changes are not seen by spelling, find or exports: deleted text reads as
  still there until it is accepted. Paragraph breaks are not tracked.
- Picture watermarks (new work since decision 0039); a `{file}` header field.
- A review-only copy (notes allowed, text locked).

## Files

- Imports (decision 0019): emphasis is not kept from plain text, RTF or PDF; a PDF
  page cannot show a heading with no intro that ends in punctuation (read as
  Action) or a paragraph split across a page (read as two); line breaks inside a
  paragraph become spaces; a scanned PDF is refused.
- Saves made outside the window (the command line, MCP) make no backup copy.
- The backups folder can be set only by editing `preferences.json`; the app does
  not warn when it is missing or full, and has no command to open it.
- No file locking: a script opened twice can be saved from both. Read-only files
  open as a copy.
- The web build cannot open, save or export files.
- Plain text with layout (indents as spaces) is not an export of its own.
- Other products' script formats: they need an interface spec (SPEC-2 20).

## Workspace

- Tabs after the first have no crash recovery of their own.
- Read-aloud is not built. Collaboration and the assistant are out of scope
  (decision 0042; question 9).
- Tablet handedness, tooltips settings and usage data are not built.

## Release and tooling (decisions 0029 and 0038)

- Updates replace the app in place only on Linux (tar.gz in a writable
  folder). macOS and unsigned Windows copies are sent to the release page:
  craft-update accepts a macOS app only when ArtCraft's own team signed it
  (decision 0038).
- No AppImage, deb, rpm or MSI; `SHA256SUMS.txt` is not GPG-signed (each release
  file has a GitHub build provenance attestation instead, decision 0040, but the
  in-app update does not check it); one DMG per macOS architecture, not a
  universal one.
- Bundles, not installers (a disk image on macOS). The macOS app is signed ad
  hoc only, not with a Developer ID, and not notarized, and opening
  a document from the Finder or Explorer does not open it in the app (the command
  line does). No application icon.
- CI runs the gate on Linux, macOS and Windows, and the release workflow
  builds the bundles for all three, but only the macOS bundle has been tried
  by hand.
- The minimum Rust version (1.95) is checked in CI with `cargo check`; the
  tests run only on the pinned toolchain (decision 0040).
- cargo-deny ignores four "unmaintained" notices (no vulnerabilities): chrono,
  used for dates in craft-libs and here (the script crate's dates all go
  through its `dates` module), and three zbus 5 crates that egui
  0.36's Linux accessibility brings in. They go when chrono is replaced and
  egui moves (`deny.toml`).
- Typing in a 150-page script took 17.2 to 17.4 ms a keystroke (median) with
  the v0.3 interface (`docs/performance.md`). CI guards the work a keystroke
  does (one pagination, one paragraph laid out again, no settings parsed
  for the element registry), not its time; the time is measured by hand on
  one Mac only.
- Printing to a real printer, the Windows print path and the print preview's
  viewer are untested (decision 0013).
- The interface was tested headlessly and by its own offscreen pictures, not by
  hand with a real mouse in a running window.

## On open-source-core (0.2, decision 0044)

What the rebuild on the core left for later, each with what it waits for:

- **PDF files name WordCraft** as their creator and producer. WordCraft's PDF
  writer fixes both names; letting an app name itself needs a patch on the
  core's WordCraft fork and a new pin there, which is a maintainer's decision.
  0.1.x wrote "Wordsmith Script".
- **Pagination is still Script's own planner**, over the core's measuring
  primitives, page labels and page plans (`jcj_layout::measure`,
  `jcj_features::locks`, `jcj_features::pages`), not a `PaginationPass` on the
  core's planner. Tried on the golden and fixture scripts, the core's planner
  with a screenplay pass makes the same page starts on six of seven and differs
  on one (Everybody's Girl from page 6): it does not keep room for the (MORE)
  line when it keeps a speaker with the start of the speech. It also cannot yet
  count a scene's pages for "CONTINUED: (2)", force a page to start at a page
  lock inside a paragraph, or add placeholder pages for deleted locked pages.
  Each is a general hook the core would need first.
- **The story board's data** (beats, lanes, several boards) stays Script's:
  the core's board (`jcj-board`) has no lanes and one board, and it is an
  interface crate.
- **Script's text markers in `.wss`** (`**`, `*`, `_` and the note, revision,
  lock and style marks) are read and written by Script's own codec, not by
  `jcj_interchange::emphasis`: the format's markers are one grammar.
- **The engine's catalog**: every registry now carries WordCraft's ~420
  word-processor commands. Script shows only its own in its interface, and
  agents see the whole list through `list_commands`.
- **The web build does not build.** The core's shell turns on its desktop
  pieces (dialogs, the update check) for every target, and the update check's
  TLS library needs a C compiler for wasm32 that the build machines lack.
  `wordsmith-web` builds and tests for the desktop; `cargo xtask wasm` fails.
  It needs the shell to leave those pieces out on wasm32.
- **An update cannot finish in place on Windows**: the shell has no hook for
  the updater's finishing step (`--finish-update`), so 0.2 on Windows offers
  the download instead. On macOS updates are offered as downloads, as in 0.1.
- **File dialogs** come from the `rfd` crate in `apps/wordsmith` directly: the
  core has no file-dialog interface for apps yet (marked `core-candidate`).
- **Icons**: the shell draws only its general icons on the ribbon, the rails
  and the File page; Script's own icons are used in its panels and status bar.
- **Shell keys**: Shift with a caret key extending the selection, and keys
  typed in the same frame as text, are handled in Script's editor until the
  shell does both (`core-candidate`).
- **Spelling and hyphenation without craft-i18n**: as before, English spelling
  uses the word list built into craft-proof (the core's `docs/UPSTREAM.md`).
