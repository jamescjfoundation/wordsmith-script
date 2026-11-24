# Writing

## Elements

Every paragraph is one element: General, Scene Heading, Action, Character,
Parenthetical, Dialogue, Transition, Shot, Cast List, New Act, End of Act, and
the outline elements Outline 1 to 3, Summary and Note. An element's look and
behavior are set once for the whole script in **Format › Element Settings**:
font, size, style, alignment, line spacing, space before, indents, the
element Return goes to, whether it starts a new page, which element's page
rules it follows, and a shortcut. You can add elements of your own there.

**Return** at the end of a paragraph starts that element's next element;
Return in the middle splits the paragraph. **Tab** switches the paragraph:

| You are in | Return gives | Tab gives |
| --- | --- | --- |
| Scene Heading | Action | Action |
| Action | Action | Character |
| Character | Dialogue | Transition, on an empty line |
| Dialogue | Action | Parenthetical |
| Parenthetical | Dialogue | Dialogue |
| Transition | Scene Heading | Scene Heading |

In a scene heading, Tab after the intro adds `. ` and after the place adds
` - `. Both separators, and the intros themselves (INT, EXT, I/E), can be
changed. The **Character After Dialogue** command (find it with the command
palette, Mod+Shift+P) makes Return after dialogue start the next speaker.

## Autocomplete

Wordsmith Script keeps six lists, learned from what you write: characters,
character extensions, scene intros, locations, times of day and transitions.
As you type in a place one of them fits, matches appear under the caret.
**Tab** or **Return** takes the highlighted one, **Up** and **Down** move,
**Escape** hides them, and simply typing on ignores them: nothing is ever
filled in for you. The lists can be edited, rebuilt from the script, or
switched off one by one.

## Spelling, find and notes

Misspelled words are underlined; right-click one for suggestions, **Ignore
in This Script** or **Add to Dictionary** (your dictionary is kept between
sessions). Character names and scene intros are never flagged.

**Mod+F** opens the find bar, with replace, match case and whole words;
Replace All is one undo step. The Script panel's Find panel searches for the
same text. Escape closes the bar when you are typing in it or nothing else is
open; it never closes the bar on its way to closing a list or a window.

Select some text and press **Mod+Alt+N** to attach a note. Noted text is
tinted on the page; put the caret in it and press Mod+Alt+N again to read or
change the note. All notes are listed in the tables panel. Notes are kept with
the script and are not printed.

## Renaming a character

In the tables panel's Characters table, click a name and type the new one.
Every cue (with its extension, like V.O.), every mention in action and
dialogue, and the autocomplete list change at once, as one undo step.
