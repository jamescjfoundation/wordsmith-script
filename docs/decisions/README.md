# Decisions

Choices that `docs/SPEC.md` and `docs/REUSE.md` leave open, one file each,
numbered in the order they were made. Each records the decision, why, and
what would make us revisit it. Decisions about what every app shares live in
open-source-core's `docs/decisions/`; they are cited here as "core decision
0014" with a link, because the two sets of numbers overlap.

## Status

A record's first line gives its date and its status, in the form the core's
records use (`Date: … · Status: … · Amended … (what)`). The status is one of:

- **accepted**: in force as written.
- **replaced by decision NNNN**, or by a core decision: no longer in force;
  the record named is. Kept for its reasoning.
- **general part moved to the core (core decision NNNN)**: the part every app
  shares is the core's now, under that record, and the screenplay part is
  still in force here. The line under the status says which part is which.
- **history only**: nothing in it is in force and no record replaced it. No
  record has this status today.

A record's reasoning is never rewritten. A later change is a dated line under
the status ("Since decision 0044 (2026-11-24) …", "Amended 2026-11-26: …"),
or a new record.

Records from before decision 0044 use the names of their time. `app_data` is
the document's `ext`. `wordsmith-geom`, `-doc`, `-fonts`, `-layout`,
`-render`, `-pdf`, `-engine`, `-mcp` and `-control-key` were Script's copies
of what the core's `jcj-` crates now provide. The fields Script had added to
the copied model are extension keys. Decision 0044 has both tables.

## The records

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-license-allowlist.md) | License allowlist as SPDX ids, and how the check reads expressions | replaced by core decision 0001 |
| [0002](0002-workspace-layers.md) | Workspace layer order and where our own crates sit | general part moved to the core (core decision 0002) |
| [0003](0003-toolchain-and-ci.md) | Toolchain pinning, the CI gate and the lint policy | replaced by core decision 0003 |
| [0004](0004-craft-libs-pins.md) | craft-libs pins when the v0.1.0 tag does not exist | replaced by core decision 0004 |
| [0005](0005-copying-wordcraft.md) | How WordCraft code is copied, renamed and marked | replaced by core decision 0004 |
| [0006](0006-engine-trim.md) | Trimming WordCraft's engine to session, undo, registry and recovery | replaced by core decision 0007 |
| [0007](0007-mcp-trim.md) | Trimming WordCraft's MCP server | replaced by core decision 0007 |
| [0008](0008-app-data.md) | Screenplay data lives in the document's `app_data` | replaced by core decision 0006 |
| [0009](0009-element-model.md) | The element model and its defaults | accepted |
| [0010](0010-typing-rules.md) | Typing rules the spec leaves open | accepted |
| [0011](0011-editor.md) | The first editor (egui, page view, focus, recovery) | general part moved to the core (core decision 0014) |
| [0012](0012-pagination.md) | Screenplay pagination | accepted |
| [0013](0013-printing.md) | PDF export and printing | general part moved to the core (core decision 0007) |
| [0014](0014-spelling-data.md) | Spelling data (Moby list, no SCOWL) | general part moved to the core (core decision 0007) |
| [0015](0015-autocomplete.md) | Autocomplete | accepted |
| [0016](0016-views.md) | Views, theme and writing modes | general part moved to the core (core decision 0014) |
| [0017](0017-native-format.md) | The native format | accepted |
| [0018](0018-fountain.md) | Fountain reading and writing | accepted |
| [0019](0019-import-rules.md) | Import classification rules (and the exports made to read back) | accepted |
| [0020](0020-saving.md) | Saving, auto-save, backups and preferences | general part moved to the core (core decisions 0009 and 0014) |
| [0021](0021-title-page-and-headers.md) | Title page, headers and footers | general part moved to the core (core decision 0010) |
| [0022](0022-scenes-and-tables.md) | Index cards, scene view and tables | general part moved to the core (core decision 0014) |
| [0023](0023-notes.md) | Notes | general part moved to the core (core decision 0010) |
| [0024](0024-revisions.md) | Revisions | general part moved to the core (core decision 0010) |
| [0025](0025-scene-numbers.md) | Scene numbers | accepted |
| [0026](0026-page-locking.md) | Page locking and the page-label scheme | general part moved to the core (core decision 0010) |
| [0027](0027-reports-and-revised-pages.md) | Revised pages, scene lengths and the character report | accepted |
| [0028](0028-outlining.md) | Outlining: story board, lanes, outline elements | general part moved to the core (core decision 0014) |
| [0029](0029-release.md) | The command line, the MCP server and the release | general part moved to the core (core decision 0007) |
| [0030](0030-spec-2.md) | SPEC-2 and what v0.1 built | accepted |
| [0031](0031-editing-additions.md) | Editing additions from SPEC-2 (Sequence, typing options, keys, bookmarks, checker) | general part moved to the core (core decision 0010) |
| [0032](0032-macros-and-styles.md) | Macros and text styles (keys, starter set, aliases, native format 4) | accepted |
| [0033](0033-characters-and-reports.md) | Characters, scene data and reports | accepted |
| [0034](0034-planning.md) | Boards, the structure strip and planning PDFs | accepted |
| [0035](0035-production.md) | Production additions (dual dialogue, locks, numbers, printing, watermark, tracking, comparison) | general part moved to the core (core decision 0010) |
| [0036](0036-workspace.md) | Workspace and productivity (templates, tabs, split, statistics, sprints, keys) | general part moved to the core (core decision 0014) |
| [0037](0037-ribbon-and-panels.md) | The ribbon, side panels and what is reused for them | general part moved to the core (core decision 0014) |
| [0038](0038-releases-and-updates.md) | Releases, downloads and updates (craft-update, release files, draft releases) | general part moved to the core (core decision 0014) |
| [0039](0039-a-fork-of-wordcraft.md) | A fork of WordCraft: the copied crates are trimmed to what a screenplay uses | replaced by core decision 0004 |
| [0040](0040-supply-chain-and-licenses.md) | Supply chain and licenses: exact toolchain, minimum-version check, cargo-deny, Dependabot, SHA-pinned actions, build provenance, license texts in bundles, privacy disclosure | general part moved to the core (core decisions 0001 and 0003) |
| [0041](0041-our-names-for-panel-and-view.md) | Our names for the side panel and the compact view (Script panel, Draft view) | accepted |
| [0042](0042-scope-and-asking-first.md) | Scope, and changes that need the maintainers first | accepted |
| [0043](0043-key-tips-and-commands.md) | Key tips that reach every control and keep their letters; every ribbon and status bar control runs a command | general part moved to the core (core decision 0014) |
| [0044](0044-rebuilt-on-open-source-core.md) | Rebuilt on open-source-core: what moved to the core, what stays, the `.wss` format 5 and the differences | accepted |
