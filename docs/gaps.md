# What Wordsmith Script does not do

The standing limits of the app: what a writer should know it will not do, with
the reason. Nothing here is waiting to be built.

What is missing and planned, or broken, is an issue:

- the plan for 1.0.0 is the
  [`v1` milestone](https://github.com/jamescjfoundation/wordsmith-script/milestone/1);
- ideas for later are the
  [open issues with no milestone](https://github.com/jamescjfoundation/wordsmith-script/issues?q=is%3Aissue+is%3Aopen+no%3Amilestone);
- what waits for open-source-core is in `docs/core-candidates.md`, each line
  with its core issue.

So a limit below changes only by a decision, and a gap is closed by closing
its issue. A change that adds a limit a writer will meet adds it here.

## Out of scope

Left out on purpose (decision 0042). An issue asking for one of these is
answered by the maintainers before any work starts.

- Proprietary file formats. Scripts come and go as Fountain, `.wss`, PDF, RTF,
  plain text and HTML. Another program's own format needs a legitimately
  published description first.
- Real-time collaboration and cloud sync.
- AI features, an assistant included.
- Production breakdown tagging, the tags report and scheduling export.
- Usage data: the app reports nothing about how it is used (the README's
  Privacy section).

## Writing

- Spelling is English only, with no thesaurus and no list of names: no data
  for them has a license the project allows (decision 0014).
- No emoji font is bundled. Emoji show only where the interface's or the
  system's fonts have them.
- Outline elements that are hidden still show in the normal and draft views
  (decision 0028).
- The ribbon's tabs and groups cannot be changed or rearranged (decision
  0037).

## Production

- Deleting across a locked page break is allowed (decision 0035).

## Files

- Imports keep the words, not all of the formatting (decision 0019):
  - emphasis is not kept from plain text, RTF or PDF;
  - line breaks inside a paragraph become spaces;
  - a PDF page cannot show a heading with no intro that ends in punctuation
    (it is read as Action), or a paragraph split across a page (read as two);
  - a scanned PDF, which holds pictures of pages and no text, is refused.
- A read-only file opens as a copy.
- A script over 64 MB, or with more than 500,000 paragraphs, is refused
  (`docs/format.md`, "Limits").
- Exported PDFs name WordCraft, the engine underneath, as the program that
  made them. The name is fixed in the engine
  ([core issue 5](https://github.com/jamescjfoundation/open-source-core/issues/5)).

## Downloads and updates

- The downloads are a disk image for macOS, a portable zip for Windows and a
  `.tar.gz` for Linux: no installer and no package for a system's own
  software manager.
- The macOS app is signed ad hoc and not notarized, and the Windows programs
  are not signed, so each system asks before it opens them the first time.
  Signing needs a developer's certificate, which the project does not have
  yet.
- An update installs itself only on Linux, from the `.tar.gz` in a folder the
  app may write to. On macOS and Windows the app opens the release page
  (decision 0038; Windows waits for
  [core issue 36](https://github.com/jamescjfoundation/open-source-core/issues/36)).
- The update check trusts GitHub and the file's checksum. It does not read
  the build provenance attestation each release file has; `gh attestation
  verify` does (the README, "Checking a download").
- There is no web version: the web build does not build
  ([core issue 35](https://github.com/jamescjfoundation/open-source-core/issues/35)).
