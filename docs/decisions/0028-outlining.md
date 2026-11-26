# 0028: Outlining

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0014](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0014-shell-theme-board.md))

Since decision 0044 (2026-11-24) the story board's canvas, its cards and the
card window are drawn by the core's `jcj-board`. Still Script's: beats, links,
lanes and boards as data (`wordsmith.storyboard` in the document's `ext`),
every command, the outline editor, and the beat tag on a paragraph
(`script.beat`).

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

## Outline editor

- **Lanes** (name, indent 0 to 2, order; at most 100) hold beats in order.
  Commands: `lane.add`, `lane.rename`, `lane.move`, `lane.indent`,
  `lane.delete` (its beats stay, in no lane), `beat.place` (a beat into a
  lane, or none, at a position). The beats are the story board's, so moving
  or editing one shows in both views (the edits sync both ways because there
  is only one copy).
- **Structure templates** add lanes after those there are: three acts; five
  acts; three acts in eight sequences (sequences indented under their acts);
  television (teaser, four acts, tag). Templates are lanes only; they add no
  beats.
- **Page ranges.** A beat sent to the script covers the pages from its
  Outline 1 paragraph to the page before the next Outline 1 paragraph, or the
  script's last page (`outline.pages`).
- **The view** (View › Outline, Mod+Alt+7): one row per lane, indented, its
  name editable in place, with buttons to move, indent and delete it; beats
  as colored cards (with their pages when sent), dragged onto a card to go
  before it or onto a lane's end to go last; a row of beats in no lane; and
  the Script lane, the script's scenes with the page each starts on (click to
  go there). Double-click a beat to edit it in the beat window.

## Showing and hiding outline elements

- **One switch** (`outline.show`, View › Show Outline Elements; `app_data`
  `wordsmith.outline`), on by default: off, the outline elements (Outline 1
  to 3, Summary, Note) leave the page view, the PDF, print, and the page
  counts and lengths worked out from them. The script keeps them, so turning
  the switch back on shows them where they were. Hiding them with the caret
  in one moves the caret to the next paragraph that shows.
- **Scope change from the plan:** the normal and draft views keep showing
  outline elements. Those views lay out the script itself, so leaving
  paragraphs out would break the caret's place; they are drafting views, and
  the page view is where the printed script is checked.
