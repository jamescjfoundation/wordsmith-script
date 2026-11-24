# Contributing

Thank you for helping. This repository is public: everything you commit, push,
or write in an issue, pull request or review comment can be read by anyone and
stays in the history for good. The rules below follow from that.

## Building and checking

```sh
cargo run --release -p wordsmith      # the app
cargo xtask ci                        # the gate every change must pass
cargo xtask screenshots <folder>      # the interface pictures, drawn offscreen
```

`cargo xtask ci` runs rustfmt, the crate-root lint check, the checks that the
settings shared with open-source-core match its pinned release and that no
WordCraft or craft-libs crate is named here, the privacy check, the license and
asset attribution checks, clippy and every test. Run it before you open a pull
request; CI runs it again (`.github/workflows/ci.yml`, the core's), and the
release runs it on Linux, macOS and Windows.

## Where things are

| Path | What it is |
| --- | --- |
| `crates/wordsmith-script` | Screenplay rules: elements, typing, pagination, revisions, scene numbers, locks, reports, and the screenplay commands |
| `crates/wordsmith-format` | The native `.wss` format, Fountain, imports and exports, and the file commands |
| `crates/wordsmith-ui` | The egui interface |
| `apps/wordsmith`, `apps/wordsmith-cli`, `apps/wordsmith-web` | The desktop app, the command line and MCP server, and the web build |
| `xtask` | The gate, bundling and screenshots |
| `docs/guide` | The user guide |
| `docs/decisions` | Why things are the way they are, one record per choice |
| `docs/history` | The first build's plans and audits, kept for the record and not maintained |

The document model, layout, PDF, the command engine, the shared document
features, the window around the script and the agent server come from
[open-source-core](https://github.com/jamescjfoundation/open-source-core)'s
`jcj-` crates (decision 0044). A change that would work the same in any app
belongs there: `AGENTS.md` says how to make one.

Every action in the app is a command in one registry. The window, the command
line and the MCP server all run the same commands, so a new feature is a
command first and a button second.

## Rules for a public repository

### Personal information

Never commit, or paste into an issue or pull request:

- anyone's name, email address, phone number or account, other than the
  commit author line you chose (see "Commits" below);
- local paths, user names, host names, IP addresses or machine details
  (`/Users/<name>/…`, `C:\Users\<name>\…`, `/home/<name>/…`);
- files from a real machine: `preferences.json`, crash recovery files,
  auto-save or backup copies, logs, or screenshots of a desktop;
- a real person's screenplay or notes. Test scripts and screenshots use
  public-domain works (the ones in `crates/wordsmith-format/tests/golden`) or
  text you wrote for the purpose;
- passwords, tokens, keys or `.env` files.

When someone attaches a file with personal content to an issue, a maintainer
deletes or edits the attachment and asks for a made-up example instead.

The app follows the same rule. It sends no telemetry and never sends script
content anywhere. Any network request it makes must be listed in the README's
Privacy section and must have a setting to turn it off.

### Security problems

Follow `SECURITY.md`. In short: report privately, keep the problem out of public
issues, pull requests, commit messages, branch names, code comments and files
until a release fixes it, and then disclose it in an advisory and the
changelog.

### Private working notes

Reviews, audits, plans that mention open security problems, and anything else
you keep while working but that is not ready to be public go in `.private/` at
the top of your checkout. Git ignores that folder. Write the public part of what
you learned as a decision record, a guide page or a changelog entry, in words
an outsider can follow. Do not refer to private conversations ("as discussed",
"the owner said"); give the reason itself.

### Clean room and names

- Labels, names, layouts, icons, templates and help text are our own. Do not
  copy them from another screenwriting app or word processor, and do not use
  another product's feature names or trademarks. The maintainers keep a list of
  names to avoid: ask if you are not sure about one.
- Do not inspect, run, decompile or read the files of a proprietary
  screenwriting program to find out how it behaves. Public manuals and public
  formatting guides are fine.
- A proprietary file format needs a legitimately published description before
  we support it.

### Upstream code and branding

- No WordCraft or craft-libs code is copied here, and no crate of theirs is a
  dependency: the core wraps them (`UPSTREAM.md`).
- Refer to an upstream issue as `storytold/wordcraft#123`, never `#123`, which
  links to an unrelated issue here.
- No ArtCraft or WordCraft name, logo or icon, and no "Craft", in anything users
  see. The credit lives in `NOTICE` and `ATTRIBUTION.md`.

### Dependencies and assets

- Every dependency's license must be on the allowlist in
  `docs/decisions/0001-license-allowlist.md`. `cargo xtask ci` checks it.
- Fonts, images, dictionaries and other data must be under an allowed license
  and listed in `ATTRIBUTION.md`. Icons are drawn in code.

## Commits and pull requests

- Contributions are licensed as the project is, MIT OR Apache-2.0, at the
  user's option, with no extra terms (Apache-2.0 section 5). There is no CLA.
- Commits are public with the name and email in your Git settings. If you do
  not want your email public, use your GitHub noreply address
  (`git config user.email <id>+<login>@users.noreply.github.com`).
- Keep one concern per pull request. Say what changed and why, and how you
  checked it. Interface changes include before and after pictures from
  `cargo xtask screenshots`.
- Update the user guide, `docs/gaps.md` and `CHANGELOG.md` when behavior
  changes.
- AI-assisted contributions are welcome. You are responsible for every line, as
  if you wrote it; the same rules apply.

## Conduct

Follow `CODE_OF_CONDUCT.md`.
