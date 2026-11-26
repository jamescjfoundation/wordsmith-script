# Contributing

Thank you for helping. This repository is public: everything you commit, push,
or write in an issue, pull request or review comment can be read by anyone and
stays in the history for good. The rules below follow from that.

## Building and checking

```sh
cargo xtask install-hooks             # once: pre-commit and pre-push hooks
cargo xtask ci                        # the gate every change must pass (CI runs the same)
cargo xtask ci --changed              # only the packages your change affects, and their dependents
cargo run --release -p wordsmith      # the app
cargo run -p wordsmith-cli -- help    # the command line and MCP server
cargo xtask screenshots               # redraw the pictures in docs/images, offscreen
```

`cargo xtask help` lists every task. `cargo xtask ci` runs rustfmt, the
crate-root lint check, the checks that no local core link is left, that every
`jcj-` dependency names one core release, that the files and rule blocks shared
with open-source-core match its pinned release and that no WordCraft or
craft-libs crate is named here, the privacy check, the license and asset
attribution checks, clippy and every test. Run it before you push or open a
pull request; CI runs it again (`.github/workflows/ci.yml`, the core's), and
the release runs it on Linux, macOS and Windows.

## Where things are

| Path | What it is |
| --- | --- |
| `crates/wordsmith-script` | Screenplay rules: elements, typing, pagination, revisions, scene numbers, locks, reports, and the screenplay commands |
| `crates/wordsmith-format` | The native `.wss` format, Fountain, imports and exports, and the file commands |
| `crates/wordsmith-ui` | The egui interface |
| `apps/wordsmith`, `apps/wordsmith-cli`, `apps/wordsmith-web` | The desktop app, the command line and MCP server, and the web build |
| `assets/brand` | Copies of the Wordsmith brand kit's logos, icons and palette, and Script's own theme file and derived icons (`AGENTS.md`, "Brand") |
| `assets/fonts` | The bundled fonts, with their licenses |
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

<!-- BEGIN open-source-core: contributing -->
<!-- Owned by open-source-core (template/CONTRIBUTING.md). Change it there, never in a copy: `cargo xtask sync` rewrites the copies. -->
## Rules for a public repository

### Personal information

Never commit, or paste into an issue or pull request: anyone's name, email address or account
(other than your commit author line); local paths, user names, host names or machine details;
files from a real machine (preferences, recovery, auto-save or backup files, logs, desktop
screenshots); anyone's private writing; passwords, tokens or keys. Test fixtures are
public-domain texts or written for the test. `cargo xtask privacy` checks files and commit
identities.

### Security problems

Follow `SECURITY.md`: report privately and keep the problem out of public places until a release
fixes it.

### Private working notes

Reviews, audits and working notes go in `.private/` (ignored by Git). One is published only when
a maintainer says so, under `docs/history/`, with a first line saying it is history
([core decision 0012](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0012-working-in-the-open.md)).
Write the public part as a decision record or changelog entry, giving the reason itself rather
than "as discussed".

## Commits, pushes and pull requests

- While no app built on the core has a public release, a push to `main` is allowed once
  `cargo xtask ci` passes. A pull request is optional; its author may merge it once its checks
  pass
  ([core decision 0013](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0013-app-and-core.md)).
- A release is cut, and in the core an upstream pin is moved, only on a commit whose full gate
  has passed; a person or an agent may do either (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
- Contributions are licensed as the project is, MIT OR Apache-2.0, with no extra terms. No CLA.
- Use your GitHub no-reply address (`git config user.email <id>+<login>@users.noreply.github.com`).
- One concern per commit or pull request; say what changed, why, and how you checked it.
- Update `CHANGELOG.md` and the decision records when behavior changes. A core change that apps
  must act on says so under "What apps must do".
- AI-assisted contributions are welcome; you are responsible for every line.
<!-- END open-source-core: contributing -->

## Wordsmith Script's own rules

These add to the rules above; they hold only in this repository.

### Scripts and the app's own privacy

- Never commit a real person's screenplay or notes. Test scripts and screenshots use
  public-domain works (the ones in `crates/wordsmith-format/tests/golden`) or text you wrote for
  the purpose.
- When someone attaches a file with personal content to an issue, a maintainer deletes or edits
  the attachment and asks for a made-up example instead.
- The app follows the same rule. It sends no telemetry and never sends script content anywhere.
  Any network request it makes must be listed in the README's Privacy section and must have a
  setting to turn it off.

### Clean room and names

- Naming another company, program, service or file format is fine where it is
  a fact a reader needs: which program's files Script opens or writes, where a
  formatting rule comes from. Cite a source by name and address
  ([project decision 0003](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0003-naming-other-companies-programs-and-formats.md)).
- Labels, names, layouts, icons, templates and help text in the interface are
  our own. Do not copy them from another screenwriting app or word processor,
  and do not use another product's name or distinctive feature name as ours.
  Script is never described as replacing, following or being inspired by
  another product.
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
- No ArtCraft or WordCraft name, logo or icon, and no "Craft", in the app's
  interface or its brand. The credit lives in `NOTICE`, `ATTRIBUTION.md` and
  the README's acknowledgments.

### Dependencies and assets

- Every dependency's license must be on the core's allowlist
  ([core decision 0001](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0001-licenses-and-supply-chain.md)),
  which `deny.toml` carries. `cargo xtask ci` checks it.
- Fonts, images, dictionaries and other data must be under an allowed license
  and listed in `ATTRIBUTION.md`. Icons are drawn in code.

### What a change brings with it

- Interface changes include before and after pictures from `cargo xtask screenshots`.
- Update the user guide, `docs/gaps.md` and `CHANGELOG.md` when behavior changes. The wiki and
  the website follow from those ([Route C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c)).

## Conduct

Follow `CODE_OF_CONDUCT.md`.
