# ADR-007: In-Process Vector Store

## Status
Accepted

## Context

Mneme's semantic search previously depended entirely on daimon for vector embeddings and ANN queries. This meant semantic search was unavailable when daimon was not running, which is common in offline or lightweight setups. A local vector search capability was needed to make Mneme fully standalone.

## Decision

### Embedding
Use ONNX Runtime (`ort 2.0.0-rc.12`) to run the all-MiniLM-L6-v2 model locally, producing 384-dimensional embeddings. The `Embedder` struct in `mneme-search/src/embedder.rs` wraps the ONNX session and tokenizer. Models are loaded from `MNEME_MODELS_DIR`.

### ANN Index
Use `usearch 2` for approximate nearest neighbor search. The `VectorStore` in `mneme-search/src/vector_store.rs` manages the index with persistent metadata, stored in `.mneme/vectors/`.

### Facade
The `SemanticEngine` in `mneme-search/src/semantic_engine.rs` combines the embedder and vector store behind a single interface. It provides graceful degradation: if the `local-vectors` feature is disabled or models are unavailable, it falls back to daimon.

### Feature Gating
The entire local vector stack is behind the `local-vectors` Cargo feature (default on). This keeps the binary lean when local vectors are not needed and avoids pulling in ONNX Runtime and usearch as hard dependencies.

## Alternatives Considered

- **instant-distance**: Pure Rust HNSW library. Simpler dependency story but less mature and slower than usearch for larger indices.
- **faiss (via bindings)**: Industry standard but heavy C++ dependency, complex build, overkill for personal knowledge base scale.
- **Daimon-only**: Status quo. Rejected because offline-first is a core design principle.

## Consequences

- **Positive**: Mneme works fully offline with semantic search — no daimon required
- **Positive**: Feature gating keeps the minimal build small
- **Positive**: Persistent index in `.mneme/vectors/` survives restarts without re-embedding
- **Negative**: ONNX Runtime adds ~30 MB to the binary and requires platform-specific libs
- **Negative**: Model files must be distributed separately (not bundled in the binary)
