# mneme — Roadmap

> Sequencing only. Live state is in [`state.md`](state.md); shipped history is in
> [`CHANGELOG.md`](../../CHANGELOG.md).

The Rust → Cyrius port is **complete** (v1.0.0, 2026-07-10). Every test-bearing module of the 21,014-LOC workspace
mirrors `rust-old/`; on top of that sit the TUI, the daimon and remote-embedding bridges over `sandhi`, and the
served MCP endpoint (`mneme serve`, 1.1.0). The releases since have been toolchain refreshes and repairs.

Shipped since and recorded in the CHANGELOG:
- **1.2.2** — the roadmap reorganisation and the Notion directory import;
- **1.2.3** — the P(-1) audit / refactor / hardening / optimization / security sweep. Its findings and repairs are
  in [`../audit/2026-11-12-audit.md`](../audit/2026-11-12-audit.md), and the items it deferred are listed below.
- **1.3.0** — PDF export on `bayan_pdf_*`
  ([ADR 017](../adr/017-pdf-export-on-bayan.md)). It retired the three live defects: no `/Encoding`, wrapping by
  character count, and CR/LF deleted. It also replaced the predicted xref offset and surfaced the unclosed-fence
  diagnostic.

## 1.3.x — the document pipeline on bayan

- **1.3.1 — `bayan_markdown_*` for HTML export.** This is now unblocked: bayan 1.11.1 ships `bayan_markdown_parse`
  (to an AST) and `bayan_markdown_to_html`. It replaces the hand-rolled subset in `mneme_io_markdown_to_html`
  (`io_export_html`).
- **When bayan's PDF gaps close** (bayan issue `2026-11-05-pdf-writer-gaps-for-a-qa-report`):
  - adopt a wrap that breaks a word wider than the column (gap 2: a long URL still runs off the page);
  - adopt embedded TrueType fonts (gap 3), so text outside WinAnsi stops being exported as `?`;
  - drop mneme's own refused-run check once bayan's write refuses an errored document itself (gap 1).

## 1.4.x — remaining parity polish and robustness

- **RFC 3339 datetime parsing.** Formatting is real since 1.2.3: clips use chrono's `epoch_to_date`. Parsing still
  only takes a date prefix (`store_db`'s parse_datetime). chrono has no parser, so this either lands in chrono first
  or is hand-rolled here. Timestamps stay stored as INT nanoseconds.
- **`GraphLayout::from_subgraph`** — the f64 force-directed spring layout (`core_graph`; no Rust test).
- **Non-ASCII text** (2026-11-12 audit).
  - Slugs (`store_files`, `io_wikilink`, `io_notion`, `io_export_pdf`) map every byte ≥ 0x80 to `-`. An all-CJK
    title becomes `.md`, and "Café" and "Cafè" collide.
  - The full-text tokenizer splits on those bytes, so CJK notes are unsearchable.
  - Obsidian tag trimming drops them.

  Rust uses `char::is_alphanumeric`. `lib/unicode/categories` has been vendored via bayan since 1.2.1.
- **Titles over 255 bytes.** patra STR columns hold 255 bytes, and since 1.2.3 a longer title fails the create
  instead of silently losing the row. Supporting them needs TEXT columns, which patra cannot compare in WHERE or
  ORDER BY, so lookups move off those columns.
- **`mneme serve` robustness.**
  - Memory grows with every request: the bump allocator never frees, and bayan's JSON tree is the largest share.
    This needs a per-request arena plus deep copies of whatever a tool stores.
  - A shared-secret header is needed for a server bound beyond loopback (`MNEME_BIND`); daimon has to send it.
- **Config round-trip.** `core_config`'s `from_toml` drops `[context_retrieval]` and the remote-embedding keys that
  Rust's serde keeps, and `_cfg_f64_str` mishandles negative and very large values. Nothing in `src/` calls these
  yet.
- **TUI input.** There is no raw mode (termios), and a read returning several bytes decodes only the first key.
- **Identity by value.** `store_versioning` keys history by the uuid handle's address, and `search_context_buffer`
  compares handles. Neither is wired yet; the context-buffer test uses integer stand-in ids.
- **Frontmatter injection.** A title holding `\n---` injects keys. Rust behaves the same, so this would be a
  divergence and needs an ADR.
- **Temporal API.** It returns only `total_notes`, where Rust serializes the whole `TemporalReport`.

## 1.5.x — performance

These are the ranked targets in [`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md). The port is
correct, not yet fast, by design.

- **ANN vector index (#1)** — HNSW-style, plus `simd`/`ganita` f64 lanes in place of the brute-force cosine scan.
  This is the only path that scales with vault size.
- **True BM25 scoring (#3)** — tf-idf ranking, posting-list skip and pre-sized vecs, in place of rank-by-match-count.
  1.2.3 already cut the 200-doc search from about 310 to 103 µs with a hash-set term dedup and bucket ranking.
- **Tokenizer (#2)** — reuse a scratch arena instead of a `str_builder` and allocation per token.
- **Value packing (#4)** — small structs off the heap, and f32 lanes where precision allows.
- **Quadratic AI passes** (2026-11-12 audit):
  - `ai_consolidation` is O(n²·U²) over linear-scan sets, about 4.5·10¹⁰ comparisons for 1k notes × 300 tokens;
  - `ai_concepts`' frequency dedup scans linearly;
  - about ten copy-pasted bubble sorts remain.

  Fix with sorted merges, `map_str` sets and one stable merge sort. (UUID entropy, #5, was done in 1.2.3: 550 →
  133 ns.)

## 2.0.0 backlog

- **Local ONNX inference** through the sovereign ML stack: akshara (BPE tokenizer) → rosnet (tensors/BLAS) →
  rupantara (transformer forward) → tula (safetensors/GGUF) → anukulana (load pretrained). It lands when the ML-AI
  stack is worked on as a whole. `build_local` degrades today, and remote embeddings via daimon cover the path; the
  embed hook and `SemanticEngine`'s degraded path mean nothing else changes when it lands.
- **Native GUI on `dhancha`.** The TUI stays the shipping UI until then.
- With live embeddings, `search_semantic_engine` must take its dimension from the backend; it hard-codes 384.
