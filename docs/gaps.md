# Gaps

What v0.1 does not do, and anything else we know is missing, as of the end of
M8 (2026-11-19). `docs/spec-audit.md` checks every Verified and Sourced spec
item for M1 to M7; the ones it marks as gaps are here with their reasons.

## Writing and formatting

- Copying element settings from a template, and global line-spacing presets
  (tighter or looser pages): v0.1 ships one screenplay template.
- A4 paper: the page size is the template's US Letter.
- More and continued texts cannot have their own font; they use the script's.
- Dark colors are not carried into the PDF.
- Emoji are drawn only where the bundled or system fonts have them; not
  tested.
- Input methods: WordCraft's composition safeguards were cut (decision 0006);
  egui's own text events work, but composing text is not specially protected.
- Shift+Tab, Page Up and Page Down, and word deletion are not assigned.
- Spelling is English only (decision 0014).
- Teleplay, stage play and other templates, writing stats and the sprint
  timer, structure colors, the inclusivity report: later (ROADMAP).

## Views

- Printing index cards and the scene view to PDF.
- Story board images and exporting the board to PDF (ROADMAP).
- Hidden outline elements still show in the normal and speed views (decision
  0028).
- The Tables panel has three fixed tables (scenes, characters, notes):
  adding, renaming, reordering and filtering tabs, and character highlight
  colors, are not built.
- Notes are not printed; threaded replies are not built; Fountain import
  leaves notes inside text out (export writes them).

## Production

- Clearing revised text clears the marks of the text selected; the spec's
  rule (a mark goes only when its whole line is cleared) is not followed.
- Page locking: relock from the cursor, adjusting a locked page's start,
  editing a page number by hand, locking a new page at the cursor and
  unlocking one page (decision 0026). Lock Pages again adds a level instead.
- Numbering shots, characters or dialogue; a settable starting page number.
- Breakdown tagging and scheduling export: out of scope (kickoff).

## Files

- Imports (decision 0019): emphasis is not kept from plain text, RTF or PDF;
  a PDF page cannot show a heading with no intro that ends in punctuation
  (read as Action) or a paragraph split across a page (read as two); line
  breaks inside a paragraph become spaces; a scanned PDF is refused.
- Saves made outside the window (the command line, MCP) make no backup copy.
- The backups folder can be set only by editing `preferences.json`.
- No file locking: a script opened twice can be saved from both.
- The web build cannot open, save or export files.

## Release and tooling (decision 0029)

- Bundles, not installers. The macOS app is not signed or notarized, and
  opening a document from the Finder or Explorer does not open it in the app
  (the command line does). No application icon.
- The CI and release workflows have never run on GitHub's runners: there is
  no remote. Only the macOS gate and the macOS bundle were run here; the
  Linux and Windows builds are untested.
- The minimum Rust version (1.95) is declared but not tested.
- Typing in a 150-page script takes 16 to 17.6 ms a keystroke (median), about
  one 60 Hz frame (`docs/performance.md`).
- Printing to a real printer, and the Windows print path, are untested
  (decision 0013).
- The interface was tested headlessly and by its own offscreen pictures, not
  by hand with a real mouse in a running window (drags, dialogs, the close
  prompt).

## Copied-code tests ignored for missing build inputs

These WordCraft tests pass when the build input is present (checked on
2026-11-19) and are `#[ignore]`d:

- `wordsmith-layout`: `auto_hyphenation_breaks_long_words`,
  `fragmented_hidden_text_lays_out_like_the_visible_text`,
  `hyphenation_and_proofing_follow_each_runs_language`: they need craft-i18n
  language data, whose English dictionaries carry the SCOWL notice (not on
  the allowlist; decision 0014).
- `wordsmith-fonts`: `a_thai_run_with_latin_falls_back_to_one_face`: it needs
  a Thai face (craft-fonts), not needed for screenplays.
