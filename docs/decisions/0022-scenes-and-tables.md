# 0022: Index cards, scene view and tables

Date: 2026-11-19

All of these show the one script; none keeps its own copy (SPEC: "All views
edit one underlying script"). They change it only through commands, so undo
works everywhere and agents can do what the views do.

## Scenes

- **Scenes are not stored.** A scene runs from one Scene Heading to the next
  (`wordsmith_script::scenes`); text before the first heading belongs to no
  scene and stays first.
- **Commands:** `scene.list` (heading, summary, first action, speakers,
  paragraph range), `scene.move {scene | scenes, before?}` (the scenes, in
  script order, before scene `before`, or to the end), `scene.setHeading`,
  `scene.setSummary` (the scene's first Summary paragraph, added after the
  heading when there is none, removed when emptied), `scene.goto`.
- **A summary is a Summary paragraph** in the scene, so it is part of the
  script, saved and exported with it. Outline elements, Summary among them,
  print until M7 adds showing and hiding them.
- **Index cards** (View › Index Cards, Mod+Alt+4): a card per scene with its
  number, heading, summary (or, greyed, the start of its first action) and
  speakers. **Scene view** (Mod+Alt+5): each scene as a block of its text,
  indented roughly as the page lays it out; only the blocks in sight are
  laid out.
- **Reordering:** drag a card or a scene block onto another. Dropped on a
  later one it goes after it, on an earlier one before it, as a writer moving
  a card along a table expects. "Open" goes to the scene in the script view.
  In these views typing does not reach the script; shortcuts with a modifier
  still run (undo among them) and Escape goes back to the script.

## Tables

- **Name.** The spec's name for this panel is another product's trademark;
  ours is "Tables" (View › Tables, Mod+Alt+T), a panel at the right of the
  window.
- **Scenes table:** number (drag it to reorder scenes, as the cards do),
  heading and summary (click to edit; Enter or leaving the cell sets it,
  Escape cancels), the page the scene starts on (click to go there) and who
  speaks in it. Scene length in eighths of a page arrives with M6.
- **Characters table** (`character.list`): name, speeches, scenes, and the
  first scene (click to go there). Editing a name renames the character
  everywhere (`character.rename`, one undo step): every cue, keeping its
  extension; the name as a whole word or phrase elsewhere ("Anna's" changes,
  "Annabel" and "Joanna" do not), in the case style it is written in there
  (capitals stay capitals, lower case stays lower case, otherwise the new name
  as typed), keeping the formatting where it stood; and the Characters
  autocomplete list. A new name may not contain brackets (they would read as
  an extension).
- **CSV** (`tables.csv`, and Export CSV… in the panel, File › Export): RFC 4180
  with CRLF line ends; a field holding a comma, quote or line break is quoted;
  a field starting with `=`, `+`, `-` or `@` gets a leading `'` so a
  spreadsheet does not run it as a formula.
- **Fixed tabs.** The spec's tabs can be added, renamed, reordered and
  filtered; v0.1 has two fixed tables, and notes (M5) are listed in a third
  (recorded in `docs/gaps.md`).
