# 0011: The M1 editor

Date: 2026-11-19

## Decisions

- **egui 0.36.2 and eframe 0.36.2**, as the kickoff and craft-libs v0.1.0
  specify (WordCraft itself is on a patched 0.37). Default features are off:
  no bundled egui fonts (their licences are not all on the allowlist; the
  interface uses our own Inter and JetBrains Mono), the OpenGL (`glow`)
  renderer instead of wgpu for a much smaller dependency tree, plus
  accessibility (AccessKit), Wayland and X11. eframe's clipboard brings in
  two BSL-1.0 crates on Windows, approved as a reviewed exception
  (decision 0001).
- **The interface owns no behavior.** Every key, click, menu item and dialog
  button runs a command by id through the session's registry. The
  interface's own actions (command palette, element settings dialog, About,
  recovery prompt, copy, cut) are registry commands too (`ui_specs`); they ask
  the window for something through `Session::push_ui_request`. So the menus,
  keys, palette, CLI and MCP reach the same commands.
- **Keys** map to the registry's portable shortcut strings (`Mod+Shift+Left`).
  On macOS, Option+Arrow moves by word, Command+Left/Right go to the line's
  ends and Command+Up/Down to the script's ends. A custom element's own
  shortcut is checked before the registry.
- **The view is the print layout.** Pages are rendered by `wordsmith-render`
  into textures, cached per page, document revision and scale, so only
  visible pages are drawn and only again after an edit. Caret, selection and
  clicks come from the layout's own hit testing. Normal and speed views are
  M3.
- **Focus.** The page view keeps keyboard focus with a focus-lock filter, so
  Tab and the arrows reach the script instead of moving egui's focus. Dialogs
  with text fields take the keyboard while they have focus.
- **One window per script** (decision 0006). Crash recovery is on, kept in the
  settings folder's `recovery` directory; the prompt appears when copies from
  an earlier run exist, and a copy restores into an untouched blank script.
- **No saving yet.** Files are M4; until then crash recovery is the only
  persistence.

## Testing

The editor is driven headlessly through egui input events (`Context::run_ui`
with key, text, copy, cut and paste events), so the tests run in CI on every
platform without a window. The real window was launched on macOS and stayed
up with no errors; its look was not checked automatically.
