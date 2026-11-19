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
