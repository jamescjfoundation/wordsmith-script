# 0013: PDF export and printing

Date: 2026-11-19

## PDF

`file.exportPdf {path}` writes the paginated script through `wordsmith-pdf`'s
`export_layout`, from the same layout the page view draws, so the PDF's
pages match the screen exactly. Text is real and selectable (embedded
Courier Prime subsets with ToUnicode maps); tests read it back with hayro and
check every page against the pagination plan. The file is written to a
temporary name beside the target and renamed into place, and the path must
pass the automation write roots (`file_access`). The app's File › Export PDF…
asks for the path with craft-dialogs' non-blocking save dialog.

## Printing (2026-11-19)

`docs/REUSE.md` plans printing with craft-print. On Windows, craft-print depends on the
WordCraft fork of `winprint`, which depends unconditionally on sigstore and
through it on reqwest (an HTTP client) and `webpki-root-certs`
(CDLA-Permissive-2.0, not on the allowlist). Printing needs none of that, so we
do not take it:

- **macOS and Linux:** `file.print {printer?, copies?}` sends the PDF to CUPS
  with `lp -d <printer> -n <copies> -t <title> -- -` on standard input, which
  is what craft-print does on those systems. Copies are 1 to 999; the title
  loses control characters and is cut to 80 characters.
- **Windows:** the PDF is written to the temporary folder and opened in the
  system's PDF viewer, to print from there.

`UPSTREAM.md` asks craft-libs to make the sigstore dependency optional; when
it is, printing can move to craft-print.

## Not tested

Sending a real job to a printer, and the Windows path. Tests check the `lp`
arguments and the limits only.
