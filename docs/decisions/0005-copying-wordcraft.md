# 0005: How WordCraft code is copied

Date: 2026-11-19

## Decision

- **What and from where.** `geom`, `doc`, `fonts`, `layout`, `render` and
  `pdf` from WordCraft `6ef13b2` (2026-11-18), tracked files only. The
  engine parts and `mcp` come in a separate branch, because the engine has to
  be trimmed rather than copied whole.
- **Renaming.** Package names become `wordsmith-*`, and every `wordcraft_*`
  path for these six crates becomes `wordsmith_*`. WordCraft's alias
  `wordcraft-proof` goes back to the real crate name, `craft-proof`, because
  craft-libs names stay unchanged. Comments that describe the code's WordCraft
  origin are kept as they are.
- **Marking.** Every file that differs from upstream starts with
  `// Modified by the Wordsmith Script contributors.` (`#` in TOML). Files we
  did not change are byte-identical to upstream.
- **Branding.** The PDF writer's creator and producer metadata now say
  "Wordsmith Script". One golden-hash test was re-recorded for that change,
  and the old value is kept in a comment.
- **Formatting and lints.** The workspace uses WordCraft's rustfmt settings
  (width 150, `use_small_heuristics = "Max"`) and adds its clippy settings, so
  copied code is not reformatted and keeps the standard it was written to.
  The six panicking lints are denied at each product crate root, not in the
  workspace lint table, so integration tests under `tests/` may unwrap, as
  they do upstream. `cargo xtask lint-roots` enforces the crate-root
  attributes. Four copied crate roots gained `#![forbid(unsafe_code)]`.
- **Lockfile.** `Cargo.lock` started from WordCraft's lock, so dependency
  versions match what upstream tests against.
- **Tests that need missing build inputs** are ignored with a reason that
  points to `docs/gaps.md`. They are not deleted.

## Revisit when

We pull fixes across from upstream: diff against the commit in `UPSTREAM.md`,
apply the same renames, and keep the marks.
