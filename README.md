# Wordsmith Script

An open-source, cross-platform screenwriting app written in Rust.

Status: early development toward v0.1. Nothing is released yet.

Wordsmith Script is based on WordCraft by the ArtCraft team.

## Building

Needs Rust 1.95 or newer (`rust-toolchain.toml` selects stable).

```sh
cargo run -p wordsmith
```

Before merging anything to `main`, run the full gate:

```sh
cargo xtask ci
```

It runs rustfmt, a crate-root lint check, clippy, all tests, the license and
asset attribution check, the layering check and the wasm build.

## Project docs

- `docs/SPEC.md`: the behavior spec, the only source for screenplay features.
- `docs/REUSE.md`: what we reuse and the license and branding rules.
- `docs/progress.md`: one line per merge to `main`.
- `docs/gaps.md`: spec items not done yet.
- `docs/decisions/`: choices the spec left open.
- `ROADMAP.md`: what is out of scope for v0.1.

## License

Licensed under either of Apache License, Version 2.0 (`LICENSE-APACHE`) or the
MIT license (`LICENSE-MIT`), at your option. See `NOTICE` and
`ATTRIBUTION.md` for third-party credits.
