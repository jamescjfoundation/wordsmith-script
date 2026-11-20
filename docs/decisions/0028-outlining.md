# 0028: Outlining

Date: 2026-11-19

The story board and the outline editor are two views of one set of beats,
stored in `app_data` (`wordsmith.storyboard`: beats, links, lanes, grid), so
both views, undo and saving share them and every change is a command. "Story
board" is our name; the other product's name for its canvas is a trademark.

## Story board

- **Beats:** id, title (one line, 300 characters), body (10,000), color
  (`#RRGGBB`; eight pastel colors offered), position and size on a 20,000-
  point canvas, and, for the outline, a lane and an order. At most 5,000
  beats and 20,000 links. Stored data is read field by field, so one bad
  value costs only itself; links to missing beats and duplicates are dropped.
- **Links** join two different beats, once each way, and are drawn from edge
  to edge with an arrow, so they stay attached however the beats move.
  Deleting a beat deletes its links.
- **Grid:** 20 points by default (5 to 200), snapping on by default: a beat
  moved or added lands on the grid.
- **Commands:** `storyboard.get`, `beat.add`, `beat.update`, `beat.delete`,
  `beat.link`, `beat.unlink`, `storyboard.configure`, `beat.toScript`.
- **Send to Script** puts the beat before the caret's paragraph: its title as
  an Outline 1 paragraph, its body (if any) as a Summary paragraph, as the
  spec chooses where sources differ. The Outline 1 paragraph is tagged with
  the beat (`ParaProps::beat`, added to the copied `wordsmith-doc`; `beat=7`
  on its `@` line in the native format), so the outline can find it.
  Splitting it keeps the tag on the first part.
- **The view** (View › Story Board, Mod+Alt+6): a dot grid; beats as colored
  cards with title and body; drag to move; double-click the canvas to add a
  beat there; drag from the dot on a beat's right edge to another beat to
  link them; Delete or Backspace removes the selected beat; a palette colors
  it. The beat window edits title and body, and Tab moves between them.
- **Not built** (ROADMAP): images in beats and exporting the board to PDF.
