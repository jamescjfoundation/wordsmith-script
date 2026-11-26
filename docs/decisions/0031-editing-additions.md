# 0031: Editing additions from SPEC-2

Date: 2026-11-20 · Status: general part moved to the core
([core decision 0010](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0010-document-features.md))

Since decision 0044 (2026-11-24) bookmarks and labels are the core's
(`jcj-features`, `core.bookmark`). Still Script's: the Sequence element,
structure rank, the typing options, the keys and the format checker.

## Decision

- **Sequence** is a new built-in element (SPEC-2 3.1 values: 1.25 in, two lines
  before, bold underlined capitals, Return and Tab make a Scene Heading). Old
  scripts gain it on opening, as any missing built-in element is restored.
- **Structure rank**, broadest first: Outline 1, acts, Outline 2, Outline 3,
  Sequence, Scene Heading. SPEC-2 left Outline 3 open; it sits under Outline 2.
- **Typing options** (smart quotes, sentence capitals) are off by default, so
  typing behaves as v0.1 did (decision 0030, question 7). They act on one typed
  character, never on pasted text.
- **Keys.** Bare Shift+digit types symbols, so the structure and outline elements
  take Mod+Shift+1 to 7. Mod+Alt+digit stays with the views. Go To is Mod+G, a
  bookmark Mod+M, the bookmarks window Mod+Shift+M.
- **Bookmarks and labels** are one list; a label is a bookmark flagged to be
  offered as a label. A bookmark is a paragraph property, so it moves with its
  paragraph and is saved as an attribute on the native `@` line, which older
  readers ignore; the format version stays 3.
- **Format checker** fixes leading spaces, empty paragraphs and Character names
  with no speech; the other two rules are reported only. Speaker comparison
  ignores the extension, so "BEN" then "BEN (CONT'D)" counts as twice in a row.

## Revisit when

Question 7 (smart quotes default) is answered, or a different keymap is
wanted.
