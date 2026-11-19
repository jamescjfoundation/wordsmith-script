# 0004: craft-libs pins without a v0.1.0 tag

Date: 2026-11-19

## Context

`docs/REUSE.md` rule 1 says to depend on the five craft-libs v0.1.0 crates
(craft-atomic, craft-settings, craft-quit, craft-print, craft-ui) with
`tag = "v0.1.0"`, and on every other craft-libs crate with one shared `rev`.
On 2026-11-19 the craft-libs repository has no tags at all. The v0.1.0
release exists only as a `CHANGELOG.md` entry, prepared in commit
`86ed498b4` (2026-11-12). A `tag = "v0.1.0"` dependency cannot resolve.

Mixing two revisions of one git repository is also risky in its own right:
Cargo treats a crate from each revision as a different package, so a crate
at the release commit and a crate at `93dac78c` that both use, say,
craft-settings would get two incompatible copies.

## Decision

Pin every craft-libs crate, including the patched `krilla`, to the one
shared rev `93dac78c93a7f1fd66e2022edeb089233b1dc45a`, the commit WordCraft
pins. None of the five release crates are used yet. When the UI work in M1
needs them, use the same shared rev, and re-check whether a `v0.1.0` tag has
appeared by then.

## Revisit when

craft-libs publishes a real `v0.1.0` tag. Then decide on purpose whether to
move the five release crates to it, keeping in mind the two-revisions problem
above.
