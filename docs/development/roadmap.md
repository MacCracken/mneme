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
- 257 tests, Criterion benchmarks, 6 ADRs, full documentation
- AGNOS marketplace recipe, systemd service, agnoshi intents

### Post-MVP
- Import: Obsidian, Notion, Markdown directories
- Export: HTML static site, PDF (printpdf)
- AI: writing assist, translation, temporal analysis, multi-modal, creative suite, flashcards
- Collaboration: versioning (Delta-ready), sharing (roles, conflicts)
- Advanced: tasks/kanban, calendar, web clipper, plugin system
- Knowledge graph visualization (force-directed layout, Canvas rendering, node cycling, zoom/pan)
- Split-pane / multi-note view in TUI (side-by-side, pane switching, note picker)

---

## Future Considerations

Items that may be explored in future versions:

- SecureYeoman MCP tool registration and sandbox verification
- Large vault stress testing (10k+ notes) and memory profiling
- Real-time WebSocket collaborative editing
- Import from Notion API (live sync, not just export)
- Plugin dynamic loading (currently static registry)

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 2026.3.15 | 2026-03-15 | Knowledge graph visualization + split-pane multi-note view |
| 2026.3.13 | 2026-03-13 | All phases complete — full feature set |
