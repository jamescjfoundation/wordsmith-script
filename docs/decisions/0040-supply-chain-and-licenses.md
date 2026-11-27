# 0040: Supply chain, release provenance and license texts in bundles

Date: 2026-11-22 · Status: general part moved to the core
([core decision 0001](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0001-licenses-and-supply-chain.md)
and
[core decision 0003](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0003-toolchain-gate-and-ci.md))

Since decision 0044 (2026-11-24) the supply-chain rules (toolchain, minimum
versions, cargo-deny, pinned actions) are the core's, carried here by `cargo
xtask sync`. Still Script's: build provenance, license texts in bundles and the
privacy disclosure.

Amended 2026-11-27: build provenance and the license texts in a package are
the core's too
([core decision 0019](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0019-app-releases.md)).
The attestation is signed by the core's shared workflow, so the check names it:
`gh attestation verify <file> --repo jamescjfoundation/wordsmith-script
--signer-repo jamescjfoundation/open-source-core`. `cargo xtask package` puts
`THIRD-PARTY-LICENSES.txt` beside the license files of every package and the
fonts' license texts under `licenses/` there; `cargo xtask bundle` is gone.
Still Script's: the privacy disclosure.

## Context

The repository is public and ships binaries. Several gaps remained:

- **Toolchain.** The toolchain tracked `stable`, so two builds of the same
  commit could use different compilers. The minimum version, 1.95, was declared
  but never built.
- **Advisories.** Dependency licenses were checked (`cargo xtask attribution`),
  but nothing checked security advisories or where crates come from, and
  nothing proposed updates.
- **CI.** Workflows used actions by tag, which the action's owner can move.
  The CI checkout kept its token in the working copy.
- **Releases.** `SHA256SUMS.txt` sits on the same release as the files it
  covers, so it shows only that the files were not damaged, not where they came
  from.
- **License texts.** Many dependency licenses (MIT, BSD, ISC, Unicode-3.0) ask
  for their notice in binary copies too. The bundles carried only our own
  licenses, and not the fonts' OFL texts.
- **Disclosure.** The update check at start was not described anywhere a user
  would look.

## Decision

- **Exact toolchain.** `rust-toolchain.toml` names an exact release (1.100.0
  at the time of writing), with rustfmt, clippy and the wasm target. Raising
  it is a change of its own, so a release can be rebuilt with the compiler
  that made it. This replaces the "track stable" choice in decision 0003.
- **Minimum version.** A CI job runs `cargo +1.95.0 check --workspace
  --all-targets --locked`. `rust-version` stays 1.95 because that builds. It
  is a check, not a test run; the tests run on the pinned toolchain.
- **wasm once.** The wasm build is the same on every host, so
  `cargo xtask ci --skip wasm` leaves it out on macOS and Windows, in CI and in
  the release workflow. `cargo xtask ci` with no flag still runs everything
  locally.
- **cargo-deny.** `deny.toml` checks:
  - licenses, against the same allowlist and reviewed exceptions as decision
    0001 (change both together);
  - RustSec advisories, where vulnerabilities, unsound and unmaintained crates
    all fail, and yanked versions too;
  - wildcard requirements;
  - sources: crates.io, plus github.com/storytold/craft-libs at an exact
    `rev`.

  CI installs a pinned version (`cargo install --locked cargo-deny --version
  0.20.2`, MIT OR Apache-2.0) and caches the binary. A GitHub action would be
  quicker, but CI uses only GitHub's own actions. Each ignored advisory has a
  reason in `deny.toml` and an entry in `docs/gaps.md`. The first run found
  four "unmaintained" notices and no vulnerabilities: chrono (also used by
  craft-libs), and zbus_names, zvariant and zvariant_derive, which egui 0.36's
  Linux accessibility brings in. The copied crates now also say
  `publish.workspace = true`; cargo-deny counted them as publishable.
- **Dependabot.** `.github/dependabot.yml` proposes weekly updates for Cargo
  and GitHub Actions. Compatible crate updates come as one pull request, with
  at most three open. It leaves alone what is pinned on purpose: craft-libs,
  krilla and the hayro crates (one shared revision, UPSTREAM.md), and egui and
  eframe beyond 0.36 (decision 0038).
  *Amended 2026-11-26:* from open-source-core 0.1.19 the settings file is the
  core's, carried by `cargo xtask sync`: weekly updates for GitHub Actions
  only. Cargo updates for the shared crates are decided in the core; a crate
  only Script uses moves when its lock file is refreshed.
- **Actions pinned by commit.** Every action is GitHub's own and is named by its
  full commit SHA, with the version in a comment, so Dependabot can update both.
  The CI checkout sets `persist-credentials: false`.
- **Build provenance.** In the release workflow, a separate `attest` job runs
  `actions/attest-build-provenance` over the release files. Only that job gets
  `id-token: write` and `attestations: write`, and it cannot write to the
  repository. The draft release waits for it. Anyone can check a download with
  `gh attestation verify <file> --repo jamescjfoundation/wordsmith-script`
  (README, "Checking a download"). Attestations of a public repository are
  recorded in the public Sigstore transparency log. The in-app update still
  checks SHA-256 only (`docs/gaps.md`).
- **Third-party license texts in every bundle.**
  - `cargo xtask third-party-licenses` writes `THIRD-PARTY-LICENSES.txt`. It
    lists every package in the locked dependency graph of `wordsmith` and
    `wordsmith-cli`, for every target, with normal and build dependencies.
    Each entry gives the package's license expression and repository,
    followed by the license and notice files from its source, each distinct
    text once.
  - The work is done in xtask, with a small JSON reader for `cargo metadata`,
    so the gate keeps no dependencies (decision 0003). cargo-about would have
    been a second tool to pin and vet.
  - A package in a git checkout without its own files uses the checkout's
    license (craft-libs keeps it at the top).
  - A registry package that ships no license file (77 of 499 at the time of
    writing) gets the standard text of its license, labelled as such, with the
    authors its manifest names. That text is Apache-2.0 when offered,
    otherwise MIT, BSD-3-Clause or BSL-1.0. Any other license with no file
    stops the bundle until someone adds its text.
  - `cargo xtask bundle` puts the file and `assets/fonts/OFL-*.txt` in a
    `licenses/` folder in every bundle. On macOS they also go inside the app
    (`Contents/Resources`), so they stay with it once it is copied out of the
    disk image. The bundle checks they are all present before packing and
    stops if any is missing.
  - The About text says where the files are, and names all five bundled
    fonts.
- **Privacy disclosure.** The README and the guide (the window, "Updates and
  privacy") say what the update check sends and how to turn it off:
  - It sends one request to `api.github.com` for the repository's releases,
    with the user agent `WordsmithScript/<version>`. GitHub sees the IP
    address.
  - craft-update has a route through github.com pages for when the API's
    request limit is used up, but that route is part of its `GitHubSource`,
    which we do not use. Our `check_for_update` call reports the limit as a
    failure and asks no other server.
  - Downloads happen only when the writer chooses to install, from the
    release on github.com.
  - There is no telemetry, and script content never leaves the computer.
  - The last card of the first-run tour says the check exists and that
    Preferences › Starting turns it off. The check stays on by default, as
    decision 0038 chose.

## Revisit when

- A new Rust release is wanted: raise `rust-toolchain.toml` and the CI cache
  key follows.
- A dependency needs a newer compiler than 1.95: raise `rust-version` and
  `MSRV` in `ci.yml` together.
- chrono is replaced, or egui moves past 0.36: drop the matching advisory
  ignores.
- The in-app update can verify attestations or signatures: make it do so.
