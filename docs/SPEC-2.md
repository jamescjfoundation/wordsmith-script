# Functional Specification: Screenwriting Application (Clean-Room Edition)

Written for this project by the Wordsmith Script maintainers on 2026-11-19,
prepared by clean-room methods, and licensed as the rest of the repository
(MIT OR Apache-2.0). Kept here word for word. How it relates to
`docs/SPEC.md` and to what v0.1 built is in decision 0030 and
`docs/history/spec2-audit.md`.

Names: where this spec says "navigator" or "speed view", the app has the
Script panel and the Draft view (decision 0041).

## 0. How to use this document

This document describes required behaviour only. It contains no code, interface text, screenshots, sample content, or data taken from any existing product.

**Rules for the implementer**

1. Work only from this document and from public references, such as published screenplay format guides and platform documentation.
2. Do not install, run, open, screenshot, or inspect any existing commercial screenwriting product, its files, its templates, or its help material while on this project.
3. Write all code, interface wording, error messages, icons, sample scripts, and help text yourself.
4. If something is missing or unclear, send a written question to the spec author and log it. Do not resolve it by looking at another product.
5. Feature names here are neutral descriptions. Choose your own product and feature names.

**Labels**

| Label | Meaning |
|---|---|
| [STD] | Industry convention. Match exactly. |
| [DEF] | Behaviour is required; the exact value is a suggestion and yours to change. |
| [OWN] | You must author it or source it under an open licence. |
| [OPEN] | Not specified. Decide yourself or ask. |

## 1. Overview

The application is a word processor built only for scripts. Every paragraph has exactly one **element type** (scene heading, action, character, dialogue, and so on). Each element type knows its margins, its style, and which element follows it, so the writer mostly presses Enter and Tab and the page formats itself.

Around the editor sit four groups of tools:

- **Planning:** beat canvas, scene cards, structure strip, outline paragraphs.
- **Production:** scene numbers, coloured revision sets, locked pages, breakdown tagging.
- **Analysis:** eight reports and a format checker.
- **Productivity:** sprint timer, writing statistics, goals.

There are two document kinds: **Script** (element-based, paginated) and **Text** (prose, such as a treatment or series bible).

## 2. Documents and files

**File commands:** new, new from template, open, close, save, save as, save as PDF, export script, export beat canvas, backup, revert to last saved, print preview, print, recent files, exit.

| Format | Read | Write | Notes |
|---|---|---|---|
| Native document | yes | yes | Your own schema [OWN]. Human-readable structured text is recommended. |
| Template | yes | yes | A native document flagged as a template; user can save any document as a custom template. |
| Plain text | yes | yes | On import, ask whether to convert to a Script or Text document. |
| Plain text with layout | yes | yes | Preserves indentation with spaces. |
| Rich Text (RTF) | yes | yes | |
| HTML | | yes | |
| PDF | | yes | From save, print, and beat canvas export. |
| Third-party interchange formats | | | [OPEN] See section 20. |

**Other file behaviour**

- Auto-save every N minutes, with an option to ask first (save, skip, or turn the feature off).
- Auto-backup to a chosen folder with a maximum file count. Warn when the folder is missing or full. Provide a command to open the folder.
- Detect read-only or in-use files and open them read-only with a notice.
- Review-only copy: save a copy that cannot be edited but accepts margin notes.
- Merge scripts: combine several scripts into a new file.
- Start window: recent documents, an option to always launch with chosen documents, a template chooser, and a tutorials area [OWN].
- Option to reopen the last documents on launch.
- The new-document chooser shows templates as a tree of categories.

## 3. Script editor

### 3.1 Element table

Page is US Letter, 8.5 × 11 in. Font is 12-point Courier, 10 characters per inch, 6 lines per inch [STD]. Indents are inches from the left paper edge [STD]. Space before is in points; 12 pt equals one blank line. Enter and Tab transitions are [DEF].

| Element | Left | Right | Space before | Style | Enter makes | Tab makes |
|---|---|---|---|---|---|---|
| General | 1.50 | 7.50 | 0 | plain | General | General |
| Scene Heading | 1.50 | 7.50 | 24 | all caps | Action | Action |
| Action | 1.50 | 7.50 | 12 | plain | Action | Character |
| Character | 3.50 | 7.25 | 12 | all caps | Dialogue | Transition |
| Parenthetical | 3.00, first line −0.10 | 5.50 | 0 | plain | Dialogue | Dialogue |
| Dialogue | 2.50 | 6.00 | 0 | plain | Action | Parenthetical |
| Transition | 5.50 | 7.10 | 12 | all caps, right-aligned | Scene Heading | Scene Heading |
| Shot | 1.50 | 7.50 | 12 | all caps | Action | Action |
| Cast List | 1.50 | 7.50 | 0 | all caps | Action | Action |
| New Act | 1.00 | 7.50 | 12, starts a new page | bold, underlined, caps, centred | Scene Heading | Scene Heading |
| Sequence | 1.25 | 7.50 | 24 | bold, underlined, caps | Scene Heading | Scene Heading |
| End of Act | 1.00 | 7.50 | 24 | bold, underlined, caps, centred | New Act | New Act |
| Outline 1 | 0.875 | 7.50 | 16 | grey sans-serif 16 bold, not printed | Summary | Outline 2 |
| Outline 2 | 1.125 | 7.50 | 14 | grey sans-serif 14 bold, not printed | Summary | Outline 3 |
| Outline 3 | 1.375 | 7.50 | 12 | grey sans-serif 12 bold, not printed | Summary | Note |
| Note | 1.62 | 7.50 | 12 | grey sans-serif 12 italic, not printed | Summary | Summary |
| Summary | 0.875 | 7.50 | 12 | grey sans-serif 12, not printed | Summary | Outline 1 |

Each element also carries two properties:

- **Paginate as:** which base element's page-break behaviour it inherits. Shot behaves like Scene Heading; Cast List behaves like Action.
- **Structure rank:** its place in the hierarchy used by the navigator and structure strip. From broadest to narrowest: Outline 1, acts, Outline 2, Sequence, Scene Heading. The rank of Outline 3 is [OPEN].

### 3.2 Typing behaviour

- Enter ends the paragraph and starts the "Enter makes" element.
- Tab changes the current paragraph to its "Tab makes" element. In a Character paragraph, an option makes Tab jump to a character extension instead.
- An element picker in the toolbar shows and changes the current element.
- The status bar shows a live hint of what Tab and Enter will produce from the current element. Holding a modifier key opens a quick element picker.
- Reset to element: restore a paragraph to its element's default formatting.
- Toggle case; optional auto-capitalisation of the first word of a sentence; smart quotes on by default.
- Keep a list of common abbreviations that do not end a sentence [OWN].
- Undo and redo are multi-level, and each step is labelled by action type (typing, paste, move, replace, and so on).
- Select scene, select all, and find-next using the current selection.
- Insert page break, insert new scene, and add a non-speaking character to a scene's cast.
- Pasting text that uses unknown element types adds those element types to the document.

**Element settings dialog.** Add, delete, and rename elements. Per element, edit:

- Font: face, size, colour, highlight, bold, italic, underline, strikeout, all caps or lowercase.
- Paragraph: alignment, line spacing, space before, left and right indents. Indents range from 0 to 8 in. The right indent must be at least 1 in greater than the left.
- Behaviour: start on a new page, paginate as, next element on Enter, keyboard shortcut.
- Apply one font and size to all elements; apply the element table from a template.

**Reformat tool.** Step through paragraphs one at a time and reassign each one's element with a single key, or delete it.

**Dual dialogue [STD].** Two characters' speeches set side by side in two columns. It requires a second character's speech immediately after the first. If that is missing, explain why the command cannot run.

## 4. Autocomplete lists

A pop-up list appears while typing in certain elements and completes the entry.

| List | Default contents | Where it appears |
|---|---|---|
| Scene intros [STD] | INT, EXT, I/E, followed by a period and a space | Start of a Scene Heading |
| Locations | Learned from the script | After the intro |
| Times of day [STD] | DAY, NIGHT, AFTERNOON, MORNING, EVENING, LATER, MOMENTS LATER, CONTINUOUS, THE NEXT DAY, MAGIC HOUR, DAWN, DUSK, SAME, SAME TIME, preceded by space-hyphen-space | After the location |
| Characters | Learned from the script | Character element |
| Extensions [STD] | (V.O.), (O.S.), (O.C.), (CONT'D), (SUBTITLE), (TEXT) | After a character name |
| Transitions [STD] | CUT TO:, FADE IN:, FADE OUT., FADE TO:, DISSOLVE TO:, BACK TO:, MATCH CUT TO:, JUMP CUT TO:, FADE TO BLACK., SMASH CUT TO:, CUT TO BLACK., TIME CUT: | Transition element |

Scene headings are parsed as intro, separator, location, separator, time of day.

**Settings:** edit each list; rebuild lists from the script; turn matching on or off for scene headings, transitions, and characters; guess the next speaker in a two-person exchange; sort characters by number of speeches; choose the pop-up colour.

## 5. Macros

Provide about 20 macro slots per document [DEF]. Each macro has:

- A name and a keyboard shortcut.
- The element to apply, the text to type, and the element to switch to afterwards.
- An optional typed alias that is replaced automatically, with options for smart replace, confirm before replacing, whole word only, and match case.
- The list of elements in which it is active.

Macros can be copied in from another document. Ship a starter set [OWN] covering scene intros, common time-of-day endings, two or three common parentheticals, and the common transitions, bound to a modifier plus the digit keys.

## 6. Text formatting

- Styles: plain, bold, italic, underline, double underline, word underline, dotted underline, strikeout, superscript, subscript, all caps, all lowercase, small caps.
- Font dialog: face, style, size, text colour, highlight colour, preview.
- Alignment: left, centred, right, justified.
- Line spacing: 1, 1.5, 2. Space before: 0 to 3 lines or a custom value. Leading: very tight, tight, regular, loose.
- Highlight palette of about a dozen named colours plus a custom colour [OWN].
- Cast List ordering: alphabetical, order of appearance in the scene, or custom.

## 7. Page layout and pagination

| Setting | Default | Label |
|---|---|---|
| Top and bottom text margins | 72 pt each | [STD] |
| Header and footer margins | 36 pt each | [STD] |
| Lines per page | 54 (9 in of text at 6 lines per inch) | [STD] |
| Break dialogue and action at sentence ends | on | [STD] |
| Smart quotes | on | [DEF] |
| Target script length | 120 pages, drives progress displays | [DEF] |

**Pagination rules.** The first three are standard practice [STD]; exact edge cases are [OPEN].

1. Space before counts in 12-pt lines and is dropped at the top of a page.
2. A Scene Heading, Character, or Parenthetical never sits alone at the bottom of a page; it moves with the paragraph that follows.
3. Split long dialogue and action only at sentence ends.
4. Elements flagged "starts a new page" force a break.

**Continuation markers**

- Dialogue split across pages: "(MORE)" at the bottom and the character name with "(CONT'D)" at the top of the next page. Both on by default [STD].
- Automatic character continued: add "(CONT'D)" when the same character speaks again with only action in between. On by default [STD].
- Scene split across pages: "(CONTINUED)" at the bottom and "CONTINUED:" at the top, optionally numbered. Off by default [STD]. These markers have their own font setting.

**Scene length.** Divide the page's text height into 8 equal parts. A scene's length is its height divided by one part, rounded up. Display it as a fraction under one page ("3/8"), a whole number for exact pages ("2"), a mixed value otherwise ("1 3/8"), and "0" for an empty scene [STD].

**Header and footer.** Options: show header, show on first page, show footer, show on first page, starting page number. Insertable fields: page number, date, scene, label, active revision, collated revisions, file name. Default: page number followed by a period at top right, hidden on page 1, no footer [STD].

**Watermark.** Text watermark with adjustable opacity (about 70% [DEF]) set diagonally, an optional image, and a recipient name for tracking distributed copies.

**Title page.** A separate editable page, optional on print and PDF. Write your own placeholder wording [OWN] for title, writer credit, source material, revision credit, and contact details.

## 8. Views and display

- **Script views:** continuous view with page breaks as lines; page view with real pages; draft view with plain unformatted text for fast typing.
- **Other views:** beat canvas, scene list view, scene cards showing script text, scene cards showing summaries. Cards per row from 1 to 9.
- **Show or hide:** action on cards, scene title, summary, structure lines, alternate lines, invisible characters, ruler, margin notes, images, outline paragraphs in the script, structure strip.
- **Split panels:** split vertically or horizontally, swap, unsplit. Each panel can hold a different view.
- **Zoom:** stepped levels from 75% to 300% [DEF]. The structure strip zooms separately from 25% to 400%.
- **Toolbars:** customisable toolbar groups and a status bar. A ribbon-style layout is optional.
- **Typewriter mode:** keeps the line being typed at a fixed height on screen. Highlight style is a frame around the line, a rule across the page at the line, or none.
- **Focus mode and full screen:** hide everything except the page.
- **Themes:** light, dark, and extra-dark [OWN colours]. Printing can optionally keep the dark colours.
- **Document tabs:** several open documents as tabs in one window.
- **Status bar:** page X of Y, current scene, current element, the Enter and Tab hint, revision-mode and tagging-mode indicators, and quick controls for sprint timer, focus mode, theme, and a zoom slider.

Outline paragraphs shown inside the script must look clearly different from script text: a different typeface and colour, plus a nesting indicator per level, such as a coloured vertical bar.

## 9. Planning tools

### 9.1 Beat canvas

A large free-form canvas for brainstorming. Canvas size is [DEF]; make it large enough to feel unbounded.

- Create a beat with one shortcut. Beats have position, size, title, body text, and colour. Body text supports simple numbered and bulleted lists and Unicode emoji.
- Drag beats anywhere. Align selected beats to a grid, into a column, into a row, or as a cascade. Grid snapping toggles on and off.
- Connect two beats with a plain line, forward arrow, back arrow, or double arrow. Connections can be coloured and removed. An option creates connections by drag and drop.
- Send selected beats to the script as content.
- Support several canvases per document.
- Export a canvas as PDF.

### 9.2 Scene cards and scene panel

- One card per scene, showing heading, editable title, summary, colour, and optionally the first action lines.
- Colour palette: none plus seven named colours [DEF].
- Reorder scenes from the cards by cut and paste and by drag and drop. Insert and delete scenes, with a confirmation before delete. Reordering is undoable.
- Scene properties panel: heading, title, colour, summary, characters in the scene (add, remove, hide), and an arc beat per character per scene.
- Print cards on 3 × 5 in card stock or a custom grid, with options for frames, grey background, and selected cards only.

### 9.3 Structure strip

A horizontal strip above the script that shows structure as lanes.

- One lane per outline level (for example acts, then sequences) and one lane for scenes.
- Blocks are sized in proportion to page span and can be coloured.
- A page-number ruler runs along the strip.
- Clicking a block navigates to that place in the script.
- Which outline level feeds each lane is configurable.

## 10. Navigation, search, and annotations

**Navigator panel.** A dockable window with tabs.

- Scenes tab: number, heading, page, length, colour, title, summary. Columns sort and filter.
- Characters tab: name, highlight colour, highlight-speeches toggle, assigned voice, and one column per trait.
- Tags tab and margin-notes tab.
- Users can add, rename, and remove tabs, and choose visible columns.

**Go to.** Jump to a page, scene, bookmark, or label, with next and previous.

**Find and replace.** Whole word, match case, direction, and restriction to chosen element types. Find next, replace, replace all.

**Rename character.** Change a character's name across the whole script.

**Bookmarks.** Named bookmarks with a list, go to, delete, next, and previous.

**Margin notes.** Notes anchored to a spot in the text.

- Fields: title, colour, type, body, created and modified dates, writer, scene number.
- A marker appears in the right margin; a count badge for several notes on one paragraph is optional.
- Next and previous note, show or hide all, user-defined note types.

**Labels.** Named markers usable in headers and in go to.

**Images.** Insert pictures into the script; show or hide all.

**Alternate lines.** Keep several versions of one line of dialogue. Add, remove, cycle through, show or hide. Reports can include alternates.

## 11. Production features

### 11.1 Scene numbers [STD]

- Number, renumber, or remove numbers for a chosen element type.
- Options: starting number, number from the current scene, keep existing numbers, restart every N.
- Inserted scenes use a letter suffix (1, 1A, 2) or a letter prefix (1, A2, 2), per a setting.
- Numbers display at left (0.75 in) and/or right (7.38 in) with their own font. Individual numbers are editable.

### 11.2 Omitted scenes [STD]

Omitting a scene hides its text and shows "OMITTED" in its place while keeping the number. An omitted scene can be viewed and restored.

### 11.3 Revisions [STD]

- In revision mode, every change gets a mark in the right margin (default an asterisk at 7.75 in) and takes the active set's text style.
- Commands: mark revised, clear revised, next and previous revision.
- 19 revision sets in the standard order: Blue, Pink, Yellow, Green, Goldenrod, Buff, Salmon, Cherry, Tan, Second White, then Second Blue through Second Tan.
- Each set has a label, mark character, page colour [OWN values], revised-text style, and a choice of full draft or revised pages only.
- Display: active set, collated script, or all sets; show or hide revision marks and page colours.
- Printing: revised pages only, unrevised pages only, or one chosen set.

### 11.4 Page locking [STD]

- Locking freezes page numbers, then offers to turn on revision mode.
- Content added later creates lettered pages (12A, 12B).
- Commands: unlock the latest lettered level, unlock all, relock, adjust a locked page break, edit a locked page number, lock or unlock a single page.
- Block deleting across a locked page break and deleting the first locked page.

### 11.5 Change tracking

- Toggle on or off. Each writer has a name and colour. Each change records who and when.
- Next, previous, accept, reject, accept all, reject all.

### 11.6 Script comparison

Compare two scripts. Match scenes first, then elements within scenes. Detect moved scenes and elements. Produce a marked-up result showing additions and deletions.

### 11.7 Breakdown tagging

- A tagging mode: select text and assign a category. Tag every occurrence at once. List the tags in a selection. Add a tag to a whole scene. Edit a tag's label, category, and links.
- Tag all characters in one step.
- Default categories [DEF], taken from standard production breakdown practice: Cast Members, Background Actors, Stunts, Vehicles, Props, Special Effects, Wardrobe, Makeup/Hair, Animals, Animal Wrangler, Music, Sound, Set Dressing, Greenery, Special Equipment, Security, Additional Labor, Visual Effects, Mechanical Effects, Notes, Comments, Camera, Script Day, Sequence, Unit, Location, Art Department, Miscellaneous.
- Each category has a colour and text style. Users can add categories.

## 12. Characters

- Default traits [DEF]: age, disability, ethnicity, gender, occupation, orientation, role. All are free text; users can add more.
- Representation chart: compare characters by any two traits against their share of characters, dialogue, and scenes. Group blanks as unassigned.
- Highlight characters: colour every speech of chosen characters.
- Arc beats per character per scene; non-speaking characters; an automatic cast list element.

## 13. Reports

Every report opens as a new document that can be printed or saved, with a font choice.

| Report | Contents | Options |
|---|---|---|
| Scene | Number, heading, page, length in eighths; longest and shortest scene; last page | Sort by script order, alphabetical, shortest, longest, production order; include tags |
| Location | Each location with occurrences, intro, and time of day; blanks flagged | Sort by script order, alphabetical, most, least |
| Character | For one character: most frequent scene partners, pages with monologues, scenes appeared in | Include scene headings, all dialogue, arc beats |
| Cast | Per character: speeches, speaking scenes, non-speaking scenes, total scenes; most and least dialogue | Sort by script order, alphabetical, scenes, speaking scenes |
| Filtered script | The script reduced to chosen element types | Output as script or text; include alternates |
| Margin notes | Title, type, body, page, dates, scene number, writer | Choose types and columns; sort |
| Statistics | Word and paragraph totals; count and percentage per element; per character speech count, word count, and share of each; per scene word count and other scenes at the same location; flagged-language list with pages and speakers | Choose sections |
| Tags | Tags grouped by scene or by tag | Include scene headings |

## 14. Checking and language tools

**Format checker.** Steps through issues with fix, next, and ignore.

1. An element starts with a space. Fix: remove the leading spaces.
2. An element is empty. Fix: delete it.
3. A character has no dialogue. Fix: delete the character paragraph.
4. The same character has two consecutive speeches. Manual fix.
5. Dialogue is not preceded by a character or parenthetical. Manual fix.

Options: scan before printing, ask before printing, or off.

**Spelling.** As-you-type and by dialog. Flags misspelled and repeated words. Actions: ignore, ignore all, replace, replace all, learn, capitalise, delete repeated word, check selection. Per-document ignore lists. An auto-replace list applies as you type. Dictionary is [OWN].

**Thesaurus.** Synonyms, related words, contrasting words, antonyms, and replace. Data is [OWN].

**Names lookup.** A large first-name list (tens of thousands of entries). Type the first letters, look up, and add the result to the character autocomplete list. Data is [OWN].

## 15. Read-aloud

- Text-to-speech reads the script. Each character maps to a voice profile with voice, pitch, and speed.
- Ship ten generic profiles [OWN]: two adult male, two adult female, two boy, two girl, one elderly male, one elderly female.
- A narrator voice reads the element types the user selects.
- Controls: play, pause, fast forward, rewind, stop. Highlight the text being read. Export the reading as audio.

## 16. Collaboration and assistant

- A host shares a script and guests join with a session code. Editing is live.
- Show presence changes when people join, leave, or rejoin.
- One person holds editing control at a time. Guests can request it and the holder can pass it.
- Include chat with a sound cue, and a manual resync of script and navigator.
- Check that all participants run compatible versions.
- Optional assistant chat panel with history, reactions, retry, and suggested prompts [OPEN].

## 17. Productivity

**Sprint timer.** Set a duration in minutes, start, and show a summary of what was written when it ends. Default duration is [OPEN].

**Writing statistics window**

- Summary cards: current streak in days, longest streak, peak weekdays, peak time-of-day window.
- Metric switch: words, pages, time.
- Range switch: day, week, month, year.
- Average per day for the range.
- Bar chart per period with the goal drawn as a line across it.
- Idle timeout in minutes: writing time stops counting after that long without input.
- A button to set goals. Tracking can be turned off in preferences.

**Goals.** Pages per day with a frequency, and minutes per day. Suggested defaults for beginners: 1 page and 25 minutes a day [DEF]. Defaults for experienced writers are [OPEN].

## 18. Printing and PDF

- Range by all, pages, or scenes; copies; collate; reverse order.
- Revision set, revised pages only, or unrevised pages only.
- Run the format checker first; include the title page.
- Keep colours: text, background, revised text, revision page colour, dark themes.
- Print a character name on each page.
- Sides [STD]: print only the scenes of a chosen character.
- Print to PDF and print preview.

## 19. Preferences and templates

**Preferences**

- General: help key, auto-capitalise sentences, reopen last documents, check for updates, track writing statistics, documents folder, number of recent items.
- Auto-save interval and auto-backup folder and limit.
- Writer name and colour, used for change tracking and collaboration.
- Handedness for tablet layouts.
- Customisable keyboard shortcuts, toolbars, and tooltips.
- Usage-data sharing is opt-in.

**Templates [OWN].** Author every template yourself from public format guidance. Aim for these categories:

- Screenplays: standard, beginner, guided, UK film and television, and several languages.
- Television: single-camera, multi-camera sitcom, short-form and vertical drama.
- Stage: stage play, musical, immersive.
- Comics and graphic novels.
- Audio and prose: podcast, short story, manuscript, treatment, query letter, series bible.
- Outlines and story structure: three, four, and five acts, and acts with sequences.

Any template based on a named third-party method or a specific company's house style needs that party's public guidance or a licence.

A template carries its own element table, autocomplete lists, macros, page layout, header and footer, revision sets, tag categories, voice profiles, scene-number options, and planning-tool setup.

**Suggested keymap [DEF].** "Mod" is Ctrl on Windows and Linux and Cmd on macOS. All bindings must be user-editable.

| Keys | Action |
|---|---|
| Mod+N, O, S, P | New, open, save, print |
| Mod+Z, Y | Undo, redo |
| Mod+F, G | Find, go to |
| Mod+B, I, U | Bold, italic, underline |
| Mod+1 to 9, Mod+0 | Apply the main script elements by number |
| Shift+digit | Apply outline and structure elements |
| Mod+Alt+digit | Run macros |
| Mod+Enter | New beat on the canvas |
| Mod+R | Mark revised |
| Mod+J, M | New margin note, new bookmark |

Assign the remaining commands yourself.

## 20. Data model, build order, and open items

**Conceptual model.** Design your own schema [OWN]. It needs to hold:

- Document: kind, element table, paragraphs, title page, and one settings group per feature.
- Paragraph: element type, text runs with style and revision reference, optional overrides, dual-dialogue flag, alternates, scene number, omitted flag.
- Scene: title, summary, colour, page, length, characters, arc beats.
- Beat canvas: beats and connections.
- Tags: categories and tagged items.

**Suggested build order**

1. Element-based editor with Enter and Tab rules, element picker, undo and redo, save and load.
2. Page view with pagination, page numbers, continuation markers, title page, PDF export.
3. Autocomplete lists and macros.
4. Navigator, find and replace, go to, bookmarks, rename character.
5. Scene cards and beat canvas.
6. Margin notes, reports, format checker.
7. Scene numbers, revisions, page locking, tagging, change tracking.
8. Themes, typewriter mode, sprint timer and goals, read-aloud, collaboration.

**Open items**

| Item | Status |
|---|---|
| Import and export of other products' script formats | Needs a separate interface specification from the spec author. Do not examine sample files yourself. |
| Exports for editing and scheduling software | Same as above. |
| Exact pagination edge cases | Follow section 7; decide the rest. |
| Experienced-writer goal defaults; sprint default | Decide yourself. |
| Theme colours, animations, icon design | Your own design. |
| Accounts, licensing, updates, onboarding, tutorials | Your own design. Out of scope here. |
| Script registration and feedback services | Out of scope unless requested. |
