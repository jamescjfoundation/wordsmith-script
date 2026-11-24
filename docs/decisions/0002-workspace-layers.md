# 0002: Workspace layer order

Date: 2026-11-19

## Context

`docs/REUSE.md` sketches eight layers, each calling only the layers under it,
and nothing below `wordsmith-ui` may depend on it, so the command line and the
MCP server can run every command without a window. The sketch puts the Files
layer (native format, Fountain) under the Screenplay layer, but the native
format and Fountain both need the screenplay element types, which live in
`wordsmith-script`. The sketch also puts
`wordsmith-pdf` in the Files layer, though PDF output only needs layout.

## Decision

Layers, lowest first. A crate may depend on its own layer or lower.

| Layer | Crates |
| --- | --- |
| 1 Model | wordsmith-geom, wordsmith-doc |
| 2 Layout | wordsmith-fonts, wordsmith-layout, wordsmith-render |
| 3 Output | wordsmith-pdf |
| 4 Engine | wordsmith-engine (session, undo, command registry, recovery), wordsmith-control-key (the key file that lets agents reach a running app) |
| 5 Screenplay | wordsmith-script |
| 6 Files | wordsmith-format (native format, Fountain, text and RTF mapping) |
| 7 Agents | wordsmith-mcp |
| 8 Interface | wordsmith-ui |
| 9 Apps | wordsmith, wordsmith-cli, wordsmith-web |

`wordsmith-format` moves above `wordsmith-script` so the file formats can
speak in screenplay elements. `wordsmith-pdf` stays a generic page writer
under the engine; screenplay-specific PDF content (revision colors, title
page) is driven from above. `wordsmith-cli` is an app, but by convention it
does not use `wordsmith-ui`, except later for offscreen screenshots if needed.

The table lives in `xtask/src/layers.rs`. A new workspace crate without a
layer fails the check, so every new crate has to be placed on purpose.

## Revisit when

A copied WordCraft crate's real dependencies do not fit this order. Record
the change here rather than adding an exception in code.
