# Audit of SPEC-2 against v0.1

Date: 2026-11-19

`docs/SPEC-2.md` (the clean-room functional specification) checked against
what v0.1.0 built. The project owner's instruction: everything built so far
is correct, and is not to be reworked to fit SPEC-2 (decision 0030). So this
audit does not schedule changes to built behaviour. Where a value labelled
[STD] differs from ours, it is listed under "Differs" and the question is
logged in `docs/spec2-questions.md` for the owner; nothing changes until they
answer.

Statuses: **Built**, **Partly** (what is missing is named), **Not built**,
**Differs** (built, with a value or rule other than SPEC-2's), **Out** (out of
scope by the kickoff or ROADMAP, kept out until the owner says otherwise).

## [STD] items whose values differ from ours

| SPEC-2 | SPEC-2 value | v0.1 value | Where |
| --- | --- | --- | --- |
| 3.1 Character indents | 3.50 to 7.25 in | 3.7 to 7.5 in | `template.rs` |
| 3.1 Parenthetical indents | 3.00 (first line −0.10) to 5.50 in | 3.1 to 5.6 in, no hanging first line | `template.rs` |
| 3.1 Transition right indent | 7.10 in | 7.5 in (right-aligned) | `template.rs` |
| 3.1 New Act, End of Act left indent | 1.00 in | 1.5 in | `template.rs` |
| 3.1 Outline elements | 0.875 to 1.62 in, grey sans-serif, never printed | 1.5 to 2.0 in, script font; printing follows `outline.show` | `template.rs`, decision 0028 |
| 4 Times of day | adds THE NEXT DAY, MAGIC HOUR, DAWN, DUSK, SAME, SAME TIME | eight entries | `autocomplete.rs` |
| 4 Extensions | adds (SUBTITLE), (TEXT) | V.O., O.S., O.C., CONT'D | `autocomplete.rs` |
| 4 Transitions | adds FADE TO:, BACK TO:, JUMP CUT TO:, CUT TO BLACK., TIME CUT: | seven entries | `autocomplete.rs` |
| 7 Scene length of an empty scene | "0" | "1/8" (every scene is at least one eighth) | `reports.rs` |
| 11.1 Scene number positions | left 0.75 in, right 7.38 in | left box ending at 1.25 in, right box from 7.55 in | `paginate/output.rs`, decision 0025 |
| 11.3 Revision mark position | 7.75 in | 8.0 in (settable) | `revisions.rs` |

The list additions in section 4 only add default entries for new scripts.
They change nothing a writer has typed, but they are still a change to a
built default, so they wait for the owner like the rest.

## Matches

| SPEC-2 | Where |
| --- | --- |
| 3.1 page, font, 10 cpi, 6 lpi; Scene Heading, Action, Shot, Dialogue, General indents | `template.rs` |
| 4 Scene intros INT, EXT, I/E with ". "; heading parsed as intro, separator, location, separator, time | `scene_heading.rs` |
| 7 margins 72 pt, 54 lines a page, sentence-end breaks, space before dropped at a page top, keep-with-next, start-new-page | `paginate/` |
| 7 (MORE) and (CONT'D) on, automatic CONT'D on, scene CONTINUED off and optionally numbered | `paginate/mod.rs` |
| 7 eighths display "3/8", "2", "1 3/8" | `reports.rs` |
| 7 page number top right with a period, hidden on page 1, no footer | `header_footer.rs` |
| 11.2 OMITTED keeps the number; recall | decision 0025 |
| 11.3 cycle Blue … Tan, Second White, Second Blue … Second Tan (ours continues past 19) | `revisions.rs` |
| 11.4 lettered pages, unlock latest level, unlock all, lock offers revision mode | decision 0026 |

## By section

### 2. Documents and files

| Item | Status | Notes |
| --- | --- | --- |
| New, open, save, save as, PDF, export, print, recover | Built | |
| Close, revert to last saved, print preview, recent files, exit | Partly | close and exit exist through the window; revert, preview and a recent list are not built |
| New from template, save as template, template tree | Not built | one template (gaps) |
| Plain text, RTF read and write; HTML and PDF write | Built | |
| Plain text: ask Script or Text on import | Differs | we ask "is this a script?" (decision 0019); there is no Text document kind |
| Plain text with layout | Not built | |
| Export beat canvas to PDF | Not built | gaps |
| Auto-save ask-first option; backup folder warnings, open-folder command | Partly | auto-save and backups built (decision 0020); ask-first, warnings and the command are not |
| Read-only and in-use detection | Not built | gaps: no file locking |
| Review-only copy, merge scripts | Not built | |
| Start window, reopen last documents | Not built | |
| Third-party interchange formats | Out | [OPEN] in SPEC-2; needs its own spec |

### 3. Script editor

| Item | Status | Notes |
| --- | --- | --- |
| Element table, Enter and Tab transitions | Built | indents differ (above); Sequence missing; Enter after outline elements differs ([DEF]) |
| Sequence element | Not built | |
| Paginate as, structure rank | Partly | paginate-as built; no structure rank |
| Element picker, change element | Built | |
| Enter/Tab hint in the status bar; quick picker on a held modifier | Not built | |
| Reset to element, toggle case, auto-capitalise, smart quotes, abbreviations list | Not built | |
| Labelled multi-level undo | Built | engine undo history |
| Select scene, find-next from selection, insert page break, insert new scene, non-speaking characters | Not built | |
| Paste adds unknown element types | Not built | |
| Element settings: add, delete, rename, font, paragraph, behaviour, apply font to all | Partly | color, highlight, strikeout and lowercase, the 0 to 8 in range rule, and applying a template's table are not built |
| Reformat tool | Not built | |
| Dual dialogue [STD] | Not built | gaps: read as ordinary dialogue |

### 4. Autocomplete

Built (decision 0015) apart from the list contents above and a pop-up color
setting.

### 5. Macros

Not built.

### 6. Text formatting

Bold, italic, underline, alignment, line spacing and space before are built.
Double, word and dotted underline, superscript, subscript, small caps, the
leading presets, a highlight palette and Cast List ordering are not.

### 7. Page layout

| Item | Status |
| --- | --- |
| Pagination and continuation markers | Built (decision 0012) |
| Markers' own font | Not built (gaps) |
| Target length driving progress displays | Not built |
| Header and footer: fields page, date, scene, label, active revision | Built |
| Collated revisions and file name fields; starting page number; show on first page per side | Partly: first page rule built; the rest not |
| Watermark | Not built |
| Title page | Built (decision 0021) |

### 8. Views

| Item | Status |
| --- | --- |
| Continuous, page and draft views; typewriter, focus, dark theme | Built (decision 0016) |
| Typewriter highlight styles; extra-dark theme; full screen | Not built |
| Scene list view, cards of script text or summaries | Built (decision 0022); cards-per-row setting not built |
| Show/hide toggles beyond outline paragraphs | Partly |
| Split panels, zoom steps, customisable toolbars, document tabs | Not built |
| Status bar with page, scene, element | Partly; hint, indicators and quick controls not built |
| Outline paragraphs drawn distinctly with nesting bars | Not built |

### 9. Planning

| Item | Status |
| --- | --- |
| Beat canvas: beats, color, drag, snap, links that stay attached, beats to script | Built (decision 0028) |
| Lists and emoji in beat bodies; align column/row/cascade; arrow styles and colored links; several canvases; PDF | Not built |
| Scene cards with title, summary, color; reorder by drag; undo | Built (decision 0022) |
| Cut/paste reorder, insert and delete with confirmation, properties panel, arc beats, printing cards | Not built |
| Structure strip | Partly: the outline's lanes with page ranges are built (decision 0028); a strip above the script with a ruler and click-to-go is not |

### 10. Navigation and annotations

| Item | Status |
| --- | --- |
| Tables of scenes, characters, notes; edit cells; CSV | Built (decision 0022) |
| Sort, filter, user tabs, column choice, tags tab, character traits columns | Not built |
| Go to scene; find and replace; rename character | Built |
| Go to page, bookmark, label; whole word, direction, element restriction | Not built |
| Bookmarks, labels, images, alternate lines | Not built |
| Notes anchored to text, listed, go to | Built (decision 0023); title, color, type, writer fields, margin marker and note types are not |

### 11. Production

| Item | Status |
| --- | --- |
| Scene numbers, renumber, A-scenes, OMITTED | Built; prefix lettering, restart every N, per-element numbering, own font, editing one number are not |
| Revisions: mode, mark and clear, sets, colors, revised-pages printing, marks on or off | Built; previous revision, per-set mark character and draft choice, collated/all-sets display, unrevised-only and one-set printing are not |
| Page locking | Partly (gaps: relock, adjust, edit number, single page; blocking deletes across a locked break) |
| Change tracking, script comparison | Not built |
| Breakdown tagging | Out (kickoff); SPEC-2 asks for it |

### 12 to 19

| Item | Status |
| --- | --- |
| Character traits, representation chart, highlight speeches, arc beats, cast list element | Not built |
| Reports: scene lengths and character | Built (decision 0027) |
| Reports: location, cast, filtered script, notes, statistics, tags, report documents | Not built |
| Format checker | Not built |
| Spelling as you type and by dialog, learn | Built (decision 0014); repeated words, ignore lists per document, auto-replace not |
| Thesaurus, names lookup | Not built; need open-licensed data |
| Read-aloud | Not built |
| Collaboration, assistant | Out (ROADMAP) |
| Sprint timer, writing statistics, goals | Not built (ROADMAP) |
| Printing: revised pages, PDF | Built; ranges, copies, character name, sides, preview, color options are not |
| Preferences file | Built; most listed preferences, editable shortcuts and toolbars are not |
| Templates beyond the screenplay | Not built |
| Keymap ([DEF]) | Partly: Mod+N/O/S/P, Z/Y, F and Mod+0 to 9 are bound; Mod+B/I/U, G, R, J, M, Mod+Enter and Shift+digit are not. Mod+Alt+digit already switches views (`docs/guide/keyboard.md`), so macros need other keys |

## After v0.2 (M9 to M14, 2026-11-20)

The milestones in `docs/plans/v0.2.md` built most of what the tables above mark
not built or partly built. Each milestone's plan lists what it covers; the
decisions are 0031 to 0036. What SPEC-2 asks for that is still not built is in
`docs/gaps.md`. In brief:

| SPEC-2 | Now |
| --- | --- |
| 2 Files | New from template, save as template, templates by category, Welcome window, recent files, reopen last, revert, merge, read-only files open as a copy. Not built: review-only copy, plain text with layout, backup-folder warnings. |
| 3 Editor | Sequence element, structure ranks, Return/Tab hint, reset to element, toggle case, smart quotes and sentence capitals (off by default), select scene, new scene, page break, reformat tool, dual dialogue, element color, highlight, strike, small letters, template formatting. Not built: pasting element types, the quick element picker on a held key. |
| 4 Autocomplete | Unchanged (list contents wait for question 3). |
| 5 Macros | Built. |
| 6 Formatting | Built, with the highlighter palette. Leading presets not built. |
| 7 Page layout | Watermark (text), header and footer start page, first-page footer, `{collated}`. Not built: picture watermarks, `{file}`, markers' own font, target length. |
| 8 Views | Split views, zoom steps, tabs, extra dark, cards per row and script-text cards, structure strip, outline nesting bars, status bar switches. Not built: ribbon, toolbar groups, typewriter highlight styles. |
| 9 Planning | Several boards, arrange, link styles and colors, lists in beats, board and card PDFs, scene properties, structure strip. Not built: pictures in beats. |
| 10 Navigation | Go To, bookmarks and labels, find by element, table sort, filter, columns and own tabs, note titles, types and colors, next and previous note. Not built: alternate lines, pictures, labels in headers. |
| 11 Production | Scene number options, revision display, per-set marks, lock tools, change tracking, comparison. Not built: breakdown tagging (question 9), blocking deletes across locked breaks. |
| 12 Characters | Traits, representation chart, highlighted speeches, arc beats, non-speaking characters. |
| 13 Reports | Every report but tags. |
| 14 Checking | Format checker. Not built: thesaurus, names lookup, repeated-word spelling actions. |
| 15 Read-aloud, 16 Collaboration | Not built. |
| 17 Productivity | Sprint timer, writing statistics, goals. |
| 18 Printing | Ranges, sets, unrevised pages, sides, reverse, collate, preview, title page choice, format check before printing. |
| 19 Preferences | Writer name, reopen last, Welcome window, statistics, sprint length, keys. Not built: tablet handedness, tooltips, update checks. |
