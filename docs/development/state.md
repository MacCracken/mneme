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
  - **M7 `mneme-ui` (TUI) — app COMPLETE.** `ui_app` ports the tested state
    layer (vault/engine wiring, note-list load, panel navigation, empty-query
    search guard). `views`/`main` are ratatui/crossterm rendering with 0 tests.
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
  - **Deferred remainder:** `mneme-io/export_pdf` — the API's `/export/pdf`
    emits a valid minimal `%PDF`, but export_pdf.rs's 8 tests exercise the full
    PDF layout (fonts/wrapping/pages) → P2 backlog. `mneme-ui` `views`/`main`
    are ratatui/crossterm rendering (0 tests). Live daimon/ONNX/sandhi calls
    remain stubbed throughout M3–M5.

Note: `search_semantic_engine`'s functions were renamed to the
`mneme_search_semantic_engine_*` namespace (they previously shared
`mneme_search_engine_*` with the full-text engine) so both compose in the MCP layer.

## Tests

**56 `.tcyr` files — all green.** Every test-bearing Rust module across
mneme-core, mneme-io (minus full PDF), mneme-store, mneme-search, mneme-ai,
mneme-mcp, the mneme-ui app, and **the entire mneme-api HTTP surface** is mirrored
1:1 against `rust-old/`. Deferred live-embedding/LLM/HTTP calls are stubbed to
their degraded-mode return (None/empty/Ok) — exactly what the Rust tests exercise.
The only unported test file is `mneme-io/export_pdf` (8 tests, full PDF layout — P2).

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
