# Wordsmith Script: what to reuse from ArtCraft

As of 2026-11-19

Since 2026-11-22 the copied WordCraft crates are a trimmed fork rather than a
copy kept in step with upstream: what a screenplay cannot use was cut, and
fixes are ported by hand (decision 0039). The plan below is kept as written.

What this repository takes from these projects is code: crates copied from
WordCraft and pinned craft-libs crates, each under its own MIT OR Apache-2.0
license. `UPSTREAM.md` lists every copy and pin, and `ATTRIBUTION.md` gives
the credits. No other storytold project's code, text or tooling is used.

## Summary

Wordsmith Script should be built as a fork-and-trim of WordCraft's engine, sitting on craft-libs, with our own screenplay layer and our own interface on top. WordCraft already has most of the hard parts. Its paragraph styles carry a "next style", which is the same idea as a screenplay element's next element. It also has keep-with-next, widow control, page-break-before, track changes, comments, a pagination engine, PDF export with real text, undo and a command system. I estimate this removes the bulk of months 1 to 4 of the parity plan; that is a judgment, not a measurement.

- **Use as pinned dependencies:** craft-libs crates such as craft-ui, craft-settings, craft-atomic, craft-quit, craft-print, craft-dialogs, craft-keymap, craft-menu, craft-proof, craft-pdf, craft-rtf, craft-markdown, craft-html, craft-i18n and craft-ai-core.
- **Copy in and modify (with attribution):** WordCraft's geom, doc, fonts, layout, render and pdf crates, plus the session, undo and command-registry parts of its engine. We need to change pagination for (MORE) and (CONT'D), so a plain dependency is not enough.
- **Build ourselves:** the screenplay element layer, the Return/Tab rules, autocomplete lists, scene model, production tools (revision colors, page locking, A-pages), story board and outliner, Fountain support, and the whole user interface.

The license allows all of this. Everything used is MIT OR Apache-2.0 at our choice, and the only restriction is the ArtCraft name, wordmark and logos, which we will not use. We may say in plain text that parts are "based on WordCraft by the ArtCraft team".

All facts below come from the repositories' own README, NOTICE, Cargo and agent files, read on 2026-11-19.

## Repo inventory

Four storytold repositories matter directly and five more are useful references; the other 50 or so (photo, video, CAD, audio, 3D, older Unreal experiments) are not relevant. All app and library code is Rust, egui 0.36, Rust 1.95+, edition 2024.

| Repo | What it gives us | License | State |
| --- | --- | --- | --- |
| [wordcraft](https://github.com/storytold/wordcraft) | Full word processor: document model, layout and pagination, fonts and shaping, PDF, rasterizer, engine with undo and 422 commands, MCP server, CLI, web build. About 117,000 lines of Rust in its crates | MIT OR Apache-2.0; ArtCraft marks excluded | Most mature text app in the suite. Pins craft-libs at commit 93dac78c |
| [craft-libs](https://github.com/storytold/craft-libs) | 196 shared crates: UI widgets and docking, settings, atomic saves, print, dialogs, menus, keymaps, spelling and hyphenation, PDF, RTF, HTML, Markdown, ODF, DOCX, fonts, i18n, AI contract, test kit | MIT OR Apache-2.0 | Only 5 crates in the v0.1.0 release (2026-10-12): craft-atomic, craft-settings, craft-quit, craft-print, craft-ui. All others are pinned by commit |
| [craft-fonts](https://github.com/storytold/craft-fonts) | Open-licensed font files and build embedding; includes Liberation Mono | Code MIT OR Apache-2.0; fonts OFL-1.1 and others per ATTRIBUTION.md | No Courier Prime; we add it there or in our repo |
| [craft-i18n](https://github.com/storytold/craft-i18n) | Translation catalogs plus spelling dictionaries and hyphenation patterns used by craft-proof | Code MIT OR Apache-2.0; word lists carry their own notices (SCOWL, public domain) | Active |
| [craft-anything](https://github.com/storytold/craft-anything) | Agent tooling for researching an existing product | MIT OR Apache-2.0 | Early alpha. Not used; its modes that inspect a product conflict with our clean-room rules |
| [craft-contrib](https://github.com/storytold/craft-contrib) | House rules, RFCs, standards (accessibility, time, fonts) | No license file | Read for conventions; do not copy text |
| [craft-ai-integrations](https://github.com/storytold/craft-ai-integrations) | AI connections, keys, providers, egui widgets | Apache-2.0 | First draft |
| [craft-extensions](https://github.com/storytold/craft-extensions) | Themes and plug-in package contract | Apache-2.0 | Provisional |
| [deckcraft](https://github.com/storytold/deckcraft), [pdfcraft](https://github.com/storytold/pdfcraft) | Reference apps for canvas-style boards and PDF reading | Apache-2.0 | Reference only |

Repos with no license file (craft-common-actions, craft-skills, craft-wiki, craft-contrib) are readable but not reusable: we cannot copy their code or text, only follow their ideas.

## Reuse map

Each row is a feature from the [clean-room spec](SPEC.md), the piece that covers it, and what we still write. "Copy" means vendor the crate into our repo and modify it; "Dep" means a pinned dependency we do not edit.

| Spec feature | Reuse | How | What we still build |
| --- | --- | --- | --- |
| Paragraph list with one element type each | wordcraft-doc (paragraphs, styles, runs) | Copy | Element registry that maps each element to a paragraph style; scene index derived from Scene Headings |
| Per-element formatting: font, indents, alignment, spacing, space before | wordcraft-doc style properties | Copy | Screenplay templates (screenplay, teleplay, stage play) with our own default values |
| Next element on Return | wordcraft-doc style field `next` (Word's "style for following paragraph") | Copy | Return handler that reads it; per-script editing UI |
| Tab switching rules, scene-heading separators | none | Build | Tab state machine and separator insertion |
| Start new page, keep Character with Dialogue, widows | wordcraft-doc `page_break_before`, `keep_next`, `keep_lines`, `widow_control`; wordcraft-layout pagination | Copy | Screenplay keep rules as style settings |
| (MORE) and (CONT'D), scene (CONTINUED) | wordcraft-layout pagination | Copy and extend | A pass that splits dialogue at sentence ends and inserts the continued text; this is the main reason to copy, not depend |
| Page geometry, page numbers, headers and footers | wordcraft-geom (inches, points), wordcraft-layout headers and fields | Copy | Header fields for scene, revision set, date |
| Fonts and shaping | wordcraft-fonts, craft-font-catalog, craft-fonts | Copy / Dep | Bundle Courier Prime (OFL-1.1) and lock 10 characters per inch metrics in tests |
| PDF export with selectable text | wordcraft-pdf on craft-pdf | Copy / Dep | Revision page colors, watermark, title page |
| Printing | craft-print | Dep (v0.1.0) | Nothing beyond the print dialog wiring |
| Undo, commands, keyboard shortcuts | wordcraft-engine session, undo and command registry; craft-keymap; craft-menu | Copy (trimmed) / Dep | Screenplay command catalog; drop Word's 422-command catalog |
| Autocomplete lists | none | Build | Six lists, rebuild, suggestion UI |
| Spelling, hyphenation | craft-proof with craft-i18n data | Dep | Script-aware ignore rules (character names, sluglines) |
| Revision mode and colored revision sets | wordcraft-doc revisions (track changes) | Copy | Revision sets with the color cycle, margin asterisks, page colors |
| Page locking, A-pages, scene numbering, OMITTED | none | Build | Lock levels and page-label scheme on top of the pagination pass |
| Notes on script text | wordcraft-doc comments | Copy | Notes panel |
| Index cards, side panel tables, story board, outliner | craft-ui panes, tabs, docking | Dep (v0.1.0) | All views |
| Native file, auto-save, backups | craft-atomic, craft-settings, wordcraft-engine recovery | Dep / Copy | Our own file format; Fountain reader and writer |
| RTF, plain text, HTML, Markdown, DOCX import and export | craft-rtf, craft-html, craft-markdown, craft-docx, wordcraft-formats | Dep / Copy | Mapping between screenplay elements and those formats; text-PDF import via craft-pdf's reader |
| Translations | craft-i18n | Dep | Our own string catalog |
| Agent control and AI features | wordcraft-mcp, craft-ai-core, craft-flags | Copy / Dep | Screenplay tools exposed over MCP; any AI stays off by default |
| Tests and CI gate | craft-testkit (hostile-input fuzzing), craft-xtask | Dep | Golden PDF tests for pagination |

Not worth taking: most of wordcraft-ui-egui (a 106,000-line Word-style ribbon; our interface is different; v0.3 copies only its ribbon fitting and icon set, decision 0037), wordcraft-docbin and wordcraft-zotero.

## Architecture

One Cargo workspace, layered so that nothing below the interface depends on it, the same rule WordCraft follows. Each layer only calls the layers under it.

Layers, top to bottom (B = we build, C = copied from WordCraft and modified, D = pinned craft-libs dependency):

| Layer | Crates |
| --- | --- |
| Apps | wordsmith, wordsmith-cli, wordsmith-web (B) |
| Interface | wordsmith-ui: editor, cards, side panels, story board (B); craft-ui, craft-menu, craft-dialogs, craft-quit (D) |
| Screenplay | wordsmith-script: elements, Return and Tab rules, autocomplete, scenes, revision sets, page locks (B) |
| Engine | wordsmith-engine: session, undo, command registry (C); wordsmith-mcp: agents drive every command (C) |
| Files | wordsmith-format: native file, Fountain (B); wordsmith-pdf (C); craft-rtf, craft-html, craft-markdown, craft-docx (D) |
| Layout | wordsmith-layout: pagination plus (MORE) and (CONT'D) (C); wordsmith-fonts, wordsmith-render (C) |
| Model | wordsmith-doc, wordsmith-geom: paragraphs, styles with next style, revisions, comments, units (C) |
| Platform | craft-settings, craft-atomic, craft-print, craft-proof, craft-i18n, craft-keymap (D) |

The new code concentrates in two places: wordsmith-script holds every screenplay rule from the spec, and wordsmith-layout gets the (MORE)/(CONT'D) pagination pass. Everything else is copied or pinned.

## Rules for contributors

These rules keep the reuse legal and the build reproducible.

1. **Pin everything.** craft-libs crates in the v0.1.0 release use `tag = "v0.1.0"`. All other craft-libs crates use one shared `rev` (start from 93dac78c, the commit WordCraft pins, so the copied WordCraft code compiles). Commit `Cargo.lock` and build with `--locked`. Upgrade all pins together, on purpose.
2. **Copied WordCraft code.** Put it under `crates/` with its original file headers. Keep WordCraft's copyright line in our NOTICE: "Portions copyright (c) 2026 ArtCraft Team and the WordCraft contributors, MIT OR Apache-2.0." Mark changed files as modified, as Apache-2.0 section 4 requires. Record the source commit for each copied crate in `UPSTREAM.md`, so fixes can be pulled across later.
3. **No ArtCraft branding.** Delete `docs/brand/`, every ArtCraft logo, the getartcraft.com links, the WordCraft app icon and the Discord badges from anything copied. Do not use "Craft" in our product name, crate names or icon. Plain-text credit "based on WordCraft by the ArtCraft team" is allowed and goes in About and the README.
4. **Rename on copy.** Copied crates become `wordsmith-geom`, `wordsmith-doc` and so on. Keep craft-libs crate names unchanged because we do not copy them.
5. **Assets.** Only original or openly licensed assets, recorded in `ATTRIBUTION.md` in the same change, using the storytold allowlist (MIT, Apache-2.0, BSD, ISC, OFL-1.1, CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0, public domain). Fonts and dictionaries keep their own license texts.
6. **Follow the house engineering rules.** No panics or `unsafe` in non-test code, hostile-input parsers, bounded recursion, time through `chrono`. These come from storytold's rules and are good practice; we adopt them as our own, written in our words.
7. **Clean room still applies.** WordCraft is a word processor in the style of Microsoft Word, so its code contains Word's names (ribbon labels, command ids, the 422-command catalog). Strip what we do not need. Screenplay behavior comes only from our [clean-room spec](SPEC.md), never from another screenwriting program or from memory of one.
8. **Never inspect a commercial screenwriting product,** by hand or with a tool such as craft-anything's inspection modes. Public documents are the only source for its behavior.
9. **Contribute fixes upstream when they are general.** A pagination or PDF fix that is not screenplay-specific goes back to WordCraft or craft-libs as a pull request, following their contribution rules.

## Gaps, risks and sources

**Risks:**

- **WordCraft moves fast.** Copied code drifts from upstream. `UPSTREAM.md` plus a monthly diff review keeps fixes flowing.
- **Pagination change is the hardest piece.** Dialogue splitting with (MORE)/(CONT'D) touches the core of wordcraft-layout. Do it first, behind golden PDF tests, before building views on top.
- **Word baggage.** wordcraft-doc models a lot of Word (sections, fields, math, drawings). Keep it for now and trim later; cutting it early risks breaking layout. (Trimmed in decision 0039: math, charts, pictures and other objects a screenplay can't hold are gone.)
- **Most craft-libs crates are unreleased.** Only 5 are in v0.1.0, so commit pins can break on upgrade. Upgrade in one step with tests.
- **Early-stage suite.** ArtCraft launched in October 2026 and much of it is AI-built alpha code. Read and test what we take; do not assume it is production-ready.
- **Brand terms can change.** Re-read WordCraft's NOTICE and `docs/brand/LICENSE-brand.txt` before each release.

**Not found in the repos, so we build from scratch:** Tab rules, autocomplete lists, page locking and A-pages, revision color sets, scene numbering, story board and outliner, Fountain, and a Courier-style font (Liberation Mono is in craft-fonts but Courier Prime is not).

**Sources** (read 2026-11-19):

- [wordcraft README](https://github.com/storytold/wordcraft/blob/main/README.md), [NOTICE](https://github.com/storytold/wordcraft/blob/main/NOTICE), [brand license](https://github.com/storytold/wordcraft/blob/main/docs/brand/LICENSE-brand.txt), [AGENTS.md](https://github.com/storytold/wordcraft/blob/main/AGENTS.md), [Cargo.toml](https://github.com/storytold/wordcraft/blob/main/Cargo.toml)
- [craft-libs README](https://github.com/storytold/craft-libs/blob/main/README.md), [CHANGELOG](https://github.com/storytold/craft-libs/blob/main/CHANGELOG.md)
- [craft-fonts](https://github.com/storytold/craft-fonts), [craft-i18n](https://github.com/storytold/craft-i18n), [craft-anything](https://github.com/storytold/craft-anything), [craft-contrib](https://github.com/storytold/craft-contrib)
- [storytold organization](https://github.com/storytold) for the full repo list and licenses
