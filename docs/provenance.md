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
| 2026-11-19 | Web search (two queries) for public-domain screenplay sources, and https://www.gutenberg.org book pages for #17903, #76829 and #70937 (gutenberg.org itself was down for maintenance) | Identified three photoplay scenarios printed in the US before 1931 (docs/golden.md) |
| 2026-11-19 | https://gutenberg.pglaf.org (official Project Gutenberg mirror): `1/7/9/0/17903/17903.txt` (SHA-256 `c0f571a2…a1`), `7/6/8/2/76829/76829-0.txt` (`efd10e39…a4`), `7/0/9/3/70937/70937-0.txt` (`c6bf6c2f…aa`) | The three scenarios' text, transcribed into Fountain for the golden tests; books' commentary not used |
| 2026-11-19 | https://fountain.io/syntax (Fountain 1.1) | Element recognition rules for the Fountain reader in `wordsmith-format` |
| 2026-11-19 | WordCraft `crates/pdf/src/tests.rs` (already copied) | The hayro-based PDF text extractor, adapted for the PDF and golden tests |
