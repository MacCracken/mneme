# mneme — Roadmap

> Sequencing only. Live state is in [`state.md`](state.md); shipped history is in
> [`CHANGELOG.md`](../../CHANGELOG.md).

The Rust → Cyrius port is **complete** (v1.0.0, 2026-07-10). Every test-bearing module of the 21,014-LOC workspace
mirrors `rust-old/`; on top of that sit the TUI, the daimon and remote-embedding bridges over `sandhi`, and the
served MCP endpoint (`mneme serve`, 1.1.0). The releases since have been toolchain refreshes and repairs.

## 1.2.2 — quick batch (this release)

- [x] **Roadmap cleanup.** Completed milestones and done items moved out (they live in the CHANGELOG); the
      remaining work is sequenced into the releases below.
- [x] **`import_notion_export` — the Notion directory import.** This was the last un-ported `mneme-io` path. The 4
      pure cleanup fns were already in `io_notion`, but the walk and page import and their 4 `#[tokio::test]`s
      were not. The release also adds the empty toggle-block removal in `clean_notion_content` that the port had
      deferred.

## 1.2.3 — P(-1) sweep: audit, refactor, hardening, optimization, security

A sweep and its repairs, with no new features. These are the known inputs; the sweep is expected to find more.

- ⛔ **Silent truncation on read: `store_files` caps every note at 256 KB.** `src/store_files.cyr:44` reads
  through a fixed `file_read_all(full, buf, 262144)`, so a larger note comes back cut short with no error. If that
  note is then edited and saved, the rest is destroyed. The same class sits in the importers, where
  `io_obsidian.cyr:121` and `io_markdown.cyr:11` cap at 256 KB. It also sits in the 1 MB caps at
  `search_vector_store.cyr:286`, `ai_training_export.cyr:161` and `ai_multimodal.cyr:102`. The stdlib's
  `file_read_whole` grows its buffer and returns the errno instead.
- **Security re-audit.** The 2026-07-10 audit (`docs/audit/2026-07-10-audit.md`) says to re-audit "when a listening
  socket is added", and `mneme serve` added one in 1.1.0. The re-audit should cover:
  - the MCP request path (bayan JSON input, stricter since bayan 1.11.1);
  - daimon self-registration;
  - the sandhi HTTP client.
- **Explicit `CreateNote.path` traversal.** This is inherited from Rust and is the audit's §2 follow-up: reject
  `..` and absolute paths.
- **One uuid string codec in `core_uuid`.** Today there are two formatters (`_db_uuid_to_str` in `store_db` and
  `mneme_store_uuid_to_string` in `store_registry`) and two parsers that disagree:
  - `mneme_store_uuid_parse` is strict and returns 0 for invalid input;
  - `_db_uuid_from_str` is lenient: it skips non-hex characters and never fails. It takes user-supplied ids at 22
    call sites in `api_server` and `mcp_tools`.

  Moving those sites to the strict parser means each one must handle 0.
- **Duplicate definitions.** `search_engine.cyr` and `search_cross_vault.cyr` both define `mneme_search_result_*`.
- **Compiler warnings in `src/`:**
  - "assigning non-pointer to typed pointer" in `ai_creative`, `api_server`, `io_export_pdf`, `io_web_clipper`,
    `search_query_dsl`, `store_db` and `store_vault`;
  - "comparison mixes f64 and integer operands" in `ai_creative`, `ai_linker`, `api_server`, `mcp_tools`,
    `search_vector_store` and `ui_render`.
- **Dead wrapper.** `_cfg_toml_str` (`core_config`) has been a passthrough since bayan 1.5.5 moved TOML unescaping
  into its parser.
- **Binary size.**
  - The compiler reports ~1.3 MB of unreachable fns in `build/mneme` (`[build] dce`).
  - `assert` and `bench` ship in `[deps].stdlib`; since 6.7.6 they can move to `[test] stdlib`.
- **Cheap optimizations** from [`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md):
  - reuse a tokenizer scratch arena (#2);
  - buffer uuid entropy instead of a syscall per id (#5).

## 1.3.x — the document pipeline on bayan

- **1.3.0 — `bayan_pdf_*` for the PDF primitives.** bayan 1.5.0 shipped the module, and mneme has vendored it since
  1.1.3. Only the object, xref and content-stream writer in `io_export_pdf` migrates; mneme keeps its markdown
  layer (`mneme_io_parse_markdown_blocks`) and the ADR-015 unterminated-fence behaviour. The migration retires
  three live defects tracked in `state.md`:
  - no `/Encoding`, so non-ASCII text renders as mojibake;
  - wrapping counts characters instead of width;
  - `_pdf_esc` silently drops CR and LF.

  It also replaces the brittle `offset = 9` / closed-form object-number constants. `scripts/pdfcheck.sh` is the
  gate on both sides of the swap. In the same release, surface `mneme_io_md_unterminated_fence()` through
  `/export/pdf` and the file exporters (`mneme_io_export_note_to_pdf` / `_notes_to_pdf`, which nothing in `src/`
  calls yet); nothing reports it today.
- **1.3.1 — `bayan_markdown_*` for HTML export.** This is now unblocked: bayan 1.11.1 ships `bayan_markdown_parse`
  (to an AST) and `bayan_markdown_to_html`. It replaces the hand-rolled subset in `mneme_io_markdown_to_html`
  (`io_export_html`).

## 1.4.x — remaining parity polish

- **RFC 3339 datetime formatting and parsing.** `io_web_clipper`'s timestamp is still an untested placeholder.
  chrono has `epoch_to_date` and `iso8601` but no parser, so this either lands in chrono first or is hand-rolled
  here. Timestamps stay stored as INT nanoseconds.
- **`GraphLayout::from_subgraph`** — the f64 force-directed spring layout (`core_graph`; no Rust test).

## 1.5.x — performance

These are the ranked targets in [`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md). The port is
correct, not yet fast, by design.

- **ANN vector index (#1)** — HNSW-style, plus `simd`/`ganita` f64 lanes in place of the brute-force cosine scan.
  This is the only path that scales with vault size.
- **True BM25 scoring (#3)** — tf-idf ranking, posting-list skip and pre-sized vecs, in place of rank-by-match-count.
- **Value packing (#4)** — small structs off the heap, and f32 lanes where precision allows.

## 2.0.0 backlog

- **Local ONNX inference** through the sovereign ML stack: akshara (BPE tokenizer) → rosnet (tensors/BLAS) →
  rupantara (transformer forward) → tula (safetensors/GGUF) → anukulana (load pretrained). It lands when the ML-AI
  stack is worked on as a whole. `build_local` degrades today, and remote embeddings via daimon cover the path; the
  embed hook and `SemanticEngine`'s degraded path mean nothing else changes when it lands.
- **Native GUI on `dhancha`.** The TUI stays the shipping UI until then.
