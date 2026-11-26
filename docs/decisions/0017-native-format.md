# 0017: The native format

Date: 2026-11-19 · Status: accepted

Since decision 0044 (2026-11-24) `.wss` is still this text format, now at
version 5: `appData` is written as `ext` (`docs/format.md`). The core's
container for other apps took the general ideas
([core decision 0009](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0009-project-container.md));
Script does not use it.

`docs/format.md` is the specification. The choices behind it:

- **Text, not JSON throughout.** The script part reads like a screenplay
  draft (`@Element`, then the words), so a diff shows changed lines of the
  script, not changed JSON. Settings are JSON because they are structured, but
  sorted and pretty-printed so their diffs are small too.
- **Emphasis like Fountain** (`**`, `*`, `_`), so writers who know Fountain
  read it at sight. Markers toggle a style each, which keeps encoding and
  decoding simple and exact.
- **Everything a script needs is in `appData`,** written as is. New features
  add keys without changing the format version; a version bump is only for a
  change to the file's structure.
- **The template fills gaps.** A file missing styles or elements reads with
  the template's, and an unknown element reads as General, so a hand-edited
  or partly damaged file still opens.
- **Stable bytes.** Writing what was read gives the same bytes (tested on the
  golden scripts), so saving without changes never makes a diff.
- **Extension `.wss`**, for Wordsmith Script.
