# 0017: The native format

Date: 2026-11-19

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
