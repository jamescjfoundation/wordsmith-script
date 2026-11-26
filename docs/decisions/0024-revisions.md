# 0024: Revisions

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0010](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0010-document-features.md))

Since decision 0044 (2026-11-24) revision sets, the cycle, revision mode, the
marks on text and rollback are the core's (`jcj-features`, `core.revised` and
`core.revisions`). Still Script's: the margin marks and page colors, which
belong to its pagination, and the marks in `.wss`.

- **Settings** (`app_data` key `wordsmith.revisions`): the sets made so far
  (name, `#RRGGBB` color, bold, underline, strikethrough), the current set's
  name, revision mode, the margin mark (one character, `*` by default), its
  position (8.0 in from the page's left edge by default, clear of scene numbers) and whether PDF and
  printed pages are colored. Up to 100 sets.
- **The cycle.** The original draft is White and carries no marks. Next
  Revision adds the next set in the industry cycle (Blue, Pink, Yellow, Green,
  Goldenrod, Buff, Salmon, Cherry, Tan, Second White, Second Blue, …), makes it
  current and turns revision mode on; turning revision mode on with no set
  starts Blue. Names and colors can be changed. The colors are our own pastel
  choices for each name.
- **What is marked.** Revised text carries a mark on its characters
  (`CharProps::revised`, the set's index), so it moves with the text. In
  revision mode, typed and pasted text takes the current set; out of it, text
  typed next to revised text is not revised. A deletion (or a change of
  element) marks its paragraph, through the paragraph mark, and the margin
  mark shows on the paragraph's first line: the deleted text itself is gone,
  so its exact line is not kept.
- **Commands:** `revision.mode`, `revision.next`, `revision.setCurrent`,
  `revision.configure`, `revision.markRevised` (the selection, or the caret's
  paragraph), `revision.clearRevised` (the selection or paragraph, or all;
  revision mode stays as it is), `revision.rollback {to}` (clears the marks of
  later sets and removes them; the text is not changed; "" is the original),
  `revision.settings`.
- **Output.** Each revised line gets the mark in the right margin: one small
  text box per revised paragraph piece, anchored to the paragraph, with a mark
  or a blank line for each of its lines (measured with the same line breaks
  as the page), so the page view, the PDF and printing show the same marks.
  The sets' text styles are applied to their revised text. Each page knows
  its newest set: the page view tints that page's side margins in its color;
  with colored pages on, the PDF and print put the color behind the whole
  page. The `{revision}` header field shows the current set.
- **Files.** The native format marks revised text (`{r2}…{/r}`) and
  paragraphs with a deletion (`revised=2` on the `@` line) from version 3. PDF
  import leaves out short runs of text in the side margins (marks, scene
  numbers). Fountain has no revisions; they are not written.
