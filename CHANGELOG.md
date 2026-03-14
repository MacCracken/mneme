# Changelog

All notable changes to Mneme will be documented in this file.

## [Unreleased]

### Added
- **Import**: Notion export directory import with UUID cleanup, link conversion
- **Export**: PDF export with printpdf (A4, Helvetica/Courier, page breaks)
- **AI Writing**: Complete, reword, and expand text via daimon with local fallbacks
- **Translation**: Per-note and batch translation with 12+ supported languages
- **Temporal Analysis**: Monthly activity tracking, concept trend detection (rising/stable/declining)
- **Multi-Modal**: Image/audio/video attachment support, Shruti transcription integration
- **API**: 5 new endpoints — `/v1/ai/write`, `/v1/ai/translate/{id}`, `/v1/ai/languages`, `/v1/ai/temporal`, `/v1/export/pdf/{id}`
- **ADR-005**: Advanced AI feature architecture decisions
- **Tasks/Kanban**: Extract tasks from Markdown checkboxes, priority markers, due dates; board views
- **Calendar**: Daily note calendar, month views, entry type detection from titles
- **Plugin System**: Plugin registry with capability-based lookup and hook points
- **Versioning**: Note version history with line-level diffs, Delta integration ready
- **Sharing**: Multi-user vault sharing with Owner/Editor/Viewer roles, conflict detection
- **Creative Suite**: Tazama shot lists, Rasa design assets, Shruti show notes, BullShift trade journal
- **Flashcards**: Spaced repetition with SM-2 scheduling, card extraction from definitions and concepts
- **Web Clipper**: HTML-to-Markdown conversion, bookmark saving with auto-tagging
- **API**: 7 new endpoints — tasks, calendar, flashcards, web clipper, plugins
- **ADR-006**: Collaboration and extensibility architecture decisions

## [2026.3.13] — 2026-03-13

### Added
- **Core**: Note, Link, Tag, Graph types with Markdown frontmatter parsing
- **Store**: SQLite persistence with CRUD operations, file-backed Markdown storage, content hashing (SHA-256)
- **Search**: Tantivy full-text search with BM25 ranking, hybrid semantic search via daimon, tag filtering
- **AI**: RAG pipeline, note summarization, auto-linking, concept extraction, auto-tagging, note templates
- **API**: Axum HTTP server (port 3838) with 18 endpoints — note CRUD, search, tags, AI, templates
- **UI**: Terminal interface with note list, viewer, search, backlinks panel, tag browser
- **MCP**: JSON-RPC 2.0 server with 5 tools for Claude integration
- **Import/Export**: Obsidian vault import (wikilinks, frontmatter), Markdown directory import, HTML static site export

### Quality
- 106 tests across 8 crates (unit + integration)
- Criterion benchmark suite for search and store operations
- Zero clippy warnings

### Documentation
- User guide with templates, auto-tagging, and import/export sections
- API reference covering all 18 endpoints
- Architecture documentation with crate dependency graph and data flow diagrams
- 4 Architecture Decision Records (ADR-001 through ADR-004)

### AGNOS Integration
- Systemd service unit (`deploy/mneme.service`)
- Marketplace recipe (`deploy/mneme.agnos-agent`)
- 5 agnoshi voice/agent intents (`deploy/agnoshi-intents.toml`)
