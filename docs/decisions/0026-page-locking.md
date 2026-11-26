# 0026: Page locking and the page-label scheme

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0010](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0010-document-features.md))

Since decision 0044 (2026-11-24) locks, labels and letters, levels,
placeholders and the lock commands are the core's (`jcj-features`,
`core.page_lock` and `core.page_locks`). Still Script's: starting a page at a
lock in its own planner, and the marks in `.wss`.

The spec asks for one documented page-label scheme (sources show mixed forms
such as 4aA). This is ours.

## The page-label scheme

- **A locked page keeps its label.** Locking labels the pages as they are:
  1, 2, 3 … (or, on a later lock, the labels they show then).
- **Pages added after a locked page take its label and a letter**, in order:
  12A, 12B, … 12Z, then 12ZA, 12ZB. Letters are capitals; there is no
  lower-case form.
- **A second lock level** locks the lettered pages too. Pages added after a
  locked lettered page take one more letter: after 12A, 12AA, 12AB.
- **Deleted pages.** A run of locked pages whose text was all deleted leaves
  one placeholder page, labelled first-last (10-11, 7B-9), or with the one
  label when a single page went.
- Pages before the first locked page (none, normally: page 1 is locked) are
  numbered 1, 2, 3.
- Labels are printed where `{page}` is in the header or footer (`12A.` with
  the default header). Page 1 still shows no number unless asked.

## How locking works

- **Lock Pages** marks the first character of every page (`CharProps::
  page_lock`, a lock id) and records each lock's label and level in
  `app_data` (`wordsmith.locks`). It can also turn revision mode on (the spec:
  locking offers to), starting Blue if there is no set.
- **Pagination** starts a page at every mark, splitting a paragraph exactly
  there when the page starts mid-paragraph, and never pulls text back across
  a mark: a locked page that loses text is short; one that gains text flows
  onto lettered pages. (MORE) and (CONT'D) still apply across these breaks.
- **Edits keep the marks.** Deleting text that holds a mark moves it to the
  character that follows; typing at the very start of a locked page puts the
  new text on that page. When a page's text is entirely gone, its mark goes
  and its placeholder appears.
- **Unlock One Level** removes the newest level's marks (its lettered pages
  flow freely again and are lettered from their locked page); **Unlock All**
  removes every mark, and the script is numbered 1, 2, 3 again.
- **Labels in print.** A locked script's output has one section per page, so
  each page's header can carry its own label. The page view's status bar
  shows the label of the caret's page.
- **Files.** The native format writes `{p7}` before a locked page's first
  character (version 3); the lock list is in `appData`. Fountain has no
  locking.

## Not built in v0.1 (`docs/gaps.md`)

Relock from the cursor, adjusting a locked page's start by a paragraph,
editing a page number by hand, locking a new page at the cursor, and
unlocking one page.
