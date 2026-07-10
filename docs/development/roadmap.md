# mneme — Roadmap

> Milestone plan for the Rust → Cyrius port. State lives in
> [`state.md`](state.md); this file is the sequencing.

## v1.0 criteria

- [ ] Rust → Cyrius surface parity (each ported module mirrors `rust-old/`'s
      `#[cfg(test)]` cases 1:1)
- [ ] Benchmarks captured in `docs/benchmarks.md`
- [ ] At least one downstream consumer green
- [ ] CHANGELOG complete from v0.1.0 onward
- [ ] Security audit pass (`docs/audit/YYYY-MM-DD-audit.md`)

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

### M3 — `mneme-search` — ▶ in progress
Pure-logic first (query_dsl, retrieval_optimizer, context_buffer, cross_vault), then
the hard subsystems: full-text = **build-in-mneme BM25 index** (Tantivy has no Cyrius
analog); vectors = brute-force cosine via `simd`/`ganita`; embeddings = **bridge to
daimon** (or the sovereign ML stack: akshara/rosnet/rupantara/anukulana).

### M4 — `mneme-ai` (RAG/summarize/concept/consolidate) — HTTP to daimon
### M5 — `mneme-api` (sandhi HTTP server) · M6 — `mneme-mcp` (bote) · M7 — TUI (darshana)

## Post-v1 / P2 backlog (deferred, non-blocking)

Skipped intentionally during the port; revisit after v1.0 parity lands:

- **`mneme-io` export_pdf** — PDF export (printpdf). Lowest value-per-effort; HTML
  export covers most needs. Needs a minimal PDF writer or an external service.
- **`mneme-io` import_notion async import** — the directory-walk `import_notion_export`
  (the 4 pure cleanup fns are already ported in `io_notion`). Mechanical: reuse
  `io_notion` + the `io_obsidian` fs-walk pattern.
- **Real markdown parser via `bayan`** — currently a hand-rolled subset
  (`mneme_io_markdown_to_html`); migrate to `bayan_markdown_*` once it ships (already
  on bayan's roadmap).
- **Canonical uuid string in `core_uuid`** — `_db_uuid_to_str`/`_from_str` exist in
  store_db/store_registry; promote a `mneme_core_uuid_to_string`/`_parse` to core_uuid.
- **`GraphLayout::from_subgraph`** — the f64 force-directed spring layout (untested by
  the Rust suite; deferred with the f64-math work).
- **Real datetime formatting** — chrono civil-calendar (RFC3339 format/parse beyond the
  year/month extraction already done); timestamps currently stored as INT ns.

## Out of scope (for v1.0)

- Native GUI (dhancha) — TUI is the v1 UI target; dhancha GUI is a post-v1 level-up.
- Local ONNX inference in-process — embeddings bridge to daimon/ML-stack instead.
