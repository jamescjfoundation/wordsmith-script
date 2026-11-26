# Files

Scripts are saved as `.wss` files: plain UTF-8 text with the settings as
readable JSON at the top and then the script, one paragraph per element, with
`**bold**`, `*italic*` and `_underline_`. They compare cleanly with `diff` and
work well in version control. The format is documented in `docs/format.md`.

Version 0.2 saves format 5. Scripts saved by 0.1 open as they were, with the
same pages; once saved by 0.2 they open only in 0.2 or later.

- **Open** (Mod+O) reads `.wss` and Fountain (`.fountain`) scripts. A
  Fountain script opens as a new script; saving asks where to put the
  `.wss`.
- **Import** reads plain text, RTF and text-based PDFs (not scans). For
  text and RTF you are asked whether it is a script: if it is, scene
  headings, characters, dialogue and transitions are recognised, by
  indentation when the file has it and by convention when it does not. A PDF
  made by Wordsmith Script comes back as the same elements, title page
  included.
- **Export** writes Fountain, plain text (laid out in columns), RTF, HTML and
  PDF. The tables and the character report export as CSV.

New, Open, Import and closing the window ask before unsaved changes are
lost.

Open, Save As and the exports ask where with your system's own file dialog.
The window is not stopped while a dialog is up: it keeps drawing, and where
your system leaves it usable (the Open dialog on macOS does) you can go on
working. One dialog shows at a time; a second one asked for meanwhile waits
its turn. A dialog belongs to the script that asked for it. If you have moved
to another tab by the time you choose, the app goes back to that script and
opens, saves or exports there; if you have closed that script, nothing is
done and the status bar says so. When your system cannot show a dialog, the
status bar says why.

**Auto-save** saves a script that has a file every 15 minutes, once you stop
typing for 10 seconds. Every save also keeps a timestamped **backup** in the
Backups folder of the app's settings folder; the newest 1,000 of each script
are kept. Both are set in **File › Preferences**. If the app stops
unexpectedly, the next start offers to recover unsaved scripts.
