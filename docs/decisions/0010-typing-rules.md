# 0010: Typing rules the spec leaves open

Date: 2026-11-19

The spec gives the Return and Tab tables and the scene-heading separators.
These are the details it does not settle, decided here.

**Return**
- At the end of a paragraph: a new paragraph of the element's next element
  (`Style::next`). An unknown next element falls back to the same element.
- In the middle: the paragraph splits and both halves keep the element, as in
  any word processor; the writer is mid-sentence, not moving on.
- In a Parenthetical whose caret sits just before the closing bracket (where
  it is after typing into "()"), Return counts as the end.
- On an empty paragraph: the same as at the end.
- With a selection: the selection is deleted first.

**Tab**
- Tab changes the caret paragraph's element to the Tab target, whether or not
  it has text (SPEC: "Tab changes the current paragraph's type").
- Character's Tab acts only on a blank line (SPEC). On a Character with text
  it does nothing in M1; M3 uses it to accept a suggestion.
- In a Scene Heading with text, Tab acts only at the end of the heading:
  - after a known intro ("int", "EXT."), it writes the intro in capitals
    followed by the intro separator ("INT. ");
  - after an intro, its separator and a location, it adds the time separator
    ("INT. KITCHEN - "), once;
  - anything else does nothing. A blank Scene Heading switches to Action per
    the Tab table.
- Elements with no Tab target do nothing.
- Shift+Tab is not assigned in M1.

**Change element**
- Applies to every paragraph the selection touches (the caret paragraph when
  there is no selection).
- Uppercase elements convert the text to capitals, run by run, so each run
  keeps its formatting even where capitals are longer ("ß" → "SS").
- Switching a lone empty paragraph to Parenthetical writes "()" and puts the
  caret between the brackets; switching an untouched "()" away removes them.

**Editing**
- Backspace at the start of a paragraph joins it to the one before, which
  keeps its element; Delete at the end joins the next one in.
- `text.insert` treats line breaks as Return, turns tabs into spaces, and
  drops other control characters. It refuses more than 1 MiB or 10,000 lines
  in one call; a refused call leaves no trace.
- Left and Right with a selection and no Shift go to the selection's start and
  end. Up and Down keep the caret's column across short lines
  (`Session::goal_x`, cleared by every other command).
