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
command the app has, without a window. Besides the general tools (list the
commands, run one or a batch, inspect the script, render a page as a
picture) it offers `open_script`, `save_script`, `export_script`,
`type_text`, `set_element`, `scenes`, `characters` and `page_count`.

To keep an agent to certain folders, start it with
`--read-root <folder>` and `--write-root <folder>`: every file it reads or
writes must be inside them.
