# 0018: Fountain reading and writing

Date: 2026-11-19

Fountain (https://fountain.io/syntax, version 1.1) is the open plain-text
screenplay format; the spec asks for import and export from day one. The
reader and writer are in `wordsmith-format::fountain`.

- **Emphasis is kept both ways** (`***`, `**`, `*`, `_`), with the same marker
  code as the native format. Fountain treats a stray `*` as text, so the
  reader is lenient: a marker kind left open at the end of a paragraph is read
  as plain characters ("5 * 3"). The writer escapes `*`, `_` and `\`.
- **Outline material is kept.** Sections `#`, `##`, `###` (and deeper) become
  Outline 1 to 3; a synopsis `=` becomes Summary; a note that is a paragraph
  on its own (`[[...]]`) becomes a Note. Notes inside other text and the
  boneyard (`/* */`) are still dropped.
- **Page breaks belong to a paragraph.** `===` sets "starts a new page" on the
  paragraph after it, as the editor does, instead of adding an empty
  paragraph (one at the very end stays as an empty paragraph).
- **Forcing only when needed.** The writer writes plain Fountain wherever the
  reader would recognise the element, and forces it (`.`, `@`, `>`, `!`)
  otherwise: a heading without a known intro, a cue not in capitals or with
  no dialogue after it, a transition not ending in `TO:`, Action that would
  read as a heading, cue, transition, section, title page or forced element.
  Empty lines inside Action are written as `!`; inside dialogue and notes as
  two spaces. A dialogue line in brackets is escaped so it is not read as a
  parenthetical, and `[[` in text is broken so it is not read as a note.
- **The reader is stricter about title pages.** A first line is a title-page
  key only when the part before the colon is letters, digits and spaces, so
  forced Action such as `!Note: ...` is not a title page.
- **A forced cue (`@NAME`) counts with no dialogue after it.**
- **Graceful loss where Fountain has no form.** Shot, Cast List, General and
  custom elements are written as Action; New Act and End of Act as centered
  text (read back as Action); Dialogue or a Parenthetical with no cue before
  it as Action; two Dialogue paragraphs under one cue read back as one, joined
  by a line break; line breaks in headings, cues, transitions and outline
  lines become spaces. Scene numbers were not written at first.
  Title page: only the title was written and kept. (Scene numbers are read
  and written since decision 0025, and the title page fields since decision
  0021.)

Tests: every element Fountain has survives write → read with its emphasis,
and the text is stable after one round; 200 generated scripts mixing awkward
words (`INT.`, `TO:`, `!`, `@`, `#`, `[[`, brackets, escapes) survive; the
three golden scripts survive Fountain → script → Fountain → script with the
same elements and text, and their reviewed PDFs did not change.
