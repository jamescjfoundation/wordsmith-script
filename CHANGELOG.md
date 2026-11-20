# Changelog

## Unreleased

Work toward v0.2 from `docs/SPEC-2.md`. Nothing built in 0.1.0 changes its
behavior; every addition below is new.

- **Editing (M9):** the Sequence element; the Return and Tab hint; reset to
  element; toggle case; smart quotes and sentence capitals as options; select
  scene, new scene, page break; Go To by page, scene or bookmark; bookmarks
  and labels; find within one element; the reformat tool; the format checker.
- **Macros and styles (M10):** twenty macros per script with shortcuts,
  aliases and a starter set; bold, italic and underline keys; double, word and
  dotted underlines, strike, raised, lowered, capitals, small capitals, small
  letters; highlighter and text colors; element color, highlight and strike;
  template formatting for all elements. The native format is now version 4
  (0.1.0 cannot open version-4 files).
- **Characters and reports (M11):** scene properties (title, color, cast,
  non-speaking characters, arc beats); character traits and speech
  highlighting; scene, location, cast, character, filtered-script, notes,
  statistics and representation reports, saved as text, CSV or PDF; tables
  that sort, filter, hide columns and keep tabs of your own; note titles,
  types, colors and dates; next and previous note.
- **Planning (M12):** several story boards per script; arrange beats on the
  grid, in a column, a row or a cascade; link styles and colors; list lines
  in beats; story board and index cards as PDF; cards per row and script-text
  cards; the structure strip above the script; nesting bars for outline
  paragraphs.
- **Production (M13):** dual dialogue (also from Fountain `^`); relock, move a
  locked page's start, lock or unlock one page, edit a page number; scene
  number options (start, restart, A2 letters, sides, font, one number by
  hand); a mark per revision set and current-set-only display; print ranges,
  one set, unrevised pages, sides, reverse, collated copies and preview; a
  watermark; header and footer start page and `{collated}`; change tracking;
  comparing two versions.
- **Workspace (M14):** twelve templates of our own and templates of your own;
  text documents for prose; the Welcome window, recent scripts and reopening
  the last ones; revert and merge; tabs; split views; zoom steps and an
  extra-dark theme; writing statistics, goals and sprints; your own keys.

## 0.1.0 (2026-11-19)

The first tagged version: a starting point, not a finished app. Everything is
new; in brief:

- **Writing:** the screenplay elements with Return and Tab, element settings
  and custom elements, scene-heading separators, autocomplete over six lists,
  spelling (English), find and replace, notes, character rename.
- **Pages:** industry pagination with (MORE), (CONT'D) and CONTINUEDs, title
  page, headers and footers with fields, PDF export and printing matching the
  page view.
- **Views:** page, normal and speed views; index cards; scene view; tables
  (scenes, characters, notes) with CSV; dark theme, typewriter scrolling,
  focus mode.
- **Production:** revision mode and sets on the color cycle with margin
  marks and page colors; scene numbers with letters and OMITTED; locked pages
  with A-pages and placeholders; revised-pages-only export and printing;
  scene lengths in eighths; character report.
- **Outlining:** story board of beats with links and grid snap; outline
  lanes with structure templates; beats sent to the script; outline elements
  shown or hidden.
- **Files:** the `.wss` native format (version 3); Fountain both ways;
  import of plain text, RTF and text PDF; export of RTF, plain text, HTML
  and CSV; auto-save, timestamped backups, crash recovery; preferences.
- **Automation:** `wordsmith-cli` (convert, export-pdf, report, info, mcp)
  and an MCP server with screenplay tools.
- **Release:** bundles for macOS, Linux and Windows (`cargo xtask bundle`).

What v0.1 does not do is listed in `docs/gaps.md`.
