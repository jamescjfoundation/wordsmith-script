# Performance

Measurements the definition of done asks for. Each row says how to repeat
it. Machine: Apple Silicon Mac, release build, 2026-11-19.

| What | Result | How to measure |
| --- | --- | --- |
| Typing in a 150-page script: one editor frame per keystroke (command, repagination, drawing the visible pages, spelling and suggestions) | median 16.3 ms, worst 17.3 to 17.7 ms over 20 keystrokes, four runs (end of M3); 14.8 ms after spelling, 14.5 ms in M2 | `cargo test -p wordsmith-ui --release typing_in_a_150_page_script -- --ignored --nocapture` |
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
