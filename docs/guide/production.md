# Production

Everything here is in the **Production** menu.

## Revisions

**Next Revision** starts the next set in the industry color cycle (Blue,
Pink, Yellow, Green, Goldenrod, Buff, Salmon, Cherry, Tan, then Second White,
Second Blue …) and turns **Revision Mode** on. While it is on, what you type
is marked as revised, and so is a paragraph where you delete text. Each
revised line gets an asterisk in the right margin, revised pages have their
margins tinted in the set's color, and the `{revision}` header field names
the set.

**Revision Sets** lets you rename sets, change their colors, give their text
bold, underline or strikethrough, change the mark and its position, color
whole pages in PDFs, and roll back to an earlier set (its later marks are
cleared; the text stays). **Mark Revised** and **Clear Revised** act on the
selection or the caret's paragraph.

## Scene numbers

**Show Scene Numbers** puts each scene's number in both margins. Until you
**Lock Scene Numbers**, they follow the order. Once locked, a scene keeps its
number, and new scenes take letters after the scene before them (12A, 12B).
**Omit Scene** leaves an OMITTED heading with the number; **Recall Omitted
Scene** brings the scene back. **Renumber Scenes** starts again from 1.

## Locked pages

**Lock Pages** freezes the page breaks: each page keeps its number however
the script changes. Text that no longer fits goes onto added pages (12A,
12B); after a second lock, pages added after 12A are 12AA, 12AB. A page whose
text is all deleted leaves a placeholder (two in a row: 10-11). **Unlock One
Level** undoes the newest lock; **Unlock All** numbers the script 1, 2, 3
again. The scheme is described in `docs/decisions/0026-page-locking.md`.

## Printing revisions and reports

**Export Revised Pages** and **Print Revised Pages** send only the pages with
revised text, with their locked numbers. Turn off **Show revision marks** in
Revision Sets to print without the asterisks.

The scenes table shows each scene's length in eighths of a page. **Character
Report** writes a CSV of every speaking character: speeches, words, scenes,
first and last scene, and pages.
