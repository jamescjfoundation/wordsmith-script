# 0042: Scope, and changes that need the maintainers first

Date: 2026-11-22

## Context

The first build worked from a written brief that is not part of this
repository; the plans in `docs/history` call it "the kickoff". Some of its
rules are still in force, and several decisions, comments and the roadmap
gave the brief as their reason. This record states those rules, so a
contributor can read them and the maintainers can change them in the open.

## Decision

- **Out of scope.** These are left out on purpose, as `ROADMAP.md` lists:
  - proprietary file formats, until a legitimately
    published description exists (decision 0030 adds other products' script
    formats: they need an interface spec, SPEC-2 section 20);
  - real-time collaboration and cloud sync;
  - AI features, the assistant included. Hooks may exist behind a flag that
    is off by default;
  - production breakdown tagging and scheduling export.

  SPEC-2 asks for some of these (question 9 in
  `docs/history/spec2-questions.md`). They stay out until the maintainers
  bring one into scope here.
- **Ask the maintainers before starting** any of the following. Open an issue
  first and wait for a decision record or a reply:
  - anything that needs a paid account or service, such as a Developer ID,
    notarization, a Windows code-signing certificate or hosting (decisions
    0029 and 0038);
  - a new network request from the app. It must also be listed in the
    README's Privacy section and have a setting to turn it off;
  - a dependency, font, dictionary or other data whose license is not on the
    allowlist (decision 0001);
  - behavior, names or files taken from anywhere other than public
    documents (the clean-room rules in `docs/SPEC.md` and
    `CONTRIBUTING.md`);
  - publishing a release (decisions 0029 and 0038).

  Older documents call these "stop rules".

## Revisit when

The maintainers bring an out-of-scope item into scope, or a rule above turns
out to block ordinary work.
