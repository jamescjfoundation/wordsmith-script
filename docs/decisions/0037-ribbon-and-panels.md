# 0037: The ribbon, side panels and what is reused for them

Date: 2026-11-20

## Context

A ribbon like WordCraft's shows more of the app's commands, grouped by task,
than a menu bar does, and the empty space beside the page can hold side panels
(`docs/history/plans/v0.3-ui.md`). `docs/REUSE.md` had listed WordCraft's
interface crate as not worth taking.

## Decision

- **Copied from WordCraft** (`crates/ui-egui`, commit `f8ec3cc`, MIT OR
  Apache-2.0), marked as modified and recorded in `UPSTREAM.md`:
  - `ribbon_fit.rs`: groups that shrink to fit the window. Only `Id::unique`
    became `Id::new` (our egui), and the language code became fixed.
  - `icons.rs`: the icon helper and general icons drawn in code. The library
    icons that need WordCraft's renderer are cut, and so is the Discord icon
    (branding rule).
- **Ours:**
  - the ribbon widgets: buttons, toggles, tiles, chips, groups;
  - the tabs, the quick-access row, the design tokens and their values;
  - the screenplay icons;
  - the side rails and panels;
  - the File page.

  WordCraft's widgets are tied to its `WordApp` and Word's command catalogue;
  writing small ones over our `Editor` was simpler than cutting them loose.
- **craft-ui's rail and panes are not used.** They take SVG image sources, and
  our icons are drawn in code. Our rails are egui side panels; this can change if
  craft-ui gains painted icons.
- **Defaults** (the plan's four open questions, answered this way): the
  old menu bar is hidden on every platform and can be shown from View; the
  Script panel opens at first launch and the Inspector does not; the ribbon shows in
  full; WordCraft's general icons are kept and ours are drawn for screenplay work.
- **Key tips:** egui cannot see Alt pressed alone, so key tips start with F10 or
  Alt+Space. A tab letter opens that tab, then letters run its controls.
- **Labels** are our own and named for screenplay work: Write, Insert, Plan,
  Production, Review, View, and contextual Story Board and Index Cards. Word's
  tab and group names are not used.
- **Renamed:** the left side panel was called the Navigator until decision
  0041, which gave it our own name, the Script panel.
