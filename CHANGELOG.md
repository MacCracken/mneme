# Changelog

All notable changes to Mneme will be documented in this file.

## [2026.3.18] — 2026-03-18

### Build & CI
- Eliminated `openssl-sys` dependency by switching `ort` to `tls-rustls` backend
- CI builds no longer require system OpenSSL/libssl-dev packages
- Improved `bump-version.sh` to update all Cargo.toml files consistently (modeled on nazar patterns)
- Release automation workflow now uses bump script for consistent version management

### Quality
- 450 tests across 8 crates (up from 382)
- 65% test coverage (threshold maintained)
- Zero clippy warnings across all crates
- Fixed all clippy lints: loop variables, clamp patterns, type complexity, range checks
- New test suites: `db.rs` (20 tests), `files.rs` (6 tests), additional MCP/search/manager tests

## [2026.3.15] — 2026-03-15

### Phase 13 — Synapse Direct Embedding Pipeline
- `mneme-search/src/embedding_backend.rs`: `EmbeddingBackend` trait with `LocalOnnxBackend` and `RemoteHttpBackend`
- `RemoteHttpBackend`: OpenAI-compatible `/v1/embeddings` client (works with Synapse, Ollama, OpenAI)
- `build_backend()` factory: "auto" tries remote first, falls back to local ONNX
- `SemanticEngine::open_with_config()` accepts `EmbeddingConfig` for backend selection
- `mneme-core/src/config.rs`: `EmbeddingSection` in `MnemeConfig` (backend, remote_url, model, api_key, dimensions)
- Health endpoint reports `embedding_backend` name and `embedding_dimension`
- Dimension probe on remote connect to auto-detect vector size

### Phase 14 — Training Feedback Export
- `mneme-ai/src/training_export.rs`: `TrainingRecord` enum (SearchClick, EditAfterSearch, TrustOverride, NoteContent)
- `TrainingLog`: append-only JSONL at `.mneme/training.jsonl` per vault
- `GET /v1/export/training-data` endpoint with `?type=`, `?since=`, `?include_notes=` filters
- Search feedback handler logs clicks with query, note title, arm name, position
- `SearchFeedbackRequest` extended with optional `query` and `position` fields

### Phase 8 — Note Consolidation & Evolution
- `mneme-search/src/semantic_engine.rs`: `find_similar_to()` for embedding-based duplicate detection
- `mneme-ai/src/consolidation.rs`: `detect_duplicates_semantic()`, `MergeSuggestion` type, `DuplicatePair.detection_method` field
- `mneme-store/migrations/002_last_accessed.sql`: `last_accessed` column + index
- `mneme-store/src/db.rs`: `touch_note()` for lightweight access tracking
- `mneme-store/src/vault.rs`: `get_note()` bumps `last_accessed` on full content reads
- Content freshness scoring: `days_since_update / max(days_since_access, 1)` in `StaleNote`
- `POST /v1/ai/consolidate/merge` endpoint for LLM merge suggestions via daimon
- `GET /v1/notes/duplicates?method=semantic|jaccard|both` query param
- TUI: Stale notes panel (`!` keybinding) showing days stale + freshness score

### Phase 9 — Schema Clustering & Emergent Structure
- `mneme-ai/src/clustering.rs`: K-means++ with deterministic init, Lloyd's iteration, elbow heuristic
- `mneme-search/src/semantic_engine.rs`: `embed()`, `embed_batch()`, `indexed_note_ids()` methods
- `mneme-search/src/vector_store.rs`: `note_ids()` method
- `mneme-ai/src/client.rs`: `label_cluster()` for LLM-generated cluster labels/summaries
- `GET /v1/ai/clusters?k=&max_k=&label=` endpoint
- TUI: Clusters panel (`c` keybinding) with expand/collapse navigation
- Types: `ClusteringResult`, `Cluster`, `NoteEmbedding`

### Phase 10 — Context-Aware Retrieval
- `mneme-search/src/context_buffer.rs`: `ContextBuffer` with bounded deque, recency-weighted averaging, `fuse_embeddings()`
- `mneme-search/src/semantic_engine.rs`: `context_search()` fuses query + context embeddings
- `mneme-core/src/config.rs`: `ContextRetrievalConfig` with `enabled`, `query_weight` (λ=0.7), `buffer_capacity`
- `mneme-api/src/state.rs`: `ContextBuffer` per vault in `VaultEngines`
- `mneme-api/src/handlers.rs`: `GET /v1/notes/{id}` pushes to context buffer; search uses it automatically; `?context=false` opt-out
- TUI: `select_note()` pushes to context buffer, `run_search()` merges context-aware semantic results

### Phase 11 — Document Provenance & Trust Scoring
- `mneme-core/src/note.rs`: `Provenance` enum (Manual 1.0, Import 0.8, WebClip 0.6, Generated 0.4), `trust_score()` method
- `mneme-core/src/frontmatter.rs`: `trust:` field parsing (high/medium/low or float), `parse_trust_value()`
- `mneme-store/migrations/003_provenance.sql`: `provenance TEXT`, `trust_override REAL` columns
- `mneme-api/src/handlers.rs`: trust as multiplicative boost on hybrid search scores, `SearchResultItem.trust` field
- `mneme-api/src/advanced_handlers.rs`: web clipper auto-sets `Provenance::WebClip`
- `CreateNote.provenance` optional field for origin tracking
- TUI: [H]/[M]/[L] trust indicators (green/yellow/red) in search results

### Phase 5 — In-Process Vector Store
- `mneme-search/src/embedder.rs`: ONNX Runtime wrapper for all-MiniLM-L6-v2 (384-dim)
- `mneme-search/src/vector_store.rs`: usearch ANN index with persistent metadata
- `mneme-search/src/semantic_engine.rs`: Facade combining embedder + vector store, graceful daimon fallback
- Feature-gated behind `local-vectors` (default on)
- Vector index persists in `.mneme/vectors/`, models from `MNEME_MODELS_DIR`

### Phase 6 — Retrieval Quality & Feedback Loop
- `mneme-search/src/retrieval_optimizer.rs`: Thompson Sampling bandit with 4 arms (balanced, fulltext_heavy, semantic_heavy, recency_boost)
- `hybrid_merge` now accepts `BlendWeights` via `weighted_hybrid_merge`
- Search response includes `search_id` for feedback correlation
- `/v1/search/feedback` endpoint records clicks, improves future ranking
- `/v1/search/optimizer` endpoint shows arm stats
- MCP `mneme_search_feedback` tool
- Optimizer state persists in `.mneme/optimizer.json`

### Phase 12 — Multi-Vault Support
- `mneme-core/src/config.rs`: MnemeConfig, VaultConfigEntry, VaultInfo types
- `mneme-store/src/registry.rs`: VaultRegistry with TOML persistence
- `mneme-store/src/manager.rs`: VaultManager managing multiple vaults
- `mneme-search/src/cross_vault.rs`: Cross-vault RRF merge with vault weights
- `mneme-api/src/state.rs`: VaultState + VaultWithEngines
- `mneme-api/src/vault_handlers.rs`: /v1/vaults CRUD + switch endpoints
- All handlers support `?vault=` parameter
- MCP: `mneme_list_vaults`, `mneme_switch_vault`, `mneme_search_feedback` tools (8 total now)
- TUI: VaultPicker panel (press `v`)
- Config: `mneme.toml` with `[vaults]` table

### Phase 7 — RAG Evaluation Metrics
- `mneme-ai/src/rag_eval.rs`: local-only token-overlap scoring (no LLM required)
- Faithfulness, answer relevance, and chunk utilization scores (0.0–1.0)
- Weighted overall score: 50% faithfulness, 30% relevance, 20% utilization
- `RagEvalAggregates` for running averages across queries per vault
- `/v1/ai/rag/stats` returns eval aggregates

### Quality
- 362 tests across 8 crates (up from 313)
- 3 new DB migrations (001–003)
- New modules: clustering, context_buffer, embedding_backend, training_export
- 4 new ADRs (011–014): consolidation, clustering, embedding backends, training export
- New deps: usearch 2, ort 2.0.0-rc.12, ndarray 0.17, tokenizers 0.21

## [2026.3.13] — 2026-03-13

### Core
- Note, Link, Tag, Graph types with Markdown frontmatter parsing
- Task extraction from Markdown checkboxes with priority markers and due dates
- Calendar types: daily notes, month views, entry type detection
- Plugin system: registry with capability-based lookup and hook points

### Store
- SQLite persistence with CRUD operations, file-backed Markdown storage, content hashing (SHA-256)
- Note versioning: version history with line-level diffs, Delta integration ready
- Vault sharing: multi-user with Owner/Editor/Viewer roles, conflict detection

### Search
- Tantivy full-text search with BM25 ranking
- Hybrid semantic search via daimon vector endpoints
- Tag-based filtering, search result pagination

### AI
- RAG pipeline: ingest, query, and Q&A over knowledge base
- Note summarization (extractive + daimon abstractive)
- Auto-linking: suggest connections between related notes
- Concept extraction: entities, topics, key terms via TF-IDF
- Auto-tagging with confidence scores and vocabulary matching
- Note templates: daily, meeting, project with variable substitution
- AI writing assistant: complete, reword, expand with daimon/local fallback
- Translation pipeline: per-note and batch, 12+ supported languages
- Temporal analysis: monthly activity tracking, concept trend detection
- Multi-modal notes: image/audio/video attachments, Shruti transcription
- Creative suite: Tazama shot lists, Rasa design assets, Shruti show notes, BullShift trade journal
- Spaced repetition: SM-2 scheduling, flashcard extraction from definitions and concepts

### API
- Axum HTTP server (port 3838) with 30+ endpoints
- Note CRUD, search, tags, health check
- AI: RAG query/ingest/stats, summarize, suggest-links, concepts, suggest-tags, write, translate, languages, temporal
- Templates: list, render (with optional note creation)
- Export: PDF
- Tasks/Kanban, Calendar, Flashcards
- Web Clipper: HTML-to-Markdown, bookmarks
- Plugins: list installed

### UI
- Terminal interface (ratatui + crossterm)
- Note list, Markdown viewer, search, backlinks panel, tag browser
- Knowledge graph visualization (force-directed layout, Canvas rendering, node cycling, zoom/pan)
- Split-pane / multi-note view (side-by-side, pane switching, note picker)
- Keyboard-driven navigation

### MCP
- JSON-RPC 2.0 server with 5 tools for Claude integration
- create_note, search, get_note, update_note, query_graph

### Import / Export
- Import: Obsidian vaults (wikilinks, frontmatter, inline tags)
- Import: Notion export directories (UUID cleanup, link conversion)
- Import: Plain Markdown directories
- Export: Static HTML site with navigation and styling
- Export: PDF (printpdf, A4, Helvetica/Courier, page breaks)
- Web clipper: HTML-to-Markdown conversion, bookmark saving

### Quality
- 257 tests across 8 crates (unit + integration)
- 77.69% test coverage
- Criterion benchmark suite for search and store operations
- Zero clippy warnings

### Documentation
- User guide covering all features
- API reference for all 30+ endpoints
- Architecture documentation with crate dependency graph and data flow
- 6 Architecture Decision Records (ADR-001 through ADR-006)

### AGNOS Integration
- Systemd service unit (`deploy/mneme.service`)
- Marketplace recipe (`deploy/mneme.agnos-agent`)
- 5 agnoshi voice/agent intents (`deploy/agnoshi-intents.toml`)
