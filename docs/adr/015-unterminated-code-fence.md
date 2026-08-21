# 015 — Flush an unterminated code fence instead of dropping it

**Status**: Accepted
**Date**: 2026-08-21

## Context

`mneme_io_parse_markdown_blocks` (`src/io_export_pdf.cyr`) flattens a note's
markdown into text blocks for the PDF writer. It tracks fenced code with an
`in_code` flag: an opening ``` sets it, a closing ``` flushes `code_buf` as an
`MD_CODE` block and clears it.

When the input ends while `in_code` is still 1 — an unterminated fence, i.e. a
plain typo — the loop simply exits. `code_buf` is never flushed and never pushed
onto `blocks`, so **every line after the stray fence is silently absent from the
exported PDF**, with no error, no warning, and a perfectly valid-looking file:

```
input:  "intro\n```\ncode\nmore code\nEVERYTHING AFTER IS LOST\n"
blocks: [("TEXT","intro")]
dropped: ['code', 'more code', 'EVERYTHING AFTER IS LOST', '']
```

The port's correctness bar is "matches what Rust did" (see `CLAUDE.md`), and the
oracle at `rust-old/crates/mneme-io/src/export_pdf.rs:247-291` has the **same**
bug — its `for line in md.lines()` loop likewise ends without flushing
`code_buf`. So this is not a porting defect; it is a faithful reproduction of an
upstream one. Fixing it therefore requires an explicit decision to diverge.

The failure mode is the worst kind for an export path: silent, total, and
plausible. A user exports a note, gets a PDF that opens fine, and only discovers
the missing half later — if ever. Nothing in the pipeline surfaces it, because a
short PDF is indistinguishable from a short note.

## Decision

**Diverge from `rust-old/`.** After the parse loop, if `in_code` is still 1:

1. Flush `code_buf` as an `MD_CODE` block — the same shape a closing fence would
   have produced — so no input line is ever dropped. An empty buffer (a bare
   trailing ```` ``` ````) emits no block, since nothing was swallowed.
2. Set a module-global `_pdf_md_unterminated`, readable via
   `mneme_io_md_unterminated_fence()`, so a caller can surface the malformed
   input rather than accepting it silently.

The flag is a module global + accessor rather than an out-param, keeping the
parser's signature and all its call sites unchanged. This mirrors bayan's
established `bayan_json_last_error()` diagnostic idiom. It is reset at the start
of every parse, so it always describes the most recent call.

Scope: parsing only. The renderer is untouched — a flushed block flows through
the existing `MD_CODE` path.

## Consequences

- **Positive** — the export path can no longer silently lose note content, which
  was the single highest-severity defect in the PDF writer. The recovered text
  renders as code, which is what the author's fence asked for.
- **Positive** — `mneme_io_md_unterminated_fence()` gives callers (CLI export,
  the API's `/export/pdf`) a hook to warn on malformed input. Nothing consumes it
  yet; that is deliberate follow-on work, not a blocker.
- **Negative** — a documented divergence from the oracle. Cross-checking this
  function against `rust-old/` will now show a real difference, and anyone
  diffing block counts must know why. This ADR is that record.
- **Negative** — a module global carries the usual caveat: it is not
  thread-safe across concurrent parses. mneme's export paths are
  single-threaded, and bayan accepted the same trade-off for its JSON error
  slots, but a future threaded exporter must revisit it.
- **Neutral** — if the Rust oracle is ever fixed upstream, this ADR can be
  retired and the behaviors reconverge.

## Alternatives considered

- **Match Rust exactly (drop the buffer).** Rejected: preserving a silent
  data-loss bug for parity's sake fails the "correctness over cleverness"
  principle in `CLAUDE.md`. Parity is a bar for *behavior we want*, not a reason
  to ship known content loss.
- **Treat an unterminated fence as a hard error and return no blocks.**
  Rejected: strictly worse for the user — it turns a partial export into a
  failed one, when the recovered text is almost certainly what they wanted. A
  markdown flattener for print should be lenient.
- **Emit the trailing lines as paragraphs rather than code.** Rejected: the
  author opened a fence, so code is the closer reading of intent, and it keeps
  the flush path identical to the closing-fence path.
- **Change the signature to return `(blocks, unterminated)`.** Rejected as
  needless churn across every call site for a diagnostic that most callers will
  ignore; the accessor gets the same information to those who want it.
