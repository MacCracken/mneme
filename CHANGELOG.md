# Changelog

All notable changes to Mneme will be documented in this file.

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
