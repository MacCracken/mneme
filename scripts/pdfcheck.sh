#!/usr/bin/env bash
# pdfcheck.sh — validate mneme's hand-rolled PDF writer against bayan's strict
# structural validator.
#
# Why this exists: tests/io_export_pdf.tcyr can assert on bytes it finds in the
# file, but it cannot verify that every xref offset points at the object it
# claims to. Real viewers (poppler/mupdf) hide that class of bug by rebuilding
# the table from a scan for "N 0 obj", so a broken PDF still opens fine. bayan's
# pdfcheck.py is deliberately NOT lenient — it checks every offset, /Length and
# /Size against the actual bytes. Reused rather than vendored so the two ports
# stay gated by one oracle (see bayan's bayan_pdf_* work).
#
# Usage:
#   scripts/pdfcheck.sh            # validate; warn-and-skip if bayan is absent
#   scripts/pdfcheck.sh --strict   # missing validator is a hard failure (CI)
#
# Validator lookup order: $MNEME_PDFCHECK, $BAYAN_HOME/scripts/pdfcheck.py,
# then ../bayan/scripts/pdfcheck.py relative to this repo.

set -uo pipefail
cd "$(dirname "$0")/.."

STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

CHECK="${MNEME_PDFCHECK:-}"
[ -z "$CHECK" ] && [ -n "${BAYAN_HOME:-}" ] && CHECK="$BAYAN_HOME/scripts/pdfcheck.py"
[ -z "$CHECK" ] && CHECK="../bayan/scripts/pdfcheck.py"

if [ ! -f "$CHECK" ]; then
    echo "pdfcheck: validator not found at '$CHECK'"
    echo "pdfcheck: set MNEME_PDFCHECK or BAYAN_HOME to bayan's checkout."
    if [ "$STRICT" = "1" ]; then
        echo "pdfcheck: --strict — treating a missing validator as failure."
        exit 1
    fi
    echo "pdfcheck: SKIPPED (pass --strict to make this an error)."
    exit 0
fi

# The PDF tests write their fixtures to /tmp/mneme_pdf<N>/; regenerate them from
# scratch so a stale file from an older build can never be what we validate.
rm -rf /tmp/mneme_pdf*
echo "pdfcheck: generating fixtures via tests/io_export_pdf.tcyr"
if ! cyrius test tests/io_export_pdf.tcyr >/dev/null 2>&1; then
    echo "pdfcheck: FAILED — the PDF test suite does not pass; fix that first."
    exit 1
fi

shopt -s nullglob
PDFS=(/tmp/mneme_pdf*/*.pdf /tmp/mneme_pdf*/*/*/*/*.pdf)
if [ ${#PDFS[@]} -eq 0 ]; then
    echo "pdfcheck: FAILED — the test run produced no PDFs to validate."
    exit 1
fi

fail=0
for f in "${PDFS[@]}"; do
    if out=$(python3 "$CHECK" "$f" 2>&1); then
        echo "  $out"
    else
        echo "  FAIL $f"
        echo "$out" | sed 's/^/      /'
        fail=1
    fi
done

if [ "$fail" != "0" ]; then
    echo "pdfcheck: FAILED — one or more PDFs are structurally invalid."
    exit 1
fi
echo "pdfcheck: OK — ${#PDFS[@]} PDF(s) passed strict validation."
