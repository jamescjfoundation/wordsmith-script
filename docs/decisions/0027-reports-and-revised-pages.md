# 0027: Revised pages, scene lengths and the character report

Date: 2026-11-19

- **Revised pages only.** Export and print take `revisedOnly` (and File ›
  Export › Export Revised Pages…, File › Print Revised Pages): only the pages
  with revised text, each with its own label when the pages are locked; the
  title page is left out. With nothing revised there is nothing to print, and
  the command says so.
- **Without marks.** Export and print take `marks: false`, and the revision
  settings have Show revision marks: the margin marks are left off; the
  revised pages and the sets' text styles stay.
- **Scene lengths** are measured on the printed pages: each scene's lines,
  with the space before each paragraph not at the top of a page, over the
  page's text height, in eighths, rounded up, at least 1/8, written "1 3/8",
  "5/8", "2". They show in the scenes table and the scenes CSV
  (`report.sceneLengths` lists them).
- **The character report** (Production › Character Report…, CSV, and
  `report.characters`): each speaking character in order of first speech,
  with speeches, words of dialogue, scenes, first and last scene (by scene
  number) and the number of pages they speak on.
- **Typing cost.** The production features read their settings while
  paginating; typing in a 150-page script went from about 15 ms to about
  16.4 ms a keystroke (`docs/performance.md`), still under a 60 Hz frame.
