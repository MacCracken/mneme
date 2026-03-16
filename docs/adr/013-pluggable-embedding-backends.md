# ADR-013: Pluggable Embedding Backends

**Status:** Accepted
**Date:** 2026-03-15

## Context

Mneme bundles an ONNX model (all-MiniLM-L6-v2, 384-dim) for local embeddings. This works offline but cannot leverage GPU acceleration or alternative models. The AGNOS ecosystem includes Synapse (LLM controller) which can serve embeddings via its REST API, and users may also have Ollama or OpenAI API access.

## Decision

### EmbeddingBackend Trait

Define a `trait EmbeddingBackend: Send + Sync` with:
- `embed(&self, text: &str) -> Result<Vec<f32>>`
- `embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>`
- `dimension(&self) -> usize`
- `name(&self) -> &str`

### Two Implementations

1. **LocalOnnxBackend** — wraps the existing `Embedder` (feature-gated behind `local-vectors`)
2. **RemoteHttpBackend** — calls any OpenAI-compatible `/v1/embeddings` endpoint via `reqwest`

### Backend Selection

The `build_backend()` factory reads `EmbeddingConfig` and selects:
- `"local"` — only try local ONNX
- `"remote"` — only try remote HTTP
- `"auto"` (default) — try remote first, fall back to local

### Dimension Probe

`RemoteHttpBackend::new()` sends a probe embedding ("dimension probe") to detect the remote model's output dimension. This ensures the vector store is initialized with the correct dimension.

### Configuration

```toml
[embedding]
backend = "auto"
remote_url = "http://127.0.0.1:8420"
model = "all-MiniLM-L6-v2"
```

## Alternatives Considered

- **gRPC client to Synapse**: Synapse doesn't expose a dedicated embeddings gRPC service yet. The OpenAI-compatible HTTP format is a universal standard supported by all major providers.
- **Trait object vs enum dispatch**: Trait object (`Box<dyn EmbeddingBackend>`) chosen for extensibility — future backends (e.g., gRPC, WebSocket) can be added without modifying the enum.

## Consequences

- SemanticEngine no longer holds an `Embedder` directly — it holds `Option<Box<dyn EmbeddingBackend>>`
- The `embed()` and `embed_batch()` methods on SemanticEngine now work regardless of which backend is active
- Remote backends add network latency to every embedding call; batch operations amortize this
- The health endpoint reports the active backend name and dimension for observability
