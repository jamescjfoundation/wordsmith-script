# 0029: The command line, the MCP server and the release

Date: 2026-11-19

## Command line

`wordsmith-cli` reads `.wss` and Fountain scripts and imports plain text, RTF
and PDF as scripts:

- `convert <input> <output>`: write in the format the output's extension
  names (wss, fountain, txt, rtf, html, pdf).
- `export-pdf <input> <output.pdf> [--revised-only] [--no-marks]`: the
  printed script, with its title page; or only the revised pages; with or
  without revision marks.
- `report characters|scenes <input> [<output.csv>]`: the character report or
  the scenes table as CSV, to the file or to standard output.
- `info <input>`: title, pages, scenes, characters, paragraphs.
- `mcp [--read-root <folder>] [--write-root <folder>]`: the MCP server.

Exit codes: 0 done, 1 failed (a file could not be read or written, nothing
revised to export), 2 a usage error. Writes are atomic.

## MCP server

`wordsmith-cli mcp` serves the Model Context Protocol (JSON-RPC 2.0 over
standard input and output) on a headless session with every screenplay
command and the file commands. Besides the general tools (list commands, run
one or a batch, inspect the script, render a page), screenplay tools:
`open_script`, `save_script`, `export_script`, `type_text`, `set_element`,
`scenes`, `characters`, `page_count`. With `--read-root` and `--write-root`,
every file a command reads or writes must be inside those folders, for the
whole process. Desktop-only tools (screenshot, click, key) answer that they
need the app.

The file commands moved from the interface to `wordsmith-format::commands`,
so the window, the command line and the MCP server share one copy; the
window still answers their requests for file dialogs.
