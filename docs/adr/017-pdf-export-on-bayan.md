# 017 — PDF export on bayan_pdf: measured wrapping, WinAnsi text, no silent deletions

**Status**: Accepted
**Date**: 2026-11-12

## Context

`src/io_export_pdf.cyr` hand-rolled its PDF from 1.0.0 to 1.2.3, because no Cyrius PDF library existed. bayan's PDF
module (`bayan_pdf_*`, bayan 1.5.0, vendored since mneme 1.1.3) was written from an audit of that writer. It names
three live defects, and neither the suite nor `scripts/pdfcheck.sh` caught them, because pdfcheck validates
structure, not text:

1. **No `/Encoding` on the font dictionary.** Viewers fall back to StandardEncoding, where only 54 of the 128 high
   codes exist. The UTF-8 bytes written raw render `Résumé` as `RÃ©sumÃ©`, and an em dash vanishes.
2. **Wrapping by character count**, 80 per line. In Helvetica at 11 pt, 80 `l`s measure 195 pt and 80 `W`s measure
   831 pt, against a 451 pt column on a 595 pt page.
3. **The show-text escaper deleted CR and LF**, welding the words either side of a lone CR into one.

The writer also predicted the first xref offset (`offset = 9`) and numbered objects by formula (`4 + 2p`). Both were
correct only for its exact header and object set.

rust-old used `printpdf` with the built-in fonts:
- it wrapped at a character count derived from the page (`usable_width / (size · 0.22)`, about 66);
- it did not wrap titles, headings or tag lines;
- it passed text through as given.

Its tests check only that a file exists, is over 100 bytes and starts with `%PDF`. The port's rule is "match
rust-old, diverge only with an ADR". Several of the decisions below are divergences, so they are recorded here.

## Decision

**The PDF primitives are bayan's; the markdown layer and the layout stay mneme's.**
- **Kept from mneme:**
  - `mneme_io_parse_markdown_blocks`, including ADR 015's flush of an unterminated fence;
  - inline stripping and the geometry: A4, x = 72 pt, first baseline 770 pt, a new page below 90 pt, the same
    sizes and leading.
- **bayan's:** the document, its objects, a measured xref, the standard-14 fonts, the content streams and `/Info`.

Within that:
- **Fonts follow rust-old:** Helvetica-Bold for the title and headings, Courier for code, Helvetica for body text.
  The port used Helvetica throughout. This is a parity repair.
- **Text is WinAnsi.** bayan declares `/WinAnsiEncoding` and transcodes. A character the standard fonts cannot
  draw (CJK, emoji, `✓`) is exported as `?` and counted, and a malformed UTF-8 byte is exported as `?` too. Nothing
  is dropped. *Divergence:* printpdf's behaviour here is not covered by any rust-old test, and the old port's
  mojibake was not it.
- **Wrapping is by measured width** (`bayan_pdf_wrap`, real AFM metrics) into the 451 pt column. That applies to
  paragraphs, list items, the title, the tag line and headings. *Divergence:* rust-old counted characters, and did
  not wrap titles, headings or tags. Code stays one line per source line, unwrapped, as in rust-old.
- **A control character becomes a space.** A show-text run cannot hold a line break, and bayan refuses one;
  deleting it welds words together. *Divergence:* rust-old passed a lone CR through.
- **A refused run fails the export.** bayan still serializes a document after refusing a text run (its open issue,
  "gap 1"), so mneme checks the document's error count. A PDF with a line missing is an error (`-22`, or HTTP 500),
  never a success. After the preparation above, no ordinary text reaches that path.
- **Diagnostics are surfaced (additive):**
  - `mneme_io_pdf_warning()` names an unclosed fence and any `?` substitutions;
  - `mneme_io_pdf_batch_warnings()` lists them per file;
  - `/export/pdf`'s response carries the warning, the `X-Mneme-Warning` field.

  Before this, nothing reported ADR 015's fence diagnostic.

## Consequences

- **Positive:**
  - Non-ASCII Latin text renders as written: the encoding test reads `Café, naïve and “quoted” — 50 €` back
    exactly.
  - No body line runs off the page.
  - No text is silently deleted or welded.
  - The xref and object numbering are measured.
  - The binary already carried `bayan_pdf_*`, and now the code that does the work is that code.
- **Negative:**
  - The file changes shape: PDF-1.7 with a two-line header instead of 1.4, three font objects, and an `/Info` dict.
  - Text outside WinAnsi still cannot appear; it is now a visible, reported `?` instead of mojibake. Embedded
    TrueType fonts are bayan's gap 3.
  - A single word wider than the column (a long URL) still runs off the page, because bayan's wrap does not break
    inside a word (its gap 2).
- **Neutral:**
  - `mneme_io_wrap_text` (character count) stays for rust-old's `word_wrap` test, but the renderer no longer uses
    it.
  - bayan's `scripts/pdfcheck.py --text` decodes content as Latin-1, so it shows WinAnsi's 0x80–0x9F characters
    (`—`, `“”`, `€`) as blanks. The bytes are correct CP1252, and bayan's own reader extracts them.

## Alternatives considered

- **Patch the hand-rolled writer.** Adding `/Encoding`, transcoding, width tables and offset measurement is
  rebuilding bayan's module inside mneme, which is the duplication the migration exists to end.
- **STRICT encoding, failing the export on any non-WinAnsi character.** One emoji would make a whole note
  unexportable. Substituting and reporting keeps the note and tells the reader.
- **Leave titles and headings unwrapped, as rust-old does.** That leaves a long title running off the page: the
  same defect class as the paragraph wrap, in a different block.
