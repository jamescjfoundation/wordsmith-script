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

## Copied-code tests ignored for missing build inputs

These WordCraft tests pass when the build input is present (checked on
2026-11-19) and are `#[ignore]`d:

- `wordsmith-layout`: `auto_hyphenation_breaks_long_words`,
  `fragmented_hidden_text_lays_out_like_the_visible_text`,
  `hyphenation_and_proofing_follow_each_runs_language`: they need craft-i18n
  language data, whose English dictionaries carry the SCOWL notice (not on the
  allowlist; decision 0014).
- `wordsmith-fonts`: `a_thai_run_with_latin_falls_back_to_one_face`: it needs a
  Thai face (craft-fonts), not needed for screenplays.
