# 0032: Macros and text styles

Date: 2026-11-20

## Decision

- **Macro keys.** SPEC-2 suggests Mod+Alt+digit, which v0.1 gave to the views.
  The views keep them; the starter macros take Mod+Alt+Shift+1 to 0. A macro's
  shortcut may be any free key.
- **Starter set** (ours): Interior, Exterior, Day, Night, Continuous, two
  parentheticals, Cut, Dissolve, Fade out. Scripts made before v0.2 get the set
  when they are opened, since they have no macros stored.
- **"Then start"** an element means Return at the end of the paragraph and a new
  paragraph of that element.
- **Aliases** are replaced when the character that ends the word is typed, never
  on paste. "Smart replace" carries the alias's capitals to the replacement
  (all capitals, or a first capital). "Whole word" off also matches the alias
  at the end of a longer word.
- **Native format 4** stores the new character styles in a `{s…}` mark. A file
  that uses them would read wrongly in 0.1.0, so the version goes up and 0.1.0
  refuses version-4 files with its "newer version" message.
- **Highlighter names** are our own (Yellow, Lime, Aqua, Rose, Sky, Red, Navy,
  Teal, Green, Violet, Maroon, Olive) plus any color.
- **"All lowercase"** is a change to the text (like toggle case) for a
  selection, and an element casing (Small letters) in element settings.
- **The indent rule** stays as v0.1 built it (0 to 8.5 in, at least half an inch
  apart) rather than SPEC-2's 0 to 8 in, one inch apart, because tightening it
  would refuse settings v0.1 accepted (decision 0030).
