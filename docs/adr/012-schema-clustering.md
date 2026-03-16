# ADR-012: Schema Clustering & Emergent Structure

**Status:** Accepted
**Date:** 2026-03-15

## Context

Users with large vaults lack visibility into the topic structure of their knowledge. Manual folder organization doesn't scale and doesn't reflect the actual semantic relationships between notes.

## Decision

### K-means++ Clustering

Implement K-means++ directly in Rust (no external clustering dependency). The algorithm:
1. Deterministic initialization: first centroid is closest to the global mean, subsequent centroids maximize min-distance to existing centroids
2. Lloyd's iteration with convergence check (max 50 iterations)
3. No RNG dependency — fully reproducible results

### Elbow Heuristic

When `k` is not specified, compute inertia for k=2..=min(max_k, 10), find the elbow via maximum second derivative. This auto-selects a reasonable number of clusters without user tuning.

### LLM Labeling

Optionally send cluster note titles to daimon's `/v1/knowledge/cluster-label` endpoint for human-readable cluster names and one-line summaries. Falls back to "Cluster N" labels when daimon is unavailable.

### Embedding Access

Expose `SemanticEngine::embed()` and `embed_batch()` for raw embedding vectors, and `VectorStore::note_ids()` for listing indexed notes. These support clustering and any future use case needing raw embeddings.

## Consequences

- Clustering is a full-vault O(n·k·d·iterations) operation — suitable for vaults up to ~5k notes
- Phase 22 (incremental re-indexing) will address scaling beyond that
- `ndarray` was considered but not needed — plain `Vec<f32>` arithmetic suffices for 384-dim vectors
- Cluster results are ephemeral (not persisted) — recomputed on demand
