# Notes for coding agents (and people)

Wordsmith Script is a screenwriting app. Its screenplay rules (elements, typing, pagination with
(MORE) and (CONT'D), Fountain, the `.wss` format, scene numbers, reports, the title page) live in
`crates/wordsmith-script` and `crates/wordsmith-format`; its interface in `crates/wordsmith-ui`;
the programs in `apps/`. Read `CONTRIBUTING.md`, `SECURITY.md` and `docs/decisions/` (decision
0044 says what the app takes from the core) before you start.

This app is built on [open-source-core](https://github.com/jamescjfoundation/open-source-core),
the J. C. J. Foundation's shared base. The core holds what every app needs; this repository holds
only what is about this app.

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
| [wordsmith](https://github.com/jamescjfoundation/wordsmith) | The project map and rules (`project/`), the brand kit (`brand/`), the website (`website/`), the wiki (`website/wiki/`), the launcher (planned) |
| [wordsmith-script](https://github.com/jamescjfoundation/wordsmith-script) | Wordsmith Script, the screenplay app, and the `.wss` file format |
| [wordsmith-manuscript](https://github.com/jamescjfoundation/wordsmith-manuscript) | Wordsmith Manuscript, the novel and long-form app, the `.wsm` project format, and the book view that Book reads |
| [wordsmith-book](https://github.com/jamescjfoundation/wordsmith-book) | Wordsmith Book, the book design and typesetting app, and the `.wsbk` project format |
| [open-source-core](https://github.com/jamescjfoundation/open-source-core) | The shared base: the `jcj-` crates and the settings and tooling every app carries. It is general and names no product |
| [wordcraft](https://github.com/jamescjfoundation/wordcraft), [craft-libs](https://github.com/jamescjfoundation/craft-libs) | Our forks of the upstream projects the core builds on |

- Each app builds on open-source-core at one release tag and carries its shared files (`cargo xtask sync`). Only the core reaches the forks.
- Wordsmith Book also builds on Wordsmith Manuscript's model and format crates, at a book-view tag. Wordsmith Manuscript reads Wordsmith Script's `.wss` files.
- Each app takes its logos, icons and palette from the brand kit, unchanged. The wiki and the website describe the apps from each app's guide, changelog and releases.

| The change | Where it goes | Route |
| --- | --- | --- |
| A fix in a `jcj-` crate, or something any app would share | open-source-core first, then the app's pin | [A](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-a) |
| Something one app needs from another app | An issue in the providing app's repository | [B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b) |
| A change users will see in an app | The app's `docs/guide/` and `CHANGELOG.md`, then the wiki and website | [C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c) |
| A wiki page | wordsmith, `website/wiki/content/docs/<app>/` | [D](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-d) |
| Website text, or an app's status on the site | wordsmith, `website/tools/build.py` | [E](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-e) |
| A logo, icon or brand color | wordsmith, `brand/source/`, then the copies | [F](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-f) |
| Screenshots | Drawn in the app, then copied to wordsmith | [G](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-g) |
| A security problem | Never public: `.private/` and a maintainer | [H](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-h) |
| A core release, or moving an app to a newer core | open-source-core, then each app | [I](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-i) |
| An app release | The app, then the website and wiki | [J](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-j) |
| A change in WordCraft or craft-libs | Our fork, then the core | [K](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-k) |
| A change that spans two repositories | The provider first; each change names the other | [L](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-l) |
| Working with all the repositories, or with only one | Side by side as sibling folders | [M](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-m) |
| A rule or instruction that several repositories carry | The block's owner: the core's `template/`, or the project map for this block | [N](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-n) |

Before changing a repository, read its own `AGENTS.md`. Its rules apply inside it; this block applies in all of them. The block is owned by the project map and copied from it by a tool: it is never edited in a copy.
<!-- END wordsmith project block -->

<!-- BEGIN open-source-core: app -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: `cargo xtask sync` rewrites the copies. -->
## Before you commit

```sh
cargo xtask install-hooks   # once: pre-commit (fmt, privacy) and pre-push (ci --changed)
cargo xtask ci --changed    # what CI will run for your change; `cargo xtask ci` for everything
cargo xtask sync            # after moving to a new core release: shared files, blocks and patch block
```

What the core owns is changed in the core, never here: `rust-toolchain.toml`, `rustfmt.toml`,
`clippy.toml`, `deny.toml`, `CODE_OF_CONDUCT.md`, the shared modules in `xtask/src/`,
`.github/workflows/ci.yml` and `core-pin.yml`, `hooks/`, the patch block in `Cargo.toml`, and the
blocks marked `open-source-core` in `AGENTS.md`, `SECURITY.md` and `CONTRIBUTING.md`.
`cargo xtask sync --check` fails if any of them differs from the pinned core release.
`xtask/src/main.rs`, any module an app adds beside it, and everything outside the marked blocks
are the app's own.

## House rules for code

- Every crate root: an SPDX line, `#![forbid(unsafe_code)]`, and `#![deny(...)]` of the six
  panicking lints. Product code returns errors; it never unwraps or panics.
- Depend only on the core's `jcj-` crates and, under a contract, another app's crates
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md));
  never on `jcj-upstream`, `wordcraft-*` or `craft-*` (`cargo xtask layers`).
- The app's brand (name, colors as `tokens.json`, fonts, logo) lives in the app's repository or
  in its project's brand kit, never in the core.
<!-- END open-source-core: app -->

<!-- BEGIN open-source-core: public -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: `cargo xtask sync` rewrites the copies. -->
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
<!-- END open-source-core: public -->

<!-- BEGIN open-source-core: app-and-core -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: `cargo xtask sync` rewrites the copies. -->
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
   undo, format import and export, PDF and printing, cards and boards, theming, agent control: the
   core.
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
  while no app has a public release, a push to `main` is allowed once the gate passes; a pull
  request is optional and its author merges it when its checks pass.
- **App-only change:** one change in the app.
- **Core change an app needs:**
  1. Open a core issue describing the general need, not the app's feature.
  2. Check out both side by side. In the app: `cargo xtask dev-link ../open-source-core`
     (`cargo xtask dev-unlink` removes it). Never commit the link; the app's CI rejects it.
  3. Build the general part in the core with its own tests and a use in `apps/core-testbed`, so it
     is proven without any product.
  4. Build the app's use of it in the app.
  5. Land the core change first, with the core's gate green. Then release the core.
  6. Move the app's pin to that release (`cargo xtask sync`), then land the app change, which
     names the core issue it needed.
- **Prototype first, promote later.** If the general shape is not clear yet, build it in the app,
  mark it `// core-candidate:` and open a core issue labelled `core-candidate`. Move it once it works
  and a second app wants it. Never copy code between app repositories.
- **Something one app needs from another app**
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md)):
  open an issue in the providing app's repository. The provider's change lands and is tagged
  there first; the consumer then moves its pin. Never edit the provider as part of the consumer's
  change.

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

### Things agents must not do

- Name WordCraft or craft-libs types in an app repository. Use the `jcj-` interfaces; if one is
  missing, add it to the core.
- Put a product name, color, logo, or screenplay, manuscript or book concept into the core.
- Change a core interface from a change made in an app repository.
- Copy code from one app repository into another.
- Release the core, or move an upstream pin, on a commit whose full gate has not passed. A
  person or an agent may do either once it has (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
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

- **Golden files are the judge.** `cargo test -p wordsmith-format --test golden` compares the
  PDF text of three public-domain photoplays with reviewed files, and `--test migration` checks
  that every file 0.1.2 saved opens and paginates as 0.1.2 paginated it. A change that alters
  them is deliberate, reviewed and recorded (`docs/golden.md`).
- **Security problems you find**: do not describe one in a commit, branch, comment, issue or pull
  request. Write it to `.private/` (ignored by Git) and tell the user; fix it on a neutrally
  named branch with a test; disclosure waits for a release.
- **Names in the interface are our own.** Never use another product's name or feature names
  (`docs/SPEC.md`, rule 2).
- **`.wss` changes** get a new format version and a migration step, and a fixture of the new
  version (`docs/format.md`).
- New assets are listed in `ATTRIBUTION.md` (`cargo xtask assets`).
