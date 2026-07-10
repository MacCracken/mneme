# Benchmarks: Rust vs Cyrius

> mneme v1.0.0 — the **previous build (Rust) → current build (Cyrius)** baseline.
> Its job isn't to win a race; it's to mark **where the port needs optimizing later**
> (see "Optimization targets" at the bottom). Humble and honest about what's measured.
>
> - **Cyrius**: cycc 6.4.42, `lib/bench.cyr`. Run 2026-07-10, host build. f64
>   (no f32), heap-allocated structs, `fncall`-dispatched hooks.
> - **Rust**: criterion harnesses exist (`rust-old/**/benches/*.rs`) but were **not
>   run** for this release — see "Why no Rust numbers yet" below. No prior criterion
>   output was captured either.
> - **Platform**: x86_64 Linux (the deploy target is AgnosticOS).

## Why no Rust numbers yet

The frozen Rust oracle at `rust-old/` no longer builds: after `cyrius port` moved the
crates under `rust-old/crates/`, the workspace's `path = "../bote"`-style sibling
dependencies dangle (`failed to read /home/macro/Repos/mneme/bote/Cargo.toml`).
Restoring them would mean editing the parity oracle, which is out of scope for the
port. So rather than invent figures, the Rust column is left blank until the
workspace's path deps are rehydrated and `cargo bench` is run. **This doc will be
updated with a real head-to-head then** — nothing here is cherry-picked to flatter
the port.

## Head-to-Head (the comparable ops)

Cyrius numbers are real (`cyrius bench benches/mneme_bench.bcyr`). Rust cells are the
harness that measures the nearest equivalent, pending a run.

| Operation | Rust (ns) | Cyrius | Rust harness | Notes |
|-----------|-----------|--------|--------------|-------|
| UUID v4 generate | _pending_ | 522 ns | — (uuid crate) | getrandom-backed both sides |
| Tokenize (~16 words) | _pending_ | 4.77 µs | — | str_builder-per-token; f64 n/a |
| Vector sq-distance (3-d) | _pending_ | 35 ns | — (in search/cluster) | f64 heap-vec vs Rust f32 slice |
| Full-text search | _pending_ | 279 µs / 200 docs | `search_500_notes` | from-scratch BM25 (no Tantivy) |
| Index a note | _pending_ | (amortized) | `index_single_note` | |
| Create note | _pending_ | I/O-bound (patra+fs) | `create_note` | |
| List 50 of 200 | _pending_ | I/O-bound (patra) | `list_50_of_200_notes` | |

## Cyrius benchmark set (measured)

| Benchmark | avg | iters |
|-----------|----:|------:|
| `core_uuid_v4` | 522 ns | 200,000 |
| `ai_tokenize` | 4.77 µs | 100,000 |
| `squared_distance_3d` | 35 ns | 1,000,000 |
| `bm25_search_200docs` | 279 µs | 20,000 |

Details + methodology in [`benchmarks.md`](benchmarks.md).

## Rust harness inventory (ready to run once deps are rehydrated)

| Crate | Benchmark | Measures |
|-------|-----------|----------|
| `mneme-search` | `index_single_note` | inverted-index insert |
| `mneme-search` | `index_100_notes` | bulk index build |
| `mneme-search` | `search_500_notes` | query over 500-doc index |
| `mneme-search` | `search_no_results` | miss path |
| `mneme-store` | `create_note` | DB insert + file write + hash |
| `mneme-store` | `list_50_of_200_notes` | paged DB query |

## Honest expectations (before the Rust numbers land)

The port optimized for **behavioral parity and correctness, not speed** — so expect
Cyrius to be **meaningfully slower on CPU-bound micro-ops**, and the gap to narrow on
the workloads mneme actually runs:

- **CPU micro-ops** (sq-distance, tokenize): the sibling `hisab` Rust-vs-Cyrius sheet
  saw ~30–600× on hot numeric kernels — Rust's f32 SIMD + inlining vs Cyrius's f64,
  heap-allocated types, and per-call `fncall` overhead.
- **Search** (BM25): closer — dominated by posting-list walks + vec growth, not float
  SIMD. Still expect Rust (Tantivy, tuned) to lead; mneme's index is a from-scratch
  OR-match, deliberately simple for v1.
- **Store / API / real workloads** (create/list/search-a-note): **I/O-bound** —
  dominated by `patra` SQL + file writes + syscalls, not language codegen. Smallest
  gap here, and where mneme actually spends its time interactively.

## Optimization targets (where the port needs work later)

The point of this baseline: rank where post-v1 effort buys the most. Priority weighs
the current cost **× how it scales with real use** (vault size, note count).

| # | Target | Current (Cyrius) | Why slow | Optimization | Priority |
|---|--------|------------------|----------|--------------|----------|
| 1 | **Semantic search (vector distance)** | 35 ns/3-d → ~4.5 µs/384-d, **×N brute force** | O(N) scan; f64 heap-vec, no SIMD | ANN index (HNSW-style) + `simd`/`ganita` f64 lanes | **High** — the only path that scales *with the vault* |
| 2 | **Tokenizer** (`ai_tokenize`) | 4.77 µs | a `str_builder` + alloc **per token**, per call | reuse a scratch arena; lowercase in place; intern stopwords | **High** — hot in concepts/tagger/rag-eval/consolidation |
| 3 | **BM25 search** (`search_500_notes`) | 279 µs/200 docs | OR-match + rank-by-*count* (not real BM25); vec regrow | true BM25 (tf-idf) scoring + posting-list skip + pre-sized vecs | **Med** — sub-ms today; matters at 10³–10⁴ notes |
| 4 | **f64-everywhere / value types** | systemic | no f32; structs are heap ptrs; `fncall` per hook | pack small structs; f32 lanes where precision allows | **Med** — systemic, moves every micro-op |
| 5 | **UUID v4** | 522 ns | a `getrandom` syscall **per id** | buffer entropy (draw 4 KB, slice) | **Low** — not a bottleneck at note volumes |

Bottom line: mneme 1.0.0 is **correct and complete, not yet fast** — exactly the right
shape for a parity port. Items 1–2 are the first place to spend once the ML stack
lands (the ANN index pairs naturally with real embeddings); the rest is incremental.
These are tracked as post-v1 optimization work, separate from the P2 feature backlog.
