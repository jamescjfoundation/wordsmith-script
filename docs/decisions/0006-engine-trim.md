# 0006: Trimming WordCraft's engine to session, undo, registry and recovery

Date: 2026-11-19

## Context

The kickoff asks for "the session, undo, command-registry and recovery parts"
of WordCraft's `engine` crate. At `6ef13b2` that crate is about 92,000 lines.
Its `Session` struct has about 70 fields, most of them for Word features
(mail merge, macros, equations, drawing canvases, Restrict Editing, Read
Aloud, the Markdown source view, citations, Quick Parts, IME receipts,
several open documents parked in one session). The command run loop calls
into those features directly. Copying the crate whole would also pull in
WordCraft's docx, docbin and formats crates, which we do not use.

## Decision

`wordsmith-engine` is assembled from copied engine files, cut down. Where a
part is kept, its code and comments are upstream's; the cuts remove Word
branches, not behavior of the kept parts.

Kept:

| Part | Upstream files | What was cut |
| --- | --- | --- |
| Command registry | `lib.rs` (`CmdError`, `CmdResult`, `CommandSpec`, `Registry`, `p` helpers) | IME outcome type, Word re-exports |
| Selection and undo steps | `doc_state.rs` | column blocks, widened cell selections, find state, equation editing, typed-word tracking, citation style |
| Undo history | `session/history.rs` | AutoCorrect exceptions, protection checks, equation and IME joins, versions |
| Run loop | `session/run.rs` | macros, Repeat, protection, equation, canvas, column dispatch, Markdown source, multi-document handling, IME receipts, connector rerouting, outline view |
| Session lifecycle and queries | `session.rs`, `session/lifecycle.rs`, `session/query.rs`, `session/snapshot.rs` | every Word-only field |
| Layout cache | `session/layout.rs` | read mode, outline view, proofing, numerals, typed word, live preview |
| Desktop recovery worker | `recovery/desktop.rs` and its tests | nothing |
| Session recovery hooks | `recovery.rs`, recovery parts of `session/documents.rs` | password barrier, several documents |
| File access confinement | `file_access.rs` | nothing |
| Time helpers | `now_iso`, `iso_from_unix_secs`, `now_unix`, `civil_from_days` from `cmd.rs` | everything else in `cmd.rs` |

Commands: the registry starts with only `edit.undo`, `edit.redo`,
`edit.undoHistory` and `document.recover`. Every screenplay command is ours
and arrives with M1. Word's 422-command catalog, the ribbon, keymap defaults,
packs, speech, printing glue and samples are not copied.

One session holds one document. A desktop window per script replaces
WordCraft's parked-documents model. The recovery worker itself still handles
many records, so several windows can share one profile folder.

## Consequences

- Recovery tests that depended on password protection, several open
  documents in one session, or saving `.docx` were not ported. The worker
  tests and the single-document session tests were.
- IME (input method) receipts are gone. Text input through egui still works,
  but WordCraft's protections for composing text in an input method are not
  carried over. Tracked in `docs/gaps.md` for M1.
- Engine tests from upstream's `tests.rs` exercise Word commands, so they
  cannot be ported as they are. The run loop's guarantees (rollback on
  failure and on panic, no-op commands leave no undo step, typing collapses
  into one step, joined steps) are covered by new tests written against
  small test commands.

## Revisit when

M1 needs something that was cut (for example find state for find and
replace in M3). Bring it across from upstream at the commit in `UPSTREAM.md`
rather than rewriting it.
