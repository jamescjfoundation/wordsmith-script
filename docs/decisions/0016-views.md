# 0016: Views, theme and writing modes

Date: 2026-11-19

- **Page view** (default) shows the paginated script (decision 0012).
- **Normal view** lays the script itself out in one continuous column, the
  width of the page's text column so lines break as on paper, with no page
  edges and no (MORE)/(CONT'D) or page numbers.
- **Draft view** is continuous too, but every element sits at the left
  margin, left-aligned, with no space before except before Scene Headings,
  and each paragraph's element name is shown in the left gutter. The text,
  paragraphs and casing are the script's own, so positions do not change.
  It is built per revision from the script with its styles overridden.
- All three draw through one path: the document and layout a view shows, and
  (for the page view only) the map back to the script. Pages are drawn in
  tiles of 600 points, only those in view, so a continuous view of a long
  script costs no more than a page view.
- **Dark theme** switches the interface to egui's dark theme and draws pages
  dark (the renderer's `dark` option: lightness inverted, hues kept). PDF
  output is not affected (SPEC's "optionally matched in PDF colors" is not
  built; `docs/gaps.md`).
- **Typewriter scrolling** keeps the caret's line at the middle of the window
  whenever the caret moves.
- **Focus mode** hides the menus, toolbar, find bar and status bar; Escape
  (with no suggestions showing) leaves it.
- Commands: `view.normal` (Mod+Alt+1), `view.page` (Mod+Alt+2),
  `view.speed` (Mod+Alt+3, the draft view), `view.darkTheme`, `view.typewriter`,
  `view.focusMode` (Mod+Shift+F). Up and Down follow the current view's lines.
- **Renamed:** the draft view was called the speed view until decision 0041;
  its command id keeps the old name.
- These are window settings, not part of the script, kept in the preferences
  file (decision 0020).
