# Changelog

All notable changes to Mneme will be documented in this file.

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
