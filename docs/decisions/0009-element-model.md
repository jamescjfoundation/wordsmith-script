# 0009: The element model and its defaults

Date: 2026-11-19 · Status: accepted

Since decision 0044 (2026-11-24) `app_data` is the document's `ext`; nothing
else here changed.

## Decisions

**Elements are paragraph styles plus a small registry.** Each element's
paragraph style holds all of its formatting: font, size, bold, italic,
underline, indents, alignment, line spacing, space before, the next element on
Return (`Style::next`), start new page (`page_break_before`) and keep rules
(`keep_next`, `keep_lines`, `widow_control`). The registry (`ElementDef`, in
`app_data["wordsmith.elements"]`, decision 0008) holds only what a style
cannot: kind, display name, casing, Tab target, whether Tab acts only on a
blank line, paginate-as and the shortcut. One source of truth for each
setting means an element settings change is one undoable edit.

**Kinds, not names.** Every built-in element has a kind
(`ElementKind::SceneHeading` and so on) and a fixed id equal to its style id.
Code asks for kinds, so renaming an element changes nothing else. Custom
elements have kind `Custom` and ids `Custom1`, `Custom2`…

**Unknown styles read as General.** A paragraph with no style or a style that
is not an element is General. The editor never blocks an unusual order.

**Hostile registries are repaired, never fatal.** A stored registry that does
not parse gives the defaults. Otherwise: ids and names are 1 to 64 bytes,
duplicates keep the first, every built-in kind is present, core shortcuts are
fixed (`Mod+0` to `Mod+9`, SPEC), references to missing elements are
cleared, and at most 128 elements are kept, with custom elements dropped
before built-in ones.

**Casing is stored, not displayed.** Uppercase elements (Scene Heading,
Character, Transition, Shot, New Act, End of Act) store capitals: typing
converts, and changing a paragraph into one converts its text. Every export
(Fountain, PDF, text) then sees real text. A display-only caps flag would
make exports disagree with the screen.

**Lines are a 12 pt grid.** Every element's line spacing is `Exactly(12 pt)`
per line, so the page keeps 6 lines per inch even if a writer changes font
size. The spec's spacing choices 1, 1.5, 2 and 3 become 12, 18, 24 and 36 pt.
Space before is given in lines (12 pt each).

**Defaults the spec's geometry table does not cover** (positions in inches
from the page's left edge, as the spec measures):

| Element | Left | Right | Align | Before | Other |
| --- | --- | --- | --- | --- | --- |
| General | 1.5 | 7.5 | left | 0 | |
| Scene Heading | 1.5 | 7.5 | left | 1 line | uppercase; keep with next |
| Action | 1.5 | 7.5 | left | 1 line | |
| Character | 3.7 | 7.5 | left | 1 line | uppercase; keep with next |
| Dialogue | 2.5 | 6.0 | left | 0 | |
| Parenthetical | 3.1 | 5.6 | left | 0 | keep with next |
| Transition | 5.5 | 7.5 | right | 1 line | uppercase |
| Shot | 1.5 | 7.5 | left | 1 line | uppercase; keep with next |
| Cast List | 1.5 | 7.5 | left | 0 | sits directly below its heading |
| New Act | 1.5 | 7.5 | center | 0 | uppercase, underlined, starts a new page |
| End of Act | 1.5 | 7.5 | center | 2 lines | uppercase, underlined |
| Outline 1 / 2 / 3 | 1.5 / 1.75 / 2.0 | 7.5 | left | 1 line | 1 and 2 bold |
| Note | 2.0 | 7.5 | left | 1 line | italic |
| Summary | 1.5 | 7.5 | left | 1 line | |

Return targets the spec does not give: Outline 1, 2 and 3 and Note create
another of themselves; custom elements create Action. A custom element starts
from Action's current settings (SPEC, Sourced).

**Courier Prime's pitch.** Courier Prime's advance width is 1228/2048 em, so
10 characters at 12 pt measure 71.95 pt instead of 72: within 0.07% of
10 characters per inch, and a 60-character line is 0.28 pt short. The test
holds it to 0.1 pt per 10 characters and checks that every character has the
same width. Line pitch is exactly 12 pt.

## Revisit when

Printed public-domain scripts are measured for the pagination golden tests
(`docs/golden.md`; SPEC open question on indents). Tune the table then and record the change here.
