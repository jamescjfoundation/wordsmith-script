# 0007: Trimming WordCraft's MCP server

Date: 2026-11-19 · Status: replaced by
[core decision 0007](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0007-engine-agents-output.md)

Since decision 0044 (2026-11-24) the agent server is the core's `jcj-agents`.

## Decision

`wordsmith-mcp` and `wordsmith-control-key` come from WordCraft `6ef13b2`
(`crates/mcp`, `crates/control-key`).

- **Kept as upstream:** the shared `craft-mcp-server` core, the bounded
  stdin reader, and the `Remote` backend that bridges to a running app. Its
  security rules stay: the control key is only ever sent to the app's own
  loopback address, and a mutating call whose reply was lost is never sent
  again. Its 12 tests are unchanged.
- **Tools:** `list_commands`, `execute`, `batch`, `inspect_document`,
  `render_page`, and the app-only `screenshot`, `click`, `key` and
  `ui_inspect`. Every command is reachable through `execute`. WordCraft's
  Word-specific tools (filled tables, templates, .docx open and save, typing
  and selecting through Word commands, feature parity) are left out. Open,
  save and typing tools came back later as screenplay tools (decision 0029).
- **One document per session**, as in the engine (decision 0006): the `doc`
  parameter and `list_documents` are gone.
- **`inspect_document`** runs the registry's `document.inspect` when there is
  one (the screenplay layer adds it); otherwise it returns the body text,
  paragraph count and selection.
- **Branding:** server name `wordsmith-script`, resource URIs
  `wordsmith://…`, our own instructions text, no website or community links.
  The control-key variable is `WORDSMITH_CONTROL_KEY`, the profile override is
  `WORDSMITH_PROFILE_DIR`, and the profile folder is `Wordsmith Script`
  (macOS and Windows) or `wordsmith-script` (Linux).
- **Tests not ported:** the oracle and differential tests compare the shared
  server with WordCraft's earlier hand-written one, which we never had.
  `tests/automation_roots.rs` drives `file.open` and `insert.picture`; it was
  not ported. The confinement is tested in `wordsmith-engine`'s `file_access`
  tests and through the file commands in `wordsmith-format`.

## Revisit when

The screenplay commands are exposed as named tools (done in decision 0029).
