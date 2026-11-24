# 0035: Production additions

Date: 2026-11-20

## Decision

- **Dual dialogue** is a flag on the second speech's Character paragraph. The pair
  prints side by side: halves of the text column starting at 1.5 in and 4.6 in,
  each element's column scaled to 48 %. The first speech is kept together on one
  page. In the page view the second speech shows in its box but is edited in the
  normal and draft views. Fountain `^` maps to the flag both ways.
- **Locked pages**: relock unlocks and locks again (1, 2, 3 from the current
  breaks). A new locked page at the caret takes the next free letters after the
  page it is on. The first locked page cannot be unlocked alone. Deleting text
  across a locked break stays allowed, as v0.1 built it (decision 0026 moves the
  page's mark onto the text that follows); refusing it would change built
  behaviour (decision 0030).
- **Scene numbers**: unlocked numbering starts at the chosen number and can start
  again every N scenes; with prefix letters, a scene added after the lock takes a
  letter before the next scene's number (A2).
- **Revisions**: "active set" shows only the current set's marks, styles and page
  colors; "collated" (the default) shows every set, as v0.1 did.
- **Printing**: page and scene ranges, revised, unrevised or one set, and reverse
  order skip the title page; sides keep each scene's number from the whole script.
  Collated copies go to CUPS with `collate=true`; preview opens the PDF in the
  system's viewer.
- **Watermark**: 66 pt bold text at 45°, behind the text, grey by strength (70 %
  by default), with the recipient's name under it. It shows in the page view too.
- **Change tracking** uses the document model's tracked insertions and deletions
  (from WordCraft), with records of who and when (the window gives the session the
  time). Paragraph breaks are not tracked. Other features (spelling, find,
  exports) read deleted text as still there until it is accepted.
- **Comparison** takes the open script as the newer version. Scenes match by the
  same heading, else by at least half their words in common (headings left out);
  scenes out of order are moved and get a note. The marked copy opens as a new,
  unsaved script.
