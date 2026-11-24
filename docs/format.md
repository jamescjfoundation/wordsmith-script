# The Wordsmith Script file format (`.wss`)

Format version 5. A `.wss` file is UTF-8 plain text made to be read by people,
compared with `diff`, and kept in version control. Every script setting lives
in the file: element definitions, autocomplete lists, scene-heading
separators, more-and-continued texts, and (as later versions added them)
title page, notes, revisions, page locks, scene numbers, outline and story
board.

## Layout

```
Wordsmith Script 5
=== settings ===
{
  "core": { "title": "...", ... },
  "ext": { ... },
  "page": { "pageW": 612.0, "pageH": 792.0, "marginTop": 72.0, ... },
  "revisions": [ { "author": "...", "date": "...", "kind": "insert" }, ... ],
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
   - `ext` (version 5; `appData` before): the document's extension data by
     key. Script's own data has `wordsmith.` keys: `wordsmith.elements`
     (element registry), `wordsmith.sceneHeading`, `wordsmith.autocomplete`,
     `wordsmith.moresContinueds`, `wordsmith.sceneNumbers`, `wordsmith.titlePage`
     and the rest. The document features Script shares with other apps on
     open-source-core keep theirs under `core.` keys: `core.notes`,
     `core.note_types`, `core.revisions`, `core.page_locks`, `core.bookmarks`,
     `core.watermark`, `core.header_footer`, `core.spelling` (the script's own
     correct words);
   - `trackChanges` (version 5): `true` while changes are tracked; left out
     when they are not;
   - `revisions` (version 4): the tracked-change records, in order: each
     one's kind (`insert` or `delete`), author and date. The text refers to
     them by their place in this list (`I3`, `X3`, see Text). Left out when
     there are none. A file holds at most 100,000; a script with more is
     not saved (the save says so) rather than saved without some of them.
   Every field is optional; what is missing comes from the screenplay
   template.
3. **Script:** the line `=== script ===`, then each paragraph in order:
   - a line `@ElementId`, optionally followed by attributes separated by
     spaces: `page-break` (the paragraph starts a new page), `#12A` (a
     locked scene number: capital letters, digits, `-` and `.`, at most 12
     bytes; a number that is not like this is neither written nor read), `omitted=N`, `beat=N`, `revised=N`, and since
     v0.2 `bookmark=N` (a bookmark or label), `dual` (the second speech
     of a dual-dialogue pair starts here) and `scene=N` (the id of a
     scene's own data). A reader keeps attributes it does not know (up to 32
     on a paragraph, each one word of at most 256 bytes) and writes them
     back unchanged when it saves, so a later version can add attributes
     without a new format version and an older version does not drop them;
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
  written `\{`. The notes themselves are in `ext` under `core.notes`
  (`{"12": {"text": "…", "author": "…"}}`; `appData` `wordsmith.notes` before
  version 5).
- From version 3, revised text is marked `{r2}…{/r}` (revision set 2, an
  index into the sets in `ext` `core.revisions`), a locked page's first
  character has `{p7}` before it (lock 7), and a paragraph with a deletion in
  set 2 has `revised=2` on its `@` line.
- From version 4, styles beyond bold, italic and plain underline are marked
  `{sD,K}…{/s}`: comma-separated codes, `D` double, `W` words-only and `O`
  dotted underline, `K` strike, `U` raised, `L` lowered, `C` capitals, `M`
  small capitals, `H` and a highlighter name, `S#RRGGBB` a highlight of any
  color, `F#RRGGBB` the text color. Tracked changes use the same mark: `I3`
  is text inserted and `X3` text deleted as change 3 (an index into
  `revisions`). A reader skips a code it does not know.
- Inline pictures and fields are not part of a script and are not written.

## Versions and migrations

Each format change gets a new version number and a migration step from the
version before, so every older file stays readable. A file from a newer
version is refused with a message naming both versions. A file is saved in
the newest version, so an older file that is saved again can then be opened
only by a version of the app that reads it.

`crates/wordsmith-format/tests/fixtures` keeps one small file per version
(`v1-short.wss`, `v2-notes.wss`, `v3-revisions.wss`, `v4-styles.wss`), each
using what its version added. The tests read every one on every build, save
it again and check that nothing was lost. `tests/fixtures/saved-by-0.1.2/`
keeps the files Wordsmith Script 0.1.2 saved for each golden script and each
fixture, with the pages 0.1.2 planned for them: a test opens every one, saves
it in the newest format, reopens it and checks that it paginates exactly as
0.1.2 did (`tests/migration.rs`).

| Version | Change | Migration from the one before |
| --- | --- | --- |
| 1 | The first format (decision 0017) | — |
| 2 | Note anchors in the text (`{n12}…{/n}`, `\{` for a brace), decision 0023 | Every `{` in version-1 text is escaped, so it stays a brace |
| 3 | Revision marks (`{r2}…{/r}`, `revised=2`) and locked page starts (`{p7}`), decisions 0024 and 0026 | None needed: braces were already escaped |
| 4 | More character styles (`{sD,K}…{/s}`: `D` double, `W` words-only and `O` dotted underline, `K` strike, `U` raised, `L` lowered, `C` capitals, `M` small capitals, `H` and a highlighter name, `S#RRGGBB` any highlight color, `F#RRGGBB` text color), decision 0032; tracked changes (the `I3` and `X3` codes and the `revisions` settings key), decision 0035 | None needed: braces were already escaped |
| 5 | Script's data becomes the document's extension data (`ext`), on open-source-core (decision 0044); the shared document features keep theirs under `core.` keys; change tracking is `trackChanges`. The text, the `@` line attributes and the marks are as in version 4: `#12A`, `omitted=`, `beat=`, `bookmark=`, `scene=`, `dual` and `revised=` now read into and write from the paragraph's extension data (`script.scene_number`, `script.omitted`, `script.beat`, `core.bookmark`, `script.scene_id`, `script.dual`, and the paragraph mark's `core.revised`), and `{n}`, `{r}`, `{p}` into the run marks `core.note`, `core.revised`, `core.page_lock` | `appData` is renamed `ext`; `wordsmith.notes`, `wordsmith.noteTypes`, `wordsmith.revisions`, `wordsmith.locks`, `wordsmith.bookmarks`, `wordsmith.watermark`, `wordsmith.headerFooter` and `wordsmith.spelling` become `core.notes`, `core.note_types`, `core.revisions`, `core.page_locks`, `core.bookmarks`, `core.watermark`, `core.header_footer` and `core.spelling` with the same values; `wordsmith.tracking` `{"on": true}` becomes `trackChanges: true` |

## Limits

A file over 64 MB, a settings part over 16 MB, or more than 500,000
paragraphs is refused. A damaged file is refused with the reason; it never
opens half-read.
