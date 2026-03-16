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
- Models loaded from `MNEME_MODELS_DIR`

### Phase 6 — Retrieval Quality & Feedback Loop
- Thompson Sampling bandit with 4 arms (balanced, fulltext_heavy, semantic_heavy, recency_boost)
- `weighted_hybrid_merge` with `BlendWeights` for adaptive ranking
- Search response includes `search_id` for feedback correlation
- `/v1/search/feedback` endpoint records clicks, improves future ranking
- `/v1/search/optimizer` endpoint exposes arm stats
- MCP `mneme_search_feedback` tool
- Optimizer state persists in `.mneme/optimizer.json`

### Phase 7 — RAG Evaluation Metrics
- `mneme-ai/src/rag_eval.rs`: local-only token-overlap scoring (no LLM required)
- Faithfulness, answer relevance, and chunk utilization scores (0.0–1.0)
- Weighted overall score (50% faithfulness, 30% relevance, 20% utilization)
- `RagAnswer` includes optional `eval: RagEvalScores` field
- `/v1/ai/rag/stats` returns eval aggregates per vault
- `RagEvalAggregates` for running averages across queries

### Phase 12 — Multi-Vault Support
- `VaultRegistry` with TOML persistence, `VaultManager` for lifecycle
- Cross-vault search: fan-out + RRF merge with vault-weight multipliers
- API: `/v1/vaults` CRUD, `/v1/vaults/{id}/switch`, all endpoints accept `?vault=`
- MCP: `mneme_list_vaults`, `mneme_switch_vault` tools (8 total now)
- TUI: VaultPicker panel (`v` key)
- Config: `mneme.toml` with `[vaults]` table

### Phase 8 — Note Consolidation & Evolution
- Embedding-based duplicate detection (`find_similar_to` on SemanticEngine)
- `detect_duplicates_semantic()` with deduped symmetric pairs
- `last_accessed` column + migration, `touch_note()` on full-content reads
- Content freshness scoring: `days_since_update / max(days_since_access, 1)`
- LLM merge suggestions via daimon (`POST /v1/ai/consolidate/merge`)
- `/v1/notes/duplicates?method=semantic|jaccard|both` query param
- Stale notes panel in TUI (`!` keybinding) with freshness scores
- `MergeSuggestion` and `DuplicatePair.detection_method` types

### Post-MVP
- Import: Obsidian, Notion, Markdown directories
- Export: HTML static site, PDF (printpdf)
- AI: writing assist, translation, temporal analysis, multi-modal, creative suite, flashcards
- Collaboration: versioning (Delta-ready), sharing (roles, conflicts)
- Advanced: tasks/kanban, calendar, web clipper, plugin system
- Knowledge graph visualization (force-directed layout, Canvas rendering, node cycling, zoom/pan)
- Split-pane / multi-note view in TUI (side-by-side, pane switching, note picker)

### Phase 9 — Schema Clustering & Emergent Structure
- K-means++ clustering on note embeddings (deterministic init, Lloyd's iteration)
- Elbow heuristic for automatic k selection (second derivative of inertia)
- `SemanticEngine::embed()` / `embed_batch()` for raw embedding access
- `VectorStore::note_ids()` for listing indexed notes
- `DaimonClient::label_cluster()` for LLM-generated cluster labels/summaries
- `GET /v1/ai/clusters?k=&max_k=&label=` API endpoint
- Clusters panel in TUI (`c` keybinding) with expand/collapse navigation
- `ClusteringResult`, `Cluster`, `NoteEmbedding` types in `mneme-ai::clustering`

---

## Next: Knowledge Effectiveness

Improvements informed by SecureYeoman's brain/KB architecture to make Mneme a
smarter, more self-maintaining knowledge base.

### Phase 10 — Context-Aware Retrieval
- `ContextBuffer`: bounded working-memory of recent note IDs (capacity ~7, recency-weighted)
- `fuse_embeddings()`: λ-weighted blend of query + context vectors, L2-normalized
- `SemanticEngine::context_search()`: fused embedding search
- `MnemeConfig.context_retrieval`: `enabled`, `query_weight` (λ=0.7), `buffer_capacity`
- API: `GET /v1/notes/{id}` pushes to context buffer; search uses it automatically
- API: `?context=false` query param on search to opt out
- TUI: `select_note()` pushes to context buffer, `run_search()` merges context-aware results

### Phase 11 — Document Provenance & Trust Scoring
- `Provenance` enum: Manual (1.0), Import (0.8), WebClip (0.6), Generated (0.4)
- `Note.provenance` + `trust_override` fields, DB migration 003
- `trust_score()` method: user override via frontmatter (`trust: high/medium/low/0.8`)
- Frontmatter parser handles `trust:` field with named levels or float values
- Search ranking applies trust as multiplicative boost on hybrid scores
- `SearchResultItem.trust` field in API responses
- TUI search results show [H]/[M]/[L] trust indicators (green/yellow/red)
- Web clipper auto-sets `Provenance::WebClip` on clipped notes
- `CreateNote.provenance` optional field for origin tracking

---

## Future Considerations

Items that may be explored in future versions:

- SecureYeoman MCP tool registration and sandbox verification
- Large vault stress testing (10k+ notes) and memory profiling
- Real-time WebSocket collaborative editing
- Import from Notion API (live sync, not just export)
- Plugin dynamic loading (currently static registry)
- Typed memory taxonomy (episodic, semantic, procedural) as note metadata

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 2026.3.15 | 2026-03-15 | Phase 5 (in-process vectors) + Phase 6 (retrieval optimizer) + Phase 7 (RAG eval metrics) + Phase 12 (multi-vault) |
| 2026.3.15 | 2026-03-15 | Knowledge graph visualization + split-pane multi-note view |
| 2026.3.13 | 2026-03-13 | All phases complete — full feature set |
