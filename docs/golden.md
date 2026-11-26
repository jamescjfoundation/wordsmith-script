# Golden files

Reviewed results the tests compare with what the app makes now. A change that
alters one is deliberate: run the tests with `UPDATE_GOLDEN=1`, read the
difference, and add a row under "Changes" saying what changed and why.

| File | What it holds | Test |
| --- | --- | --- |
| `tests/golden/<name>.golden.txt`, for each of the three golden scripts | The script's PDF, read back: every page's text, line by line, with its page breaks | `crates/script-format/tests/golden.rs`, `golden_scripts_paginate_and_print_as_reviewed` |
| `tests/fixtures/saved-by-0.1.2/<name>.plan.txt`, seven | The pages 0.1.2 planned for each script it saved (`docs/fixtures.md`). Never rewritten: no switch writes them | `crates/script-format/tests/migration.rs` |
| `docs/guide/keys.md` | The guide's page of keys, made from the commands the window shows | `crates/script-ui/src/tests.rs`, `the_keyboard_guide_matches_the_commands` |

    UPDATE_GOLDEN=1 cargo test -p wordsmith-script-format --test golden
    UPDATE_GOLDEN=1 cargo test -p wordsmith-script-ui the_keyboard_guide_matches_the_commands

## The golden scripts

Three public-domain screenplays (1910s "photoplay" scenarios), transcribed into
Fountain, are the golden corpus for pagination and PDF output
(`tests/golden/`).

| Script | Author | Printed in | Pages |
| --- | --- | --- | --- |
| Everybody's Girl (156 scenes, five reels) | A. Van Buren Powell, adapted from O. Henry's "Brickdust Row"; Vitagraph, ©1918 | J. Berg Esenwein and Arthur Leeds, *Writing the Photoplay*, revised edition (Home Correspondence School, Springfield, Mass., 1919), Chapter XX | 24 |
| Timid Teddy (52 scenes) | Embrie Zuver | Embrie Zuver, *How to Write Photoplays* (1915) | 9 |
| The Salt of Vengeance (40 scenes) | Henry Albert Phillips | Henry Albert Phillips, *The Photodrama* (©1914, published September 1914) | 8 |

## Why they are public domain

Each was published in the United States before 1931. US copyright in works
published before 1931 has expired, so as of 2026 these texts are in the
public domain in the United States (Project Gutenberg marks each book "public
domain in the USA"). Only the scripts are used, not the books' commentary or
footnotes. Outside the United States their status may differ; they are test
data, not shipped with the app.

## How they were transcribed

`tools/golden/convert.py` converts the Project Gutenberg plain text of each
book (sources and checksums in `docs/provenance.md`) into Fountain. The texts
are kept word for word; the 1910s scenario form maps to screenplay elements as
the script's header explains: numbered scene lines become scene headings
(with their numbers as Fountain scene numbers), descriptions become Action,
title cards ("leaders", captions, subtitles, inserts) become Action lines
starting `TITLE:`, `INSERT:` or `LETTER:`, and a spoken title card whose
speaker the text right before it names becomes Character and Dialogue. The
original capitals of title cards are kept. Two scene lines that begin with a
quote mark or a bracket lose those marks, because Fountain forces a heading
only when the `.` is followed by a letter or digit.

The output was read by eye, page by page: page numbers (none on page 1),
headings never alone at a page foot, speakers of spoken titles, and that no
text is lost. Four problems found that way were fixed in the converter and the
test's PDF reader before the golden files were committed.

## The test

`cargo test -p wordsmith-script-format --test golden` reads each Fountain file,
paginates it, checks that the layout engine agrees with the pagination plan
page for page, exports the PDF, reads its text back line by line, and
compares it with `<name>.golden.txt`. After a deliberate change to
pagination or PDF output, rewrite them with the switch above and read the
diff of the golden files before committing it.

The test reads the PDF back through the core's PDF reader (`jcj_output::read`,
craft-libs' guarded interpreter underneath); before 0.2 it called that
interpreter directly.

## Files saved by 0.1.2

`tests/fixtures/saved-by-0.1.2/` keeps, for each golden script and each
`.wss` fixture, the file Wordsmith Script 0.1.2 (commit `29ed4bf`) saved and
the page plan it made. `cargo test -p wordsmith-script-format --test migration`
opens each in the current version, saves it in the newest format, reopens it
and checks that the pages are exactly those 0.1.2 planned.

## The rebuild on open-source-core (0.2)

The rebuild was checked against 0.1.2's outputs before any code moved: the
three golden files above, the page plans of every golden script and fixture,
and the PDFs. With 0.2 on open-source-core (v0.1.20):

- the golden text files are unchanged: every page reads back line for line as
  0.1.2 printed it;
- every page plan is unchanged (the migration test);
- the PDF files are not compared byte for byte, before or after: with default
  settings both 0.1.2 and 0.2 write the time of export into the file. 0.2
  writes through the core's PDF writer (WordCraft v0.18.1), whose file
  structure differs, and whose files name WordCraft as their creator and
  producer where 0.1.2's named Wordsmith Script (`docs/gaps.md`). The text,
  its places on the page and the page count are what the tests compare, and
  those are the same.

The scripts have little dialogue, so they exercise headings, action splits
and keep rules more than (MORE)/(CONT'D); the unit tests in
`wordsmith-script-model::paginate` cover those.

## Changes

| Date | File | What changed |
| --- | --- | --- |
| 2026-11-19 | the three `.golden.txt` | First version, read by eye page by page ("How they were transcribed") |
| 2026-11-24 | none | The rebuild on open-source-core left every golden file and every page plan as it was ("The rebuild on open-source-core") |
| 2026-11-26 | none; the files moved | The golden scripts and their results moved from `crates/wordsmith-format/tests/golden/` to `tests/golden/`, the fixtures to `tests/fixtures/`, and the guide's `keyboard.md` became `keys.md` (decisions 0045 and 0046). No byte of any of them changed. The keys page is rewritten with `UPDATE_GOLDEN=1` now; it was `UPDATE_GUIDE=1` |
