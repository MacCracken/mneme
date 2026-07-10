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
  - **M5 `mneme-api` — DEFERRED (HTTP bridge).** The handlers have no unit tests;
    `tests/api_integration.rs` (30 tests) drives the axum router via
    `tower::oneshot`. Ports with the sandhi HTTP router/server bridge, alongside
    the deferred daimon client / event bus / embeddings.
  - **Deferred remainder (bridge tier):** `mneme-api` (30 HTTP tests),
    `mneme-io/export_pdf` (8 tests — needs a PDF lib; P2 backlog), and the live
    daimon/ONNX/sandhi calls stubbed throughout M3/M4. All are the same
    "external bridge" class; the local/pure logic is fully ported.

Note: `search_semantic_engine`'s functions were renamed to the
`mneme_search_semantic_engine_*` namespace (they previously shared
`mneme_search_engine_*` with the full-text engine) so both compose in the MCP layer.

## Tests

**55 `.tcyr` files — all green.** Every test-bearing Rust module across
mneme-core, mneme-io (minus PDF), mneme-store, mneme-search, mneme-ai, mneme-mcp,
and the mneme-ui app is mirrored 1:1 against `rust-old/`. Deferred
live-embedding/LLM/HTTP calls are stubbed to their degraded-mode return
(None/empty/Ok) — exactly what the Rust tests for those modules exercise.
The remaining unported test files are all the external-bridge tier (mneme-api
HTTP, export_pdf).

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
