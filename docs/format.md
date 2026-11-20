# The Wordsmith Script file format (`.wss`)

Format version 3. A `.wss` file is UTF-8 plain text made to be read by people,
compared with `diff`, and kept in version control. Every script setting lives
in the file: element definitions, autocomplete lists, scene-heading
separators, more-and-continued texts, and (as later milestones add them)
title page, notes, revisions, page locks, scene numbers, outline and story
board.

## Layout

```
Wordsmith Script 1
=== settings ===
{
  "appData": { ... },
  "core": { "title": "...", ... },
  "page": { "pageW": 612.0, "pageH": 792.0, "marginTop": 72.0, ... },
  "styles": { ... }
}
=== script ===
@SceneHeading
INT. KITCHEN - NIGHT

@Action
Rain *hammers* the window.

@Character
ANNA
```

1. **First line:** `Wordsmith Script` and the format version.
2. **Settings:** the line `=== settings ===`, then one JSON object,
   pretty-printed with every object's keys in alphabetical order, so a changed
   setting is a small, readable diff:
   - `page`: page size and margins, in points;
   - `styles`: the paragraph style of every element (font, size, bold, italic,
     underline, indents, alignment, line spacing, space before, next element,
     start new page, keep rules) and the document defaults;
   - `core`: title and other document properties;
   - `appData`: the script's own data by key: `wordsmith.elements` (element
     registry), `wordsmith.sceneHeading`, `wordsmith.autocomplete`,
     `wordsmith.moresContinueds`, `wordsmith.spelling`, and later keys.
   Every field is optional; what is missing comes from the screenplay
   template.
3. **Script:** the line `=== script ===`, then each paragraph in order:
   - a line `@ElementId`, optionally followed by attributes separated by
     spaces: `page-break` (the paragraph starts a new page), `#12A` (a
     locked scene number), `omitted=N`, `beat=N`, `revised=N`, and since
     v0.2 `bookmark=N` (a bookmark or label) and `dual` (the second speech
     of a dual-dialogue pair starts here). A reader ignores attributes it
     does not know, so these new ones need no new format version;
   - its text, one line per manual line break; a line holding only `\`
     stands for an empty line inside the paragraph;
   - a blank line.
   An empty paragraph is its `@` line and the blank line. A paragraph whose
   element the file does not define reads as General.

## Text

- `**` toggles bold, `*` italic and `_` underline, so `**Bold** and
  *italic*` mean what they look like. Each marker toggles only its own style.
- `\` escapes the next character: `\*`, `\_`, `\\`. A text line that would
  start with `@` or `=` is written with a `\` in front.
- From version 2, notes' anchors are marked in the text: `{n12}` starts the
  text note 12 is attached to and `{/n}` ends it; a `{` in the text is
  written `\{`. The notes themselves are in `appData` under
  `wordsmith.notes` (`{"12": {"text": "…", "author": "…"}}`).
- From version 3, revised text is marked `{r2}…{/r}` (revision set 2, an
  index into `appData` `wordsmith.revisions`), a locked page's first
  character has `{p7}` before it (lock 7), and a paragraph with a deletion in
  set 2 has `revised=2` on its `@` line.
- Inline pictures and fields are not part of a script and are not written.

## Versions and migrations

Each format change gets a new version number and a migration step from the
version before, so every older file stays readable. A file from a newer
version is refused with a message naming both versions. Version 1 is the
first; `crates/wordsmith-format/tests/fixtures/v1-short.wss` is a stored
version-1 file that the tests read on every build.

| Version | Change | Migration from the one before |
| --- | --- | --- |
| 1 | The first format (M4) | — |
| 2 | Note anchors in the text (`{n12}…{/n}`, `\{` for a brace), M5 | Every `{` in version-1 text is escaped, so it stays a brace |
| 3 | Revision marks (`{r2}…{/r}`, `revised=2`) and locked page starts (`{p7}`), M6 | None needed: braces were already escaped |
| 4 | More character styles (`{sD,K}…{/s}`: `D` double, `W` words-only and `O` dotted underline, `K` strike, `U` raised, `L` lowered, `C` capitals, `M` small capitals, `H` and a highlighter name, `S#RRGGBB` any highlight color, `F#RRGGBB` text color), M10 | None needed: braces were already escaped |

## Limits

A file over 64 MB, a settings part over 16 MB, or more than 500,000
paragraphs is refused. A damaged file is refused with the reason; it never
opens half-read.
