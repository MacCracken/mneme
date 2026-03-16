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

## Next: Knowledge Effectiveness

Improvements informed by SecureYeoman's brain/KB architecture to make Mneme a
smarter, more self-maintaining knowledge base.

### Phase 5 — In-Process Vector Store

Remove hard dependency on daimon for semantic search so Mneme works standalone.

- Embed a Rust-native ANN index (usearch or instant-distance) in `mneme-search`
- Add local embedding via ONNX Runtime (all-MiniLM-L6-v2, 384-dim)
- Fall back to daimon when available; run fully offline when not
- Persist index alongside Tantivy in `.mneme/vectors/`

### Phase 6 — Retrieval Quality & Feedback Loop

Learn from user behavior to improve search over time.

- Track which results the user opens/uses after a search (click-through signal)
- Implement lightweight Thompson Sampling over RRF blend weights
  (full-text weight, semantic weight, recency boost)
- Store arm stats in SQLite; decay toward priors on cold start
- Add `/v1/search/feedback` endpoint and MCP `mneme_search_feedback` tool

### Phase 7 — RAG Evaluation Metrics

Measure RAG quality so users know when to trust answers.

- Score each RAG response on: faithfulness (token overlap), answer relevance
  (cosine to query), and chunk utilization (% of retrieved context used)
- Expose per-query scores in API response and MCP tool output
- Log aggregate p50/p95 to `/v1/ai/rag/stats` for vault-level health

### Phase 8 — Note Consolidation & Evolution

Keep the vault healthy as it grows — reduce staleness and duplication.

- Periodic consolidation pass: detect near-duplicate notes (cosine > 0.92)
  and surface merge/split/update suggestions
- LLM-powered via daimon: evaluate overlap, propose edits, respect user approval
- Track content freshness (last-edited vs. last-accessed ratio)
- Surface stale notes in TUI and API (`/v1/notes/stale?days=90`)

### Phase 9 — Schema Clustering & Emergent Structure

Automatically discover topic structure in the vault.

- K-means++ clustering on note embeddings (configurable k or elbow heuristic)
- LLM-generated cluster labels and one-line summaries
- Surface as "suggested folders" or virtual tags in TUI and API
- Re-cluster incrementally as notes are added/updated

### Phase 10 — Context-Aware Retrieval

Improve relevance during interactive sessions (TUI, MCP).

- Fuse current session context with query embedding before search:
  `search_vec = λ·query_emb + (1−λ)·context_emb`
- Track a lightweight working-memory buffer of recent note IDs (capacity ~7)
- Bias retrieval toward notes related to the current editing context
- Configurable via `mneme.toml` (`context_retrieval.enabled`, `query_weight`)

### Phase 11 — Document Provenance & Trust Scoring

Not all notes are equal — weight retrieval by source quality.

- Assign provenance scores to notes by origin: manual (high), import (medium),
  web clip (lower), auto-generated (lowest)
- Allow user overrides per-note via frontmatter (`trust: high`)
- Factor provenance into hybrid search ranking as a multiplicative boost
- Display trust indicators in TUI search results

### Phase 12 — Multi-Vault Support

Manage multiple knowledge bases with unified cross-vault operations.

- `VaultRegistry` in mneme-store: track multiple vault roots with metadata
  (name, path, description, last-opened)
- Each vault is fully independent: own `.mneme/` dir, SQLite DB, search index,
  vector store
- Cross-vault search: fan out queries to all (or selected) vaults, merge results
  via RRF with vault-weight multipliers
- Cross-vault linking: `[[vault:note-title]]` syntax in Markdown, resolved at
  render time
- TUI vault switcher: quick-switch between vaults, vault picker on startup
- API: `/v1/vaults` CRUD, all existing endpoints accept optional `?vault=` param
- MCP: `mneme_list_vaults`, `mneme_switch_vault` tools; default vault from config
- CLI: `mneme vault create|list|switch|remove`
- Config in `mneme.toml`: `[vaults]` table with default vault and registry path
- Use cases: separate work/personal, per-project knowledge, shared team vaults

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
| 2026.3.15 | 2026-03-15 | Phase 5 (in-process vector store) + Phase 12 (multi-vault support) |
| 2026.3.15 | 2026-03-15 | Knowledge graph visualization + split-pane multi-note view |
| 2026.3.13 | 2026-03-13 | All phases complete — full feature set |
