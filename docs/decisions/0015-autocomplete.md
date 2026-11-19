# 0015: Autocomplete

Date: 2026-11-19

The spec gives the six lists, learning from the script, editing, rebuilding,
switching off per list, Tab accepting, and the options (extension on Tab,
next-speaker guess, ordering). Its specific popup and gray inline text were
Refuted, and reviews ask that suggestions be easy to ignore and never
auto-insert. Our design:

**Which list.** The caret decides: in a Character, the name (before any
bracket) uses Characters and an open bracket uses Character Extensions; in a
Scene Heading, the part before the intro separator uses Scene Intros, after it
Locations, after the time separator Times of Day; a Transition uses
Transitions. Other elements offer nothing.

**What is offered.** Entries starting with what was typed since that place
(ignoring case), not the exact text already typed, at most ten, in the list's
order: alphabetical (default), custom (the writer's arrangement; moving an
entry switches to it) or, for Characters, by number of speeches. Nothing is
offered for an empty prefix except the next-speaker guess.

**Next speaker.** On an empty Character, when the last two distinct speakers in
the scene are A then B, A is offered. It is only offered: Tab or Enter
accepts it, and since Tab accepts a showing suggestion, Tab on such a line
accepts the guess instead of switching to Transition (Escape first to switch).

**Learning.** Leaving a paragraph with Return adds its name, extensions,
location, time of day or transition to the lists. Rebuild (SPEC) rebuilds
Characters, Locations and Transitions from the script, dropping unused entries
and adding missing ones; Extensions and Times of Day keep their entries and
gain the script's. Defaults: Extensions V.O., O.S., O.C., CONT'D; Times of Day
DAY, NIGHT, MORNING, AFTERNOON, EVENING, LATER, CONTINUOUS, MOMENTS LATER;
Transitions CUT TO:, DISSOLVE TO:, FADE IN:, FADE OUT., FADE TO BLACK.,
MATCH CUT TO:, SMASH CUT TO:.

**Scene Intros** are the scene-heading settings' intros, one list, so Tab's
separators and the suggestions always agree.

**Extension on Tab** (on by default): Tab at the end of a named Character adds
" (" and the extensions list shows; accepting closes the bracket.

**The suggestion list.** A small list below the caret with a hint line. Tab
or Enter accepts the highlighted entry; Up and Down move; Escape hides it until
the text or caret changes; clicking an entry accepts it. Typing on narrows the
list or makes it go away. Nothing changes the script without a key press or
click, and accepting is one undo step.

**Storage.** `app_data["wordsmith.autocomplete"]`, per script and undoable;
read field by field so one bad value loses only itself; at most 5,000 entries
of up to 100 bytes per list.
