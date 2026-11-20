# 0025: Scene numbers

Date: 2026-11-19

- **Unlocked numbers follow the order:** 1, 2, 3 …, worked out whenever they
  are shown, so they always match the script.
- **Locking stores them** on the headings (`ParaProps::scene_number`, added to
  the copied `wordsmith-doc`). A stored number moves with its scene. A scene
  added after the lock has no stored number; it shows the number of the
  numbered scene before it with a letter: after 12, 12A, then 12B; after 12A,
  12B, and when that is taken, 12AA, 12AB; before the first numbered scene, a
  letter in front of its number (A1, then B1). Until the next lock stores
  them, these lettered numbers follow their neighbours (moving the scene in
  front of them can change their letters). Locking again stores every shown
  number; Renumber starts again from 1; unlocking clears the stored numbers.
- **Omit** turns a scene into a heading reading OMITTED that keeps the
  scene's number (`ParaProps::omitted` points at the kept text, which is
  stored in `app_data` under `wordsmith.omitted` and saved with the script).
  **Recall** puts the scene back in its place. Both are single undo steps.
  Omit and Recall act on the caret's scene when no scene is given.
- **Showing:** off until switched on (or until locking, which shows them).
  Each number prints in both margins of its heading's first line: on the left
  ending at 1.25 in, on the right from 7.55 in; the revision mark's default
  place moved to 8.0 in to stay clear of it. Splitting a heading keeps the
  number on the first part only.
- **Files:** the native format keeps `#12A` and `omitted=3` on the heading's
  `@` line. Fountain's `#12A#` is read as a stored number, and a file with
  numbers is read as locked and shown; shown numbers are written as `#12A#`.
  PDF import leaves out text in the side margins. The golden scripts carry
  Fountain numbers, so their reviewed PDFs now show them: the diff was
  checked to be exactly the 248 headings, each with its number either side.
- **Not built:** numbering shots, characters or dialogue (`docs/gaps.md`).
