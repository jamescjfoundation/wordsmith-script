# Upstream tracking

Wordsmith Script names no upstream project's code. It is built on
[open-source-core](https://github.com/jamescjfoundation/open-source-core), The
James C. J. Foundation's shared base, and the core is the only place that talks
to WordCraft and craft-libs (decision 0044). The core records which WordCraft and
craft-libs commits it builds on, and every patch it carries, in its own
`docs/UPSTREAM.md`.

## The core this app builds on

| Source | Pin | Crates |
| --- | --- | --- |
| open-source-core | tag `v0.1.25` (`Cargo.toml`, `[workspace.dependencies]`) | `jcj-doc`, `jcj-layout`, `jcj-engine`, `jcj-output`, `jcj-interchange`, `jcj-proof`, `jcj-project`, `jcj-features`, `jcj-theme`, `jcj-board`, `jcj-shell`, `jcj-agents` |

All `jcj-` crates move together, to a release tag, with `cargo xtask sync`
afterwards (it copies the core's toolchain, lint, cargo-deny and license
settings, the shared xtask modules, the CI, core-pin and release workflows, the
git hooks and the `[patch]` block into this repository). `cargo xtask layers` fails if any crate
here depends on a WordCraft or craft-libs crate, or on `jcj-upstream`.

While a change in the core is not released yet, a working copy may build
against a local checkout with `cargo xtask dev-link <core>`; `cargo xtask
dev-unlink` removes it. The link lives in a folder Git ignores and changes no
tracked file, and `cargo xtask ci --linked` runs the gate against it.

## Before the rebuild

The first version of the app (its builds were numbered 0.1.0 to 0.1.2,
November 2026) copied WordCraft's model, layout, fonts, rendering, PDF,
engine, control key and MCP crates (commit
`6ef13b2cc8a60e6391d7a5b92459ec47598d58a9`) and two files of its interface
(`f8ec3cc0fa884369c4f3ebbd9ca3085398f5ec31`), and pinned craft-libs at
`93dac78c93a7f1fd66e2022edeb089233b1dc45a` (decisions 0005 and 0039, both
replaced).

That code is not in this repository, and its history was not kept here. It
was written before the app was rebuilt on the core, and that history is not
published: no commit, tag or release of 0.1.x exists, and it cannot be built
from this repository (decision 0047). This repository's history starts with
the rebuild: the core's template, then one commit that brings in the first
version's own screenplay code as it stood at 0.1.2, without the copied
WordCraft crates. What is kept of the first version is its changelog notes,
the decision records, its plans and log in `docs/history/`, and the files its
last build saved, which the tests hold the rebuilt app to
(`docs/fixtures.md`).

The fixes it carried that were candidates to offer upstream
(`Document::app_data`, craft-print's sigstore dependency on Windows) are the
core's business now: extension data is a patch the core carries on its
WordCraft fork, and craft-print's dependency no longer needs changing at the
core's pin.

Build inputs WordCraft's CI supplies (the craft-i18n language data for
dictionaries and hyphenation, craft-fonts' extra script faces) are the core's
too; without them English spelling uses the word list built into craft-proof.
