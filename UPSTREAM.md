# Upstream tracking

Where our code comes from, which exact versions we pin, and fixes worth
offering back. Upgrade every pin together, on purpose, with tests.

## Copied crates

Copied from WordCraft (https://github.com/storytold/wordcraft), MIT OR
Apache-2.0. Each copied file keeps its header; changed files add
"Modified by the Wordsmith Script contributors".

| Our crate | Upstream path | Upstream commit | Copied on | Notes |
| --- | --- | --- | --- | --- |
| wordsmith-geom | `crates/geom` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed only |
| wordsmith-doc | `crates/doc` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed only |
| wordsmith-fonts | `crates/fonts` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed; one test ignored (needs a Thai face from craft-fonts) |
| wordsmith-layout | `crates/layout` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed; three tests ignored (need craft-i18n language data) |
| wordsmith-render | `crates/render` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed only |
| wordsmith-pdf | `crates/pdf` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Renamed; PDF creator and producer say Wordsmith Script; one golden hash re-recorded |
| wordsmith-engine | `crates/engine` (parts) | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Trimmed to session, undo, registry, recovery and file-access confinement; see `docs/decisions/0006-engine-trim.md`. `file_access.rs` and `recovery/desktop.rs` are upstream with one-line changes; the rest is cut down from upstream files |
| wordsmith-control-key | `crates/control-key` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Profile folder and variables renamed |
| wordsmith-mcp | `crates/mcp` | `6ef13b2cc8a60e6391d7a5b92459ec47598d58a9` | 2026-11-19 | Tools trimmed, one document per session, rebranded; see `docs/decisions/0007-mcp-trim.md` |

## Pinned dependencies

| Source | Pin | Crates |
| --- | --- | --- |
| craft-libs | `rev = "93dac78c93a7f1fd66e2022edeb089233b1dc45a"` (the commit WordCraft pins) | every craft-libs crate we use, plus the patched `krilla` and the patched Hayro reader crates (`hayro-syntax`, `hayro-interpret`, `hayro-cmap`, `hayro-postscript`, `hayro-jbig2`) that craft-pdf's reader needs |
| craft-libs v0.1.0 | no git tag exists; see `docs/decisions/0004-craft-libs-pins.md` | craft-atomic, craft-settings, craft-quit, craft-print, craft-ui (not used yet) |

Build inputs WordCraft's CI supplies and we do not yet (see `docs/gaps.md`):

| Input | WordCraft's pin | Used for |
| --- | --- | --- |
| craft-i18n (`CRAFT_I18N_DIR`) | `e56d26244bbac099bfd42e23fb12f78a4be418a7` | spelling dictionaries and hyphenation patterns in craft-proof |
| craft-fonts (`CRAFT_FONTS_DIR`) | `b076417b5880bd4154bd22f00695b1d28c6bea83` | extra script faces (CJK, Arabic, Thai) |

Everything the kickoff lists for the bootstrap is now copied.

## Candidates to contribute back

General fixes (not screenplay-specific) that should be offered to WordCraft or
craft-libs, following their contribution rules. We do not contact upstream
projects from here; a person decides when to send these.

| Fix | Our commit | Upstream project | Status |
| --- | --- | --- | --- |
| `Document::app_data`: application data kept with a document, included in equality, `same_content` and serialization, `Arc`-shared values (decision 0008) | m1/doc-app-data | WordCraft (`crates/doc`) | candidate |
| craft-print on Windows pulls winprint → sigstore → reqwest → webpki-root-certs (CDLA-Permissive-2.0) even with winprint's `pdfium` feature off; make sigstore optional (decision 0013) | m2/pdf | craft-libs (`crates/print`) and storytold/winprint.rs | candidate |

Not used: craft-settings. It depends on `dirs` → `option-ext` (MPL-2.0, outside
the licence allowlist); preferences are a small JSON file of our own instead
(decision 0020).
