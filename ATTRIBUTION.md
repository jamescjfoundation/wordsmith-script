# Attribution

Every asset in this repository that we did not write, and every block of
copied code, is listed here with its source and license. `cargo xtask
attribution` fails if a file under `assets/` is not named in this file, or if
any dependency's license is outside the allowlist in `docs/REUSE.md` rule 5
(see `docs/decisions/0001-license-allowlist.md`).

Add the entry in the same commit as the asset.

## Assets

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| _none yet_ | | | | |

## Copied code

| Our crate | Upstream | License | Notes |
| --- | --- | --- | --- |
| _none yet_ | | | Copied WordCraft crates are added in the bootstrap branch and tracked in `UPSTREAM.md`. |

## Dependencies

Rust crate dependencies are not copied into the repository. Their licenses
are checked on every CI run by `cargo xtask attribution`, against the pinned
`Cargo.lock`.
