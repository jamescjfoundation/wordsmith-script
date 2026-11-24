# Performance

How fast the app is, measured. Each row says how to repeat it. Machine: Apple Silicon Mac, release build, 2026-11-19.

| What | Result | How to measure |
| --- | --- | --- |
| Typing in a 150-page script: one editor frame per keystroke (command, repagination, drawing the visible pages, spelling and suggestions) | median 17.2 to 17.4 ms with everything shown against 16.0 to 17.1 ms with no chrome in the same runs (the finished v0.3 interface, 2026-11-20; the machine ran slower than for the run before); median 15.6 to 15.8 ms with the v0.3 ribbon and the Script panel open, two runs (2026-11-20); median 15.2 to 16.4 ms over 20 keystrokes, two runs (the SPEC-2 features, 2026-11-20, worst 17.1 to 17.5 ms); median 16.0 to 17.6 ms over 20 keystrokes, four runs (v0.1.0; single worst frames of 18 to 180 ms vary run to run and look like machine noise); 16.3 to 16.5 ms with revisions and page locks; 14.7 to 15.6 ms with notes and the title page; 16.3 ms with autocomplete, spelling and find, 14.8 ms after spelling, 14.5 ms with the first paginator | `cargo test -p wordsmith-ui --release typing_in_a_150_page_script -- --ignored --nocapture` |
| Paginating a 150-page script | 109 ms with empty caches, 14 ms warm (after an edit) | `cargo test -p wordsmith-script --release a_feature_length_script -- --nocapture` |

## The regression guard

Frame times depend on the machine and on what else it is doing, so a timing
test in CI would either fail at random or need a limit too loose to catch
anything. The timing tests above stay `#[ignore]`d and are run by hand.

What CI checks instead is the work a keystroke does, counted by
`wordsmith_script::work` on the test's own thread:
`a_keystroke_in_a_long_script_does_only_its_own_work` (in
`crates/wordsmith-ui/src/tests.rs`, part of `cargo xtask ci`) types into a
150-page script, with the Scenes panel open and with no panels, and fails if
a keystroke frame

- paginates more than once,
- lays out more than two paragraphs again (the edited one; the rest come
  from the measure cache), or
- reads the element registry from the script's settings more than once;

and if a frame without input paginates or lays out anything. These are the
costs that grow with the script, so a change that makes typing scale with
its length (a second pagination in a panel, a cache key that stops
matching, a setting parsed per paragraph) fails the gate on every machine.
Work that does not grow with the script (drawing, the panels' own lists) is
not counted; the timing test is still the way to measure it.

To see the counts while working: `wordsmith_script::work::so_far()` before
and after, as the guard does.

## Element registry reads (2026-11-22)

The counters showed a keystroke at 150 pages read the element registry from
the script's JSON settings 23 to 28 times (and an idle frame 9 or 10 times),
each read about 18 µs against 2.4 µs for a copy of the result, measured in
the test profile on Linux. `Elements::of_doc` now keeps the last registry
read on each thread with the stored value it came from, and reads again only
when that value is replaced; a keystroke now reads it none. That is about
0.4 ms a keystroke in the test profile, less in a release build. The
150-page frame on the Mac has not been measured again since.

Notes:

- Repagination is fast after an edit because paragraphs are measured one at a
  time through a cache (only edited paragraphs are laid out again) and the
  output script's paragraphs keep stable revisions, so the layout cache
  serves every unchanged one (decision 0012).
- The headless frame does not include uploading textures to the GPU or
  presenting the window. Caret Up and Down still lay out the whole script
  (about 35 ms at 150 pages); that is a known cost, not yet optimised.
- The figure with find is about 1.5 ms slower than the one after spelling; it
  arrived with the find branch and is not yet explained. A likely cost: the element
  registry and other per-script settings are parsed from the script's stored
  JSON several times per keystroke; caching them per revision is the first
  thing to try. Still under one 60 Hz frame (16.7 ms) at the median. (The
  element registry is now cached; see above.)
- Linux and Windows have not been measured.

## After v0.1

Typing at 150 pages is responsive but sits at about one 60 Hz frame. The
element registry is now read once per change rather than about 24 times a
keystroke. The other production and outlining settings are still read from
`app_data` (JSON) several times per keystroke, and the status bar and
overlays walk the whole script every frame; caching those by revision is the
next thing to try to win back the margin the notes and title page build had.
