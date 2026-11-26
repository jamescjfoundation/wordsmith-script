# Fixtures

Every file the tests read from `tests/fixtures/` and `tests/golden/` is a
public-domain text or was written for the test. This page lists each with
where it came from. A real person's script or notes is never a fixture
(`CONTRIBUTING.md`, "Scripts and the app's own privacy"); try one from
`.private/`, which Git ignores.

A new fixture gets its row here in the change that adds it.

## One script per format version (`tests/fixtures/*.wss`)

Small scripts written for the tests, one for each version of the `.wss`
format, each using what its version added (`docs/format.md`, "Other readers",
says what). The tests read every one, save it again and check that nothing
was lost; a format version with no file here fails a test.

| File | Format | Where it came from |
| --- | --- | --- |
| `v1-short.wss` | 1 | Written for the test |
| `v2-notes.wss` | 2 | Written for the test |
| `v3-revisions.wss` | 3 | Written for the test |
| `v4-styles.wss` | 4 | Written for the test |
| `v5-ext.wss` | 5 | Written for the test |

Wordsmith Manuscript tests its own `.wss` reader against unchanged copies of
these files (its `tests/fixtures/scripts/` and `docs/fixtures.md`). When one
changes here, or a version is added, the same change opens an issue in the
wordsmith-manuscript repository that names the file (`AGENTS.md`, "`.wss`
changes").

## Files saved by 0.1.2 (`tests/fixtures/saved-by-0.1.2/`)

What the last build of Script before it was rebuilt on open-source-core
(0.1.2) wrote, kept so that the rebuilt app can be held to it
(`crates/script-format/tests/migration.rs`). 0.1.2 was never published and
its code is not in this repository's history (`UPSTREAM.md`, "Before the
rebuild"); these files are what is left of it to test against.

| Files | What they are |
| --- | --- |
| `everybodys-girl.wss`, `timid-teddy.wss`, `salt-of-vengeance.wss` | The three golden scripts below, opened from Fountain and saved by 0.1.2 |
| `v1-short.wss`, `v2-notes.wss`, `v3-revisions.wss`, `v4-styles.wss` | The format fixtures above, opened and saved again by 0.1.2 (so each is in format 4) |
| `<name>.plan.txt`, one for each of the seven | The pages 0.1.2 planned for that script: whether the layout matched the plan, each page's newest revision set, the page labels and every page's pieces, written by a test of 0.1.2's own code |

They are never written again: no build can make them now.

## The golden scripts (`tests/golden/*.fountain`)

Three public-domain photoplays, transcribed into Fountain by
`tools/golden/convert.py`. `docs/golden.md` says why they are public domain
and how they were transcribed; `docs/provenance.md` has the sources' addresses
and checksums.

| File | Work | Printed in | Source |
| --- | --- | --- | --- |
| `everybodys-girl.fountain` | "Everybody's Girl", a photoplay in five reels (Vitagraph, 1918) | *Writing the Photoplay*, revised edition (1919) | Project Gutenberg e-text 17903 |
| `timid-teddy.fountain` | "Timid Teddy" | *How to Write Photoplays* (1915) | Project Gutenberg e-text 76829 |
| `salt-of-vengeance.fountain` | "The Salt of Vengeance" | *The Photodrama* (1914) | Project Gutenberg e-text 70937 |

The `<name>.golden.txt` files beside them are not fixtures but reviewed
results (`docs/golden.md`).

The same scripts serve other tests: the command line's tests convert and
export them, the hostile-input test damages copies of `timid-teddy.fountain`,
and the README's pictures (`cargo xtask screenshots`) show
`everybodys-girl.fountain`.

## Scripts made by the tests themselves

Everything else a test reads, it writes first: scripts typed through commands,
generated scripts for the round trips (`crates/script-format/tests/round_trips.rs`),
and the 150-page script of the typing and pagination tests. None is kept in
the repository.
