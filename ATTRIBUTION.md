# Attribution

Every asset in this repository that we did not write is listed here with its
source and license. `cargo xtask assets` fails if a file under `assets/` is
not named in this file, and `cargo xtask licenses` if any dependency's license
is outside the allowlist (open-source-core's decision 0001).

Add the entry in the same commit as the asset.

## Assets

Courier Prime is the script font: `wordsmith-script-model` adds it to the core's font
set. Inter and JetBrains Mono are the interface fonts: `wordsmith-script-ui` gives
them to the core's shell. Source Sans 3 and Source Serif 4 come with the
core's bundled fonts (WordCraft's) and are in the app; only their license
texts are kept here, for the release packages' `licenses` folder.

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| `assets/fonts/Inter-Medium.ttf` | Inter Medium, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/Inter-Regular.ttf` | Inter Regular, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/Inter-SemiBold.ttf` | Inter SemiBold, by the Inter Project Authors | https://github.com/rsms/inter; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | 2026-11-19 |
| `assets/fonts/JetBrainsMono-Regular.ttf` | JetBrains Mono Regular, by the JetBrains Mono Project Authors | https://github.com/JetBrains/JetBrainsMono; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (`assets/fonts/OFL-JetBrainsMono.txt`) | 2026-11-19 |
| `assets/fonts/OFL-Inter.txt` | License text for Inter | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-JetBrainsMono.txt` | License text for JetBrains Mono | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-SourceSans3.txt` | License text for Source Sans 3 | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/OFL-SourceSerif4.txt` | License text for Source Serif 4 | upstream font project; copied from WordCraft `assets/fonts/` at 6ef13b2 | OFL-1.1 (license text) | 2026-11-19 |
| `assets/fonts/CourierPrime-Regular.ttf` | Courier Prime Regular, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-Bold.ttf` | Courier Prime Bold, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-Italic.ttf` | Courier Prime Italic, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/CourierPrime-BoldItalic.ttf` | Courier Prime BoldItalic, Copyright 2015 The Courier Prime Project Authors; designer Alan Dague-Greene | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (`assets/fonts/OFL-CourierPrime.txt`) | 2026-11-19 |
| `assets/fonts/OFL-CourierPrime.txt` | License text for Courier Prime | https://github.com/google/fonts/tree/cc5661d43cd4f9aad5567033a8e15ca28dc35c86/ofl/courierprime (upstream project https://github.com/quoteunquoteapps/CourierPrime) | OFL-1.1 (license text) | 2026-11-19 |

## Brand

The Wordsmith Script mark, wordmark and app icon, and the parent brand's
files, are the project's own artwork (`assets/brand/README.md` says what each
is for). The logos, icons and palette card are the Wordsmith brand kit's files
(the `brand/` folder of the wordsmith repository), written here unchanged by
the kit's tool; the kit's `brand/LICENSE.md` says how the names and logos may
be used. `assets/brand/kit.json` records the kit commit they were taken at,
and `cargo xtask brand` fails when one differs from that record.
`wordsmith-script-ui` (`src/brand.rs`) draws the mark and wordmark from the
masks and sets the window icon; `cargo xtask package` puts the icons in the
macOS and Linux packages. The masks are rendered here from the kit's
artwork.

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| `assets/brand/README.md` | What the brand files are for, and the rules for using them | Wordsmith Script contributors | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/kit.json` | The record the brand kit's tool writes: the kit commit the brand files were taken at, and what the folder holds | Written by the Wordsmith brand kit's tool (`brand/` in the wordsmith repository) | MIT OR Apache-2.0 | 2026-11-27 |
| `assets/brand/favicon.svg` | The app icon tuned for small sizes: the mark in Page White and Brad Brass on an Iron Gall Ink tile | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-1024.png` | The app icon at 1024 px (macOS and Linux packages) | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-512.png` | The app icon at 512 px (macOS and Linux packages), from the brand kit | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-256.png` | The app icon at 256 px (the window icon; macOS and Linux packages), from the brand kit | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-mark-ink.svg` | The mark: an S and a brass caret bar, in Iron Gall Ink | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-mark-paper.svg` | The mark, in Page White, for dark backgrounds | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-script-wordmark-ink.svg` | The wordmark, in Iron Gall Ink with the brass caret bar | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-script-wordmark-paper.svg` | The wordmark, in Page White, for dark backgrounds | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-script-stacked-ink.svg` | The name on two lines, in Iron Gall Ink | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-script-stacked-paper.svg` | The name on two lines, in Page White | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-script-palette.svg` | The app's colors, as a swatch card | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-icon.svg` | The parent brand's icon | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-mark-ink.svg` | The parent brand's mark, in Iron Gall Ink | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-mark-paper.svg` | The parent brand's mark, in Page White | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-imprint-ink.svg` | The parent brand's name on two lines, in Iron Gall Ink | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-imprint-paper.svg` | The parent brand's name on two lines, in Page White | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-family-ink.svg` | The parent brand above the three app icons, on a light background | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-family-paper.svg` | The parent brand above the three app icons, on a dark background | The Wordsmith brand kit (`brand/` in the wordsmith repository) | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/mark-mask.png` | The mark's S alone, white, for the app to tint (rendered from `wordsmith-mark-ink.svg`) | Made here from the brand kit's artwork | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordmark-mask.png` | The wordmark's letters alone, white, for the app to tint (rendered from `wordsmith-script-wordmark-ink.svg`) | Made here from the brand kit's artwork | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/script/tokens.json` | The interface's design tokens (colors in three themes, fonts, sizes), in the shape of open-source-core's `jcj-theme` | Wordsmith Script contributors | MIT OR Apache-2.0 | 2026-11-24 |

## Copied code

None: WordCraft's code is used through open-source-core, not copied
(`UPSTREAM.md`, decision 0044). The first version of the app (0.1.x, never
published) carried copies of WordCraft's crates. Those copies are not in this
repository and their history was not kept here (`UPSTREAM.md`, "Before the
rebuild", names the upstream commits they came from).

## Test data

| Path | What it is | Source | License |
| --- | --- | --- | --- |
| `tests/golden/everybodys-girl.fountain` | "Everybody's Girl" by A. Van Buren Powell (Vitagraph, 1918), transcribed into Fountain | *Writing the Photoplay* (1919), Project Gutenberg #17903 | public domain in the USA (published before 1931) |
| `tests/golden/timid-teddy.fountain` | "Timid Teddy" by Embrie Zuver, transcribed into Fountain | *How to Write Photoplays* (1915), Project Gutenberg #76829 | public domain in the USA (published before 1931) |
| `tests/golden/salt-of-vengeance.fountain` | "The Salt of Vengeance" by Henry Albert Phillips, transcribed into Fountain | *The Photodrama* (1914), Project Gutenberg #70937 | public domain in the USA (published before 1931) |
| `docs/images/*.png` | Pictures of the app drawn by its own offscreen renderer (`cargo xtask screenshots`), showing "Everybody's Girl" above, in the bundled fonts | made by this project from the public-domain text above | MIT OR Apache-2.0, like the code; the script text is public domain |

These are test data and are not shipped with the app. See `docs/golden.md`.

## Dependencies

Rust crate dependencies are not copied into the repository. Their licenses
are checked on every CI run by `cargo xtask licenses` and by cargo-deny
(`deny.toml`), against the pinned `Cargo.lock`. Every release package carries
their license texts in `THIRD-PARTY-LICENSES.txt`, made by
`cargo xtask third-party-licenses` (decision 0040), with the fonts'
`OFL-*.txt` files in the `licenses` folder beside it.
