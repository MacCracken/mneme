# Mneme Roadmap

Version: 2026.3.15

---

## Completed

All planned phases and post-MVP features have been implemented.

### Phases 1–4: Foundation → Polish (MVP Complete)
- Core types, SQLite persistence, file-backed Markdown storage
- Tantivy full-text search, hybrid BM25 + semantic ranking
- AI pipelines: RAG, summarization, auto-linking, concept extraction, tagging, templates
- Axum HTTP API (30+ endpoints), TUI (ratatui), MCP server (5 tools)
- 313 tests, Criterion benchmarks, 10 ADRs, full documentation
- AGNOS marketplace recipe, systemd service, agnoshi intents

### Phase 5 — In-Process Vector Store
- ONNX Runtime wrapper for all-MiniLM-L6-v2 (384-dim embeddings)
- usearch ANN index with persistent metadata in `.mneme/vectors/`
- SemanticEngine facade: embedder + vector store, graceful daimon fallback
- Feature-gated behind `local-vectors` (default on)

### Phase 6 — Retrieval Quality & Feedback Loop
- Thompson Sampling bandit with 4 arms for adaptive ranking
- `weighted_hybrid_merge` with `BlendWeights`
- `/v1/search/feedback` + `/v1/search/optimizer` endpoints
- Optimizer state persists in `.mneme/optimizer.json`

### Phase 7 — RAG Evaluation Metrics
- Token-overlap faithfulness, relevance, and chunk utilization scoring
- Weighted overall score, `RagEvalAggregates` for running averages
- `/v1/ai/rag/stats` returns eval aggregates per vault

### Phase 8 — Note Consolidation & Evolution
- Embedding-based duplicate detection + Jaccard token-overlap
- `last_accessed` column, `touch_note()`, content freshness scoring
- LLM merge suggestions via daimon (`POST /v1/ai/consolidate/merge`)
- Stale notes panel in TUI (`!` keybinding)

### Phase 9 — Schema Clustering & Emergent Structure
- K-means++ clustering on note embeddings with elbow heuristic
- `SemanticEngine::embed()` / `embed_batch()` for raw embedding access
- `DaimonClient::label_cluster()` for LLM-generated labels
- `GET /v1/ai/clusters` endpoint, Clusters panel in TUI (`c` key)

### Phase 10 — Context-Aware Retrieval
- `ContextBuffer`: bounded working-memory of recent note IDs (capacity ~7)
- `fuse_embeddings()`: λ-weighted blend of query + context vectors
- `SemanticEngine::context_search()`, configurable via `mneme.toml`
- API and TUI automatically use session context in search

### Phase 11 — Document Provenance & Trust Scoring
- `Provenance` enum: Manual (1.0), Import (0.8), WebClip (0.6), Generated (0.4)
- User override via frontmatter (`trust: high/medium/low/0.8`)
- Trust as multiplicative boost on hybrid search scores
- TUI search results show [H]/[M]/[L] trust indicators

### Phase 12 — Multi-Vault Support
- `VaultRegistry` with TOML persistence, `VaultManager` for lifecycle
- Cross-vault search with RRF merge and vault-weight multipliers
- API: `/v1/vaults` CRUD, `/v1/vaults/{id}/switch`
- TUI: VaultPicker panel (`v` key)

### Post-MVP
- Import: Obsidian, Notion, Markdown directories
- Export: HTML static site, PDF (printpdf)
- AI: writing assist, translation, temporal analysis, multi-modal, creative suite, flashcards
- Collaboration: versioning (Delta-ready), sharing (roles, conflicts)
- Advanced: tasks/kanban, calendar, web clipper, plugin system
- Knowledge graph visualization (force-directed layout, Canvas rendering, zoom/pan)
- Split-pane / multi-note view in TUI

---

## Next: AGNOS Ecosystem Integration

Deep integration with Synapse (LLM controller), daimon (agent runtime),
and Agnostic (QA platform) to close the knowledge-feedback loop.

### Phase 13 — Synapse Direct Embedding Pipeline (Complete)

Pluggable embedding backends — use Synapse, Ollama, OpenAI, or local ONNX.

- `EmbeddingBackend` trait with `embed()`, `embed_batch()`, `dimension()`, `name()`
- `LocalOnnxBackend`: wraps existing in-process ONNX embedder
- `RemoteHttpBackend`: OpenAI-compatible `/v1/embeddings` client (works with Synapse, Ollama, OpenAI)
- `build_backend()` factory: "auto" tries remote first, falls back to local
- `SemanticEngine::open_with_config()` accepts `EmbeddingConfig`
- `EmbeddingConfig` / `EmbeddingSection` in `mneme.toml`: backend, remote_url, model, api_key, dimensions
- Health endpoint reports `embedding_backend` and `embedding_dimension`
- Dimension probe on remote connect to auto-detect vector size

### Phase 14 — Training Feedback Export (Complete)

Export user signals for Synapse fine-tuning to close the learning loop.

- `TrainingRecord` enum: SearchClick, EditAfterSearch, TrustOverride, NoteContent
- `TrainingLog`: append-only JSONL file at `.mneme/training.jsonl`
- `GET /v1/export/training-data` endpoint with `?type=`, `?since=`, `?include_notes=` filters
- Search feedback handler logs clicks with query, note title, arm name, position
- `SearchFeedbackRequest` extended with optional `query` and `position` fields
- Per-vault training log in `VaultEngines`

### Phase 15 — Daimon Event Bus Integration

Real-time pub/sub for cross-agent knowledge sync.

- Publish events on note CRUD: `mneme:vault:{id}:note:{created|updated|deleted}`
- Publish on concept extraction: `mneme:vault:{id}:concept:extracted`
- Subscribe to agent context: other agents can push context into Mneme
- daimon IPC client in `mneme-api` using `/run/agnos/` socket
- Enable collaborative editing across multiple agnoshi sessions
- Audit log forwarding to daimon's event store

### Phase 16 — Agnostic Knowledge QA

Automated quality assurance for knowledge base health.

- Auto-generate assertion suites from knowledge graph structure
  ("Note X should reference concept Y", "Tag Z should have >3 notes")
- `POST /v1/qa/run` endpoint triggers Agnostic QA suite via HTTP bridge
- Import Agnostic run results as quality metrics per vault
- Retrieval latency benchmarking under synthetic load
- Link/reference validation (dead links, orphan notes, circular refs)

### Phase 17 — Advanced agnoshi Query DSL

Rich natural language queries for the AI shell.

- Temporal: "notes edited last week about Rust"
- Boolean: "notes tagged #project AND NOT #archived"
- Graph: "notes connected to X within 2 hops"
- Bulk ops: "archive all stale notes older than 6 months"
- DSL parser in agnoshi's intent translator with fallback to semantic search
- Results streamed back via daimon message bus

---

## Next: Knowledge Base Intelligence

Improvements to how Mneme understands and manages knowledge over time.

### Phase 18 — Knowledge Decay Model

Notes have different half-lives based on domain and usage patterns.

- Per-cluster decay rates derived from historical access patterns
- Technical docs decay fast, reference material doesn't
- Decay-adjusted freshness score replaces simple ratio
- Auto-suggest review for notes past their half-life
- Configurable per-vault decay profiles in `mneme.toml`

### Phase 19 — Cross-Vault Knowledge Links

Notes in different vaults can reference each other.

- `vault://work/note-id` URI scheme for cross-vault links
- Link resolution at query time (not import time)
- Cross-vault backlinks visible in note view
- Graph visualization spans vaults with color-coded edges

### Phase 20 — AI Provenance Chain

Full traceability for AI-generated or AI-modified content.

- Track which model, prompt, and source notes fed each generation
- `ProvenanceChain` stored alongside content: model ID, timestamp, input refs
- Diff view: "this paragraph was generated by X from notes A, B, C"
- Re-generation: replay the chain with updated source notes
- Confidence decay: AI-generated content trust erodes faster without human review

### Phase 21 — Spaced Repetition × Context Integration

Active recall feeds back into retrieval ranking.

- Flashcard review results update note access patterns
- Notes you struggle to recall surface more in search
- SM-2 intervals integrated with knowledge decay model
- Context buffer weighted by recall difficulty

### Phase 22 — Incremental Re-indexing

Scale to 10k+ notes without full-vault passes.

- Delta-based clustering: assign new notes to nearest existing cluster
- Incremental vector index updates (no full rebuild)
- Consolidation runs on changed notes only (since last pass)
- Background indexer service with configurable batch size

---

## Future Considerations

Items that may be explored in future versions:

- SecureYeoman MCP tool registration and sandbox verification
- Real-time WebSocket collaborative editing
- Import from Notion API (live sync, not just export)
- Plugin dynamic loading with AGNOS marketplace distribution
- Typed memory taxonomy (episodic, semantic, procedural) as note metadata
- 3D knowledge graph visualization (WebGL/wgpu)
- Multi-tenant vault hosting with daimon auth
- Cross-vault semantic clustering and vault merge/split suggestions

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 2026.3.15 | 2026-03-15 | Phases 5–12 complete — knowledge effectiveness suite |
| 2026.3.13 | 2026-03-13 | Phases 1–4 + post-MVP — full feature set |
