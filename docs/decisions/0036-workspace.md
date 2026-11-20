# 0036: Workspace and productivity

Date: 2026-11-20

## Decision

- **Templates** are built in code, all our own, from public conventions: Screenplay,
  Screenplay with notes, Single-camera episode, Multi-camera sitcom, Stage play,
  Audio drama, Treatment, Series bible, and three-, four- and five-act and
  act-and-sequence outlines. None follows a named method or a company's house
  style. A writer's own template is a native file saved as `.wst`.
- **Text documents** are scripts flagged `text`: prose in General paragraphs, no
  mores and continueds. The same editor serves both.
- **Tabs** keep each script's own session; the editor swaps sessions and forgets
  what it drew for the last one. Only the first script has crash recovery. Closing
  the window asks about every tab with unsaved changes in turn.
- **Split** shows a second view (cards, scenes, story board, outline or the
  script) beside or below the main one; Swap exchanges them.
- **Zoom** steps are 75 % to 300 %; the earlier Zoom In and Out (by a quarter)
  stay as v0.1 built them.
- **Statistics** belong to the writer, so they are kept in `stats.json` beside the
  preferences, not in scripts: words and pages added (never taken away by
  deleting), and writing time while input came within the idle timeout (3 minutes
  by default), by day and hour (UTC). A streak is days in a row with words or a
  minute of writing.
- **Goals**: SPEC-2's beginner suggestion, a page and 25 minutes a day. We do not
  set separate defaults for experienced writers (SPEC-2 left them open): the goals
  are the writer's to change.
- **Sprints** last 25 minutes by default (SPEC-2 left it open).
- **Keys**: a writer's key is looked up before the built-in ones; mapping a key to
  nothing turns a built-in key off. Keys are kept in the preferences.
- **Read-only files** open as a copy (Save asks where), since nothing in the
  editor makes a document read-only.
