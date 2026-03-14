# Mneme Roadmap

Version: 2026.3.13

---

## Phase 1 — Foundation (MVP Core)

Build the essential note-taking and search infrastructure.

### mneme-core
- [ ] Note type: id, title, content hash, timestamps, path
- [ ] Link type: source, target, link text, context
- [ ] Tag type: name, color, hierarchy (nested tags)
- [ ] Graph types: nodes, edges, adjacency queries
- [ ] Markdown frontmatter parsing (YAML metadata)

### mneme-store
- [ ] SQLite schema: notes, tags, links, note_tags, attachments
- [ ] Migrations framework (sqlx)
- [ ] CRUD operations for notes, tags, links
- [ ] File-backed note storage (read/write Markdown files)
- [ ] Content hashing for change detection (SHA-256)
- [ ] File watcher for external edits (notify crate)

### mneme-search
- [ ] Tantivy index schema: title, body, tags, path
- [ ] Index build / incremental update
- [ ] Full-text search with ranking
- [ ] Tag-based filtering
- [ ] Search result types and pagination

### mneme-api
- [ ] Axum HTTP server scaffold
- [ ] Note CRUD endpoints (GET/POST/PUT/DELETE)
- [ ] Search endpoint (query + filters)
- [ ] Tag management endpoints
- [ ] Health check, version endpoint

**Exit criteria**: Can create, edit, search, and tag notes via API. Plain Markdown files on disk.

---

## Phase 2 — Intelligence

Add AI-powered features via daimon integration.

### mneme-search (additions)
- [ ] Vector/semantic search via daimon `/v1/vectors/*`
- [ ] Embedding generation for notes (on create/update)
- [ ] Hybrid search: combine BM25 + semantic ranking
- [ ] "Similar notes" suggestions

### mneme-ai
- [ ] daimon RAG client (`/v1/rag/*` endpoints)
- [ ] Note summarization pipeline
- [ ] Auto-linking: suggest links between related notes
- [ ] Concept extraction: entities, topics, key terms
- [ ] Knowledge graph population from extracted concepts
- [ ] Q&A over knowledge base (RAG pipeline)

### mneme-api (additions)
- [ ] AI endpoints: summarize, suggest-links, extract-concepts
- [ ] RAG query endpoint
- [ ] Knowledge graph query endpoints
- [ ] Background indexing / re-indexing jobs

**Exit criteria**: Semantic search works. Can ask questions across notes. Auto-link suggestions appear.

---

## Phase 3 — Interface & Integration

Desktop GUI and Claude integration.

### mneme-ui
- [ ] Markdown editor with live preview
- [ ] Note list / tree navigation
- [ ] Search interface (full-text + semantic)
- [ ] Backlinks panel (what links here)
- [ ] Tag browser and management
- [ ] Knowledge graph visualization (force-directed or hierarchical)
- [ ] Split-pane / multi-note view

### mneme-mcp
- [ ] MCP 2.0 server scaffold
- [ ] `mneme_create_note` tool
- [ ] `mneme_search` tool
- [ ] `mneme_get_note` tool
- [ ] `mneme_update_note` tool
- [ ] `mneme_query_graph` tool
- [ ] SecureYeoman MCP tool registration

**Exit criteria**: Usable desktop app. Claude can create/search/read notes via MCP.

---

## Phase 4 — Polish & Ecosystem (MVP Complete)

Production readiness and AGNOS marketplace.

### Quality & Performance
- [ ] 65%+ test coverage across all crates
- [ ] Benchmark suite: search latency, indexing throughput
- [ ] Large vault stress testing (10k+ notes)
- [ ] Memory profiling and optimization

### AGNOS Integration
- [ ] Marketplace recipe (`.agnos-agent` bundle)
- [ ] 5 agnoshi intents for voice/agent interaction
- [ ] Systemd service unit for background indexing
- [ ] SecureYeoman sandbox verification

### Documentation
- [ ] User guide
- [ ] API reference
- [ ] Architecture documentation
- [ ] ADRs for all major decisions

**Exit criteria**: Marketplace-ready. Passes AGNOS sandbox verification. Documented.

---

## Post-MVP

### Import / Export
- [ ] Import from Obsidian vaults (wikilinks, dataview)
- [ ] Import from Notion (API export)
- [ ] Import from plain Markdown directories
- [ ] Export to static site (HTML)
- [ ] Export to PDF

### Advanced AI
- [ ] AI-assisted writing: completions, rewording, expansion
- [ ] Translation pipeline (per-note or batch)
- [ ] Automatic tagging suggestions
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
- [ ] Templating system (daily notes, meeting notes, etc.)
- [ ] Kanban / task views derived from note metadata
- [ ] Spaced repetition (flashcards from notes)
- [ ] Calendar integration (daily notes, event linking)
- [ ] Web clipper / bookmark integration
- [ ] Plugin/extension system

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 2026.3.13 | 2026-03-13 | Project scaffolding, roadmap |
