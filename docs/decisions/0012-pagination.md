# 0012: Screenplay pagination

Date: 2026-11-19 · Status: accepted

Since decision 0044 (2026-11-24) the planner below is still Script's own (that
record says why). It measures with the core's `jcj-layout`, which took the
place of `wordsmith-layout`, and `app_data` is the document's `ext`.

## How

Pagination is ours, in `wordsmith-script::paginate`; line breaking is the
layout engine's. Changing `wordsmith-layout`'s Word-style paginator was the
riskiest option (REUSE.md "Risks"), so:

1. The script is laid out once (print view). Each body paragraph's lines,
   their byte ranges and heights, and the resolved space before, keep-with-next
   and start-new-page come from that layout.
2. The planner fills pages of `page height − top − bottom margin` points
   (54 lines on US Letter) with the rules below.
3. The output script is built from the plan, with every page's first
   paragraph marked to start a page and keep rules switched off, so the
   layout engine places it exactly as planned. Paginating checks the page
   count agrees (`Paginated::matches_plan`); every test asserts it.
4. A map from output paragraphs to source paragraph and byte offset converts
   caret positions and clicks both ways.

## Rules the spec leaves open

- **Space before** is dropped for the first paragraph on a page.
- **Keep with next** means: the paragraph, plus the least the next paragraph
  needs to start (its first two lines, plus a line for (MORE) when it is
  dialogue, or all of it when it cannot split), must fit; otherwise the
  paragraph moves. When a paragraph moves, the keep-with-next paragraphs placed
  just before it move with it (a Character is never left alone with a
  (MORE)), unless that would leave the page with no paragraph. Scene Heading,
  Character, Parenthetical, Shot, New Act and Outline 1 and 2 keep with next
  by default (decision 0009).
- **Splits.** Only Action, Dialogue, General and custom elements split (by
  their paginate-as element). A split falls only after a sentence end: `.`,
  `!` or `?` (also `...`), with any closing quotes or brackets, followed by a
  space. At least 2 lines stay at the foot and at least 2 are carried. The
  first part ends at the sentence; the rest is laid out again as its own
  paragraph. With no such sentence end in reach, the paragraph moves whole.
- **Last resort.** A paragraph taller than a whole page with no usable
  sentence end splits at a line break, so pagination always ends. Pages are
  capped at 100,000.
- **Speeches.** A speech is a Character and the Parentheticals and Dialogue
  after it. When a page break falls inside a speech, between paragraphs or
  inside one, the page ends with (MORE) at the Character position and the next
  starts with the name and (CONT'D), no space before. A speech paragraph that
  is followed by more of the speech reserves a line for (MORE).
- **Automatic CONT'D.** A Character whose speaker (the name before any
  extension) is the same as the previous speech's in the same scene gets
  " (CONT'D)" unless it already has it. A Scene Heading resets it, so a speech
  carried into the next scene gets none. The script itself is not changed;
  only the output is.
- **Scene CONTINUED** (off by default). When on, every page reserves a line
  at the foot; a scene broken across pages gets "(CONTINUED)" right-aligned at
  the foot and "CONTINUED:" at the head of the next page, optionally with the
  scene's page count, "CONTINUED: (2)".
- **Texts and switches** live in `app_data["wordsmith.moresContinueds"]`:
  `dialogue`, `character`, `scene`, `scenePageCount`, `more`, `contd`,
  `continuedTop`, `continuedBottom`. Empty, over-long (more than 40 bytes) or
  multi-line texts fall back to the defaults.
- **Page numbers** are a header holding a `PAGE` field and a period,
  right-aligned at 7.5 in, half an inch from the top. A separate empty
  first-page header leaves page 1 unnumbered.

## Performance

Paragraphs are measured one at a time with `layout_para` through a
`MeasureCache` keyed by paragraph revision and a fingerprint of the styles
and page width, so the source script is never laid out as a whole. Output
paragraphs get stable revisions derived from their source paragraph, byte
range and flags, so the layout cache serves every unchanged one. With both,
repaginating a 150-page script after an edit takes about 14 ms
(`docs/performance.md`), so the editor repaginates on every edit.

## Page view

The editor draws the paginated output (with its (MORE), (CONT'D), CONTINUED
and page numbers). The caret and selection are mapped into the output to be
drawn; clicks on the output are mapped back to the script. Commands always
work on the script itself.
