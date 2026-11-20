# Gaps

What the app does not do, and anything else we know is missing, as of the end of
M14 (2026-11-20). `docs/spec-audit.md` checks `docs/SPEC.md` for v0.1;
`docs/spec2-audit.md` checks `docs/SPEC-2.md`, with what v0.2 added in its last
section. Questions for the SPEC-2 author are in `docs/spec2-questions.md`; until
they are answered, the values v0.1 built stay (decision 0030).

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
  not built.

## Views

- The second speech of dual dialogue is edited in the normal and speed views; the
  page view shows it but the caret cannot go into it there.
- Hidden outline elements still show in the normal and speed views (decision
  0028).
- No ribbon layout or customisable toolbar groups.
- Notes are not printed; threaded replies are not built; Fountain import leaves
  notes inside text out (export writes them).

## Production

- Clearing revised text clears the marks of the text selected; the rule that a
  mark goes only when its whole line is cleared is not followed.
- Deleting across a locked page break is allowed (decision 0035).
- Numbering shots, characters or dialogue.
- Breakdown tagging, scheduling export, the tags report and the tagging-mode
  indicator: out of scope until the owner says otherwise (question 9).
- Tracked changes are not seen by spelling, find or exports: deleted text reads as
  still there until it is accepted. Paragraph breaks are not tracked.
- Picture watermarks; a `{file}` header field.
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
- Read-aloud, collaboration and the assistant are not built (question 9 and
  ROADMAP).
- Tablet handedness, tooltips settings and usage data are not built.

## Release and tooling (decisions 0029 and 0038)

- Updates replace the app in place only on Linux (tar.gz in a writable
  folder). macOS and unsigned Windows copies are sent to the release page:
  craft-update accepts a macOS app only when ArtCraft's own team signed it
  (decision 0038).
- No AppImage, deb, rpm or MSI; `SHA256SUMS.txt` is not GPG-signed; one DMG per
  macOS architecture, not a universal one.
- Bundles, not installers (a disk image on macOS). The macOS app is signed ad
  hoc only, not with a Developer ID, and not notarized, and opening
  a document from the Finder or Explorer does not open it in the app (the command
  line does). No application icon.
- The CI and release workflows are on GitHub, but their runs (Linux, macOS,
  Windows) have not been checked from here; only the macOS gate and the macOS
  bundle were run on this machine.
- The minimum Rust version (1.95) is declared but not tested.
- Typing in a 150-page script takes 16 to 17.6 ms a keystroke (median) as of
  v0.1 (`docs/performance.md`); not measured again after v0.2's additions.
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
