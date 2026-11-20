# Gaps

Spec items from `docs/SPEC.md` (Verified and Sourced, milestones M1 to M7)
that are not built yet, and anything else we know is missing. An item leaves
this list when it ships with tests, or stays with a reason if v0.1 will not
cover it.

## M1: done, with these gaps

M1's spec items (element table, Return and Tab tables, separators, change
element and Mod+0 to Mod+9, casing, Dialogue-then-Character option, element
settings, custom elements, font for all, undo of all of it) are built and
tested. Left open:

- Saving and opening files: M4. Crash recovery is the only persistence until
  then.
- IME (input method) composition safeguards were cut from the engine
  (decision 0006); egui's own text events work, but composing text is not
  protected the way WordCraft protected it.
- Shift+Tab, Page Up/Down and word deletion (Alt+Backspace) are not assigned.
- The editor's look was not checked automatically; headless tests cover its
  behavior.

## M2 gaps

- More and continued texts can be edited and switched (`moresContinueds.*`),
  but not given their own font (SPEC, Sourced); they use the script font.
  There is no dialog for them yet; the command palette and agents reach the
  commands.

- Fountain reading strips emphasis (`*`, `**`, `_`) and keeps only the text;
  formatting, the writer and round trips are M4. Dual dialogue (`^`) reads as
  ordinary dialogue (dual dialogue is an open question in the spec).
- The golden scripts have little dialogue; (MORE)/(CONT'D) are covered by
  unit tests rather than the golden corpus.

- The web build cannot export a PDF yet (native dialogs are desktop-only;
  the web needs a download instead).
- Printing to a real printer, and the Windows print path, are untested
  (decision 0013).

## M3 gaps

- "Add to Dictionary" lasts for the session; saving the personal dictionary
  between runs needs the settings store (M4). "Ignore in This Script" is
  saved with the script.
- Spelling is English only (decision 0014).
- Dark PDF colours (SPEC: "optionally matched in PDF colors") are not built.
- View, theme, typewriter and focus settings are not remembered between runs
  (settings store, M4).
- Writing stats, sprint timer and structure colours are later items
  (`ROADMAP.md`).

## Not started

Everything in M1 to M8 not marked done in `docs/progress.md`. Each milestone plan in `docs/plans/` breaks its part
of the spec into items that are tracked here as they land.

## Copied-code tests ignored for missing build inputs

These WordCraft tests pass when the build input is present (checked locally
on 2026-11-19) and are `#[ignore]`d until we decide how to supply it:

- `wordsmith-layout`: `auto_hyphenation_breaks_long_words`,
  `fragmented_hidden_text_lays_out_like_the_visible_text`,
  `hyphenation_and_proofing_follow_each_runs_language`. They need craft-i18n
  language data (`CRAFT_I18N_DIR`). Revisit in M3 (spelling), which needs the
  same data. Its English dictionaries carry the SCOWL notice, which is not on
  the allowlist, so that needs a decision first.
- `wordsmith-fonts`: `a_thai_run_with_latin_falls_back_to_one_face`. It needs
  a Thai face with OpenType tables (craft-fonts `CRAFT_FONTS_DIR`). Not
  needed for screenplays; revisit if we ship craft-fonts.

## Cut from WordCraft's engine, to revisit

- IME (input method) receipts and composition safeguards. Revisit with the
  editor in M1, so composing text in an input method stays safe.
- Find state and the find commands. Bring them across for find and replace in
  M3 instead of rewriting them.
- MCP tools for opening, saving and typing, and WordCraft's
  `automation_roots` MCP test, wait for file and text commands (M1, M4, M8).

## Known gaps in the tooling

- The CI workflow has never run on a real runner because there is no remote
  yet. Only the macOS gate has been run locally. Copied tests that read system fonts may behave differently on Linux and Windows runners without craft-fonts.
- The minimum Rust version (1.95) is declared but not tested in CI; local and
  CI builds use current stable.

## Imports (M4, decision 0019)

- Emphasis (bold, italic, underline) is not kept by the plain-text, RTF and
  PDF imports; only Fountain and the native format keep it.
- A PDF page cannot show two things: a scene heading with no intro that ends
  in punctuation reads back as Action in capitals, and a paragraph split
  across a page reads back as two paragraphs. Line breaks inside a paragraph
  become spaces.
- PDFs with scene numbers in the margins (M6) and title pages (M5) are not
  handled yet; a scanned PDF is refused (no text recognition).

## Files (M4, decision 0020)

- The MCP bridge has no file commands yet (open, save, import, export live in
  the interface layer); agents save through the window's session only. Saves
  made outside the window make no backup copy. Revisit in M8.
- The web build cannot open or save files.
- The backups folder can be set only by editing `preferences.json`.
- The desktop window's file dialogs, close prompt and auto-save were tested
  headlessly (egui input, made-up clock), not by hand in a running app.

## Structure views (M5)

- Printing index cards and the scene view to PDF (named in public material)
  is not built in v0.1.
- Scene summaries are Summary paragraphs: they print unless outline elements
  are hidden (View › Show Outline Elements, M7).
- The card and scene views were tested headlessly (rendering, keys, moves
  through the commands), not with a real mouse drag in a running window.
- The Tables panel has fixed tables (scenes, characters, notes); adding,
  renaming, reordering and filtering tabs is not built in v0.1.

## Notes (M5, decision 0023)

- Fountain import leaves notes inside text out (Fountain export writes them);
  notes are not printed; threaded replies are not built.

## Production (M6)

- Numbering shots, characters or dialogue is not built; scenes only
  (decision 0025).
- Page locking: relock from the cursor, adjusting a locked page's start,
  editing a page number by hand, locking a new page at the cursor and
  unlocking one page are not built (decision 0026). Lock Pages again adds a
  level instead.

## Outlining (M7, decision 0028)

- Story board images and exporting the board to PDF are not built (ROADMAP).
- Hidden outline elements still show in the normal and speed views.
- The story board and outline views were tested headlessly, not with a real
  mouse.
