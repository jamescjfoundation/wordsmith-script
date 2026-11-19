# Wordsmith Script: clean-room spec from public Final Draft docs

As of 2026-11-19

## Purpose and clean-room rules

This is the behavior spec the Wordsmith Script build agent works from. It describes what Final Draft 13 does, as stated in Final Draft's own public manuals, knowledge base and blog, plus public screenplay-format guides. Nobody inspected, ran or decompiled the product to write it.

How sure each statement is:

- **Verified**: three independent checkers re-read the source and agreed (3-0 or 2-1).
- **Sourced**: pulled from a primary vendor page but not put through the 3-vote check (the run's budget covered 25 of 107 claims). Treat it as accurate but re-read the cited page before building to it.
- **Refuted**: checkers rejected it. Do not build to it.

Rules for the build agent:

1. Copy behavior and concepts only. Write every name, UI string, dialog layout, icon, default template and help text yourself.
2. Never use Final Draft trademarks or feature names in the product: Final Draft, SmartType, Beat Board, ScriptNotes, Navigator, Typeset, Courier Final Draft, Midnight Mode. Pick our own names (for example "Autocomplete lists", "Story board", "Notes").
3. Never open, run, decompile or read the files of a Final Draft install, including bundled templates and fonts. Final Draft's license forbids reverse engineering and using the software to create similar software.
4. Do not derive FDX import/export from Final Draft files or their internals. It needs its own legitimately sourced spec (see the last section). Use our own open format and Fountain for interchange first.
5. Where this spec is silent, follow public screenplay standards (the formatting section) or design it fresh. Do not fill gaps by guessing what Final Draft does.
6. Legal sign-off on observing a licensed copy is still open; until a lawyer decides, this document and public pages are the only inputs.

## Document model and elements

A script is an ordered list of paragraphs, and every paragraph carries exactly one element type (Verified, [elements](https://kb.finaldraft.com/hc/en-us/articles/27646947570196-What-are-script-elements)). Element types work like word-processor styles: one definition per type, applied to every paragraph of that type. Scenes are not stored separately; a scene runs from one Scene Heading to the next, so derive scenes from Scene Heading paragraphs.

| Element | What it is | Casing and notes | Status |
| --- | --- | --- | --- |
| General | Untyped text | None | Verified |
| Scene Heading (slug line) | Intro + location + time of day, e.g. INT. KITCHEN - NIGHT | Always uppercase. Default intros INT, EXT, I/E; list is user-editable | Verified |
| Action | What we see and hear | Sentence case | Verified |
| Character | Name of the speaker, before dialogue | Always uppercase; may carry an extension such as (V.O.), (O.S.), (CONT'D) | Verified |
| Dialogue | Spoken words | Follows Character or Parenthetical | Verified |
| Parenthetical | Delivery direction | Before or between lines of dialogue | Verified |
| Transition | CUT TO:, DISSOLVE TO: | Uppercase, right side; marks a scene end | Sourced |
| Shot | Camera shot | Uppercase, not numbered | Sourced |
| Cast List | Characters in the scene | Directly below the Scene Heading; mainly multi-cam sitcom | Sourced |
| New Act / End of Act | Act boundaries | Used by TV templates | Verified (named in Return defaults) |
| Outline 1, 2, 3, Note, Summary | Outline elements sent from the outliner into the script; can be shown or hidden | Outline 1 = beat title, Summary = beat body | Verified names; behavior Sourced |
| Custom elements | User-created | New ones start from Action's settings | Verified exists; defaults Sourced |

These are default flows, not hard rules: the editor never blocks an unusual order. Scripts also carry a title page, headers and footers with placeholder fields (date, scene, label, page), and optional scene numbers (Sourced, [FD13 manual section](https://kb.finaldraft.com/hc/en-us/sections/27495762684564-Final-Draft-13-manual)).

## Typing behavior

The writer never formats by hand: two keys drive an element-type state machine, and the user can reconfigure both (Verified, [shortcuts](https://kb.finaldraft.com/hc/en-us/articles/27977488282644-What-keyboard-shortcuts-can-I-use-in-Final-Draft), [element settings](https://kb.finaldraft.com/hc/en-us/articles/27647938211604-How-do-I-modify-element-behavior-change-element-fonts-and-margins-or-create-new-custom-elements)).

**Return at the end of a paragraph** creates a new paragraph whose type is the current element's "next element" setting. Each element stores its own next element, editable per element. Example given by the vendor: change Dialogue's next from Action to Character for rapid back-and-forth.

**Tab** changes the current paragraph's type (or, on an empty paragraph, the type you are about to type).

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

- Inside a Scene Heading, Tab after the intro inserts ". " and moves to the location; Tab after the location inserts " - " and moves to the time (Verified). Separators are stored per list and can be changed (Sourced).
- Change element directly: a menu command, right-click submenu or toolbar selector reformats the paragraph under the cursor (Verified). Shortcuts Mod+0 to Mod+9 select General, Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot, Cast List, New Act (Sourced). Core shortcuts are fixed; custom elements can be given their own (Sourced).
- An option can make the element after Dialogue default to Character instead of Action (Sourced).

**Autocomplete lists** (Final Draft calls this SmartType; we must name ours differently). Six lists: Characters, Character Extensions, Scene Intros, Locations, Times of day, Transitions (Verified, [SmartType](https://kb.finaldraft.com/hc/en-us/articles/27750003388948-What-is-SmartType-and-how-do-I-use-it)).

- Lists fill themselves from text typed in the script; users can also add, edit and delete entries, and switch matching off per list.
- Rebuild compares each list with the script, dropping terms no longer used and adding missing ones.
- Tab accepts a suggestion (Verified). Options exist for: Tab after Character inserting an extension instead of a Parenthetical, guessing the next speaker in a two-person exchange, and list order alphabetical, custom, or by number of speeches (Sourced).
- The exact popup and inline "ghost text" behavior was Refuted (0-3). Design the suggestion UI ourselves.
- Public reviews say users dislike predictive features overriding what they type ([Filmmaker](https://filmmakermagazine.com/?p=120862)). Make suggestions easy to ignore and never auto-insert.

## Formatting, page geometry and pagination

Each element type owns one set of formatting and behavior settings, and a change applies to every paragraph of that type across the script (Verified, [element settings](https://kb.finaldraft.com/hc/en-us/articles/27647938211604-How-do-I-modify-element-behavior-change-element-fonts-and-margins-or-create-new-custom-elements)).

Per-element settings to support:

- Font, size and style. A command applies font and size to all elements but deliberately leaves bold and italic alone (Verified).
- Alignment: left, centered, right, justified. Line spacing: 1, 1.5, 2, 3. Space before: 0, 1, 2, or any value up to 30 lines (Verified).
- Left and right indents, measured in inches from the left edge of the page (Verified).
- Next element on Return; start new page (forces a page break before the element); paginate-as (which element's page-break rules this element follows); optional shortcut (Verified).
- Element settings can be copied from a template into the current script (Sourced). Global line-spacing presets change page count: a 100-page script becomes about 90 pages at the tightest setting and 107 at the loosest, so default to normal to keep one page per minute (Sourced, [FD13 Mac guide](https://www.finaldraft.com/downloads/manuals/fd13-user-guide-mac-exported.pdf)).

**Page geometry defaults.** Take these from public format standards, not from Final Draft. The guides agree on the page and font but differ by a few tenths of an inch on indents, so treat the indents below as our starting template and tune them against printed reference scripts.

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

**Mores and continueds** (Sourced, [KB](https://kb.finaldraft.com/hc/en-us/articles/27747514444180-How-do-I-modify-or-remove-Mores-and-Continueds-CONT-Ds)). One settings dialog controls all of them, and each text can be edited and given its own font.

- Dialogue split across a page: "(MORE)" at the bottom, and the character name repeated with "(CONT'D)" at the top of the next page.
- Automatic character continueds: when Action interrupts one character's speech within a scene, add CONT'D after the name on the next speech. Do not add it when the speech carries into the next scene.
- Scene split across a page: "(CONTINUED:)" at the top of the next page, optionally with the scene's page count, such as (2).

Not found in public sources, so design from screenplay convention: keep Character with at least one line of Dialogue, never leave a Scene Heading alone at the foot of a page, split Dialogue and Action only at sentence boundaries, and set widow and orphan minimums. RTF and plain-text exports drop the mores and continueds (Sourced).

## Views, outlining, navigation and notes

Around the editor, FD13 offers three script display modes plus card, scene, outline and table views of the same script (existence Verified, [FD13 manual section](https://kb.finaldraft.com/hc/en-us/sections/27495762684564-Final-Draft-13-manual); details Sourced from [FD13 launch post](https://blog.finaldraft.com/final-draft-13-is-here) and the [quick start](https://www.finaldraft.com/learn/final-draft-quick-start/)). All views edit one underlying script; none keeps its own copy.

| Feature | Behavior to build | Status |
| --- | --- | --- |
| Normal view | Continuous text, no page edges | Verified exists |
| Page view | WYSIWYG pages with breaks, headers and revision colors in the margin | Verified exists |
| Speed view | Draft mode with minimal formatting for fast typing | Verified exists |
| Typewriter mode | Keeps the line being edited fixed at eye level while scrolling | Sourced |
| Focus mode, dark theme | Distraction-free writing; a dark-gray theme, optionally matched in PDF colors | Sourced |
| Index cards | One card per scene; reorder cards to reorder scenes | Verified exists |
| Scene view | Scenes as movable blocks of script text | Verified exists |
| Story board (their Beat Board) | Free-form canvas of colored beats with title and body; Tab toggles title and body; beats link with flow lines that stay attached when moved; snap to grid; drag an image in to make an illustrated beat; export to PDF | Sourced |
| Beat to script | Sending a beat puts its title in as Outline 1 and its body as Summary (FD13). An older source says Scene Heading and Action; use the outline-element behavior | Sourced, sources differ |
| Outline editor | Any number of horizontal lanes (for example acts, then sequences) that can be added, renamed, moved, indented; a Script lane; beats get page ranges; edits sync both ways with the story board; ships structure templates | Sourced |
| Structure lines | Colors assigned to outline levels, acts, sequences and scenes, shown across script, navigator and outliner, optionally in PDF | Sourced |
| Navigator | Tabbed tables (scenes, characters, more): add, rename, reorder and filter tabs; edit cells; reorder rows to reorder the script; export a tab as CSV. Characters tab sets highlight colors and renames a character everywhere | Sourced |
| Notes | Notes attached to script text, edited in a popover, listed in the navigator; threaded replies in the cloud version | Sourced |
| Writing stats | Writing time and pages added per day, week, month, year; a sprint timer | Sourced |
| Templates | Screenplay, teleplay, stage play, comics, graphic novel, immersive | Sourced |
| Collaboration | Real-time co-writing and track changes (desktop FD12+); a browser version with cloud storage, sharing and sync | Sourced |

Other items named in public material: emoji support everywhere, a Character Report and an inclusivity report exported as CSV, and printing index cards and scene view to PDF (Sourced). Our competitive edge, from public reviews, is real-time collaboration without a subscription lock-in and safe local-first storage ([Filmmaker](https://filmmakermagazine.com/?p=120862)).

## Production: revisions, locking and numbering

Production tools keep page and scene numbers stable once a schedule depends on them, and mark every change in color. This is the feature set reviewers call Final Draft's main strength, so parity here matters most for professional users (existence Verified; mechanics Sourced from [Revision Mode](https://kb.finaldraft.com/hc/en-us/articles/27813588722708-How-do-I-use-Revision-Mode), [locking tools](https://kb.finaldraft.com/hc/en-us/articles/27817054455828-How-do-I-lock-or-unlock-script-pages-and-use-the-locking-tools) and [Production section](https://kb.finaldraft.com/hc/en-us/sections/15877856011796-Production)).

**Revision mode.** A toggle. While on, inserted or edited text, and lines where text was deleted, are marked as revised in the active revision set's style.

- Each revision set has a name, a color and text styles (bold, underline, strikethrough).
- The default sequence is the industry color cycle: White, Blue, Pink, Yellow, Green, Goldenrod, Buff, Salmon, Cherry, Tan, then Second White, Second Blue, and so on. A Next Revision command advances it.
- A revision mark (default asterisk, changeable character) sits in the right margin of each revised line; its horizontal position is configurable.
- Page view tints the margins of revised pages in the set's color; PDF output can color the whole page. A header field can show the current set name and date.
- Mark revised (manual) and clear revised. Clearing only removes the mark when the whole line is cleared, and it does not switch revision mode off.
- Print only revised pages, print without marks, and roll a script back to an earlier revision set.

**Page locking.** Locking freezes page numbers so edits do not repaginate the rest of the script. Locking offers to turn on revision mode, and is meant for pre-production and production, not drafting.

- Overflow from a locked page 12 flows onto a new page 12A; overflow from 12A makes 12AA (sources also show mixed forms such as 4aA, so pick one documented scheme). Each revision can add a lock level.
- Deleted pages leave a placeholder with a combined label, such as 10-11 or 7BB-9A.
- Unlock one level (removes the newest A level), Unlock all (renumbers consecutively), Relock from the cursor, Adjust a locked page's start up or down a paragraph, Edit page number by hand, Lock new page at the cursor, Unlock one page so text flows back.

**Scene numbering.** Number scenes; show or hide numbers; added scenes after lock get letter suffixes (101A); cut scenes keep their number with an OMITTED placeholder; omitted scenes can be recalled. Shots, characters or dialogue can also be numbered. The starting page number and whether page 1 shows a number are settable (Sourced, partly from KB article titles only).

**Breakdown and reports.** Tag script items (props, cast, locations) for production breakdown and export them to scheduling tools; cast and character reports; scene lengths in eighths of a page (Sourced).

## Files: native format, import, export, saving

Final Draft's native file is .fdx, an XML-based text format since version 8, with .fdxt templates; opening a template makes a new untitled script (Verified, [PRONOM fmt/1845](https://www.nationalarchives.gov.uk/PRONOM/fmt/1845), FD13 [Mac](https://www.finaldraft.com/downloads/manuals/fd13-user-guide-mac-exported.pdf) and [Windows](https://www.finaldraft.com/downloads/manuals/fd13-user-guide-win-exported.pdf) guides). No public source documents the FDX schema, so our format is our own design.

**Our native format (design brief, not from Final Draft):** a plain-text, diff-friendly file that stores the ordered paragraph list with element type, inline styles, revision marks, locks, scene numbers, notes, outline and story-board data, and per-script element definitions. Fountain import and export from day one, since it is an open, documented format.

| Direction | Formats Final Draft supports | Parity priority for us |
| --- | --- | --- |
| Open/import | .fdx, .fdxt, legacy .fdr/.fdt (FD 5 to 7), .txt, .rtf, .fcf, text-based PDF (FD12/13 only, not scanned images) | Fountain, .txt, .rtf, text PDF first; FDX once a legitimate spec exists |
| Export | Template, RTF, plain text, text with layout, HTML, scheduling export (.sex), .fcf, Avid script-based editing text, PDF, tab-delimited dialogue, story board to PDF | PDF (exact pagination) first, then Fountain, RTF, TXT, HTML, CSV reports |

All of the table is Verified from the [import formats article](https://kb.finaldraft.com/hc/en-us/articles/15575252515988-What-file-formats-can-I-import-into-Final-Draft) and the Windows guide. Plain-text and RTF import ask whether the text is a script and then auto-format it as a screenplay (Sourced). Scheduling tools such as Movie Magic Scheduling read .fdx directly, which is why FDX export matters for production users.

**Saving and backup** (Sourced, Mac guide): auto-save on by default every 15 minutes, waiting about 10 seconds of idle time so it never interrupts typing; auto-backup keeps up to 1,000 timestamped copies by default (maximum 10,000), deleting the oldest. Files opened over a network can be saved only by the first user; others must Save As. Known complaint to avoid: on Windows, PDFs can be far larger than the script file.

## Build order, gaps and sources

Build in this order, so every stage gives writers something usable:

1. Core model and editor: paragraph list, element types, per-element style definitions, Return and Tab state machine, change-element commands.
2. Pagination engine and Page view: geometry defaults, mores and continueds, keep-together rules, exact PDF output. Golden-file tests against public-domain scripts.
3. Autocomplete lists, scene-heading separators, character extensions.
4. Native format, Fountain, TXT, RTF, text-PDF import; auto-save and backups.
5. Index cards, scene view, navigator tables, notes, title page, headers and footers.
6. Production: revision sets and marks, page locking, scene numbering, omitted scenes, reports, breakdown tags.
7. Story board, outline editor, structure colors, writing stats, collaboration.

**Refuted, do not build to these:** the specific autocomplete popup and gray inline-completion behavior (0-3), and a claim that Final Draft imports Fountain and that PDF imports come in partial or blank (0-3).

**Open questions:**

- Default indents per element: confirm one set against printed public-domain scripts.
- How paginate-as options behave in detail, and exact keep-together rules.
- FDX interoperability: find a legitimately published description, or rely on Fountain plus PDF until a lawyer clears an approach.
- Title page fields and layout, dual dialogue, and the Typeset-style tightening feature were not covered.
- Whether observing a licensed copy is allowed at all (legal question, deferred).

**Sources** (all public, opened by the research run):

- Final Draft KB: [script elements](https://kb.finaldraft.com/hc/en-us/articles/27646947570196-What-are-script-elements), [keyboard shortcuts](https://kb.finaldraft.com/hc/en-us/articles/27977488282644-What-keyboard-shortcuts-can-I-use-in-Final-Draft), [element settings](https://kb.finaldraft.com/hc/en-us/articles/27647938211604-How-do-I-modify-element-behavior-change-element-fonts-and-margins-or-create-new-custom-elements), [SmartType](https://kb.finaldraft.com/hc/en-us/articles/27750003388948-What-is-SmartType-and-how-do-I-use-it), [FD13 manual section](https://kb.finaldraft.com/hc/en-us/sections/27495762684564-Final-Draft-13-manual), [import formats](https://kb.finaldraft.com/hc/en-us/articles/15575252515988-What-file-formats-can-I-import-into-Final-Draft), [Importing and exporting](https://kb.finaldraft.com/hc/en-us/sections/15877624673940-Importing-Exporting), [Revision Mode](https://kb.finaldraft.com/hc/en-us/articles/27813588722708-How-do-I-use-Revision-Mode), [locking tools](https://kb.finaldraft.com/hc/en-us/articles/27817054455828-How-do-I-lock-or-unlock-script-pages-and-use-the-locking-tools), [locked pages and numbering](https://kb.finaldraft.com/hc/en-us/articles/15573887116948-Final-Draft-Locked-Pages-and-Page-Numbering), [Mores and Continueds](https://kb.finaldraft.com/hc/en-us/articles/27747514444180-How-do-I-modify-or-remove-Mores-and-Continueds-CONT-Ds), [Production section](https://kb.finaldraft.com/hc/en-us/sections/15877856011796-Production)
- Final Draft manuals and site: [FD13 Mac user guide](https://www.finaldraft.com/downloads/manuals/fd13-user-guide-mac-exported.pdf), [FD13 Windows user guide](https://www.finaldraft.com/downloads/manuals/fd13-user-guide-win-exported.pdf), [quick start](https://www.finaldraft.com/learn/final-draft-quick-start/), [FD13 launch post](https://blog.finaldraft.com/final-draft-13-is-here), [beat board post](https://www.finaldraft.com/blog/how-to-use-a-beat-board-for-pretty-much-everything-in-your-script)
- Format standards and context: [PRONOM fmt/1845](https://www.nationalarchives.gov.uk/PRONOM/fmt/1845), [Celtx margins guide](https://blog.celtx.com/screenplay-margins-guide/), [screenplay.com format](https://screenplay.com/pages/basic-screenplay-format), [Wikibooks screenplay format](https://en.wikibooks.org/wiki/Movie_Making_Manual/Writing/Screenplay_Format), [Filmmaker Magazine review](https://filmmakermagazine.com/?p=120862)
