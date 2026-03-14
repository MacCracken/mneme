# Mneme Roadmap

Version: 2026.3.13

---

## Phase 1 — Foundation (MVP Core) ✓

Build the essential note-taking and search infrastructure.

### mneme-core ✓
- [x] Note type: id, title, content hash, timestamps, path
- [x] Link type: source, target, link text, context
- [x] Tag type: name, color, hierarchy (nested tags)
- [x] Graph types: nodes, edges, adjacency queries
- [x] Markdown frontmatter parsing (YAML metadata)

### mneme-store ✓
- [x] SQLite schema: notes, tags, links, note_tags, attachments
- [x] Migrations framework (sqlx)
- [x] CRUD operations for notes, tags, links
- [x] File-backed note storage (read/write Markdown files)
- [x] Content hashing for change detection (SHA-256)
- [x] File watcher for external edits (notify crate)

### mneme-search ✓
- [x] Tantivy index schema: title, body, tags, path
- [x] Index build / incremental update
- [x] Full-text search with ranking
- [x] Tag-based filtering
- [x] Search result types and pagination

### mneme-api ✓
- [x] Axum HTTP server scaffold
- [x] Note CRUD endpoints (GET/POST/PUT/DELETE)
- [x] Search endpoint (query + filters)
- [x] Tag management endpoints
- [x] Health check, version endpoint

---

## Phase 2 — Intelligence ✓

Add AI-powered features via daimon integration.

### mneme-search (additions) ✓
- [x] Vector/semantic search via daimon `/v1/vectors/*`
- [x] Embedding generation for notes (on create/update)
- [x] Hybrid search: combine BM25 + semantic ranking
- [x] "Similar notes" suggestions

### mneme-ai ✓
- [x] daimon RAG client (`/v1/rag/*` endpoints)
- [x] Note summarization pipeline
- [x] Auto-linking: suggest links between related notes
- [x] Concept extraction: entities, topics, key terms
- [x] Knowledge graph population from extracted concepts
- [x] Q&A over knowledge base (RAG pipeline)

### mneme-api (additions) ✓
- [x] AI endpoints: summarize, suggest-links, extract-concepts
- [x] RAG query endpoint
- [x] Knowledge graph query endpoints
- [x] Background indexing / re-indexing jobs

---

## Phase 3 — Interface & Integration ✓

Desktop GUI and Claude integration.

### mneme-ui ✓
- [x] Markdown editor with live preview
- [x] Note list / tree navigation
- [x] Search interface (full-text + semantic)
- [x] Backlinks panel (what links here)
- [x] Tag browser and management
- [ ] Knowledge graph visualization (force-directed or hierarchical)
- [ ] Split-pane / multi-note view

### mneme-mcp ✓
- [x] MCP 2.0 server scaffold
- [x] `mneme_create_note` tool
- [x] `mneme_search` tool
- [x] `mneme_get_note` tool
- [x] `mneme_update_note` tool
- [x] `mneme_query_graph` tool
- [ ] SecureYeoman MCP tool registration

---

## Phase 4 — Polish & Ecosystem (MVP Complete) ✓

Production readiness and AGNOS marketplace.

### Quality & Performance ✓
- [x] 106 tests across all crates (unit + integration)
- [x] Criterion benchmark suite: search latency, indexing throughput, store operations
- [ ] Large vault stress testing (10k+ notes)
- [ ] Memory profiling and optimization

### AGNOS Integration ✓
- [x] Marketplace recipe (`.agnos-agent` bundle)
- [x] 5 agnoshi intents for voice/agent interaction
- [x] Systemd service unit for background indexing
- [ ] SecureYeoman sandbox verification

### Documentation ✓
- [x] User guide
- [x] API reference
- [x] Architecture documentation
- [x] ADRs for all major decisions (001–004)

---

## Post-MVP

### Import / Export ✓ (mneme-io)
- [x] Import from Obsidian vaults (wikilinks, dataview)
- [ ] Import from Notion (API export)
- [x] Import from plain Markdown directories
- [x] Export to static site (HTML)
- [ ] Export to PDF

### Advanced AI (partial)
- [ ] AI-assisted writing: completions, rewording, expansion
- [ ] Translation pipeline (per-note or batch)
- [x] Automatic tagging suggestions
- [x] Note templates (daily, meeting, project)
- [ ] Temporal analysis (how knowledge evolves over time)
- [ ] Multi-modal notes (images, audio transcription via Shruti)

### Collaboration
- [ ] Delta integration for note version control
- [ ] Shared vaults (multi-user, conflict resolution)
- [ ] Real-time collaborative editing

### Creative Suite Integration
- [ ] Tazama: link notes to video projects, auto-generate shot lists
- [ ] Rasa: embed image annotations, link design assets
- [ ] Shruti: audio note transcription, podcast show notes
- [ ] BullShift: trade journal, research notes linked to positions

### Advanced Features
- [ ] Kanban / task views derived from note metadata
- [ ] Spaced repetition (flashcards from notes)
- [ ] Calendar integration (daily notes, event linking)
- [ ] Web clipper / bookmark integration
- [ ] Plugin/extension system

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 2026.3.13 | 2026-03-13 | Phase 4 complete — MVP ready |
