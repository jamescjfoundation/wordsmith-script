# Notes for coding agents (and people)

Wordsmith Script is a screenwriting app. Its screenplay rules (elements, typing, pagination with
(MORE) and (CONT'D), Fountain, the `.wss` format, scene numbers, reports, the title page) live in
`crates/script-model` and `crates/script-format`; its interface in `crates/script-ui`; the
programs in `apps/`; the files its tests read in `tests/`. Read `CONTRIBUTING.md`, `SECURITY.md`
and `docs/decisions/` (decision 0044 says what the app takes from the core, decision 0045 what
the crates and programs are called) before you start.

This app is built on [open-source-core](https://github.com/jamescjfoundation/open-source-core),
The James C. J. Foundation's shared base. The core holds what every app needs; this repository
holds only what is about this app.

Two kinds of text below are carried word for word and never edited here. The blocks marked
`open-source-core` are the core's rules: `cargo xtask sync` writes them from the pinned core
release, and `cargo xtask sync --check` fails when one differs. The Wordsmith project block is
owned by the project map in the wordsmith repository, which writes and checks it. Everything
else in this file is this app's own.

<!-- BEGIN wordsmith project block -->
## The Wordsmith project

Wordsmith is five repositories and two forks, all under [jamescjfoundation](https://github.com/jamescjfoundation). The full map, the steps of every route and the project's rules are in the [project map](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md), which the wordsmith repository owns.

| Repository | Owns |
| --- | --- |
| [wordsmith](https://github.com/jamescjfoundation/wordsmith) | The project map and rules (`project/`), the brand kit (`brand/`), the website (`website/`), the wiki (`website/wiki/`), and Wordsmith Launcher (`launcher/`), which installs and updates the apps from their releases |
| [wordsmith-script](https://github.com/jamescjfoundation/wordsmith-script) | Wordsmith Script, the screenplay app, and the `.wss` file format |
| [wordsmith-manuscript](https://github.com/jamescjfoundation/wordsmith-manuscript) | Wordsmith Manuscript, the novel and long-form app, the `.wsm` project format, and the book view that Book reads |
| [wordsmith-book](https://github.com/jamescjfoundation/wordsmith-book) | Wordsmith Book, the book design and typesetting app, and the `.wsbk` project format |
| [open-source-core](https://github.com/jamescjfoundation/open-source-core) | The shared base: the `jcj-` crates and the settings and tooling every app carries. It is general and names no product |
| [wordcraft](https://github.com/jamescjfoundation/wordcraft), [craft-libs](https://github.com/jamescjfoundation/craft-libs) | Our forks of the upstream projects the core builds on |

- Each app builds on open-source-core at one release tag and carries its shared files (`cargo xtask sync`). Only the core reaches the forks.
- Wordsmith Book also builds on Wordsmith Manuscript's model and format crates, at a book-view tag. Wordsmith Manuscript reads Wordsmith Script's `.wss` files.
- The launcher reads each app's public releases by the core's release contract, so what an app's release holds must keep to that contract.
- Each app takes its logos, icons and palette from the brand kit, unchanged, by the kit's tool. Each app's `docs/guide/` is its one user guide: the wiki's pages for the app are built from it. The website and the wiki read each app's latest release from its GitHub releases.

| The change | Where it goes | Route |
| --- | --- | --- |
| A fix in a `jcj-` crate, or something any app would share | open-source-core first, then the app's pin | [A](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-a) |
| Something one app needs from another app | An issue in the providing app's repository | [B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b) |
| A change users will see in an app | The app's `docs/guide/` and `CHANGELOG.md`; then wordsmith builds the wiki's pages from the guide | [C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c) |
| A wiki page | About one app: that app's `docs/guide/`, never the wiki's copy. About the project: wordsmith, `website/wiki/content/docs/start/` | [D](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-d) |
| Website text, or the words for an app's state on the site | wordsmith, `website/tools/build.py` and `website/data/apps/` | [E](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-e) |
| A logo, icon or brand color | wordsmith, `brand/source/`; then its tool writes the copies into each app's `assets/brand/` | [F](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-f) |
| Screenshots | Drawn in the app into `docs/images/`; wordsmith's tool carries them | [G](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-g) |
| A security problem | Never public: `.private/` and a maintainer | [H](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-h) |
| A core release, or moving an app to a newer core | open-source-core, then each app | [I](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-i) |
| An app release | The app; the website and wiki pick it up by themselves | [J](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-j) |
| A change in WordCraft or craft-libs | Our fork, then the core | [K](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-k) |
| A change that spans two repositories | The provider first; each change names the other | [L](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-l) |
| Working with all the repositories, or with only one | Side by side as sibling folders | [M](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-m) |
| A rule or instruction that several repositories carry | The block's owner: the core's `template/`, or the project map for this block | [N](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-n) |

Before changing a repository, read its own `AGENTS.md`. Its rules apply inside it; this block applies in all of them. The block is owned by the project map and copied from it by a tool: it is never edited in a copy.
<!-- END wordsmith project block -->

<!-- BEGIN open-source-core: app -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## Before you commit

```sh
cargo xtask install-hooks   # once: pre-commit (fmt, privacy, no dev-link in a tracked file) and pre-push (ci --changed)
cargo xtask ci --changed    # what CI will run for your change; `cargo xtask ci` for everything
cargo xtask sync            # after moving to a new core release: shared files, blocks and patch block
```

What the core owns is changed in the core, never here:

- whole files: `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml` (but for the
  app's own sources in it, which `sync` writes from `xtask/src/main.rs`), `CODE_OF_CONDUCT.md`,
  `xtask/Cargo.toml`, every file in `xtask/src/` but `main.rs` and the modules the app adds,
  `.github/workflows/ci.yml` and `core-pin.yml`, `.github/pull_request_template.md`,
  `.github/dependabot.yml` and `hooks/`;
- the blocks marked `open-source-core` in files that are otherwise the app's: in `AGENTS.md`,
  `SECURITY.md` and `CONTRIBUTING.md`, and in `Cargo.toml` (lints and build profiles),
  `.cargo/config.toml`, `.gitignore` and `.gitattributes`;
- the patch block in `Cargo.toml`.

`cargo xtask sync --check` fails if any of them differs from the pinned core release.

`xtask/src/main.rs` is the app's own and holds only what is the app's: its data (the product's
name, the programs it ships, the repositories it builds on besides the core) and its own tasks.
The gate, its list of checks and how each is run are the core's (`xtask/src/gate.rs`), so a
check the core adds runs here after `cargo xtask sync`, with no edit. The app's layers are the
table marked `layer table` in this file: `cargo xtask app-layers` reads that table, so the table
is the rule, and a layer is changed there in the same change as the code. The two workflow
files only say when CI runs: the jobs are the core's shared workflows, which they call, so a fix
to CI is made in the core and reaches the app without a change here.

## House rules for code

- Every crate root: an SPDX line, `#![forbid(unsafe_code)]`, and `#![deny(...)]` of the six
  panicking lints. Product code returns errors; it never unwraps or panics.
- Depend only on the core's `jcj-` crates and, under a contract, another app's crates
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md));
  never on `jcj-upstream`, `wordcraft-*` or `craft-*` (`cargo xtask layers`).
- The app's brand (its name, logo, icons, fonts and colors) lives in the app's repository, or in
  its project's brand kit with copies in the app, never in the core. The app's interface theme
  is its own `tokens.json`, which the core's theming code reads.
<!-- END open-source-core: app -->

<!-- BEGIN open-source-core: public -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## What is public

This repository is public, history included, forever
([core decision 0012](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0012-working-in-the-open.md)).
Before you write a commit, branch name, issue, pull request or comment, check it holds none of:

- anyone's name, email, account names, or details of their machine (local paths such as a home
  folder, user names, host names, IP addresses);
- what the user told you in conversation, quoted or summarised; write the reason for a change as
  its own argument;
- an open security problem (follow `SECURITY.md`: keep it in `.private/` and tell the user);
- secrets, or files from a real machine (preferences, recovery, auto-save, backups, logs,
  screenshots of a desktop);
- anyone's private writing, even as test data. Fixtures are public-domain or written for the test.

Commit with a GitHub no-reply address or a foundation bot identity. Where a repository has
`cargo xtask`, `cargo xtask privacy` checks its files and every commit. Never push with `--all`
or `--mirror`.

The Foundation's name is always written "The James C. J. Foundation", with its capital "The"
wherever it stands in a sentence, never in a shorter form.
<!-- END open-source-core: public -->

<!-- BEGIN open-source-core: app-and-core -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## Building between an app and the core

"The core" is [open-source-core](https://github.com/jamescjfoundation/open-source-core); "an app"
is any repository built on it. A record of the core is cited from anywhere as "core decision
0011", with its repository named, never as a bare number: an app has records with the same
numbers.

### Step 1: decide where the change belongs

Ask in order; the first "yes" decides.

1. **Does it change how we use WordCraft, craft-libs or another upstream project?** The core's
   adapters (`jcj-upstream` and the `jcj-` crate that wraps it). If upstream code itself must
   change, it goes in our fork first (the core's `docs/UPSTREAM.md` and
   [core decision 0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)).
2. **Would it look or work the same in any foundation app?** The window, ribbon, panels, dialogs,
   File page, preferences, auto-save, updates, saving and backups, spelling, find and replace,
   undo, format import and export, PDF and printing, cards and boards, theming, entries kept
   beside documents, interface translations, agent control: the core.
3. **Does a second app already need it, or have a written plan to use it?** The core.
4. **Otherwise** the app. Screenplay elements, Fountain, scene numbers, (MORE)/(CONT'D), a
   manuscript binder's compile rules and a book's themes are app code.

When unsure, put it in the app: moving code into the core later is cheap; taking a wrong general
interface back out is not.

| Example | Where | Why |
| --- | --- | --- |
| Redesigned ribbon, new panel docking | Core (`jcj-shell`) | Every app has a ribbon and panels |
| A new app-specific ribbon tab | The app | Only the tab's contents are the app's |
| Faster auto-save, better backups | Core (`jcj-project`, `jcj-shell`) | Same for every app |
| Index cards a second app also wants | Core (`jcj-board`) once it needs them | Second app |
| DOCX import fix | Core (`jcj-interchange`), offered upstream too | General |
| Colors, fonts, logo | The app's repository or its project's brand kit | The core holds theming code, never a brand |

### Step 2: build it

- **How a change lands**
  ([core decision 0013](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0013-app-and-core.md)):
  every change, in the core and in an app, lands on `main` through a pull request whose
  required checks pass, and its author, a person or an agent, merges it. GitHub refuses a direct
  push to `main`. No approval is asked for: run the gate before you push the branch.
- **App-only change:** one change in the app.
- **Core change an app needs:**
  1. Open a core issue describing the general need, not the app's feature.
  2. Check out both side by side. In the app: `cargo xtask dev-link ../open-source-core`
     (`cargo xtask dev-unlink` removes it). The link changes no file Git tracks: it is a folder
     Git ignores, `.cargo/dev-link/`, and `Cargo.lock` is left alone. While linked, builds,
     tests and the editor use the local core, and `cargo xtask ci --linked` runs the gate
     against it. The plain gate refuses to run linked, because CI builds the pinned release.
  3. Build the general part in the core with its own tests and a use in `apps/core-testbed`, so it
     is proven without any product.
  4. Build the app's use of it in the app.
  5. To have CI check the app's half before the core is released, give the app's branch a
     temporary pin. Push the core change (a branch will do). On the app's branch, on every
     `jcj-` line, write `rev = "<that core commit, all 40 characters>"` in place of the tag; then
     `cargo metadata --format-version 1 >/dev/null` and `cargo xtask sync`. The gate passes on
     that branch and in its pull request, with a warning. It fails on `main`, on a tag and in a
     release, so the pull request is not merged while it holds the pin.
  6. Land the core change first, with the core's gate green. Then release the core.
  7. Move the app's pin to that release's tag (`cargo xtask sync`), then land the app change,
     which names the core issue it needed.
- **Prototype first, promote later.** If the general shape is not clear yet, build it in the app,
  mark it `// core-candidate: open-source-core#N` and open that core issue, labelled
  `core-candidate`. Move it once it works and a second app wants it. A promotion is finished only
  when the app the code came from uses the core's copy and its own copy is deleted: the core
  issue stays open until then, and the marker on the app's copy names it. Never copy code
  between app repositories.
- **Something one app needs from another app**
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md)):
  open an issue in the providing app's repository. The provider's change lands and is tagged
  there first; the consumer then moves its pin. Never edit the provider as part of the consumer's
  change. To try the two together, the consumer names the provider in its `xtask/src/main.rs`
  (`providers`) and links a checkout of it like the core: `cargo xtask dev-link <name>=<folder>`.

### Step 3: let it reach the other apps safely

- Everything in the core is shared by default.
- A change that alters what users see in other apps goes behind an option (a shell setting or a
  Cargo feature) that defaults to the old behavior; each app turns it on when ready.
- Versioning
  ([core decision 0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)):
  while at 0.x a patch release only adds or fixes; a minor release may break and its changelog
  says exactly what each app must do. A breaking change to a public `jcj-` interface says so in
  the title of its commit or pull request, and every app is updated in follow-up changes.
- A core change that breaks another app is fixed in the core, not worked around in the app.
- An app moves to the newest core before new work starts on it and before its own release. Every
  `jcj-` line names one tag. `cargo xtask core-pin` says how far behind an app is, and a
  scheduled check opens an issue when it is more than three releases behind.

### Releasing an app

One contract and one workflow for every app
([core decision 0019](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0019-app-releases.md);
the contract, with what each release file is named and holds, is the core's
[`docs/app-releases.md`](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/app-releases.md)).
The gate runs first and the workflow makes the tag, as for the core.

1. Move the app to the newest core. Write what the release changes under `## Unreleased` of
   `CHANGELOG.md`.
2. `cargo xtask release X.Y.Z` makes the changes of the release commit: the version in
   `Cargo.toml` and `Cargo.lock`, and the changelog heading with the date. Run `cargo xtask ci`,
   commit, and land the commit on `main` through a pull request.
3. `gh workflow run release.yml --ref main`. It checks the release commit, runs the whole gate
   on Linux, macOS and Windows, builds and checks the package of each system and
   `SHA256SUMS.txt`, and only then makes the tag `vX.Y.Z` and publishes. A red job leaves no
   tag: land the fix on `main` and start it again.
4. To try it without releasing: `gh workflow run release.yml -f dry-run=true`. The same dry run
   starts by itself every week. `cargo xtask package` makes this computer's package in `dist/`.
   The app's whole gate also runs on macOS and Windows every night (`platforms.yml`; a failure
   opens an issue): an issue it opened is fixed before a release.

What a release carries is data in `xtask/src/main.rs` (`package`). The packaging and the
workflow are the core's: an app has no release workflow and no packaging script of its own.

### Things agents must not do

- Name WordCraft or craft-libs types in an app repository. Use the `jcj-` interfaces; if one is
  missing, add it to the core.
- Put a product name, color, logo, or screenplay, manuscript or book concept into the core's
  code, interface text or `template/`.
- Change a core interface from a change made in an app repository.
- Copy code from one app repository into another.
- Release the core, or move an upstream pin, on a commit whose full gate has not passed. A
  person or an agent may do either once it has (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
- Push a `v*` tag, or create or edit a release, by hand: in the core and in an app the release
  workflow makes both, after the gate. A tag cannot be taken back.
- Edit WordCraft or craft-libs code anywhere but our forks' topic branches.
- Edit a file or a marked block the core owns anywhere but in the core.
- Commit personal details, private files or secrets.

### Where each kind of record goes

| Record | Lives in |
| --- | --- |
| Decisions about shared interfaces, upstream, tooling | The core's `docs/decisions/` |
| Decisions about one app's behavior | That app's `docs/decisions/` |
| Pins, carried patches, follow / hold / own | The core's `docs/UPSTREAM.md` |
| What changed for apps in each core release | The core's `CHANGELOG.md`, with "What apps must do" |
| Ideas waiting to be promoted to the core | Core issues labelled `core-candidate` |
| A request to another app | An issue in that app's repository |
| Reviews, audits, working notes | `.private/`, or `docs/history/` once a maintainer publishes one |
<!-- END open-source-core: app-and-core -->

## Wordsmith Script's own rules

### Layers

`cargo xtask app-layers` reads this table, so the table is the rule. A row is a layer, lowest
first. A crate or program depends only on those of earlier rows, and uses a core crate only when
its own row or an earlier one names it in backticks. Every package of the workspace but `xtask`
is in one row.

<!-- BEGIN layer table (cargo xtask app-layers reads it) -->
| Layer | Crate or program | What it holds | May use |
| --- | --- | --- | --- |
| 1 | `crates/script-model` | The screenplay rules: elements, typing, pagination, revisions, scene numbers, locks, reports, and the screenplay commands | `jcj-doc`, `jcj-engine`, `jcj-features`, `jcj-layout`, `jcj-output`, `jcj-project`, `jcj-proof` |
| 2 | `crates/script-format` | The `.wss` format, Fountain, imports and exports, and the file commands | the screenplay rules, `jcj-interchange` |
| 3 | `apps/script-cli` | The command line and the MCP server | the two crates above, `jcj-agents` |
| 4 | `crates/script-ui` | The interface | the two crates above, `jcj-board`, `jcj-shell`, `jcj-theme` |
| 5 | `apps/script`, `apps/script-web` | The desktop app; the web build | the interface |
<!-- END layer table -->

What holds the layers:

- `cargo xtask app-layers` fails when a crate or program depends on one of its own row or a
  later one, or uses a core crate that its row and the rows before it do not name. So the
  screenplay rules sit under the format and both sit under the interface (decisions 0002 and
  0044), and nothing below `wordsmith-script-ui` depends on it or on `jcj-shell`, `jcj-theme` or
  `jcj-board`: the command line and the MCP server run every command without a window. The
  command line is its own row below the interface for that reason.
- `cargo xtask layers` fails when any crate here depends on a `wordcraft-*` or `craft-*` crate or
  on `jcj-upstream`.
- `cargo xtask lint-roots` fails unless `wordsmith-script-model` and `wordsmith-script-format`
  deny indexing and slicing as well as the six panicking lints.

A new core crate in a row, or a new crate or program, is added to the table in the same change.
A library that is neither ours nor the core's is not the table's business.

### Rules

- **The writer's words are kept.** Opening, saving, importing and exporting never drop or
  rewrite what the writer typed, and nothing is "cleaned up" unasked. What proves it: every
  format fixture is saved again without loss, generated scripts go through `.wss` and Fountain
  and come back the same (`crates/script-format/tests/round_trips.rs`), every file 0.1.2 saved
  opens with the same pages, and the golden scripts' PDFs are read back word for word.
  **Today:** the imports from plain text, RTF and PDF and the exports to RTF, text and HTML
  have no word-for-word test
  ([issue 11](https://github.com/jamescjfoundation/wordsmith-script/issues/11)), and a Fountain
  import leaves out notes written inside the text
  ([issue 12](https://github.com/jamescjfoundation/wordsmith-script/issues/12)).
- **Golden files are the judge.** The golden scripts' PDF text, the pages 0.1.2 planned for
  every file it saved, and the guide's page of keys are compared with reviewed files
  (`docs/golden.md` lists them and their tests). A change that alters one is deliberate:
  rewrite it with `UPDATE_GOLDEN=1`, the one switch, read the difference, and add a row to
  `docs/golden.md` saying what changed and why.
- **Fixtures** are public-domain texts or written for the test. They live in `tests/fixtures/`
  and `tests/golden/` at the root, and each is listed in `docs/fixtures.md` with where it came
  from. A real person's script is never one (`CONTRIBUTING.md`).
- **Names.** Code, documents, commits and fixtures may name another company, program, service
  or file format where that is a fact a reader needs: what Script opens or writes, where a
  formatting rule comes from. Labels, feature names, templates and help text in the interface
  are our own, never another product's name or distinctive feature name, and Script is never
  described as replacing or following another product
  ([project decision 0003](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0003-naming-other-companies-programs-and-formats.md)).
  The clean-room rules still hold: `CONTRIBUTING.md`, "Clean room and names".
- **Brand.** The logos, the icons at every size and the palette card in `assets/brand/` are
  files of the Wordsmith brand kit (the `brand/` folder of the wordsmith repository), byte for
  byte. They are never edited, recolored, redrawn or resized here: a change starts in the kit,
  and the kit's tool writes its files into this folder
  ([Route F](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-f);
  [project decision 0007](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0007-brand-files-reach-the-apps-by-one-route.md)).
  - **To take the kit's files:** with the wordsmith repository checked out beside this one,
    `npm run brand -- --app script` in its `website/wiki`. It writes the files and
    `assets/brand/kit.json`, the record of the kit commit they were taken at and of what the
    folder holds; commit the result here, naming the wordsmith commit. The record is never
    edited by hand.
  - **Script's own files** in the folder are `script/tokens.json`, the interface theme in
    `jcj-theme`'s format, whose `palette` must hold the kit's seven colors, and `README.md`.
    Two are made here from the kit's artwork: `mark-mask.png` and `wordmark-mask.png`, the mark
    and the wordmark as white shapes the app tints. A new file of the app's own is named in the
    kit's list first (`brand/apps.json` there, by an issue in the wordsmith repository).
  - **`cargo xtask brand`**, in the gate, fails when a carried file is not the one the record
    names, when the folder holds a file the record does not list, and when the theme's palette
    is not the kit's. **`cargo xtask brand --kit`** asks the wordsmith repository whether the
    kit has moved since; the workflow `brand-kit.yml` runs it every week and is red until the
    files are taken again.
  - `ATTRIBUTION.md` lists every file there.
  - **Today:** no tool here makes the two masks
    ([issue 29](https://github.com/jamescjfoundation/wordsmith-script/issues/29)), and a red
    `brand-kit.yml` run opens no issue: someone has to look at the Actions page.
- **`.wss` changes** get a new format version and a migration step, and a fixture of the new
  version in `tests/fixtures/` (`v1-short.wss`, `v2-notes.wss`, `v3-revisions.wss`,
  `v4-styles.wss`, `v5-ext.wss`; a version with no fixture fails a test).
  Wordsmith Manuscript also reads `.wss` files, with a reader of its own that is tested against
  copies of the same fixtures and refuses a version it was not written for. So a format change
  is not finished here: the same change opens an issue in wordsmith-manuscript that names the
  new version, what changed in the text and its marks, and the fixture
  ([Route B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b)).
  Manuscript's reader is never edited from here. `docs/format.md`, "Other readers", has the
  steps.
- **Code that waits for the core.** `docs/core-candidates.md` lists every change Script waits
  for in the core and every `// core-candidate:` marker here, with its core issue and state. A
  marker is written `core-candidate: open-source-core#N` and names an open issue that names the
  file (`cargo xtask docs` fails on a marker with no issue). A marker, its row and its issue
  change together; when Script moves onto the core's code, the marker and the copy go and the
  row says "used".
- **A change users will see** updates the guide (`docs/guide/`) and `CHANGELOG.md` here, under
  `## Unreleased`; the wiki and the website follow from those
  ([Route C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c)).
  Every piece of work that lands on `main` adds a row to `docs/progress.md`.
- **What is planned is the issues**, under the milestone
  [`v1`](https://github.com/jamescjfoundation/wordsmith-script/milestone/1) (decision 0046).
  No file here holds a list of work to do: `ROADMAP.md` links to the milestone. Something
  missing or broken is an issue, in the milestone when 1.0.0 needs it; what waits for the core
  is a core issue with its row in `docs/core-candidates.md`. `docs/gaps.md` holds only the
  standing limits a writer should know, each with its reason, and changes by a decision. Close
  the issue in the change that does the work.
- **Versions** (decision 0047). The first public release is 0.9.0, then 1.0.0; nothing
  earlier was published. A release turns `## Unreleased` into `## <version> (<date>)` and is
  tagged `v<version>`. `docs/history/` and the changelog's "Before the first release" are kept
  as written: do not bring them up to date.
- New assets are listed in `ATTRIBUTION.md` (`cargo xtask assets`).
