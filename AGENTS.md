# Notes for coding agents (and people)

This app is built on [open-source-core](https://github.com/jamescjfoundation/open-source-core),
the J. C. J. Foundation's shared base. The core holds what every app needs; this repository holds
only what is about this app. Read the core's AGENTS.md too: the rules below are its rules, which
every app carries.

## Before you commit

```sh
cargo xtask install-hooks   # once: pre-commit (fmt, privacy) and pre-push (ci --changed)
cargo xtask ci --changed    # what CI will run for your change; `cargo xtask ci` for everything
cargo xtask sync            # after moving to a new core release: shared settings and patch block
```

Files marked as owned by the core (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
`deny.toml`, `xtask/src/` except `main.rs`, `.github/workflows/ci.yml`, `hooks/`, and the patch
block in `Cargo.toml`) are changed in the core, never here; `cargo xtask sync --check` fails if
they differ from the pinned core release.

## What is public

This repository is public, history included, forever. Before you write a commit, branch name,
issue, pull request or comment, check it holds none of:

- anyone's name, email, account names, or details of their machine (local paths such as a home
  folder, user names, host names, IP addresses);
- what the user told you in conversation, quoted or summarised; write the reason for a change as
  its own argument;
- an open security problem (follow `SECURITY.md`: keep it in `.private/` and tell the user);
- secrets, or files from a real machine (preferences, recovery, auto-save, backups, logs,
  screenshots of a desktop);
- anyone's private writing, even as test data. Fixtures are public-domain or written for the test.

Commit with a GitHub no-reply address or a foundation bot identity (`cargo xtask privacy` checks
every commit). Never push with `--all` or `--mirror`.

## Building between an app and the core

### Step 1: decide where the change belongs

Ask in order; the first "yes" decides.

1. **Does it change how we use WordCraft, craft-libs or another upstream project?** The core's
   adapters (`jcj-upstream` and the `jcj-` crate that wraps it). If upstream code itself must
   change, it goes in our fork first (the core's `docs/UPSTREAM.md` and decision 0004).
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
| Colors, fonts, logo | The app's own repository | The core holds theming code, never a brand |

### Step 2: build it

- **App-only change:** one pull request in the app.
- **Core change an app needs:**
  1. Open a core issue describing the general need, not the app's feature.
  2. Check out both side by side. In the app: `cargo xtask dev-link ../open-source-core`
     (`cargo xtask dev-unlink` removes it). Never commit the link; the app's CI rejects it.
  3. Build the general part in the core with its own tests and a use in `apps/core-testbed`, so it
     is proven without any product.
  4. Build the app's use of it in the app.
  5. Open two linked pull requests: the core one first, then the app one, which says which core
     pull request it needs.
  6. Merge the core pull request, release the core, update the app's pin, merge the app.
- **Prototype first, promote later.** If the general shape is not clear yet, build it in the app,
  mark it `// core-candidate:` and open a core issue labelled `core-candidate`. Move it once it works
  and a second app wants it. Never copy code between app repositories.

### Step 3: let it reach the other apps safely

- Everything in the core is shared by default.
- A change that alters what users see in other apps goes behind an option (a shell setting or a
  Cargo feature) that defaults to the old behavior; each app turns it on when ready.
- Versioning (decision 0011): while at 0.x a patch release only adds or fixes; a minor release may
  break and its changelog says exactly what each app must do. A breaking change to a public `jcj-`
  interface says so in the pull request title, and every app is updated in follow-up pull requests.
- A core change that breaks another app is fixed in the core, not worked around in the app.
- Apps stay at most one core release behind.

### Things agents must not do

- Name WordCraft or craft-libs types in an app repository. Use the `jcj-` interfaces; if one is
  missing, add it to the core.
- Put a product name, color, logo, or screenplay, manuscript or book concept into the core.
- Change a core interface from an app repository's pull request.
- Copy code from one app repository into another.
- Release the core, move upstream pins, or merge pull requests without a person's approval.
- Edit WordCraft or craft-libs code anywhere but our forks' topic branches.
- Commit personal details, private files or secrets.

### Where each kind of record goes

| Record | Lives in |
| --- | --- |
| Decisions about shared interfaces, upstream, tooling | The core's `docs/decisions/` |
| Decisions about this app's behavior | `docs/decisions/` here |
| Pins, carried patches, follow / hold / own | The core's `docs/UPSTREAM.md` |
| What changed for apps in each core release | The core's `CHANGELOG.md`, with "What apps must do" |
| Ideas waiting to be promoted to the core | Core issues labelled `core-candidate` |

## House rules for code

- Every crate root: an SPDX line, `#![forbid(unsafe_code)]`, and `#![deny(...)]` of the six
  panicking lints. Product code returns errors; it never unwraps or panics.
- Depend only on the core's `jcj-` crates; never on `jcj-upstream`, `wordcraft-*` or `craft-*`
  (`cargo xtask layers`).
- The app's brand (name, colors as `tokens.json`, fonts, logo) lives here, never in the core.
