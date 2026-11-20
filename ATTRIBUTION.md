# Attribution

Every asset in this repository that we did not write, and every block of
copied code, is listed here with its source and license. `cargo xtask
attribution` fails if a file under `assets/` is not named in this file, or if
any dependency's license is outside the allowlist in `docs/REUSE.md` rule 5
(see `docs/decisions/0001-license-allowlist.md`).

Add the entry in the same commit as the asset.

## Assets

The fonts are embedded by `wordsmith-fonts` as its bundled faces. Courier Prime is the
script font; the others come with the copied WordCraft font code.

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
| `assets/fonts/CourierPrime-Regular.ttf` | Courier Prime Regular, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-Bold.ttf` | Courier Prime Bold, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-Italic.ttf` | Courier Prime Italic, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-BoldItalic.ttf` | Courier Prime BoldItalic, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/OFL-CourierPrime.txt` | License text for Courier Prime | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (license text) | 2026-11-19 |

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
| wordsmith-control-key | WordCraft `crates/control-key` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Renamed (decision 0007) |
| wordsmith-mcp | WordCraft `crates/mcp` at 6ef13b2 | MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team and the WordCraft contributors | Trimmed and rebranded (decision 0007) |

## Test data

| Path | What it is | Source | License |
| --- | --- | --- | --- |
| `crates/wordsmith-format/tests/golden/everybodys-girl.fountain` | "Everybody's Girl" by A. Van Buren Powell (Vitagraph, 1918), transcribed into Fountain | *Writing the Photoplay* (1919), Project Gutenberg #17903 | public domain in the USA (published before 1931) |
| `crates/wordsmith-format/tests/golden/timid-teddy.fountain` | "Timid Teddy" by Embrie Zuver, transcribed into Fountain | *How to Write Photoplays* (1915), Project Gutenberg #76829 | public domain in the USA (published before 1931) |
| `crates/wordsmith-format/tests/golden/salt-of-vengeance.fountain` | "The Salt of Vengeance" by Henry Albert Phillips, transcribed into Fountain | *The Photodrama* (1914), Project Gutenberg #70937 | public domain in the USA (published before 1931) |
| `docs/images/*.png` | Pictures of the app drawn by its own offscreen renderer (`cargo xtask screenshots`), showing "Everybody's Girl" above, in the bundled fonts | made by this project from the public-domain text above | MIT OR Apache-2.0, like the code; the script text is public domain |

These are test data and are not shipped with the app. See `docs/golden.md`.

## Dependencies

Rust crate dependencies are not copied into the repository. Their licenses
are checked on every CI run by `cargo xtask attribution`, against the pinned
`Cargo.lock`.
