# 0020: Saving, auto-save, backups and preferences

Date: 2026-11-19

**File commands live in the interface layer** (`wordsmith-ui::files`), since
they need `wordsmith-format` (layer 6) and the script layer (5) cannot depend
on it. Each command without a path asks the window for a native file dialog,
and the dialog's answer runs the command that takes the path, so menus, keys,
the palette and agents run the same code:

- `file.open` / `file.openPath {path, asScript?, discard?}`: a `.wss` file
  opens as itself; Fountain opens as a new, unsaved script named after the
  file (Save asks where, so the Fountain file is never overwritten in the
  native format); plain text and RTF ask "is this a script?" (with
  `looks_like_script` as the default answer); PDF imports.
- `file.import`: the same, from a dialog listing the import formats.
- `file.save` (to the script's `.wss`, else Save As), `file.saveAs`,
  `file.saveAsPath {path}` (`.wss` added when missing).
- `file.exportFountain`, `file.exportText`, `file.exportRtf`,
  `file.exportHtml` (with or without a path); PDF export is decision 0013's.

**Unsaved changes are never lost without asking.** In the window, New, Open,
Import and closing with unsaved changes ask Save / Don't Save / Cancel; Save
saves first (asking where if needed) and then carries on. A command that would
replace unsaved changes is refused unless it says `"discard": true`, so an
agent cannot lose work by accident either. The window title shows the
script's name and a dot while it has unsaved changes.

**Writes are atomic.** Saves, exports, backups and the PDF export go through
craft-atomic: the old file stays whole if a write fails. Every path passes the
automation file gateway.

**Auto-save** (SPEC: on by default, 15 minutes, about 10 seconds idle): the
clock starts at the first change after a save; a save is due once that change
is the interval old and nothing has been typed for the idle time. Only a
script with a `.wss` file is auto-saved; an untitled one is kept by crash
recovery (decision 0011) until the writer saves it. A failed auto-save waits another
interval rather than retrying every frame. Times are passed in, so the tests
run on a made-up clock.

**Backups** (SPEC: 1,000 kept by default, at most 10,000, oldest removed):
every save from the window copies the saved file into `Backups` in the
settings folder as `<name> <YYYY-MM-DD HH.MM.SS> UTC.wss` (a number is added
for a second backup in the same second; an existing file is never replaced),
then removes that script's oldest backups beyond the number to keep. The time
is UTC so the names sort and never repeat across a clock change. Saves made by
agents through MCP do not make backups.

**Preferences** (view settings, auto-save, backups) are kept in
`preferences.json` in the settings folder, written through craft-atomic at
most once a second while they change and on exit. A missing key takes its
default, a damaged file is moved aside (`preferences.json.damaged`) and the
defaults are used, and every value is bounded when applied. `docs/REUSE.md` named
craft-settings; it depends on `dirs` → `option-ext`, which is MPL-2.0, outside
the licence allowlist, and a small file of our own was preferred to an
exception. The **personal dictionary** is craft-proof's own file
(`dictionary.txt` in the settings folder), loaded at start and saved, merged
with other windows' words, on exit.
