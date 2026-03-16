# Changelog

All notable changes to Mneme will be documented in this file.

## [2026.3.15] — 2026-03-15

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

### Quality
- 297 tests (up from 257)
- New deps: usearch 2, ort 2.0.0-rc.12, ndarray 0.17, tokenizers 0.21
- 3 new ADRs (007–009)

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
