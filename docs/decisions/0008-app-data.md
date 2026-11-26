# 0008: Screenplay data lives in the document's `app_data`

Date: 2026-11-19 · Status: replaced by
[core decision 0006](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0006-extension-data.md)

Since decision 0044 (2026-11-24) `app_data` is the document's `ext`, and the
fields Script had added to the model are typed extension keys. Script's own
`wordsmith.` keys are unchanged.

## Context

Element settings, the Tab table, scene-heading separators and (later)
autocomplete lists, revision sets and page locks are per script, and undo
and redo must cover them like any other edit to the script. The engine's undo
steps are whole-`Document` snapshots, so this data has to be inside
`Document`. The copied `wordsmith-doc` had no place for application data.

## Decision

`Document` gains `app_data: BTreeMap<String, Arc<serde_json::Value>>`.

- Keys are namespaced (`wordsmith.elements`, `wordsmith.separators`, …).
- It is part of the document: `==`, `same_content` (so a command that only
  changes it is an edit, not a no-op) and serialization all include it. An
  empty map is not written, and older data without it still reads.
- Values are `Arc`-shared, so an undo snapshot copies a pointer per entry.
- `wordsmith-script` owns the typed views over its keys and treats every
  value as hostile input: a value that does not parse falls back to the
  default and never panics.
- Formatting still lives in paragraph styles. `app_data` holds only what a
  style cannot express.

The change is general (any app built on the document model can use it), so
it is listed in `UPSTREAM.md` as a candidate for WordCraft.

## Revisit when

The native format is designed: it decides how `app_data` is written in our
own file (done in decision 0017).
