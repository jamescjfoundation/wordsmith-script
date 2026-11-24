# 0043: Key tips, and every control as a command

Date: 2026-11-22

## Context

Key tips (F10, then a tab's letter, then a control's letters) gave the first
20 controls of a tab one letter each and the rest two letters ("AA", "AB"…).
Typing "A" ran the first control at once, so no two-letter tip could be typed:
from the 21st control on, a tab could not be used from the keyboard. Letters
were handed out in drawing order, so they moved whenever a group folded to
fit the window, and a tip ran its control's command without the control's
parameters.

Some controls also changed the window directly instead of running a command,
so they were missing from the palette and the menus and could not take a key
or a key tip. The lists the tests checked them against were typed by hand.

## Decision

- **Prefix-free letters.** A tab's controls are numbered; number *n* below
  19 is one letter of `ABCDEFGHJKLMNQSTUXY`, and from 19 on it is "Z"
  followed by the letters of *n* − 19 ("ZA", "ZB", … "ZZA"). No tip begins
  another, and a control's tip does not depend on how many controls the tab
  has. The letters I, O, P, R, V and W are left out of the single tips.
- **Stable letters.** Each tab keeps, for the run, the controls in the order
  they were first seen, known by command and parameters (leaving out those
  that follow the window's state, such as the beats selected). A control
  seen later, because a group unfolded or a drop-down opened, goes at the
  end of the list. The plan was to put it right after the control drawn
  before it, which keeps the order closer to the screen's, but that changes
  the letters of every control after it the first time a drop-down opens,
  so we append instead: once a control has letters it keeps them.
- **Drop-downs.** A folded group and a drop-down in a group (More,
  Highlight, All Macros, the revision set) have a tip that opens them; their
  items then show letters too. Two controls running the same command with
  the same parameters share their letters.
- **Every control runs a command.** The ribbon's controls, the quick-access
  row and the status bar run commands from the registry, with their
  parameters, through the ribbon's widgets, which give them tooltips with
  the command's key, key tips and disabled states. The window-only switches
  became commands (`view.spelling`, `view.scriptPanel`, `view.inspector`,
  `view.lightTheme`, `view.menuBar`, `view.zoomTo`, `cards.print`,
  `cards.perRow`, `cards.scriptText`, `edit.paste`).
- **Checked against what is drawn.** Each control notes, as it draws, the
  command it runs and where it is. The tests draw every tab, menu and File
  page and check the record, instead of lists typed by hand. In debug builds
  they also ask egui to outline every clickable control and fail on one in
  the quick-access row, the ribbon's groups or the status bar that ran no
  command.

## Revisit when

- A tab has so many controls that three-letter tips become common; a code
  with more two-letter tips would then be worth its less regular letters.
- The writer can arrange the ribbon: the order should then follow the
  writer's arrangement.
