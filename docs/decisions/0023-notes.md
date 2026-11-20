# 0023: Notes

Date: 2026-11-19

SPEC: notes attached to script text, edited in a popover, listed in the
tables. (Threaded replies belong to the other product's cloud version and are
not built.)

- **Anchor as a mark on the text.** A note is attached to the characters it
  covers through a `note` mark in their character properties
  (`CharProps::note`, added to the copied `wordsmith-doc`, recorded in
  `UPSTREAM.md`). The mark moves with the text through typing, deleting,
  splitting, joining and moving scenes, and the script's text is never
  changed, so cues, headings, find, spelling, rename and autocomplete see the
  same text as before. The document's comment markers were considered and
  rejected: they are characters inside the text, and every text-reading
  feature would have had to skip them.
- **Typing at a note's edge does not extend it;** typing inside it does.
  Pasting text does not carry notes.
- **The notes' texts** are in `app_data` (`wordsmith.notes`, by id: text and
  author), so adding, editing and deleting are undoable and saved with the
  script. At most 10,000 notes of 10,000 characters each.
- **A note whose text was all deleted stays,** listed as detached, until it is
  deleted, so a note is never lost by an edit.
- **Commands:** `note.add {text}` (the selection, or the caret's paragraph),
  `note.edit`, `note.delete`, `note.list` (in script order, with the text it
  is attached to, its scene and whether it is attached), `note.at`,
  `note.goto` (selects its text).
- **In the editor:** noted text is tinted in the page view with a mark in the
  left margin; Edit › Note… (Mod+Alt+N) opens the note at the caret in a
  popover beside it, or a new note for the selection; the Tables panel has a
  Notes table (go to, edit, delete).
- **Files:** the native format marks anchors in the text from version 2
  (`docs/format.md`, with a migration from version 1). Fountain export writes
  each note as `[[…]]` where its text ends, outside any emphasis; Fountain
  import leaves notes inside text out, as before. Notes are not printed.
