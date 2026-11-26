# 0033: Characters, scene data and reports

Date: 2026-11-20 · Status: accepted

## Decision

- **Scene data lives with the heading.** A scene is still derived from Scene
  Headings (decision 0022); its title, color, non-speaking cast, hidden speakers
  and arc beats are kept by an id on the heading paragraph. Moving scenes moves
  them; a copied heading is given a new id the first time its data changes.
- **Scene colors** are none plus seven of our own: Red, Orange, Yellow, Green,
  Blue, Purple, Gray.
- **Traits** are free text; the default set is Age, Disability, Ethnicity,
  Gender, Occupation, Orientation, Role (SPEC-2 [DEF]). Blank values group as
  "Unassigned" in the representation chart, whose shares are of characters, of
  dialogue words and of scene appearances.
- **Highlighted speeches** are drawn on screen only; they never print.
- **A monologue** is a speech of 100 words or more.
- **Production order** in the scene report groups scenes by location, then intro,
  then time of day.
- **Flagged language**: a short default list of strong words, editable per script
  (`report.setFlaggedWords`).
- **Dates** come from the caller (`now`, Unix seconds) so the screenplay layer has
  no clock and the web build works.
- **Reports** show in a window and save as text, CSV, or for the filtered script,
  PDF. They are not opened as new documents until the app has document tabs.
