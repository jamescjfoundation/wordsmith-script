# 0039: A fork of WordCraft

Date: 2026-11-22

## Context

Decision 0005 copied WordCraft's model, layout, font, render and PDF crates so
that fixes could be pulled across later: diff against the pinned commit, apply
the renames, keep the marks. Decisions 0006 and 0007 trimmed the engine and the
MCP server the same way.

That plan kept about 104,000 lines of copied Rust beside about 37,000 of our
own, and most of the copied code serves a word processor: equations, charts,
SmartArt, WordArt warps, pictures and metafiles, ruby text, legacy form fields,
codepage tables and the guards for Office containers. None of it can be reached
from a screenplay. Nothing in the app, the native format, Fountain, RTF or PDF
import, or the command registry makes those objects. The code still had to
compile, pass clippy, be read by reviewers and be kept working when we changed
the layout for screenplays. Keeping it in step with upstream file for file never
happened: our changes to pagination and the document model already make a
whole-file diff against upstream noisy.

## Decision

The copied crates are a fork. We no longer try to keep them diffable against
WordCraft, and we cut what a screenplay cannot use.

Cut in this change:

| Area | What went | Crates |
| --- | --- | --- |
| Equations | the `Equation` inline object; the linear, LaTeX and MathML model; math layout; the OpenType MATH reader; the equation-number counter threaded through layout | doc, layout, fonts |
| Charts and diagrams | the `Graphic` inline object (charts, SmartArt, native ink) and its drawing | doc, layout, pdf |
| WordArt warps | text warps on shapes and the WordArt style presets | doc, layout |
| Connector routing | gluing connector ends to other shapes and re-routing them | doc |
| East Asian objects | ruby (phonetic guide), EQ-field enclosed characters, the IME visible-text view | doc, layout |
| Pictures | the `Image` inline object, cell and page background pictures, picture effects, embedded OLE records, the document's media store, wrapping around a picture's transparency, the strict-export picture checks | doc, layout, render, pdf |
| Metafiles | WMF and EMF drawing | layout, render, pdf |
| Legacy formats | legacy form fields (check boxes, drop-downs), codepage tables, the ZIP and CFB container guards, locale number formats and collation, the Insert › Icons library | doc, layout, render |

Dependencies that went with them: `craft-metafile`, `craft-codepage` and
`craft-zip` from the workspace, and the `image` crate from layout and PDF.

Kept, though at first sight they look unused:

- **Text effects** (`CharProps::text_effects` and the WordArt drawing that
  paints them). They are character properties, which the element styles saved
  in a `.wss` file can hold, and the same code draws the outline, shadow,
  emboss and engrave character flags.
- **Kinsoku and Asian/Latin spacing.** They apply to any CJK text a writer
  types.
- **Vertical text and the CJK number formats.** They are values of the page
  settings a `.wss` file saves (`TextDirection`, `NumFormat`); dropping a
  variant would make such a file fail to open.
- **The picture codec layer in `wordsmith-geom`, `craft-image-codecs` and
  `craft-tiff`.** The renderer decodes colour emoji bitmaps through it (PNG,
  and TIFF in sbix fonts) and encodes PNG pages with it.
- **Bidi**, which right-to-left dialogue needs.

Everything else keeps its behavior. The remaining tests pass; a few shared test
fixtures lost the picture they held, and tests of a cut feature went with it.
The Fountain golden files and the screenplay PDF golden hashes are unchanged. Saved files read as
before. The document model types that a `.wss` file serializes (styles, page
settings, core properties, revisions, app data) lost only fields the app never
sets, and serde ignores unknown fields, so a file that somehow holds one still
opens.

### Porting fixes from upstream

`UPSTREAM.md` still records the commit each crate came from. A WordCraft fix
that matters to us is ported by hand: read the upstream change, find where the
same logic lives here (it may have moved or lost a branch), apply it, and add or
keep a test. Copied files still carry the `Modified by the Wordsmith Script
contributors` line when they differ from upstream, as `NOTICE` and the licenses
require. "Candidates to contribute back" in `UPSTREAM.md` stays: general fixes
we make are still worth offering upstream.

### Possible later work

- Screenplay attributes sit in WordCraft's `ParaProps` (`scene_number`,
  `omitted`, `beat`, `bookmark`, `dual`, `scene_id`) and `CharProps` (`note`,
  `revised`, `page_lock`), against the spirit of decision 0008. Moving them to
  an extension map changes the native format, so it needs its own decision and
  a format version.
- More of the model can go, for example fields a screenplay never shows (tables
  of contents, citations), content controls and Restrict Editing ranges, once
  each is checked against what a saved file can hold.

## Revisit when

- We want a WordCraft feature back (pictures on a title page, say): port it as
  new work, with the screenplay's needs in mind, rather than restoring the old
  code wholesale.
- WordCraft ships a fix to code we kept: port it by hand as above.
- Upstream splits its model and layout into published crates we could depend
  on without the word-processor parts: then a dependency may beat the fork.
