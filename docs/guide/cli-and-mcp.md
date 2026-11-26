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
rtf, html, pdf).

The command line works like that of every app built on open-source-core, so
a script written for one works with the next:

- What a command prints (a report with no output file, `info`, the page
  count of `export-pdf`) goes to standard output. What went wrong goes to
  standard error, as `wordsmith-cli: <reason>`.
- `wordsmith-cli --help` (or `-h`, or `help`) prints the usage.
  `wordsmith-cli --version` (or `-V`) prints `wordsmith-cli <version>` on
  its first line and the app's name on the next.
- Run with no argument, it prints the usage on standard error and exits
  with 2: nothing was asked, so it does not report success.

| Exit code | Means |
| --- | --- |
| 0 | Done |
| 1 | It failed: a file could not be read or written, or there was nothing to export (no revised page for `--revised-only`) |
| 2 | The command line was wrong: no command, an unknown command or option, a missing or extra argument. Nothing was done |
| 3 | Kept for a check that ran and did not pass (two texts differ). No command of `wordsmith-cli` checks or compares yet, so it never exits with 3 |

## MCP

`wordsmith-cli mcp` is a Model Context Protocol server over standard input
and output, for AI agents and other tools. It edits a script with every
command the app has, without a window, until its input ends.

```sh
wordsmith-cli mcp
wordsmith-cli mcp draft.wss
wordsmith-cli mcp draft.wss --read-root scripts --write-root scripts/out
```

Name a script and it is open when the agent connects; the server tells the
agent so. Any file `convert` reads can be named. Without one the agent starts
on a new, empty script.

The tools are of two kinds:

- The general tools every app on open-source-core shares, under the same
  names everywhere: `list_commands`, `execute`, `batch`, `type_text`,
  `select_text`, `get_text`, `inspect_document`, `render_page` and a few
  more.
- Each screenplay command as a tool of its own, named `script_` and the
  command's id with its dots turned into underscores: `script_scene_list`,
  `script_element_action`, `script_character_list`,
  `script_report_characters`, `script_titlePage_set` and so on. The prefix
  says whose tool it is when an agent is connected to several apps.

Any other command runs through `execute` by its id. Scripts are opened and
saved with the app's own file commands:
`execute {"command": "file.openPath", "params": {"path": ...}}`, `execute file.save` and
`execute file.saveAsPath {"path": ...}` (the extension picks the format).
The general `open_document` and `save_document` tools know word-processor
files, not `.wss`.

Since 0.2 the server is the shared one: the tools `open_script`,
`save_script`, `export_script`, `set_element`, `scenes`, `characters` and
`page_count` of 0.1 are now `execute file.openPath`, `execute file.save`,
`execute file.saveAsPath`, `script_element_set`, `script_scene_list`,
`script_character_list` and `execute document.pages`.

### Keeping an agent to certain folders

Start the server with `--read-root <folder>` and `--write-root <folder>`:
every file the agent's commands read must be inside the first, and every file
they write inside the second. Printing and the print preview are refused
(they hand a file to another program).

- Each folder must exist.
- One option alone grants only its own kind of access: with `--read-root`
  alone the agent can write nothing, and with `--write-root` alone it can
  read nothing.
- The script named on the command line is your choice, not the agent's: it
  is opened before the folders hold, so it may lie outside the folder the
  agent may read. Saving is still the agent's act: `file.save` writes that
  script back only when it lies inside the folder the agent may write. When
  it does not, the agent saves a copy there with `file.saveAsPath`.
- Without either option nothing is confined: the agent can read and write
  any file you can, and the server says so on standard error when it starts.
