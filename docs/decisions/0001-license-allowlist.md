# 0001: License allowlist as SPDX ids

Date: 2026-11-19

Status: replaced by open-source-core decision 0001 (licenses and supply chain), whose allowlist and `deny.toml` the app now syncs (decision 0044).

## Context

`docs/REUSE.md` rule 5 allows MIT, Apache-2.0, BSD, ISC, OFL-1.1, CC0-1.0,
CC-BY-4.0, CC-BY-SA-4.0 and public domain. Cargo manifests state licenses as
SPDX expressions, so the check needs exact ids.

## Decision

`cargo xtask attribution` accepts these SPDX ids:

| Allowlist entry | SPDX ids |
| --- | --- |
| MIT | `MIT` |
| Apache-2.0 | `Apache-2.0`, also `Apache-2.0 WITH LLVM-exception` (the exception only adds permissions) |
| BSD | `BSD-2-Clause`, `BSD-3-Clause`, `0BSD` |
| ISC | `ISC` |
| OFL-1.1 | `OFL-1.1` |
| CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0 | same ids |
| Public domain | `Unlicense` (a public-domain dedication) |

Expressions are evaluated properly: `A OR B` passes if either side is
allowed, `A AND B` only if both are. The older `A/B` form means OR. Anything
else, including a missing license field, fails the check.

Not on the list, so they stop the build and need a person's decision:
`MPL-2.0`, any GPL or LGPL, and any custom license file. When the
build needs one of these only as an OR alternative, it passes without asking.

cargo-deny (`deny.toml`, decision 0040) checks the same list, with the reviewed
exceptions below, in CI. Change both together.

## Reviewed exceptions

Approved by the maintainers on 2026-11-19, when the WordCraft copy
(decision 0005) first brought them in. All five are short permissive
licenses, and the build cannot do without them; the table says when each was
approved and for which packages. They are listed as
`REVIEWED_EXCEPTIONS` in `xtask/src/spdx.rs`.

| License | Packages that needed it (2026-11-19) | Condition |
| --- | --- | --- |
| `Zlib` | zlib-rs (compression under flate2, used by image codecs, zip and the PDF test reader); foldhash (hashing under hashbrown) | none beyond keeping notices in source distributions |
| `Unicode-3.0` | unicode-ident (needed by every Rust derive macro); unicode-normalization (craft-proof); yoke, zerofrom (krilla PDF writer, hayro test reader) | keep the notice with distributed data and software |
| `BSL-1.0` | clipboard-win, error-code (Windows clipboard, through arboard and egui-winit; approved 2026-11-19 when the editor was first built) | none for binaries; keep the notice in source distributions |
| `CDLA-Permissive-2.0` | webpki-root-certs (Mozilla's root certificates, through rustls-platform-verifier and ureq in craft-update; approved 2026-11-20 for updates, decision 0038) | the certificate data is shared under this license; `NOTICE` says so and links its text |
| `IJG` | jpeg-encoder (craft-image-codecs, reached from wordsmith-geom) | the documentation must say the software is based in part on the work of the Independent JPEG Group; this is in `NOTICE` |

The SCOWL and Kuenning notices on craft-i18n's English spelling dictionaries
need no exception: decision 0014 ships spelling without those dictionaries,
so the language data is not built in.

## Revisit when

Re-check the reviewed exceptions whenever pins are upgraded, in case new
packages start depending on them. A needed dependency carries a license outside this list. That is a stop
rule: ask the person before adding an exception, and record the answer here.
