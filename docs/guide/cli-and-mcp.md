# Command line and agents

`wordsmith-cli` comes with the app.

```sh
wordsmith-cli convert draft.fountain draft.wss
wordsmith-cli export-pdf draft.wss draft.pdf
wordsmith-cli export-pdf draft.wss blue-pages.pdf --revised-only
wordsmith-cli report characters draft.wss cast.csv
wordsmith-cli report scenes draft.wss
wordsmith-cli info draft.wss
```

Inputs can be `.wss` or Fountain; plain text, RTF and PDF are imported as
scripts. Outputs take the format of their extension (wss, fountain, txt,
rtf, html, pdf). It exits with 0 when done, 1 when something failed and 2 for
a usage mistake.

## MCP

`wordsmith-cli mcp` is a Model Context Protocol server over standard input
and output, for AI agents and other tools. It edits a script with every
command the app has, without a window. It has the general tools every app on
open-source-core shares (`list_commands`, `execute`, `batch`, `type_text`,
`select_text`, `get_text`, `inspect_document`, `render_page` and a few more),
and each screenplay command as a tool of its own, named after it:
`scene_list`, `element_action`, `character_list`, `report_characters`,
`titlePage_set` and so on. Any other command runs through `execute` by its
id. Scripts are opened and saved with the app's own file commands:
`execute {"command": "file.openPath", "params": {"path": ...}}`, `execute file.save` and
`execute file.saveAsPath {"path": ...}` (the extension picks the format).
The general `open_document` and `save_document` tools know word-processor
files, not `.wss`.

Since 0.2 the server is the shared one: the tools `open_script`,
`save_script`, `export_script`, `set_element`, `scenes`, `characters` and
`page_count` of 0.1 are now `execute file.openPath`, `execute file.save`,
`execute file.saveAsPath`, `element_set`, `scene_list`, `character_list` and
`execute document.pages`.

To keep an agent to certain folders, start it with
`--read-root <folder>` and `--write-root <folder>`: every file it reads or
writes must be inside them, and printing and the print preview are refused
(they hand a file to another program). Without either option nothing is
confined: the agent can read and write any file you can, and the server says
so on standard error when it starts.
