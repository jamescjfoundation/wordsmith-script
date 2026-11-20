# Performance

Measurements the definition of done asks for. Each row says how to repeat
it. Machine: Apple Silicon Mac, release build, 2026-11-19.

| What | Result | How to measure |
| --- | --- | --- |
| Typing in a 150-page script: one editor frame per keystroke (command, repagination, drawing the visible pages, spelling and suggestions) | median 16.0 to 17.6 ms over 20 keystrokes, four runs (end of M8; single worst frames of 18 to 180 ms vary run to run and look like machine noise); 16.3 to 16.5 ms at the end of M6; 14.7 to 15.6 ms at the end of M5; 16.3 ms at the end of M3, 14.8 ms after spelling, 14.5 ms in M2 | `cargo test -p wordsmith-ui --release typing_in_a_150_page_script -- --ignored --nocapture` |
| Paginating a 150-page script | 109 ms with empty caches, 14 ms warm (after an edit) | `cargo test -p wordsmith-script --release a_feature_length_script -- --nocapture` |

Notes:

- Repagination is fast after an edit because paragraphs are measured one at a
  time through a cache (only edited paragraphs are laid out again) and the
  output script's paragraphs keep stable revisions, so the layout cache
  serves every unchanged one (decision 0012).
- The headless frame does not include uploading textures to the GPU or
  presenting the window. Caret Up and Down still lay out the whole script
  (about 35 ms at 150 pages); that is a known cost, not yet optimised.
- The end-of-M3 figure is about 1.5 ms slower than mid-M3; it arrived with
  the find branch and is not yet explained. A likely cost: the element
  registry and other per-script settings are parsed from the script's stored
  JSON several times per keystroke; caching them per revision is the first
  thing to try. Still under one 60 Hz frame (16.7 ms) at the median.
- Linux and Windows have not been measured.

## After v0.1

Typing at 150 pages is responsive but sits at about one 60 Hz frame. The
production and outlining features read their settings from `app_data` (JSON)
several times per keystroke, and the status bar and overlays walk the whole
script every frame; caching those by revision is the first thing to try to
win back the margin M5 had.
