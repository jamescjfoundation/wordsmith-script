# 0019: Import classification rules

Date: 2026-11-19 · Status: accepted

Since decision 0044 (2026-11-24) the RTF reader and writer and the PDF text
reader under these rules are the core's (`jcj-interchange`, `jcj-output`). The
classification rules and the shape of each export are Script's, unchanged.

`wordsmith-format::import` reads plain text, RTF and text-based PDF into a
script. The spec asks that plain-text and RTF import ask whether the text is a
script and auto-format it; the question is the `as_script` parameter, and
`looks_like_script` gives the editor a default answer (a scene heading, two
transitions, or three names in capitals with speech under them).

**One classifier.** Every source becomes lines: text, whether a blank line (or
a gap as tall) comes before, and, when the source says, the left edge in
inches. Then:

- **Not a script:** each paragraph of text (lines up to a blank line) becomes
  one Action paragraph, wrapped lines joined.
- **Laid-out lines** (a PDF always; text or RTF when enough lines are indented
  three-quarters of an inch or more past the leftmost text): classified by
  column, using the template's positions: below 2.0 in the action column
  (scene heading by intro, transition ending `TO:`, else Action); to 2.8 in
  Dialogue; to 3.4 in Parenthetical (when it starts with a bracket); to 4.6 in
  a cue (capitals) or Action; beyond, a transition (capitals) or Action.
  Wrapped lines of one paragraph are joined. A paragraph in capitals in the
  action column without closing punctuation is a scene heading with no intro.
  The source's margin is taken as is unless a scene heading in the leftmost
  column shows where the action column is.
- **Flat lines** (text with no layout): by convention, as Fountain does
  without its forcing marks: an intro starts a scene heading, a name in
  capitals with lines under it is a cue followed by parentheticals and
  dialogue, a capitals line on its own ending `TO:` is a transition, a
  capitals line on its own without closing punctuation is a heading with no
  intro (as on a laid-out page), the rest is Action. Capitals lines after a
  heading's first line are the rest of a wrapped heading.

**Positions per source.** Plain text: 10 characters per inch, tabs every 8
columns. RTF: the paragraph's left and first-line indents (an RTF with
indents on fewer than three paragraphs is read as plain text, with space
before of half a line or more as a blank line). PDF: each
line's left edge, from glyphs read with craft-pdf's guarded interpreter and
grouped by baseline; a gap over one and a half line pitches (measured across
the whole script) is a blank line.

**Pagination marks** are taken out: `(MORE)`, `(CONTINUED)`, `CONTINUED:`, a
cue's `(CONT'D)` (the app adds it again when it paginates), page numbers (any
lone number in a PDF; in text, one at the right). A speech split by `(MORE)`
and its repeated cue is joined into one.

**Joining wrapped lines** puts a space between them, except after a dash
attached to a word ("MEET--" then "HELPS"), where the line was broken without
one.

**What a page cannot show** (recorded in `docs/gaps.md`): a heading with no
intro that ends in punctuation reads as Action, and a paragraph split across
a page reads as two. Merging across pages was considered and rejected: a page
that ends exactly at its last line with a whole paragraph would then be
merged wrongly, and losing a paragraph boundary is worse than keeping an
extra one.

**Exports** (`wordsmith-format::export`) are made to come back through these
rules. Plain text puts each element at its style's indent in spaces at 10
characters per inch, wraps it at the style's width, right-aligns or centres
it as the style says, writes the style's space before as blank lines and a
form feed before a paragraph that starts a page. RTF (craft-rtf) writes each
paragraph with its style's indents, alignment and space before, emphasis
kept, in Courier Prime 12 pt. HTML writes one `<p>` per paragraph with an
`el-<element>` class and a stylesheet made from the element styles, emphasis
as `<b>`, `<i>`, `<u>`, everything escaped. None has mores or continueds.

Tests: each element from flat text, indented text, RTF with and without
indents; marks and split speeches; the guess; a long script this app printed
comes back as exactly the same elements; the three golden scripts come back
from their PDFs with only the two ambiguities above (5 of 736 paragraphs), and
from their plain-text and RTF exports with only the first (2 of 736);
bad and hostile files are refused or read without a panic.
