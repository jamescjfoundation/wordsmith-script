# 0047: The first public release is 0.9.0; nothing before it was published

Date: 2026-11-26 · Status: accepted

## Context

Script's records spoke of things nobody could find. The changelog dated three
versions, 0.1.0 to 0.1.2, as if they had been released. `UPSTREAM.md` and
`ATTRIBUTION.md` said the code of those versions was "in the history of this
repository up to the `v0.1.2-final` tag". The README said release files came
with attestations "from the release after 0.1.1 on". The repository has no
tag and no release, and its history does not hold that code.

What happened:

- The first version of the app was written in November 2026, on its own copy
  of WordCraft. Its builds were numbered 0.1.0, 0.1.1 and 0.1.2. None was
  published from this repository.
- The app was then rebuilt on open-source-core (decision 0044). This
  repository's history starts there: with the core's template, then one
  commit that brings in the first version's own screenplay code as it stood
  at 0.1.2, without the copies of WordCraft's crates it was built on, then
  the port.
- The workspace was numbered 0.2.0 after the rebuild, and the changelog's
  first section was "0.2.0 (not released yet)". "v0.2" and "v0.3" were also
  the names of two plans whose work was in the 0.1.1 build.

So "what did release N change" had no answer for any N.

## Decision

- **The first public release is 0.9.0**, a preview: a version people can
  download to try the app, the update check and the launcher before 1.0.0.
  **1.0.0 follows it.** The three Wordsmith apps do the same.
- **The workspace's version is 0.9.0 from now.** 0.2.0 was never released and
  is not used; no version between 0.1.2 and 0.9.0 exists.
- **The records say plainly that the 0.1.x history was not kept.** That code
  was written before the app was rebuilt on the core, and its history is not
  published. No tag or release of it exists, and it cannot be built from this
  repository. `CHANGELOG.md` keeps its notes under "Before the first release",
  marked as never released; `UPSTREAM.md` keeps the upstream commits it
  copied from, as the credit requires.
- **The changelog has the core's form.** `## Unreleased` is first. When a
  release is cut the section becomes `## <version> (<date>)` and its tag is
  `v<version>`; only a released version has a heading of that kind.
- **Words for the two.** "The first build" is 0.1.x. "The rebuild" is the app
  on open-source-core, the one that will be released. Comments in the code
  and older records that say "0.1" and "before 0.2" mean the first build, and
  "0.2" the rebuild.

## Consequences

- Scripts the first build saved (formats 1 to 4) still open: someone may have
  them, and the migration test holds the app to it
  (`tests/fixtures/saved-by-0.1.2/`, whose name stays: 0.1.2 did save them).
- The same goes for the first build's preferences file, which is still read
  into the new shape.
- A release, its tag and its changelog section will agree from the first one
  on, because there is nothing earlier to agree with.

## Revisit when

- The first version's history is published after all. Then its last commit
  gets a tag, and `UPSTREAM.md` and the changelog point to it.
