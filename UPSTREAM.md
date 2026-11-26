# Upstream tracking

Since 0.2, Wordsmith Script names no upstream project's code. It is built on
[open-source-core](https://github.com/jamescjfoundation/open-source-core), the
J. C. J. Foundation's shared base, and the core is the only place that talks to
WordCraft and craft-libs (decision 0044). The core records which WordCraft and
craft-libs commits it builds on, and every patch it carries, in its own
`docs/UPSTREAM.md`.

## The core this app builds on

| Source | Pin | Crates |
| --- | --- | --- |
| open-source-core | tag `v0.1.18` (`Cargo.toml`, `[workspace.dependencies]`) | `jcj-doc`, `jcj-layout`, `jcj-engine`, `jcj-output`, `jcj-interchange`, `jcj-proof`, `jcj-project`, `jcj-features`, `jcj-theme`, `jcj-board`, `jcj-shell`, `jcj-agents` |

All `jcj-` crates move together, to a release tag, with `cargo xtask sync`
afterwards (it copies the core's toolchain, lint, cargo-deny and license
settings, the shared xtask modules, the CI workflow, the git hooks and the
`[patch]` block into this repository). `cargo xtask layers` fails if any crate
here depends on a WordCraft or craft-libs crate, or on `jcj-upstream`.

While a change in the core is not released yet, a working copy may point at a
local checkout with `cargo xtask dev-link <core>`; `cargo xtask dev-unlink`
removes it, and CI fails if the link is committed.

## Before 0.2

0.1.x copied WordCraft's model, layout, fonts, rendering, PDF, engine, control
key and MCP crates (commit `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9`) and two
files of its interface (`f8ec3cc0fa884369c4f3ebbd9ca3085398f5ec31`), and pinned
craft-libs at `93dac78c93a7f1fd66e2022edeb089233b1dc45a` (decisions 0005 and
0039, both replaced). That code is in the history of this repository up to the
`v0.1.2-final` tag. The fixes it carried that were candidates to offer upstream
(`Document::app_data`, craft-print's sigstore dependency on Windows) are the
core's business now: extension data is a patch the core carries on its
WordCraft fork, and craft-print's dependency no longer needs changing at the
core's pin.

Build inputs WordCraft's CI supplies (the craft-i18n language data for
dictionaries and hyphenation, craft-fonts' extra script faces) are the core's
too; without them English spelling uses the word list built into craft-proof.
