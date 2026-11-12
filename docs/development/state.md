# mneme — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**1.2.1** — toolchain + vendored stdlib refresh (2026-11-12). Pin `6.7.9` → **`6.7.18`**. `lib/` was re-resolved
with `cyrius deps` after copying the two kept modules from the `6.7.18` stdlib. **Every file in `lib/` matches the
`6.7.18` stdlib byte-for-byte**: 86 locked files, 56 changed, 23 unchanged, and 7 new under `lib/unicode/`
(casefold, categories, normalize and their data tables), which bayan 1.11.1 includes itself. Folded libs: bayan
`1.5.13` → `1.11.1`, patra `1.16.0` → `1.16.6`, sandhi `1.10.9` → `1.13.7`, sakshi `2.5.8` → `2.5.10`, sigil
`3.13.11` → `3.13.15`. The suite is unchanged at 62 files / 1,133 assertions with **identical per-suite counts**.
- **Source change:** bayan 1.11.0 deprecated the bare TOML lookups. `src/core_config.cyr`'s three `bayan_toml_get`
  and two `bayan_toml_get_sections` calls now use their `_by_cstr` forms, which the old names forwarded to, so this
  is a pure rename. Every key is a C-string literal, including the ones passed through `_cfg_toml_str`.
- **Runtime change, no source change:** bayan 1.11.1's JSON tree parser now refuses a duplicate object key, a
  leading-zero number (`01`), a number with no digits and an out-of-range integer. mneme's only bayan JSON parse is
  the `mneme serve` request body (`src/mcp_server.cyr:104`), so such a body now gets the existing `-32700 Parse
  error` reply instead of being dispatched. No test sends one.
- **Warnings:** sigil's shared-static-buffer warning and the "large static data" warning no longer appear in the
  build or the tests.
- **Binary size:** `build/mneme` grew from 2,595,568 to 2,899,832 B (+304,264). The folded libs' source grew (bayan
  +259 KB plus 353 KB of unicode tables, patra +78 KB, sandhi +67 KB, sigil +45 KB), and mneme builds without
  dead-code elimination, so all of it is in the binary.

**1.2.0** — toolchain onto the cyrius **6.7.x language minor** + vendored stdlib refresh (2026-11-12). Pin
`6.6.20` → **`6.7.9`**; `lib/` re-resolved with `cyrius deps`, the two kept modules (`hashmap_fast`, `ws_server`)
copied from the `6.7.9` stdlib, and `cyrius.lock` re-locked. **Every file in `lib/` matches the `6.7.9` stdlib
byte-for-byte** (79 modules: 54 changed, 25 unchanged, none added). Folded libs: bayan `1.5.12` → `1.5.13`,
patra `1.15.2` → `1.16.0`, sandhi `1.10.8` → `1.10.9`, sakshi `2.5.7` → `2.5.8`, sigil `3.13.10` → `3.13.11`.
**No source change.** The suite is unchanged at 62 files / 1,133 assertions with **identical per-suite counts** at
both pins, and the build and test warnings are the same set. Every 6.7.0–6.7.9 change in `ecosystem-migration.md`
that can silently change a computed value was checked against mneme, and none applies:
- `!` was silently dropped before 6.7.3; mneme has no `!` operator anywhere.
- mneme has no `impl`, no untyped `self` and no `*T` typed pointer, so the 6.7.0 `self`, 6.7.8 narrow-`*p` and
  6.7.9 pointer-step changes can't reach it.
- mneme defines none of the new reserved words (`trait`, `const`, `true`, `false`, `do`, `dyn`) and none of the new
  stdlib names (`f64_parse_n`, `sock_set_nodelay`, `sys_ppoll`).
- patra 1.16.0 now refuses `ORDER BY` on a TEXT/BYTES chain column. mneme orders only by `notes.updated_at` (INT)
  and `tags.name` (STR, an inline comparable type), so neither query is affected.

**1.1.5** — toolchain + vendored stdlib refresh (2026-11-12). Pin `6.6.2` → **`6.6.20`**; `lib/` re-resolved
with `cyrius deps`, and **every file in `lib/` now matches the `6.6.20` stdlib byte-for-byte** (79 modules: 70
changed, 5 unchanged, 4 newly pulled in transitively — `alloc_cx` and `poison` via `alloc`, `sys` via `sigil`,
`tls_hostid` via `tls`). Folded libs: bayan `1.5.5` → `1.5.12`, patra `1.14.1` → `1.15.2`, sandhi `1.9.16` →
`1.10.8`, sakshi `2.5.1` → `2.5.7`, sigil `3.12.16` → `3.13.10`. New **`cyrius.lock`** (79 hashes): `cyrius deps`
now locks a stdlib-only project, and `cyrius deps --verify` checks it. The suite is unchanged at 62 files / 1,133
assertions with **identical per-suite counts** at both pins, so the 6.6.13 correctly-rounded `f64_parse` moved no
asserted value. One source change: bayan 1.5.10 deprecated `bayan_json_v_obj_get`, so the 8 call sites
(`src/mcp_server.cyr` ×5, `tests/mcp_server.tcyr` ×3) now call `bayan_json_v_obj_get_by_cstr`, which the old name
forwarded to. That is a pure rename with no behavior change: every key is a C-string literal.

**1.1.4** — toolchain `6.5.35` → `6.6.2` (2026-09-11). No source change.

**1.1.3** — toolchain + vendored stdlib refresh (2026-08-22). Pin `6.5.33` → **`6.5.35`**; `lib/` re-resolved
with `cyrius deps`. **Exactly two of the 73 vendored modules changed** — `bayan.cyr` and `patra.cyr` — the other 71
are byte-identical across the two toolchains, and every file in `lib/` now matches the `6.5.35` stdlib byte-for-byte.
**bayan `1.4.2` → `1.5.2`** (215,533 → 641,083 B) carries eight fixed upstream security/correctness defects (two heap overflows in `csv`, plus `base64`, `yaml`,
`cyml`, `json`, `bigint`, `bayan_u64_mulmod`); the one that
touches mneme is the **remote memory-exhaustion DoS in `json`**, because `_mcp_srv_handle` (`src/mcp_server.cyr:104`)
runs `bayan_json_v_parse_buf` directly on the socket body under `mneme serve`. **patra `1.13.9` → `1.13.10`** is a
single hunk: `patra_init` no longer calls the process-global `sakshi_set_level(SK_WARN)`. **No mneme source change
was required** — all 19 `bayan_*` and 10 `patra_*` symbols mneme calls are present and signature-unchanged — and the
suite is unchanged at 62 files / 1,122 assertions with *identical per-suite counts* before and after. `pdfcheck.sh
--strict` still passes 7/7 under bayan 1.5.2's validator. ⚠ Refresh `lib/` with **`cyrius deps`** (manifest-driven),
never `cyrius update` — the latter copies the toolchain's whole `lib/`, adding 29 modules mneme never declared.

**1.1.2** — PDF export silent-data-loss repair + toolchain `6.5.33` (2026-08-21). Two verified bugs in
`src/io_export_pdf.cyr`, both silent: (a) `mneme_io_parse_markdown_blocks` never flushed an unterminated ``` fence,
so **every line after a stray fence was dropped from the exported PDF** with no diagnostic and a valid-looking
file — now flushed as an `MD_CODE` block plus a new `mneme_io_md_unterminated_fence()` diagnostic; a **deliberate
divergence** from `rust-old/`, which carries the identical bug (ADR
[`015`](../adr/015-unterminated-code-fence.md)). (b) `mneme_io_export_note_to_pdf` discarded `file_write_all`'s
result and `mneme_io_export_notes_to_pdf` discarded `sys_mkdir`'s, so disk-full / bad-path / EACCES all reported
success with nothing written — both now propagate, and the one-level `sys_mkdir` became the stdlib's `xmkdir_p`
(= `create_dir_all`), so nested output dirs work and re-export is not an error. PDF tests strengthened 21 → 49
assertions (they previously asserted only exists / >100 bytes / `%PDF`, and caught neither bug; the new suite
fails 7 against the pre-fix code) and gated by `scripts/pdfcheck.sh` over bayan's strict validator — all 7
fixtures pass, including 5-page/13-object. Also: `tests/mcp_server.tcyr` ended `return 0`, so its failures could
never fail the build; `scripts/bump-version.sh` still sed-ed a `Cargo.toml` that has not existed since the port.
Toolchain pin `6.4.58` → **`6.5.33`** with `lib/` re-resolved (51 modules) and bayan's
`bayan_json_v_parse_str` → `bayan_json_v_parse_buf` rename applied.

**1.1.1** — MCP self-registration sends the tool `inputSchema` (2026-07-13). The 1.1.0 `_mcp_reg_body`
(`src/mcp_server.cyr`) POSTed only `name`/`description`/`callback_url` to daimon, **omitting the schema**, so daimon
advertised every `mneme_*` tool with an empty `{}` — which is not a valid Anthropic `input_schema` (`{"type":"object",…}`
required), so an Anthropic-backed consumer (thoth→hoosh) had its whole tools request rejected (empty completion /
502) whenever mneme was in the registry. Registration now serializes the schema (new `_mcp_schema_json`:
`{"type":"object"}` + typed-string `properties` + `required` for a tool's required fields) and orders `callback_url`
**before** the nested `inputSchema` (daimon's flat parser 400s otherwise). Unit-tested (`tests/mcp_server.tcyr`) +
live-verified (daimon advertises typed `mneme_*` schemas; a thoth full-registry agentic turn completes). Consumer
thoth also hardened (0.34.2) to tolerate any empty/typeless schema. Toolchain pin `6.4.58`.

**1.1.0** — served MCP endpoint (2026-07-12): **`mneme serve [port]`** (default 8100) runs an MCP endpoint over
the existing in-process dispatch (`src/mcp_server.cyr`; socket + HTTP wire via sandhi, tool logic via
`mneme_mcp_handle_tool_call`) so daimon can HOST mneme (external registration) and thoth's memory seam consumes it
— mneme as a served domain like hoosh. Adds a JSON-RPC envelope parse + a JSON→args-vec converter + daimon
self-registration (POST the 8 tool defs on start when `MNEME_DAIMON_URL` is set). Live-verified end-to-end: thoth
`/remember` → daimon → mneme round-trips a real note. Hardened against a remote DoS (non-string method/name null
-deref, caught in adversarial review). Toolchain pin `6.4.57` → `6.4.58`.

**1.0.1** — launch repair (2026-07-12): the TUI vault root is now resolved to a
WRITABLE location (`MNEME_VAULT` → `$HOME/.mneme` → `/mneme`) instead of the
hardcoded `/mneme`, so a bare launch off-AGNOS no longer dies at patra open
("cannot open or create file"). Toolchain pin refreshed; `cyrius.cyml` version now
reads `${file:VERSION}` (single source of truth).

**1.0.0** — ported from Rust (2026-07-09) via `cyrius port`. **21,014 lines** of
Rust preserved at `rust-old/` (the full original workspace) for parity reference.

## Toolchain

- **Cyrius pin**: `6.7.18` (in `cyrius.cyml [package].cyrius`)
- **Vendored stdlib**: bayan `1.11.1`, patra `1.16.6`, sandhi `1.13.7`, sakshi `2.5.10`, sigil `3.13.15`. Refresh
  with `cyrius deps` (never `cyrius update`), then `cyrius deps --verify` against `cyrius.lock`.
  ⚠ `lib/hashmap_fast.cyr` and `lib/ws_server.cyr` are **not declared** in `[deps].stdlib` and nothing includes
  them, so `cyrius deps` never refreshes them. They are **kept on purpose** (decided at 1.1.5), so every pin bump
  refreshes them by hand. **Copy them FIRST, then resolve:**
  `cp ~/.cyrius/versions/<pin>/lib/{hashmap_fast,ws_server}.cyr lib/`, then `cyrius deps`, then
  `cyrius deps --verify`. Since 6.7.18, `cyrius deps` refuses to re-lock across a pin move while an undeclared
  `lib/` file differs from the new snapshot. It suggests `rm` or `cyrius lib sync --full`; don't use either. Once
  the copies match, it locks them and notes them as "outside the declared footprint".

## Source

- Rust reference: 21,014 lines at `rust-old/` (frozen, do not edit — parity oracle).
- Cyrius port in progress:
  - **M1 `mneme-core` — COMPLETE.** All 9 modules ported into `src/core_*.cyr`
    (tag, frontmatter, note, link, graph, plugin, calendar, task, config) + a
    `core_uuid` helper (v4 random / v5 sha1).
  - **M2 `mneme-io` / `mneme-store` — COMPLETE.** `io_*` (wikilink, notion,
    web_clipper, obsidian, markdown, export_html) + `store_*` (versioning,
    sharing, registry, db [patra], files [real SHA-256], vault, manager).
  - **M3 `mneme-search` — COMPLETE.** All 10 modules: query_dsl,
    retrieval_optimizer, context_buffer, cross_vault, engine (inverted-index
    BM25, built from scratch — no Tantivy analog), vector_store (brute-force
    cosine — no usearch analog), embedder, embedding_backend, semantic
    (RRF hybrid merge), semantic_engine. Live ONNX/HTTP `embed()` DEFERRED to
    the daimon/sandhi bridge (backlog); all config/dispatch/ranking is ported.
  - **M4 `mneme-ai` — COMPLETE.** All 19 test-bearing modules (~5,405 LOC):
    pure tier (concepts, tagger, temporal, templates, training_export, clustering
    [deterministic K-means++], rag_eval, consolidation) + bridge tier (client,
    summarizer, linker, event_bus, rag, writer, flashcards, translator, qa_bridge,
    multimodal, creative). Every LLM `generate()`/embedding/HTTP call is DEFERRED
    to the daimon/sandhi bridge; the local fallbacks, extraction, serde, and
    scheduling logic the Rust tests exercise are all ported.
  - **M6 `mneme-mcp` — COMPLETE.** protocol (JSON-RPC 2.0 helpers + 8 tool
    schemas) + tools (full tool-dispatch: create/search/get/update note,
    query_graph, search_feedback, list/switch vault). The tool integration
    tests run against a real on-disk vault, composing VaultManager + SearchEngine
    + SemanticEngine(disabled) + RetrievalOptimizer. JSON `serde_json::Value`
    args are modelled as a tagged key→value object; daimon/HTTP paths deferred.
  - **M7 `mneme-ui` (TUI) — COMPLETE.** `ui_app` (state), `ui_render` (all 9
    panel views + status bar → in-memory `vec<Str>` lines, since there's no
    ratatui analog), `ui_events` (per-panel `handle_key` dispatch). views.rs and
    main.rs had 0 Rust tests, so `ui_render`/`ui_events` add characterization
    tests. Only the raw terminal adapter (crossterm/ratatui frame → ANSI over
    `darshana`) is deferred — the TUI equivalent of the HTTP/embedding bridges.
  - **M5 `mneme-api` — COMPLETE.** `api_server` ports the full HTTP surface as an
    in-process router: `handle_request(state, method, path, body) → (status,
    content_type, body)`. Since `tower::oneshot` drives the axum router without a
    socket, no live HTTP server is needed — all 30 `tests/api_integration.rs`
    cases pass. Covers notes CRUD, search, tags, AI endpoints (concepts,
    suggest-tags, summarize, write, languages, translate, temporal, rag-stats),
    templates, tasks, calendar, flashcards, clip (html/bookmark), plugins, and
    PDF export. serde_json request bodies are hand-parsed (string/bool/array,
    with `\n`/`\t` unescaping); the daimon client / event bus stay deferred
    (daimon always reported unavailable → local-fallback paths).
  - **`mneme-io/export_pdf` — COMPLETE (hand-rolled).** `io_export_pdf` is a real
    PDF writer (catalog/pages/font objects + byte-accurate xref; markdown → text
    blocks with word-wrap + pagination; strip-inline / block-parse / wrap-text
    helpers). Backs the API `/export/pdf`. As of 1.1.2 the writer is gated by
    bayan's strict validator (`scripts/pdfcheck.sh`) rather than by "starts with
    %PDF" — every xref offset, `/Length` and `/Size` is checked against the real
    bytes, which lenient viewers hide by rebuilding the table from a scan.
    `bayan_pdf_*` has since landed (bayan 1.5.0): **mneme keeps its markdown
    layer and migrates only the PDF primitives**, so the unterminated-fence fix
    (ADR 015) stays mneme's.
  - **Network + terminal bridges — WIRED (no longer deferred).**
    - `net_http` — live HTTP over **`sandhi`** (works on AGNOS via sock_connect/
      send/recv). Remote embeddings (`search_embedding_backend`'s embed hook →
      OpenAI-compatible `POST {base}/v1/embeddings`, installed by an fnptr hook so
      the 7 config-test consumers stay HTTP-free) + the daimon REST client
      (`/health`, `/v1/rag/query|ingest`). Request-build + response-parse are
      unit-tested; the socket round-trip runs against a live daimon.
    - `ui_terminal` — real console render (ANSI clear+home over `sys_write` fd 1) +
      stdin key decoder (`sys_read` fd 0, arrow/ctrl escape sequences) + the event
      loop. `src/main.cyr` is now the real TUI entry (`cyrius build` → a working
      binary) that installs the embedding hook and runs the loop.
  - **Port complete.** The only bridged-out path is **local ONNX inference**
    (`build_local` → sovereign ML stack akshara/rosnet/rupantara), now a **P2
    backlog** item that lands with the ML-AI stack as a whole. Remote embeddings
    (daimon) cover the path meanwhile; the AI modules keep their local fallbacks.

Note: `search_semantic_engine`'s functions were renamed to the
`mneme_search_semantic_engine_*` namespace (they previously shared
`mneme_search_engine_*` with the full-text engine) so both compose in the MCP layer.

## Tests

**62 `.tcyr` files, 1,133 assertions — all green.** The entire 21,014-LOC workspace is ported (core,
io incl. PDF, store, search, ai, mcp, full api HTTP surface, ui app), plus the
mneme-ui `views`/`main` glue (`ui_render`, `ui_events`) and the **live network +
terminal bridges** (`net_http` = sandhi HTTP to daimon + remote embeddings;
`ui_terminal` = real console I/O; `src/main.cyr` builds to a working TUI binary).
Modules that need a live server keep a graceful-degrade path (verified fast/green);
their pure request/response + hook wiring is unit-tested. Only local ONNX inference
(the sovereign ML stack) remains bridged-out.

## Dependencies

Direct (declared in `cyrius.cyml [deps].stdlib`): string, fmt, alloc, vec, str,
hashmap, math, chrono, random, sha1, ct, keccak, thread, thread_local, slice,
sigil, tagged, fnptr, bayan, patra, fs, syscalls, io, args, assert, **net, http,
tls, ws, async, atomic, sandhi, fdlopen, dynlib, mmap, freelist, process, sakshi**.

- Carves (explicit `include "lib/X.cyr"` in tests): `patra`, `bayan`, `sigil`
  (+ `ct`/`keccak`). `sandhi`/`net`/`tls`/`fnptr` auto-resolve from stdlib.
- `sandhi` (HTTP) backs `net_http` (daimon + remote embeddings). `patra` (embedded
  SQL) backs `mneme-store`. `sigil` = real SHA-256.

## Consumers

`src/main.cyr` — the TUI binary (builds via `cyrius build src/main.cyr build/mneme`).

## Next

Port complete — pure logic, the full API surface, the TUI, and the network/terminal
bridges are all done and green. Everything remaining is [`roadmap.md`](roadmap.md)
P2 backlog: local ONNX inference (lands with the ML-AI stack), `bayan_markdown`/
`bayan_pdf` migration, and minor polish (canonical uuid string, graph layout,
RFC3339 datetime).

New in 1.1.3 — **bayan's PDF module was shaped around auditing mneme's hand-rolled writer**, and its findings
name defects that are still live here. `bayan/src/pdf.cyr` cites `src/io_export_pdf.cyr` seven times, and bayan
1.5.0's CHANGELOG says the hand-roll "is superseded here, and the defects found while reading it are what this
module is shaped around". Triaged against mneme at 1.1.3 — **none is caught by the 62-suite run or by
`pdfcheck.sh`**, which validates *structure* (xref/`/Length`/`/Size`), not text encoding or layout:

- ⛔ **LIVE — no `/Encoding` on the font dict.** `io_export_pdf.cyr:391` emits
  `<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>`, so viewers fall back to StandardEncoding, where only 54 of
  the 128 high codes exist: `Résumé` renders `RÃ©sumÃ©` and an em-dash disappears. A **silent content change on
  export** — the same class as the ADR-015 fence bug. bayan ships `bayan_pdf_from_winansi` + declares
  `/WinAnsiEncoding` (it does both, since declaring without transcoding only changes the flavour of mojibake).
- ⛔ **LIVE — wrapping counts characters, not width.** `io_export_pdf.cyr:330,346` call
  `mneme_io_wrap_text(..., 80)`. In a proportional face 80 characters spans a **4.26× range**: 80 `l` is 195 pt in
  a 451 pt column, 80 `W` is 831 pt — **308 pt off a 595 pt page**. Confirmed against the live geometry
  (`MediaBox[0 0 595 842]`, x=72, 11 pt). bayan ships `bayan_pdf_text_width` / `bayan_pdf_wrap` on real AFM metrics.
- ⛔ **LIVE — `_pdf_esc` silently deletes newlines.** `io_export_pdf.cyr:250` is
  `if (c != 10 && c != 13) { str_builder_putc(sb, c); }` — CR and LF are dropped with no diagnostic. bayan
  *rejects* a newline inside show-text instead: it means "break the run", and accepting-then-dropping it is a
  silent content change.
- ⚠ **Brittle, correct today** — `var offset = 9;` (`:412`) predicts the first xref offset from the length of
  `"%PDF-1.4\n"`, and object numbers use closed-form `4+2p`/`5+2p`. Both are right for the current single-line
  header and fixed object set, and `pdfcheck.sh` passes 7/7 — but each silently becomes wrong the moment a second
  header line, `/Info`, a second font, or an `/ID` is added. bayan measures offsets off the builder and counts
  objects instead.
- ✅ **Already fixed here — bayan's audit is stale on this one.** It lists "mneme ignores `file_write_all`'s
  return and reports success on a full disk"; that was repaired in **1.1.2** (`io_export_pdf.cyr:452`).

Also surfaced by the bayan fold, in a different module and **fixed in this release**: `core_config.cyr` wrote
TOML values **unescaped** — vault `name`/`path`/`description`, `default_vault` and the embedding `backend` were
interpolated straight between literal quotes, so a value holding a `"`, a newline or a backslash emitted malformed
TOML and silently lost data on re-read. `rust-old` escaped correctly via `toml::to_string`, so this was a port
regression, repaired as parity (no ADR). Values are now escaped on write, and — because **bayan does not unescape
TOML basic strings**, returning the raw slice with backslashes intact — all 8 string-value reads go through
`_cfg_toml_str` (`bayan_toml_get` + unescape). ⚠ **Those two halves are a matched pair**: if bayan ever grows TOML
unescaping, mneme would double-decode. The `toml escape` assertions in `tests/core_config.tcyr` exist to fail
loudly at that moment. Tests 13 → 24 assertions, **6 of the 11 new ones mutation-proven against the pre-fix
writer**.

The bayan upgrade independently improved how the *old* bug failed: an unterminated basic string used to scan across
newlines and swallow the following config lines into one value, and now stops at the newline. Moot after the fix,
but it is why the defect surfaced here.

These strengthen the case for the now-unblocked `bayan_pdf_*` primitives migration in
[`roadmap.md`](roadmap.md) — it retires three live defects and two brittle constants at once, and makes the
+199 KB of `bayan_pdf_*` the binary already carries into the code actually doing the work. **Not actioned in
1.1.3**, which is a toolchain/dependency refresh with no mneme source change.

Carried forward from 1.1.2:

- **`bayan_pdf_*` has landed** (bayan 1.5.0), so the PDF migration is now
  actionable. Scope is the **primitives only** — the object/xref/content-stream
  writer in `io_export_pdf.cyr`. mneme keeps its own markdown layer, so
  `mneme_io_parse_markdown_blocks` and the ADR-015 unterminated-fence behavior do
  **not** migrate. `scripts/pdfcheck.sh` is the shared acceptance gate on both
  sides of that swap.
- **Nothing consumes `mneme_io_md_unterminated_fence()` yet.** The parser records
  a malformed fence but no caller surfaces it: the CLI export path and the API's
  `/export/pdf` still accept the input silently (they no longer *lose* content —
  that was the 1.1.2 fix — but they do not warn). Wiring a warning through those
  two callers is the follow-on.
- **`mneme_io_export_pdf`** (the `/export/pdf` wrapper) renders in-memory and has
  no write path, so it needed no error propagation. If it ever grows one, it
  should return the same negative-errno contract as `mneme_io_export_note_to_pdf`.
