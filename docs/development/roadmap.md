# mneme — Roadmap

> Milestone plan for the Rust → Cyrius port. State lives in
> [`state.md`](state.md); this file is the sequencing.

## v1.0 criteria — ✅ met (v1.0.0 shipped 2026-07-10)

- [x] Rust → Cyrius surface parity (each ported module mirrors `rust-old/`'s
      `#[cfg(test)]` cases 1:1) — 61 `.tcyr` files, all green.
- [x] Benchmarks captured in `docs/benchmarks.md`
- [x] At least one downstream consumer green — `src/main.cyr` → working TUI binary;
      full suite green.
- [x] CHANGELOG complete from v0.1.0 onward — see `[1.0.0]`.
- [x] Security audit pass (`docs/audit/2026-07-10-audit.md`) — no port-introduced
      high/critical findings.

## Milestones

### M0 — Port scaffold — ✅ done (2026-07-09)
`cyrius port`; Rust → `rust-old/` (21,014 LOC oracle); manifest + CI.

### M1 — `mneme-core` — ✅ done
10 modules (tag, frontmatter, note, link, graph, plugin, calendar, task, config,
uuid), 99 assertions green.

### M2 — `mneme-store` + `mneme-io` — ✅ done (bar P2 backlog below)
- store (7): db (patra), files, versioning, sharing, registry, vault, manager — 154 asserts.
- io: wikilink, notion, web_clipper, obsidian, markdown, export_html (+ a hand-rolled
  markdown→HTML renderer).

### M3 — `mneme-search` — ✅ done
All 10 modules. Full-text = **build-in-mneme BM25 index** (Tantivy has no Cyrius
analog); vectors = brute-force cosine; embeddings config/dispatch/ranking ported,
the live embed() **bridge to daimon** deferred (degraded path = what the tests hit).

### M4 — `mneme-ai` — ✅ done
All 19 test-bearing modules (pure + bridge tiers). Local fallbacks / extraction /
SM-2 / serde ported; daimon LLM/embedding/HTTP calls deferred.

### M6 — `mneme-mcp` (bote) — ✅ done
protocol (JSON-RPC 2.0 + 8 tool schemas) + tools (full dispatch, tested against a
real on-disk vault composing store + search + optimizer).

### M7 — TUI — ✅ done (incl. terminal adapter)
`ui_app` (state), `ui_render` (all 9 panel views → line buffer, no ratatui analog),
`ui_events` (`handle_key`), and **`ui_terminal`** — the real console adapter (ANSI
render over `sys_write`, stdin key decode over `sys_read`, event loop). `src/main.cyr`
is the working TUI entry (`cyrius build` → binary). Characterization tests cover the
renderers, event dispatch, and key decoder.

### Network bridge — ✅ done (daimon + remote embeddings over `sandhi`)
`net_http` wires the previously-deferred HTTP: remote embeddings (embed-hook →
`POST /v1/embeddings`) + the daimon REST client (`/health`, `/v1/rag/*`). Runs on
AGNOS via sandhi's TCP syscalls; graceful-degrades when unreachable. Only local
ONNX inference (sovereign ML stack) is still bridged out.

### M5 — `mneme-api` — ✅ done
Full HTTP surface as an in-process router (`handle_request(method, path, body) →
(status, content_type, body)`). `tower::oneshot` drives axum without a socket, so
no live server is needed — all 30 `tests/api_integration.rs` cases pass. daimon
client / event bus stay deferred (reported unavailable → local-fallback paths).

## Post-v1 / P2 backlog (deferred, non-blocking)

Skipped intentionally during the port; revisit after v1.0 parity lands:

- **Local ONNX inference — in-process embeddings via the sovereign ML stack.** The
  only remaining bridged-out path. Remote embeddings already work (daimon over
  `sandhi`, `POST /v1/embeddings`), so this is the *offline* alternative:
  `build_local` currently returns None (degraded) when `all-MiniLM-L6-v2.onnx`/
  `tokenizer.json` are present. Wiring = akshara (BPE tokenizer) → rosnet (tensor/
  BLAS) → rupantara (transformer forward) → tula (safetensors/GGUF) → anukulana
  (load pretrained). **Lands when the ML-AI stack is worked on as a whole** (the
  embed hook + `SemanticEngine` degraded path mean nothing else changes when it does).
- **`mneme-io` import_notion async import** — the directory-walk `import_notion_export`
  (the 4 pure cleanup fns are already ported in `io_notion`). Mechanical: reuse
  `io_notion` + the `io_obsidian` fs-walk pattern.
- **Real markdown parser via `bayan`** — currently a hand-rolled subset
  (`mneme_io_markdown_to_html`); migrate to `bayan_markdown_*` once it ships (already
  on bayan's roadmap).
- **Real PDF read/write via `bayan`** — `io_export_pdf` is now a hand-rolled PDF
  writer (valid catalog/pages/font objects + xref; markdown flattened to text
  blocks with wrapping + pagination; all 8 export_pdf tests + the API `/export/pdf`
  pass). Migrate to `bayan_pdf_*` once it ships (added to bayan's roadmap) — that
  brings embedded fonts, images, and a PDF *reader* (for import) the hand-roll skips.
- **Canonical uuid string in `core_uuid`** — `_db_uuid_to_str`/`_from_str` exist in
  store_db/store_registry; promote a `mneme_core_uuid_to_string`/`_parse` to core_uuid.
- **`GraphLayout::from_subgraph`** — the f64 force-directed spring layout (untested by
  the Rust suite; deferred with the f64-math work).
- **Real datetime formatting** — chrono civil-calendar (RFC3339 format/parse beyond the
  year/month extraction already done); timestamps currently stored as INT ns.
- **Performance (post-v1)** — the port is correct-not-fast by design. Ranked
  optimization targets (vector ANN index, tokenizer arena reuse, real BM25 scoring,
  f64/value-type packing) are in [`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md).

## Out of scope (for v1.0)

- Native GUI (dhancha) — TUI is the v1 UI target; dhancha GUI is a post-v1 level-up.
  (Local in-process ONNX inference moved to the P2 backlog above — it lands with the
  sovereign ML stack; remote embeddings via daimon cover the path meanwhile.)
