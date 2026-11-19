# 0003: Toolchain, CI gate and lint policy

Date: 2026-11-19

## Decision

- **Toolchain.** `rust-version = "1.95"` in the workspace is the minimum.
  `rust-toolchain.toml` tracks `stable` with rustfmt, clippy and the
  `wasm32-unknown-unknown` target, so local runs and CI agree without
  pinning an exact release that would need upgrading by hand.
- **Gate.** `cargo xtask ci` has no dependencies of its own, so the gate
  never depends on a crate the gate itself has to vet. It shells out to
  `cargo fmt`, `cargo clippy`, `cargo test`, `cargo tree` and `cargo build`
  with `--locked`.
- **Lints.** The six panicking lints and `unsafe_code` are set once as
  workspace lints, and every crate root also states them as attributes so the
  rule is visible in the code. `cargo xtask lint-roots` checks the
  attributes; clippy enforces them. `clippy.toml` lets tests unwrap and
  panic.
- **wasm.** The gate builds `wordsmith-web` for `wasm32-unknown-unknown`.
  It is a compile check only in v0.1.
- **CI.** `.github/workflows/ci.yml` runs `cargo xtask ci` on Linux, macOS
  and Windows. It uses only GitHub's own actions. It has never run, because
  there is no remote yet.

## Revisit when

A dependency needs a newer compiler than 1.95 (raise `rust-version` on
purpose), or the CI workflow first runs on a real remote.
