# Gaps

Spec items from `docs/SPEC.md` (Verified and Sourced, milestones M1 to M7)
that are not built yet, and anything else we know is missing. An item leaves
this list when it ships with tests, or stays with a reason if v0.1 will not
cover it.

## Not started

Everything in M1 to M8. Each milestone plan in `docs/plans/` breaks its part
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
