# History

The working plans and audits from the first build of Wordsmith Script, kept
for the record. They are not maintained: they describe the project as it was
when each was written, and some of what they call planned or missing has
been built since.

- `plans/`: one plan per milestone (`M1.md` to `M19.md`) and the plans for
  v0.2 (`v0.2.md`) and the v0.3 interface (`v0.3-ui.md`).
- `progress-first-build.md`: the log of what each milestone merged, up to
  2026-11-22. "v0.2" and "v0.3" are the names of those plans; no version was
  published under any number (decision 0047).
- `spec-audit.md`: `docs/SPEC.md` checked against what v0.1 built.
- `spec2-audit.md`: `docs/SPEC-2.md` checked against what v0.1 built, with
  what v0.2 added in its last section.
- `spec2-questions.md`: the questions SPEC-2 raised for its author, as first
  logged. They are still open, and they live in
  [issue 10](https://github.com/jamescjfoundation/wordsmith-script/issues/10)
  now, under the same numbers: other documents that say "question 9" mean
  those. Until one is answered, the behavior the first build had stays
  (decision 0030).

These files mention "the kickoff" and "the owner": the first build worked
from a brief that is not part of this repository, and the project's choices
were made by its maintainers. Where a rule from that time still holds, it is
written down in `docs/decisions/`, which is where to look for the current
reason for anything.

Two finished plans stay in `docs/`, because the decision records cite them
there: `docs/REUSE.md` and `docs/provenance.md`. Each says in its first lines
that it is history.

For what the app does now, read the user guide (`docs/guide`) and
`CHANGELOG.md`; for what it will not do, `docs/gaps.md`; for what is planned,
the [`v1` milestone](https://github.com/jamescjfoundation/wordsmith-script/milestone/1).
