# 0046: One shape with the other Wordsmith apps

Date: 2026-11-26 · Status: accepted

## Context

The three Wordsmith apps were started months of work apart, and each put its
files where it thought of first. Script kept its fixtures and golden files
inside one crate, had no list of its fixtures, rewrote one reviewed file with
`UPDATE_GOLDEN` and another with `UPDATE_GUIDE`, named its guide's pages
differently, and kept what was planned in four files that no longer agreed.
None of that was a choice. Someone who has worked in one app should find the
same thing in the same place in the next.

## Decision

Where the other two apps already agree, Script moves to what they do. Where
all three differ, Script takes the form the core uses for itself, and the
other apps are asked to follow by an issue in their repositories.

| What | Before | Now |
| --- | --- | --- |
| Test files | `crates/wordsmith-format/tests/fixtures/`, `…/tests/golden/` | `tests/fixtures/`, `tests/golden/` at the root |
| The list of fixtures | none | `docs/fixtures.md`, and a rule in `AGENTS.md` |
| Rewriting a reviewed file | `UPDATE_GOLDEN=1` for the golden scripts, `UPDATE_GUIDE=1` for the guide's keys | `UPDATE_GOLDEN=1` for every one; `docs/golden.md` lists the files and logs each change |
| The guide's page of keys | `docs/guide/keyboard.md` | `docs/guide/keys.md` |
| The guide's page for the command line and agents | `docs/guide/cli-and-mcp.md` | `docs/guide/command-line-and-agents.md` |
| The writer's words | no rule | a rule in `AGENTS.md`, with what proves it and what does not yet |
| The changelog | a heading for each planned version, "(not released yet)" | `## Unreleased` first, in the core's form; a version gets its heading and date when it is released |
| What is planned | `ROADMAP.md`, `docs/gaps.md`, `docs/progress.md` and the questions in `docs/history/` | Issues in this repository under the milestone `v1`. `ROADMAP.md` links to it and holds no list |
| What the app does not do | `docs/gaps.md`: limits, missing features, unfinished work and tooling notes in one list | `docs/gaps.md` holds only standing limits a writer should know; anything to be done is an issue |
| Finished plans and logs | in `docs/`, some reading as current | `docs/history/`, each with a first line saying it is history |

What Script keeps that the others lack:

- **Two specifications**, `docs/SPEC.md` and `docs/SPEC-2.md` (decision 0030).
- **The issue forms** in `.github/ISSUE_TEMPLATE/`. With the plan kept as
  issues they matter more, not less; the core is asked to carry them for every
  app, as it carries the pull request template.
- **`UPSTREAM.md`** at the root, a legal file the `assets` check wants: Script
  is the one app that once held copied upstream code.

## Consequences

- A test that needs a fixture reads `../../tests/…` from its crate, which
  `cargo xtask ci --changed` follows to the packages that read it.
- The wiki's addresses follow the guide's file names
  ([project decision 0005](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0005-one-guide-per-app.md)),
  so two of Script's pages move there when the wiki is next built.
- A gap is closed by closing its issue; no file has to be edited to say so.

## Revisit when

- The core's template gives apps a `tests/` layout or a changelog check of
  its own.
