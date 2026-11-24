# Spec audit for v0.1

Date: 2026-11-19

Every Verified and Sourced item in `docs/SPEC.md` for milestones M1 to M7,
checked against the code. "Built" names where it lives and the tests that
cover it; "Gap" items are in `docs/gaps.md` with their reason. Items the spec
itself marks as refuted, open, or out of scope are listed at the end.

## Document model and elements

| Item | Status | Where |
| --- | --- | --- |
| Ordered paragraphs, one element each; scenes derived from headings | Built | `wordsmith-script::element`, `scenes` |
| General, Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, Shot, Cast List, New Act, End of Act, Outline 1 to 3, Note, Summary, custom elements | Built | `element.rs`; decision 0009 |
| Casing (heading, character, transition in capitals) | Built | `Casing`, M1 tests |
| Title page | Built | `title_page.rs`; decision 0021 |
| Headers and footers with date, scene, label (page labels when locked), page | Built | `header_footer.rs`, `locks.rs`; decisions 0021, 0026 |
| Optional scene numbers | Built | `scene_numbers.rs`; decision 0025 |

## Typing

| Item | Status | Where |
| --- | --- | --- |
| Return creates each element's next element; Tab table | Built | `commands/text.rs`; decision 0010 |
| Tab in a Scene Heading inserts `. ` and ` - `; separators stored and changeable | Built | `scene_heading.rs` |
| Change element by menu, toolbar and right-click submenu | Built | `chrome.rs`; right-click menu in `editor.rs` (M8) |
| Mod+0 to Mod+9; custom elements' own shortcuts | Built | `element.rs`, `keys.rs` |
| Option: Character after Dialogue | Built | `element.dialogueThenCharacter` |
| Six autocomplete lists, learning, add/edit/delete, off per list, rebuild, Tab accepts, extension on Tab, next-speaker guess, list order | Built | `autocomplete.rs`; decision 0015 |
| Suggestions never auto-insert | Built | decision 0015 |

## Formatting and pagination

| Item | Status | Where |
| --- | --- | --- |
| Per-element font, size, style, alignment, line spacing, space before (to 30 lines), indents, next element, start new page, paginate as, shortcut | Built | `element.configure` |
| Font and size for all elements, bold and italic left alone | Built | `element.applyFontToAll` |
| Copy element settings from a template | Gap | one template in v0.1 |
| Global line-spacing presets (tighter or looser pages) | Gap | |
| Page geometry defaults (US Letter, Courier Prime 12, margins, indents) | Built | `template.rs` |
| A4 as an option | Gap | the page size is the template's |
| Page numbers top right with a period, none on page 1 | Built | `header_footer.rs` default |
| (MORE) and (CONT'D) for split speeches; automatic CONT'D within a scene; CONTINUED for scenes, with a page count | Built | `paginate`; decision 0012 |
| Mores and continueds texts editable | Built | `moresContinueds.configure` |
| Their own font | Gap | |
| Keep Character with Dialogue, heading never alone at the foot, split at sentences, widows and orphans | Built | `paginate/plan.rs` |
| RTF and plain-text exports without mores and continueds | Built | `export.rs` |

## Views, outlining, navigation and notes

| Item | Status | Where |
| --- | --- | --- |
| Normal, page and draft views (SPEC's speed view); typewriter mode; focus mode; dark theme | Built | `editor.rs`; decision 0016 |
| Dark colors matched in PDF | Gap | |
| Index cards (reorder to reorder the script); scene view | Built | `structure.rs`; decision 0022 |
| Printing cards and scene view to PDF | Gap | |
| Story board: colored beats with title and body, Tab between them, links that stay attached, snap to grid | Built | `storyboard.rs`, `board.rs`; decision 0028 |
| Images in beats; story board to PDF | Gap | ROADMAP |
| Beat to script as Outline 1 and Summary | Built | `beat.toScript` |
| Outline editor: lanes added, renamed, moved, indented; Script lane; page ranges; both-way sync; templates | Built | `outline.rs`; decision 0028 |
| Outline elements shown or hidden | Built | `outline.show` (page view, PDF, print) |
| Structure colors | Gap | ROADMAP |
| Tables: scenes and characters, edit cells, reorder rows, CSV, rename a character everywhere | Built | `tables.rs`, `character.rs`; decision 0022 |
| Tables: add, rename, reorder and filter tabs; character highlight colors | Gap | |
| Notes attached to text, popover, listed | Built | `notes.rs`; decision 0023 |
| Writing stats and sprint timer; teleplay and other templates; inclusivity report | Gap | ROADMAP |
| Character report | Built | `reports.rs`; decision 0027 |
| Emoji everywhere | Gap | drawn only where the bundled or system fonts have them; not tested |

## Production

| Item | Status | Where |
| --- | --- | --- |
| Revision mode; sets with name, color and styles; the color cycle; Next Revision | Built | `revisions.rs`; decision 0024 |
| Revision mark beside each revised line, character and position changeable | Built | `paginate/margin.rs` |
| Page view tints revised pages; PDF can color whole pages; header field shows the set | Built | |
| Mark and clear revised; print only revised pages; print without marks; roll back | Built | decisions 0024, 0027 |
| Clearing only removes a mark when the whole line is cleared | Gap | clearing removes the marks of the text selected |
| Locking with A-pages; deleted pages leave a combined placeholder; locking offers revision mode; unlock one level; unlock all | Built | `locks.rs`; decision 0026 |
| Relock from the cursor; adjust a locked page's start; edit a page number; lock a new page at the cursor; unlock one page | Gap | |
| Scene numbers shown or hidden; letters after lock; OMITTED kept and recalled | Built | decision 0025 |
| Numbering shots, characters or dialogue | Gap | |
| Starting page number settable; page 1 number shown or not | Partly | page 1 rule built (`firstPage`); a starting number is a gap |
| Scene lengths in eighths | Built | `reports.rs` |
| Breakdown tagging and scheduling export | Out of scope | kickoff |

## Files

| Item | Status | Where |
| --- | --- | --- |
| Native, diff-friendly format with everything the brief lists | Built | `native.rs`, `docs/format.md`; decision 0017 |
| Fountain import and export | Built | decision 0018 |
| Plain text, RTF and text-based PDF import; the "is this a script?" question | Built | `import/`; decision 0019 |
| PDF, Fountain, RTF, plain text, HTML and CSV export | Built | |
| Auto-save every 15 minutes after 10 seconds idle; 1,000 timestamped backups (up to 10,000) | Built | decision 0020 |
| Files opened over a network saved only by the first user | Gap | no file locking |

## Refuted, open or out of scope in the spec

- The other product's suggestion popup and inline grey text: refuted; ours is
  our own (decision 0015).
- Proprietary formats: out of scope (kickoff).
- Dual dialogue, the tightening feature: open questions; dual dialogue reads
  as ordinary dialogue.
- Collaboration and cloud sync: out of scope.
