# 0021: Title page, headers and footers

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0010](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0010-document-features.md))

Since decision 0044 (2026-11-24) headers and footers, their fields and their
layout are the core's (`jcj-features`, under `core.header_footer`). Still
Script's: the title page, the `{scene}` field, and the Fountain and PDF-import
rules for title pages.

The spec names a title page and headers and footers with placeholder fields,
but leaves the title page's fields and layout open. Both are designed from
screenplay convention.

## Title page

- **Fields** (`app_data` key `wordsmith.titlePage`, so they are undoable and
  saved with the script): title, credit ("Written by"), authors, source
  ("Based on …"), notes, contact, draft date, copyright. Plain text with line
  breaks; each field is bounded (5,000 characters, 40 lines). Setting the
  title also sets the script's title.
- **Layout:** the title centered about a third of the way down the page; the
  credit a blank line below it, the authors a blank line below that; the
  source and notes two blank lines below; at the foot, the contact on the left
  and the draft date and copyright on the right, ending at the bottom margin.
  Everything is in the script font, so the page reads as part of the script.
- **Printing:** the title page is printed only when a field has text. It is a
  section of its own before the script, with no header, and the script's
  pages are numbered from 1 after it, as the convention has it. The page view
  shows the script alone; the title page is edited in its own window
  (Format › Title Page…).
- **Fountain** reads and writes the title page keys (`Title`, `Credit`,
  `Author`/`Authors`, `Source`, `Notes`, `Contact`, `Draft date`,
  `Copyright`); multi-line values use indented continuation lines. Other keys
  are read and kept only for the import's caller.
- **PDF import** recognises a title page: a first page, before more pages,
  with no scene heading and no speech. Its centered blocks become the title,
  credit and authors (a block ending in "by" is the credit), source ("Based
  on …") and notes; the foot's left side the contact, its right side the
  draft date. A page cannot tell a credit that names the authors ("By …")
  from the authors.

## Headers and footers

- **Settings** (`app_data` key `wordsmith.headerFooter`): a header and a
  footer, each with left, center and right text (200 characters each), whether
  page 1 shows them, and a fixed date (empty for today's).
- **Fields** in braces: `{page}` (a live page-number field), `{scene}` (a live
  STYLEREF field to the scene heading element: the first scene heading on the
  page, or, on a page with none, the scene running onto it), `{date}` (today
  as "November 19, 2026", or the fixed text), `{title}`, and `{revision}` (the
  `current` name kept under `wordsmith.revisions`, decision 0024; empty when
  there is no revision set).
  Anything else in braces is printed as it is.
- **Layout:** one slot used is aligned where it belongs; two or three use tab
  stops at the center and the right margin, half an inch from the page edge.
- **Default:** `{page}.` at the top right, none on page 1, which is what the
  paginator printed before; the golden PDFs did not change.
- The page view shows the same header and footer as the PDF, since both come
  from the paginated output.
