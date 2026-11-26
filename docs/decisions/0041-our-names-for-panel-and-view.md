# 0041: Our names for the side panel and the compact view

Date: 2026-11-22 · Status: accepted

## Context

The clean-room rules (`docs/SPEC.md`, rule 2) forbid using another
product's feature names in ours. Two had reached the interface, both names
another screenwriting program uses for the same things:

- **Navigator**, the left side panel (scenes, outline, bookmarks, notes,
  characters, find).
- **Speed View**, the compact view beside Page and Normal.

## Decision

- The left side panel is the **Script panel**: it holds where things are in
  the script, as the Inspector on the right holds what is at the caret. The
  Inspector keeps its name.
- The compact view is the **Draft view**: plain text at the left margin, made
  for getting a draft down quickly.
- Every label, tooltip, menu item, command label, status bar tip, tour step,
  File page text and guide page uses the new names. So do the code's own
  names (`View::Draft`, `draft_document`, the `view.scriptPanel` and
  `view.draft` icons), so the code and the interface say the same thing.
- **Kept for compatibility:**
  - The command id stays `view.speed`. Custom keys in `preferences.json`, the
    command line and MCP agents refer to commands by id, and the registry
    has no aliases, so a new id would break them. The window shows the id
    only as a tooltip on the Keys page.
  - `preferences.json` now writes `"view": "draft"`, and still reads
    `"speed"` as the draft view, so a file written by an older version opens
    in the same view (tested). An older version reading a newer file falls
    back to the page view, its default for a name it does not know.
  - Nothing in `.wss` files named either one, so the format does not change.
- Decisions 0016 and 0037 now use the new names, with a note. Earlier plans,
  the progress log and released changelog entries keep the old names as a
  record of what was built then. `docs/SPEC.md` uses the new names;
  `docs/SPEC-2.md` keeps its descriptive words.

## Revisit when

- The registry gains aliases: the command id can then become `view.draft`
  with `view.speed` kept as a hidden alias.
