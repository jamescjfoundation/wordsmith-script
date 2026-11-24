# Wordsmith Script: behavior spec

As of 2026-11-19

## Purpose and clean-room rules

This is the behavior spec Wordsmith Script v0.1 was built from. It describes
what a professional screenwriting editor does, drawn from public
screenplay-format guides and general screenwriting practice, and our own
design where those are silent. `docs/SPEC-2.md` extends it (decision 0030).

Rules for contributors:

1. Build behavior and concepts only. Write every name, UI string, dialog
   layout, icon, default template and help text yourself.
2. Names in the interface are our own. Never use another screenwriting
   product's name, trademarks or feature names, even where the word would fit.
   The maintainers keep the list of names to avoid; if you are not sure about
   a name, ask in the issue or pull request before you use it.
3. Never open, run, decompile or read the files of another screenwriting
   program, including its bundled templates and fonts, to find out how it
   behaves.
4. Proprietary file formats are out of scope (decision 0042). Interchange is
   our own open format and Fountain.
5. Where this spec is silent, follow public screenplay standards (the
   formatting section) or design it fresh. Do not fill gaps by guessing what
   another program does.

## Document model and elements

A script is an ordered list of paragraphs, and every paragraph carries exactly
one element type. Element types work like word-processor styles: one
definition per type, applied to every paragraph of that type. Scenes are not
stored separately; a scene runs from one Scene Heading to the next, so derive
scenes from Scene Heading paragraphs.

| Element | What it is | Casing and notes |
| --- | --- | --- |
| General | Untyped text | None |
| Scene Heading (slug line) | Intro + location + time of day, e.g. INT. KITCHEN - NIGHT | Always uppercase. Default intros INT, EXT, I/E; the list is user-editable |
| Action | What we see and hear | Sentence case |
| Character | Name of the speaker, before dialogue | Always uppercase; may carry an extension such as (V.O.), (O.S.), (CONT'D) |
| Dialogue | Spoken words | Follows Character or Parenthetical |
| Parenthetical | Delivery direction | Before or between lines of dialogue |
| Transition | CUT TO:, DISSOLVE TO: | Uppercase, right side; marks a scene end |
| Shot | Camera shot | Uppercase, not numbered |
| Cast List | Characters in the scene | Directly below the Scene Heading; mainly multi-camera sitcoms |
| New Act / End of Act | Act boundaries | Used by TV templates |
| Outline 1, 2, 3, Note, Summary | Outline elements sent from the outliner into the script; can be shown or hidden | Outline 1 = beat title, Summary = beat body |
| Custom elements | User-created | New ones start from Action's settings |

These are default flows, not hard rules: the editor never blocks an unusual
order. Scripts also carry a title page, headers and footers with placeholder
fields (date, scene, label, page), and optional scene numbers.

## Typing behavior

The writer never formats by hand: two keys drive an element-type state
machine, and the user can reconfigure both.

**Return at the end of a paragraph** creates a new paragraph whose type is the
current element's "next element" setting. Each element stores its own next
element, editable per element. For example, changing Dialogue's next element
from Action to Character suits rapid back-and-forth.

**Tab** changes the current paragraph's type (or, on an empty paragraph, the
type you are about to type).

| Current element | Return creates | Tab switches to |
| --- | --- | --- |
| General | General |  |
| Scene Heading | Action | Action |
| Action | Action | Character |
| Character | Dialogue | Transition (only if the line is blank) |
| Dialogue | Action | Parenthetical |
| Parenthetical | Dialogue | Dialogue |
| Transition | Scene Heading | Scene Heading |
| Shot | Action |  |
| Cast List | Action |  |
| New Act | Scene Heading |  |
| End of Act | New Act |  |
| Summary | Summary |  |
| Outline 1 / 2 / 3 |  | Outline 2 / Outline 3 / Note |
| Note |  | Note |

Other typing rules:

- Inside a Scene Heading, Tab after the intro inserts ". " and moves to the
  location; Tab after the location inserts " - " and moves to the time.
  Separators are stored per list and can be changed.
- Change element directly: a menu command, right-click submenu or toolbar
  selector reformats the paragraph under the cursor. Shortcuts Mod+0 to Mod+9
  select General, Scene Heading, Action, Character, Parenthetical, Dialogue,
  Transition, Shot, Cast List, New Act. Core shortcuts are fixed; custom
  elements can be given their own.
- An option can make the element after Dialogue default to Character instead
  of Action.

**Autocomplete lists.** Six lists: Characters, Character Extensions, Scene
Intros, Locations, Times of day, Transitions.

- Lists fill themselves from text typed in the script; users can also add,
  edit and delete entries, and switch matching off per list.
- Rebuild compares each list with the script, dropping terms no longer used
  and adding missing ones.
- Tab accepts a suggestion. Options: Tab after Character inserts an extension
  instead of a Parenthetical; guess the next speaker in a two-person exchange;
  list order alphabetical, custom, or by number of speeches.
- Design the suggestion popup ourselves. Writers dislike predictive features
  that override what they type, so make suggestions easy to ignore and never
  insert one automatically.

## Formatting, page geometry and pagination

Each element type owns one set of formatting and behavior settings, and a
change applies to every paragraph of that type across the script.

Per-element settings to support:

- Font, size and style. A command applies font and size to all elements but
  deliberately leaves bold and italic alone.
- Alignment: left, centered, right, justified. Line spacing: 1, 1.5, 2, 3.
  Space before: 0, 1, 2, or any value up to 30 lines.
- Left and right indents, measured in inches from the left edge of the page.
- Next element on Return; start new page (forces a page break before the
  element); paginate-as (which element's page-break rules this element
  follows); optional shortcut.
- Element settings can be copied from a template into the current script.
  Global line-spacing presets change page count (a 100-page script becomes
  about 90 pages at the tightest setting and 107 at the loosest), so default
  to normal to keep one page per minute.

**Page geometry defaults.** Take these from public format standards. The
guides agree on the page and font but differ by a few tenths of an inch on
indents, so treat the indents below as our starting template and tune them
against printed reference scripts.

| Setting | Default | Source |
| --- | --- | --- |
| Paper | US Letter 8.5 x 11 in; A4 as an option | [Celtx](https://blog.celtx.com/screenplay-margins-guide/), [screenplay.com](https://screenplay.com/pages/basic-screenplay-format) |
| Font | 12 pt monospaced Courier-style: 10 characters per inch, 6 lines per inch. Ship an open-licensed font such as Courier Prime | [Wikibooks](https://en.wikibooks.org/wiki/Movie_Making_Manual/Writing/Screenplay_Format) |
| Margins | Top 1 in, bottom 1 in, left 1.5 in, right 1 in | Wikibooks, Celtx |
| Scene Heading, Action, Shot | Left 1.5 in, right 7.5 in | Wikibooks |
| Character | Left about 3.7 to 3.8 in | Celtx, Wikibooks |
| Dialogue | Left about 2.5 in, width about 3.5 in | Wikibooks, Celtx |
| Parenthetical | Left about 3.0 to 3.1 in, width about 2.5 in | Wikibooks, Celtx |
| Transition | Right-aligned to the right margin | All three |
| Spacing before | Scene Heading 1 to 2 blank lines; Dialogue and Parenthetical 0 | Wikibooks |
| Page numbers | Top right, followed by a period; none on page 1 | Wikibooks |

**Mores and continueds.** One settings dialog controls all of them, and each
text can be edited and given its own font.

- Dialogue split across a page: "(MORE)" at the bottom, and the character
  name repeated with "(CONT'D)" at the top of the next page.
- Automatic character continueds: when Action interrupts one character's
  speech within a scene, add CONT'D after the name on the next speech. Do not
  add it when the speech carries into the next scene.
- Scene split across a page: "(CONTINUED:)" at the top of the next page,
  optionally with the scene's page count, such as (2).

Design the rest from screenplay convention: keep Character with at least one
line of Dialogue, never leave a Scene Heading alone at the foot of a page,
split Dialogue and Action only at sentence boundaries, and set widow and
orphan minimums. RTF and plain-text exports drop the mores and continueds.

## Views, outlining, navigation and notes

Around the editor: three script display modes, plus card, scene, outline and
table views of the same script. All views edit one underlying script; none
keeps its own copy.

| Feature | Behavior to build |
| --- | --- |
| Normal view | Continuous text, no page edges |
| Page view | WYSIWYG pages with breaks, headers and revision colors in the margin |
| Draft view | Minimal formatting for fast typing |
| Typewriter mode | Keeps the line being edited fixed at eye level while scrolling |
| Focus mode, dark theme | Distraction-free writing; a dark-gray theme, optionally matched in PDF colors |
| Index cards | One card per scene; reorder cards to reorder scenes |
| Scene view | Scenes as movable blocks of script text |
| Story board | Free-form canvas of colored beats with title and body; Tab toggles title and body; beats link with flow lines that stay attached when moved; snap to grid; drag an image in to make an illustrated beat; export to PDF |
| Beat to script | Sending a beat puts its title in as Outline 1 and its body as Summary |
| Outline editor | Any number of horizontal lanes (for example acts, then sequences) that can be added, renamed, moved, indented; a Script lane; beats get page ranges; edits sync both ways with the story board; ships structure templates |
| Structure colors | Colors assigned to outline levels, acts, sequences and scenes, shown across the script, the Script panel and the outliner, optionally in PDF |
| Tables | Tabbed tables (scenes, characters, more): add, rename, reorder and filter tabs; edit cells; reorder rows to reorder the script; export a tab as CSV. The characters table sets highlight colors and renames a character everywhere |
| Notes | Notes attached to script text, edited in a popover, listed in the Script panel |
| Writing stats | Writing time and pages added per day, week, month, year; a sprint timer |
| Templates | Screenplay, teleplay, stage play, comics, graphic novel, immersive |
| Track changes | Who changed what, and when; accept and reject. Real-time co-writing and cloud sync are out of scope (decision 0042) |

Also: emoji support everywhere, a character report and a representation
report exported as CSV, and printing index cards and scene view to PDF. Our
aims: local-first storage that never loses work, open formats, and no
subscription.

## Production: revisions, locking and numbering

Production tools keep page and scene numbers stable once a schedule depends
on them, and mark every change in color. Professional users depend on these
most, so they need to be complete and exact.

**Revision mode.** A toggle. While on, inserted or edited text, and lines
where text was deleted, are marked as revised in the active revision set's
style.

- Each revision set has a name, a color and text styles (bold, underline,
  strikethrough).
- The default sequence is the industry color cycle: White, Blue, Pink,
  Yellow, Green, Goldenrod, Buff, Salmon, Cherry, Tan, then Second White,
  Second Blue, and so on. A Next Revision command advances it.
- A revision mark (default asterisk, changeable character) sits in the right
  margin of each revised line; its horizontal position is configurable.
- Page view tints the margins of revised pages in the set's color; PDF output
  can color the whole page. A header field can show the current set name and
  date.
- Mark revised (manual) and clear revised. Clearing only removes the mark
  when the whole line is cleared, and it does not switch revision mode off.
- Print only revised pages, print without marks, and roll a script back to an
  earlier revision set.

**Page locking.** Locking freezes page numbers so edits do not repaginate the
rest of the script. Locking offers to turn on revision mode, and is meant for
pre-production and production, not drafting.

- Overflow from a locked page 12 flows onto a new page 12A; overflow from 12A
  makes 12AA. Use one documented scheme. Each revision can add a lock level.
- Deleted pages leave a placeholder with a combined label, such as 10-11 or
  7BB-9A.
- Unlock one level (removes the newest A level), Unlock all (renumbers
  consecutively), Relock from the cursor, Adjust a locked page's start up or
  down a paragraph, Edit page number by hand, Lock new page at the cursor,
  Unlock one page so text flows back.

**Scene numbering.** Number scenes; show or hide numbers; added scenes after
lock get letter suffixes (101A); cut scenes keep their number with an OMITTED
placeholder; omitted scenes can be recalled. Shots, characters or dialogue
can also be numbered. The starting page number and whether page 1 shows a
number are settable.

**Reports.** Cast and character reports; scene lengths in eighths of a page.
Production breakdown tagging and scheduling export are out of scope
(decision 0042).

## Files: native format, import, export, saving

**Our native format:** a plain-text, diff-friendly file that stores the
ordered paragraph list with element type, inline styles, revision marks,
locks, scene numbers, notes, outline and story-board data, and per-script
element definitions. A template opens as a new untitled script. Fountain
import and export from day one, since it is an open, documented format.

| Direction | Priority |
| --- | --- |
| Open/import | Fountain, .txt, .rtf, text-based PDF (not scanned images) |
| Export | PDF (exact pagination) first, then Fountain, RTF, TXT, HTML, CSV reports, tab-delimited dialogue, story board to PDF |

Plain-text and RTF import ask whether the text is a script and then format it
as a screenplay.

**Saving and backup:** auto-save on by default every 15 minutes, waiting
about 10 seconds of idle time so it never interrupts typing; auto-backup
keeps up to 1,000 timestamped copies by default (maximum 10,000), deleting
the oldest. A file opened over a network can be saved only by the first user
to open it; others must Save As. Keep PDF output small: a PDF should not be
far larger than the script it comes from.

## Build order, gaps and sources

Build in this order, so every stage gives writers something usable:

1. Core model and editor: paragraph list, element types, per-element style
   definitions, Return and Tab state machine, change-element commands.
2. Pagination engine and Page view: geometry defaults, mores and continueds,
   keep-together rules, exact PDF output. Golden-file tests against
   public-domain scripts.
3. Autocomplete lists, scene-heading separators, character extensions.
4. Native format, Fountain, TXT, RTF, text-PDF import; auto-save and backups.
5. Index cards, scene view, tables, notes, title page, headers and footers.
6. Production: revision sets and marks, page locking, scene numbering,
   omitted scenes, reports.
7. Story board, outline editor, structure colors, writing stats, track
   changes.

**Open questions:**

- Default indents per element: confirm one set against printed public-domain
  scripts.
- How paginate-as options behave in detail, and exact keep-together rules.
- Title page fields and layout, dual dialogue, and squeezing a script to fit
  a page count by small spacing changes.

**Sources** for the format standards above: [Celtx margins
guide](https://blog.celtx.com/screenplay-margins-guide/), [screenplay.com
format](https://screenplay.com/pages/basic-screenplay-format), [Wikibooks
screenplay
format](https://en.wikibooks.org/wiki/Movie_Making_Manual/Writing/Screenplay_Format).
