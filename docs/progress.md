# Progress

One line per merge to `main`, newest last.

| Date | Branch | What changed |
| --- | --- | --- |
| 2026-11-19 | bootstrap/skeleton | Workspace skeleton, licenses, NOTICE, ATTRIBUTION, UPSTREAM, `cargo xtask ci` gate, CI workflow, decisions 0001-0003 |
| 2026-11-19 | bootstrap/wordcraft-copy | Copied WordCraft geom, doc, fonts, layout, render, pdf (6ef13b2) as wordsmith-*; craft-libs pinned to 93dac78c; Zlib, Unicode-3.0, IJG allowed as reviewed exceptions; 1,460 tests pass, 4 ignored for missing build inputs |
| 2026-11-19 | bootstrap/engine | wordsmith-engine trimmed from WordCraft's engine (session, undo, registry, recovery, file access); 1,502 tests pass |
| 2026-11-19 | bootstrap/mcp | wordsmith-mcp and wordsmith-control-key from WordCraft, trimmed and rebranded; bootstrap complete; 1,528 tests pass |
| 2026-11-19 | m1/doc-app-data | M1 plan; Document::app_data so per-script data is undoable (decision 0008); 1,532 tests pass |
| 2026-11-19 | m1/elements | Element registry, screenplay template, scene index, Courier Prime bundled and measured; 1,551 tests pass |
| 2026-11-19 | m1/typing | Typing, Return, Tab with separators, change element with Mod+0..9, casing, caret and selection commands; 1,568 tests pass |
| 2026-11-19 | m1/element-settings | Element settings, custom elements, font for all, Dialogue-then-Character option, scene-heading settings; 1,576 tests pass |
| 2026-11-19 | m1/editor | Desktop editor on egui 0.36: page view, keyboard and mouse through the registry, element selector, palette, element settings dialog, crash recovery; M1 complete; 1,587 tests pass |
| 2026-11-19 | m2/paginate | Screenplay paginator: keep rules, sentence splits, (MORE)/(CONT'D), automatic CONT'D, scene CONTINUED option, page numbers; layout agrees with every plan; 150 pages in 73-99 ms; 1,602 tests pass |
| 2026-11-19 | m2/page-view | Page view shows the paginated script; incremental repagination; typing at 150 pages 14.5 ms median per keystroke frame; 1,604 tests pass |
| 2026-11-19 | m2/pdf | PDF export with selectable text from the page view's layout; printing via CUPS lp (Windows: system viewer) instead of craft-print; Export PDF dialog; 1,609 tests pass |
| 2026-11-19 | m2/golden | Fountain reader; three public-domain photoplays as golden scripts (page breaks and PDF text); more/continued settings commands; M2 complete; 1,616 tests pass |
| 2026-11-19 | m3/autocomplete | Six autocomplete lists, learning, rebuild, editing, options, suggestion list in the editor; 1,626 tests pass |
| 2026-11-19 | m3/spelling | Spelling (Moby list) skipping character names, intros and capitals; underlines and right-click fixes; 1,632 tests pass |
| 2026-11-19 | m3/find | Find and replace (case, whole word, one-step Replace All), find bar with highlights; 1,636 tests pass |
| 2026-11-19 | m3/views | Normal, page and speed views, dark theme, typewriter scrolling, focus mode; M3 complete; typing at 150 pages 16.3 ms median; 1,638 tests pass |
| 2026-11-19 | m4/native-format | Native .wss format with migrations, documented in docs/format.md; exact round trips and stable bytes; 1,644 tests pass |
| 2026-11-19 | m4/fountain | Fountain writer; emphasis kept both ways; sections, synopses and lone notes read as outline elements; golden and generated round trips; 1651 tests pass |
| 2026-11-19 | m4/import | Plain text, RTF and text-PDF import with the is-it-a-script choice and a guess; PDFs this app prints come back as the same elements; 1661 tests pass |
| 2026-11-19 | m4/export | Plain text, RTF and HTML export from the element styles, no mores or continueds; golden scripts come back from text and RTF; 1667 tests pass |
| 2026-11-19 | m4/save | Open, import, save, save as, exports in the editor; unsaved-changes prompt; auto-save (15 min, 10 s idle); timestamped backups; preferences file; all writes atomic; M4 complete; 1681 tests pass |
| 2026-11-19 | m5/title-page | Title page fields, window and printed page before the script (numbered from 1 after it); Fountain and PDF import read it; `wordsmith-cli convert`; 1687 tests pass |
| 2026-11-19 | m5/headers | Header and footer bands with {page}, {scene}, {date}, {title}, {revision}; first-page rule; window; golden PDFs unchanged; 1692 tests pass |
| 2026-11-19 | m5/scenes | Scene commands (list, move, heading, summary, go to); index cards and scene view with drag to reorder; 1697 tests pass |
| 2026-11-19 | m5/tables | Tables panel (scenes, characters) with cell editing, row reorder and CSV; character rename everywhere; 1702 tests pass |
| 2026-11-19 | m5/notes | Notes anchored by a text mark; commands, page view tint, popover, Notes table; native format 2 with migration; Fountain writes notes; M5 complete; 1709 tests pass |
| 2026-11-19 | main | Typing at 150 pages measured at the end of M5: median 14.7 to 15.6 ms |
| 2026-11-19 | m6/revisions | Revision mode and sets on the color cycle; marks beside revised lines; set styles; page tints and colored PDF pages; native format 3; 1718 tests pass |
| 2026-11-19 | m6/scene-numbers | Scene numbers shown or hidden, locked with letter suffixes, OMITTED and recall; Fountain #n#; golden PDFs re-recorded with numbers (diff checked); 1723 tests pass |
| 2026-11-19 | m6/locking | Page locking with lettered overflow pages, levels, placeholders, unlock one level or all; labels in the header; 1729 tests pass |
| 2026-11-19 | m6/reports | Revised pages only and without marks (export and print); scene lengths in eighths; character report; M6 complete; typing 16.3–16.5 ms at 150 pages; 1734 tests pass |
| 2026-11-19 | m7/board | Story board: beats, links, grid snap, beat window with Tab, send to script as Outline 1 and Summary; 1742 tests pass |
| 2026-11-19 | m7/outline | Outline lanes (add, rename, move, indent, delete), beats placed in lanes, structure templates, beat page ranges, Outline view with the Script lane; 1745 tests pass |
| 2026-11-19 | m7/show-outline | Show or hide outline elements in the pages, PDF and print; M7 complete; 1746 tests pass |
| 2026-11-19 | m8/cli-mcp | wordsmith-cli convert, export-pdf, report, info, mcp; MCP screenplay tools; file commands shared from wordsmith-format; 1750 tests pass |
| 2026-11-19 | m8/screenshots | Offscreen interface renderer; cargo xtask screenshots; five README pictures; index cards fixed size and wrapping (found in the pictures); 1751 tests pass |
| 2026-11-19 | m8/bundles | cargo xtask bundle (macOS .app zip, Linux tar.gz with .desktop, Windows zip); hand-started release workflow that publishes nothing; macOS bundle built and run; 1752 tests pass |
| 2026-11-19 | m8/docs | User guide, README with screenshots, CHANGELOG, spec audit, gaps rewritten, hostile-input corpus (no crashes), right-click element menu, keyboard guide from the registry; typing 16.0–17.6 ms at 150 pages; 1757 tests pass |
| 2026-11-19 | main | Version 0.1.0, tagged v0.1.0 as a starting point; remote added and pushed |
| 2026-11-19 | spec2/intake | SPEC-2 added word for word; audit against v0.1 (11 [STD] values differ, kept), questions for the spec author, decision 0030, v0.2 plan M9–M14; docs only |
| 2026-11-20 | m9/editing | Sequence element, Return/Tab hint, reset to element, toggle case, typing options (off by default), select/new scene, page break, Go To, bookmarks and labels, find by element, reformat tool, format checker; decision 0031 |
| 2026-11-20 | m10/macros | Macros (20 slots, aliases, starter set, copy from a script), text styles and keys, highlighter and color, element color/highlight/strike/small letters, template formatting for all; native format 4; decision 0032 |
