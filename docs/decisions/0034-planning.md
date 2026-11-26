# 0034: Boards, the strip and planning PDFs

Date: 2026-11-20 · Status: accepted

Since decision 0044 (2026-11-24) boards and cards are drawn by the core's
`jcj-board`; the board's data and the planning PDFs are still built here.

## Decision

- **Several boards** share one beat list; each beat names its board. Outline
  lanes still hold beats from any board. Deleting a board deletes its beats; the
  last board cannot be deleted.
- **Arrange** works on the selection (Shift-click adds beats) from its top-left
  beat. "On the grid" snaps whether snapping is on or not.
- **Links** are drawn forward by default (as v0.1 drew them); line, back and both
  are new styles.
- **Lists in beats** are plain text lines starting "• " or "1. ", added with two
  buttons; they print as written.
- **Board PDF**: one landscape Letter page per board, scaled to fit (never
  enlarged). **Cards PDF**: one 3 × 5 in card per page, or a grid of up to 6 × 6
  on Letter. Both are built as documents of shapes and use the script's own
  layout and PDF writer, so no new dependency.
- **The strip** has a lane per chosen structure element (Outline 1, New Act and
  Sequence by default) and one for scenes. A block runs to the next paragraph of
  the same or a broader rank; its length is in pages, measured from the
  paginated script.
- **Outline paragraphs** get nesting bars on screen; their typeface and color
  stay as v0.1 set them (decision 0030).
