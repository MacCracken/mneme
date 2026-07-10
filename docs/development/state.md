# mneme — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**0.1.0** — ported from Rust (2026-07-09) via `cyrius port`. **21,014 lines** of
Rust preserved at `rust-old/` (the full original workspace) for parity reference.

## Toolchain

- **Cyrius pin**: `6.4.42` (in `cyrius.cyml [package].cyrius`)

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
    helpers). All 8 export_pdf tests pass, and it backs the API `/export/pdf`.
    Migrate to `bayan_pdf_*` when it lands (on bayan's roadmap) — same play as the
    markdown subset.
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
  - **Still deferred:** local ONNX inference (`build_local` → the sovereign ML
    stack akshara/rosnet/rupantara) — the only remaining external bridge. The AI
    modules keep their local fallbacks (the daimon client is available for
    higher-level dispatch when wired).

Note: `search_semantic_engine`'s functions were renamed to the
`mneme_search_semantic_engine_*` namespace (they previously shared
`mneme_search_engine_*` with the full-text engine) so both compose in the MCP layer.

## Tests

**61 `.tcyr` files — all green.** The entire 21,014-LOC workspace is ported (core,
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

See [`roadmap.md`](roadmap.md). Pure logic + network/terminal bridges are complete.
Remaining: local ONNX inference (sovereign ML stack), and optional higher-level
AI-module → daimon dispatch (modules currently prefer their local fallbacks).
