# Provenance

Every source consulted beyond `docs/SPEC.md` and `docs/REUSE.md`: where, when
and what we took from it. Add a row whenever you open something new.

| Date | Source | What we took |
| --- | --- | --- |
| 2026-11-19 | Rust toolchain license texts shipped with rustup (`share/doc/rust/licenses/Apache-2.0.txt`, `MIT.txt`) | Verbatim license texts for `LICENSE-APACHE` and `LICENSE-MIT` |
| 2026-11-19 | https://github.com/storytold/wordcraft at `6ef13b2` (git clone, read-only, outside the project) | Copied crates geom, doc, fonts, layout, render, pdf; bundled OFL fonts; `Cargo.lock` as the starting lock; rustfmt and clippy settings; dependency versions from its `Cargo.toml` |
| 2026-11-19 | https://github.com/storytold/craft-libs at `bc186ca` (git clone, read-only) | Checked that the `93dac78c` pin exists and that no `v0.1.0` tag exists; read `CHANGELOG.md` for the v0.1.0 crate list. Its crates are used as pinned git dependencies, not copied |
| 2026-11-19 | https://github.com/storytold/craft-i18n at `e56d262` and https://github.com/storytold/craft-fonts at `b076417` (git clone, read-only) | Used only to confirm that four ignored tests pass when these build inputs are present. Nothing copied |
| 2026-11-19 | https://github.com/google/fonts at `cc5661d43cd4f9aad5567033a8e15ca28dc35c86`, folder `ofl/courierprime` (downloaded with curl from raw.githubusercontent.com) | Courier Prime Regular, Bold, Italic and Bold Italic TTFs and `OFL.txt`, unmodified. SHA-256: Regular `72f79337…78af`, Bold `ff1f3878…3638`, Italic `f1b9a582…006b`, BoldItalic `3355273f…7dcc`, OFL `9a755af0…97b0` |
