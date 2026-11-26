# 0003: Toolchain, CI gate and lint policy

Date: 2026-11-19 · Status: replaced by
[core decision 0003](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0003-toolchain-gate-and-ci.md)

Since decision 0044 (2026-11-24) the toolchain, the gate and the CI workflow
are the core's, carried here by `cargo xtask sync`.

## Decision

- **Toolchain.** `rust-version = "1.95"` in the workspace is the minimum.
  `rust-toolchain.toml` names rustfmt, clippy and the
  `wasm32-unknown-unknown` target, so local runs and CI agree. It first
  tracked `stable`; decision 0040 pins an exact release instead, and CI
  checks the minimum version.
- **Gate.** `cargo xtask ci` has no dependencies of its own, so the gate
  never depends on a crate the gate itself has to vet. It shells out to
  `cargo fmt`, `cargo clippy`, `cargo test`, `cargo tree` and `cargo build`
  with `--locked`.
- **Lints.** The six panicking lints and `unsafe_code` are set once as
  workspace lints, and every crate root also states them as attributes so the
  rule is visible in the code. Since the WordCraft copy (0005), the six
  panicking lints live only in the crate-root attributes. `cargo xtask
  lint-roots` checks the attributes; clippy enforces them. `clippy.toml` lets tests unwrap and
  panic.
- **wasm.** The gate builds `wordsmith-web` for `wasm32-unknown-unknown`.
  It is a compile check only in v0.1.
- **CI.** `.github/workflows/ci.yml` runs `cargo xtask ci` on Linux, macOS
  and Windows. It uses only GitHub's own actions. Decision 0040 adds
  cargo-deny, the minimum-version build and SHA-pinned actions.

## Revisit when

A dependency needs a newer compiler than 1.95 (raise `rust-version` on
purpose).
