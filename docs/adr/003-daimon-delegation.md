# ADR-003: Daimon Delegation for AI Features

## Status
Accepted

## Context

Mneme needs AI capabilities: semantic search, summarization, RAG, concept extraction. We can either:
1. Embed AI models directly (ONNX Runtime, local transformers)
2. Delegate to daimon's existing endpoints

## Decision

Delegate all AI inference to daimon's REST API (`/v1/rag/*`, `/v1/vectors/*`, `/v1/knowledge/*`). Keep only stateless extraction logic (concept extraction, extractive summarization) in mneme-ai.

## Consequences

- **Positive**: No model loading, no GPU management, no duplicate infrastructure. Mneme stays lightweight. Any improvements to daimon's RAG pipeline benefit Mneme automatically.
- **Positive**: Graceful degradation — core note-taking works without daimon.
- **Negative**: AI features require a running AGNOS instance with daimon.
- **Negative**: Network latency for AI operations (mitigated: localhost only).
- **Mitigation**: Extractive summarization and concept extraction work offline as fallbacks.
