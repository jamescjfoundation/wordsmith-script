# 0038: Releases, downloads and updates

Date: 2026-11-20

## Context

The owner asked us to handle downloads, releases and updates the way the
ArtCraft apps do. We reviewed WordCraft (`storytold/wordcraft`), craft-libs'
`craft-update` crate and the shared release workflow in
`storytold/craft-common-actions`. Here is what they do:

- **Releases:** pushing the `release` branch runs one shared workflow. It
  reads the version from `Cargo.toml` and checks that it is semver; a
  `-suffix` makes it a pre-release. It refuses a tag that is already published.
  It builds and signs each platform's packages, writes `SHA256SUMS.txt`, and
  puts everything on a draft GitHub release. A last step publishes the draft,
  unless a build was cancelled or the draft has fewer files than the last
  release.
- **Downloads:** GitHub Releases is the only place to download from. Files are
  named `<app>-<version>-<os>-<arch>[-portable].<ext>`, and the update picks
  its file by that name.
- **Updates:** `craft-update` checks the repository's published releases (with
  a fallback for when GitHub's request limit runs out). It downloads only from
  that repository and checks each file's SHA-256. It then installs the update
  the way the copy was installed: a DMG on macOS, an MSI or a portable zip on
  Windows, an AppImage, deb, rpm or Flatpak on Linux. Anything else, and any
  unsigned Windows copy, gets a link to the release page instead.

## Decision

- **We use `craft-update` for the work and draw the window ourselves.** It
  comes from craft-libs at our existing pin (`93dac78c`, the commit WordCraft
  pins), so we take no new source, and it is MIT OR Apache-2.0. Its ready-made
  dialogs (the `egui` feature) are built for egui 0.37, and we stay on 0.36.2
  (the kickoff's pin). So we leave that feature off and use only its core:
  `check_for_update`, `self_update::{current, is_self_updating,
  release_installs, prepare_update, take_outcome, finish}`.
- **The work and the window live in different crates.**
  - `wordsmith-ui::updates` holds only what to show and what the writer asked
    for. The interface crate has no network code, and the web build has
    nothing to run.
  - `apps/wordsmith/src/updates.rs` runs checks and installs on threads of
    their own. It also starts the restart helper after the app exits, once
    everything is saved.
- **Our identity, not ArtCraft's** (branding rule, `docs/REUSE.md`):
  - id `wordsmith-script` (the stem of every release file);
  - the repository `jamescjfoundation/wordsmith-script`;
  - bundle id `org.wordsmith-script.app`;
  - developer "the Wordsmith Script contributors";
  - user agent `WordsmithScript/<version>`, which says nothing about the
    writer.
- **What the writer sees:**
  - Help ▸ Check for Updates… (`help.checkForUpdates`) and an Updates section
    on the File page's About page.
  - A "Software Update" window that offers the update.
  - A preference, "Check for updates when the app starts", on by default as
    in the ArtCraft apps. A check at start that fails says nothing (the writer
    may be offline); a check the writer asked for always answers.
  - When this copy can install the update itself, the window offers "Install
    and Restart", then "Restart Now". Restarting asks about unsaved scripts as
    closing does. Otherwise the window offers the release page.
  - The next start reports what came of the update.
- **Release files** (`cargo xtask bundle`):

  | Platform | File |
  |---|---|
  | macOS | `wordsmith-script-<v>-macos-<arch>.dmg`: the app signed ad hoc, the licences and a link to Applications |
  | Windows | `wordsmith-script-<v>-windows-<x64\|arm64>-portable.zip` |
  | Linux | `wordsmith-script-<v>-linux-<arch>.tar.gz` |

- **The release workflow** (`.github/workflows/release.yml`) follows the
  ArtCraft pattern: version check, bundles on four runners (Apple silicon and
  Intel macOS), `SHA256SUMS.txt`, a draft release with generated notes, and a
  publish step with the file-count guard.
  - It does not call ArtCraft's shared workflow. That workflow is tracked at
    `@main` and would get our release token and signing secrets.
  - Publishing stays deliberate, as decision 0029 required. The update reads
    published releases only, and the workflow publishes only when started by
    hand on the `release` branch with "publish" ticked. Otherwise the owner
    publishes the draft on GitHub.

## What works without signing

| Platform | Update in place? | Why |
|---|---|---|
| macOS | Yes | craft-update checks the bundle's signature only when the release names a developer team. But the first download from a browser still meets Gatekeeper ("unidentified developer"), because the app is not notarized |
| Windows | No | craft-update refuses an unsigned copy, so the writer gets the release page |
| Linux tar.gz | Only when the folder can be written | Otherwise, the release page |

## Not done (see `docs/gaps.md`)

- Developer ID signing and notarization on macOS, and Authenticode on Windows.
  They need paid accounts, a stop rule.
- AppImage, deb, rpm and MSI packages; GPG-signed `SHA256SUMS.txt`.
- A universal macOS binary (we ship one DMG per architecture, which the update
  also reads).
- The ArtCraft nightly dry run of the release build.
- The update has not been run end to end against a real published release.
  That needs the first release on GitHub.

## Revisit when

- egui moves to 0.37: craft-update's own dialogs could replace ours.
- Signing accounts exist: add the identities to the workflow and pass the
  developer team, so the macOS update checks the signature.
