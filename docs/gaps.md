# Gaps

Spec items from `docs/SPEC.md` (Verified and Sourced, milestones M1 to M7)
that are not built yet, and anything else we know is missing. An item leaves
this list when it ships with tests, or stays with a reason if v0.1 will not
cover it.

## Not started

Everything in M1 to M8. Each milestone plan in `docs/plans/` breaks its part
of the spec into items that are tracked here as they land.

## Known gaps in the tooling

- The CI workflow has never run on a real runner because there is no remote
  yet. Only the macOS gate has been run locally.
- The minimum Rust version (1.95) is declared but not tested in CI; local and
  CI builds use current stable.
