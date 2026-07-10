# mneme — Benchmarks

Micro-benchmarks over the hot paths of the ported subsystems. The mneme port
targets **behavioral parity** with the Rust original first; these numbers establish
a baseline for the pure-Cyrius implementations (built-from-scratch BM25 index,
brute-force cosine, hand-rolled tokenizers) so regressions are visible.

## Running

```sh
cyrius bench benches/mneme_bench.bcyr
```

Uses `lib/bench.cyr` (batch loop → `bench_report`). Sub-microsecond ops use large
batch sizes (≥1M iters) to amortize the ~240 ns clock overhead per start/stop pair.

## Results

Captured 2026-07-10 · Cyrius 6.4.42 · x86_64 Linux (host build; the deploy target
is AgnosticOS).

| Benchmark | Op | avg | iters |
|---|---|---:|---:|
| `core_uuid_v4` | random UUID v4 generation (getrandom-backed) | **522 ns** | 200,000 |
| `ai_tokenize` | concept tokenizer: lowercase + split + stopword filter over a ~16-word string | **4.77 µs** | 100,000 |
| `squared_distance_3d` | f64 squared distance over 3-d vectors (k-means / cosine inner loop) | **35 ns** | 1,000,000 |
| `bm25_search_200docs` | full-text OR-match + rank over a 200-doc in-memory inverted index | **279 µs** | 20,000 |

### Reading the numbers

- **`squared_distance_3d` (35 ns)** — the vector inner loop is tight; k-means++ and
  brute-force cosine scale linearly in it. A 384-d embedding distance is ~128× this
  (~4.5 µs), so a brute-force search over N vectors is ~N·4.5 µs (fine to ~10³–10⁴
  vectors; the usearch-analog ANN index is a future optimization, not a v1 need).
- **`bm25_search_200docs` (279 µs)** — the from-scratch inverted index (no Tantivy
  analog) does an OR-match over posting lists + rank-by-match-count; sub-millisecond
  at 200 docs. Index build is amortized (done once per note on create/update).
- **`ai_tokenize` (4.77 µs)** — string-heavy (str_builder per token); dominates the
  concept/tagger/rag-eval paths. Acceptable for interactive note volumes.
- **`core_uuid_v4` (522 ns)** — real entropy via `getrandom`; every note/tag/card gets one.

## Scope

These cover the CPU-bound cores. Not benchmarked here (I/O-bound, dominated by the
syscall/network/disk they wrap, not mneme logic): patra SQL, the file store, and the
`sandhi` HTTP bridges. Add cases to `benches/mneme_bench.bcyr` as hot paths emerge.
