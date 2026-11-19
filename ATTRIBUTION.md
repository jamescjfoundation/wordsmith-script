# Attribution

Every asset in this repository that we did not write, and every block of
copied code, is listed here with its source and license. `cargo xtask
attribution` fails if a file under `assets/` is not named in this file, or if
any dependency's license is outside the allowlist in `docs/REUSE.md` rule 5
(see `docs/decisions/0001-license-allowlist.md`).

Add the entry in the same commit as the asset.

## Assets

The fonts are embedded by `wordsmith-fonts` as its bundled faces. They are not the
script font; Courier Prime is added in M2.

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| `assets/fonts/Inter-Medium.ttf` | Inter Medium, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/Inter-Regular.ttf` | Inter Regular, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/Inter-SemiBold.ttf` | Inter SemiBold, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/JetBrainsMono-Regular.ttf` | JetBrains Mono Regular, by the JetBrains Mono Project Authors | https://github.com/JetBrains/JetBrainsMono; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-JetBrainsMono.txt`) | 2026-11-19 |
| `assets/fonts/SourceSans3-Bold.ttf` | Source Sans 3 Bold, Adobe | https://github.com/adobe-fonts/source-sans; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | 2026-11-19 |
| `assets/fonts/SourceSans3-It.ttf` | Source Sans 3 It, Adobe | https://github.com/adobe-fonts/source-sans; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | 2026-11-19 |
| `assets/fonts/SourceSans3-Regular.ttf` | Source Sans 3 Regular, Adobe | https://github.com/adobe-fonts/source-sans; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | 2026-11-19 |
| `assets/fonts/SourceSans3-Semibold.ttf` | Source Sans 3 Semibold, Adobe | https://github.com/adobe-fonts/source-sans; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | 2026-11-19 |
| `assets/fonts/SourceSerif4-Bold.ttf` | Source Serif 4 Bold, Adobe | https://github.com/adobe-fonts/source-serif; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | 2026-11-19 |
| `assets/fonts/SourceSerif4-BoldIt.ttf` | Source Serif 4 BoldIt, Adobe | https://github.com/adobe-fonts/source-serif; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | 2026-11-19 |
| `assets/fonts/SourceSerif4-It.ttf` | Source Serif 4 It, Adobe | https://github.com/adobe-fonts/source-serif; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | 2026-11-19 |
| `assets/fonts/SourceSerif4-Regular.ttf` | Source Serif 4 Regular, Adobe | https://github.com/adobe-fonts/source-serif; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | 2026-11-19 |
| `assets/fonts/SourceSerif4-Semibold.ttf` | Source Serif 4 Semibold, Adobe | https://github.com/adobe-fonts/source-serif; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | 2026-11-19 |
| `assets/fonts/OFL-Inter.txt` | License text for Inter | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-JetBrainsMono.txt` | License text for JetBrains Mono | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-SourceSans3.txt` | License text for Source Sans 3 | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-SourceSerif4.txt` | License text for Source Serif 4 | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |

## Copied code

| Our crate | Upstream | License | Notes |
| --- | --- | --- | --- |
| wordsmith-geom | WordCraft `crates/geom` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-doc | WordCraft `crates/doc` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-fonts | WordCraft `crates/fonts` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-layout | WordCraft `crates/layout` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-render | WordCraft `crates/render` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-pdf | WordCraft `crates/pdf` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed; changed files carry "Modified by the Wordsmith Script contributors" |
| wordsmith-engine | WordCraft `crates/engine` at 6ef13b2 (session, undo, command registry, recovery, file access) | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Trimmed (decision 0006); every file carries "Modified by the Wordsmith Script contributors" except our new `tests.rs` |

## Dependencies

Rust crate dependencies are not copied into the repository. Their licenses
are checked on every CI run by `cargo xtask attribution`, against the pinned
`Cargo.lock`.
