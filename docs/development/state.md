# mneme — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**0.1.0** — ported from Rust (2026-07-09) via `cyrius port`. **21,014 lines** of
Rust preserved at `rust-old/` (the full original workspace) for parity reference.

## Toolchain

- **Cyrius pin**: `6.4.41` (in `cyrius.cyml [package].cyrius`)

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
  - **Deferred (external bridges only):** the raw terminal adapter (`darshana`
    ANSI/termios) under `ui_render`/`ui_events`, and the live daimon/ONNX/sandhi
    calls stubbed throughout M3–M5. No pure logic remains unported.

Note: `search_semantic_engine`'s functions were renamed to the
`mneme_search_semantic_engine_*` namespace (they previously shared
`mneme_search_engine_*` with the full-text engine) so both compose in the MCP layer.

## Tests

**59 `.tcyr` files — all green.** **The entire 21,014-LOC workspace is ported** —
every test-bearing Rust module 1:1 against `rust-old/` (core, io incl. PDF, store,
search, ai, mcp, api HTTP surface, ui app), **plus** the mneme-ui `views`/`main`
rendering + event glue (which had no Rust tests) now carried by characterization
tests (`ui_render`, `ui_events`). Deferred live-embedding/LLM/HTTP calls are
stubbed to their degraded-mode return (None/empty/Ok). The only code NOT ported is
the raw terminal adapter (darshana ANSI/termios) — pure logic is 100% done.

## Dependencies

Direct (declared in `cyrius.cyml [deps].stdlib`):

- string, fmt, alloc, vec, str, hashmap, math, chrono, random, sha1, tagged,
  fnptr, bayan, syscalls, io, args, assert
- `bayan` (TOML/JSON) is a carve — also explicitly `include "lib/bayan.cyr"`.
- Planned: `patra` (embedded SQL) for `mneme-store`; sibling deps (`bote`, daimon
  HTTP) in later milestones.

## Consumers

_None yet._

## Next

See [`roadmap.md`](roadmap.md). Finish M2's pure-logic modules, then the
`fs` + `patra` + async I/O tier (store/db, files, vault, manager; io imports).
