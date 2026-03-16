# Mneme

*Greek: μνήμη (memory — muse of memory)*

AI-native knowledge base and personal notes built in Rust with semantic search, auto-linking, and RAG over personal documents.

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│                  mneme (binary)                   │
├──────────┬──────────┬──────────┬─────────────────┤
│ mneme-ui │ mneme-mcp│ mneme-ai │   mneme-api     │
├──────────┴──────────┴──────────┴─────────────────┤
│                  mneme-search                     │
│          (full-text + semantic indexing)           │
├──────────────────────┬───────────────────────────┤
│     mneme-store      │       mneme-core           │
│  (SQLite, files)     │  (types, document model)   │
└──────────────────────┴───────────────────────────┘
```

| Crate | Purpose |
|-------|---------|
| `mneme-core` | Zero-I/O core types: notes, links, tags, graph, markdown AST |
| `mneme-store` | Persistence layer: SQLite metadata, file-backed note storage, multi-vault registry |
| `mneme-search` | Full-text search (Tantivy) + pluggable vector search (local ONNX or remote HTTP) + adaptive retrieval optimizer + context buffer |
| `mneme-ai` | AI pipelines: summarization, auto-linking, concept extraction, RAG, clustering, consolidation, training export |
| `mneme-api` | HTTP API server, integrates with daimon RAG/vector/knowledge endpoints |
| `mneme-ui` | GUI: editor, graph visualization, search, backlinks panel |
| `mneme-mcp` | MCP 2.0 server exposing 8 tools for Claude integration |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (2024 edition) |
| Full-text Search | Tantivy |
| Vector Search | Pluggable: local (usearch + ONNX) or remote HTTP (Synapse/Ollama/OpenAI) |
| RAG | daimon `/v1/rag/*` API |
| Knowledge Graph | daimon `/v1/knowledge/*` API |
| Markdown | `comrak` (GFM-compatible) |
| Database | SQLite via `sqlx` |
| GUI | `iced` or Tauri v2 |
| AI Inference | Local models via Synapse, daimon endpoints |

## Quick Start

```bash
# Build
make build

# Run
make run

# Development mode (auto-reload)
make dev

# Run tests
make test

# Full quality check (fmt + lint + test)
make check
```

## AI Features

- **Pluggable Embeddings** — local ONNX (offline) or remote HTTP (Synapse, Ollama, OpenAI) with auto-fallback
- **Semantic Search** — find notes by meaning, not just keywords
- **Context-Aware Retrieval** — session context biases search toward your working topic
- **Adaptive Ranking** — Thompson Sampling learns from your clicks to improve search over time
- **Multi-Vault** — manage multiple knowledge bases with cross-vault search (RRF merge)
- **Note Consolidation** — detect duplicates (Jaccard + cosine), track freshness, LLM merge suggestions
- **Schema Clustering** — K-means++ discovers topic structure; elbow heuristic auto-selects k
- **Document Provenance** — trust scoring by origin (manual, import, web clip, AI-generated) with frontmatter overrides
- **Training Export** — JSONL export of search feedback and note content for model fine-tuning
- **Auto-Linking** — automatically discover and suggest links between related notes
- **Summarization** — generate summaries of long notes or collections
- **RAG** — ask questions across your entire knowledge base with quality metrics
- **Concept Extraction** — automatically extract entities, topics, and key concepts
- **AI-Assisted Writing** — completions, rewording, expansion, and translation
- **Knowledge Graph** — visualize connections between notes, concepts, and tags

## MCP Tools (Claude Integration)

| Tool | Description |
|------|-------------|
| `mneme_create_note` | Create a new note with title, content, and tags |
| `mneme_search` | Search notes by keyword, semantic similarity, or tag |
| `mneme_get_note` | Retrieve a note with its content, backlinks, and metadata |
| `mneme_update_note` | Update note content, tags, or metadata |
| `mneme_query_graph` | Query the knowledge graph for related concepts |
| `mneme_search_feedback` | Record search result feedback to improve ranking |
| `mneme_list_vaults` | List all registered vaults |
| `mneme_switch_vault` | Switch the active vault |

## Ecosystem

Mneme is part of the AGNOS AI-native application suite:

| Project | Role |
|---------|------|
| [**Tazama**](https://github.com/anomalyco/tazama) | AI-native video editor |
| [**Rasa**](https://github.com/agnostos/rasa) | AI-native image editor |
| [**Shruti**](https://github.com/MacCracken/shruti) | AI-native DAW / audio workstation |
| **Mneme** | AI-native knowledge base (this project) |
| [**Delta**](https://github.com/agnostos/delta) | Code hosting platform |
| [**BullShift**](https://github.com/MacCracken/BullShift) | Trading platform |
| [**Synapse**](https://github.com/anomalyco/synapse) | LLM controller & model manager |
| [**Photis Nadi**](https://github.com/MacCracken/photisnadi) | Kanban task management & daily rituals |

Mneme leverages existing AGNOS infrastructure — daimon's RAG, vector store, and knowledge graph APIs — making it the highest-ROI Tier 1 application.

## Design Principles

1. **Local-first** — all data stays on-device, no cloud dependency
2. **Plain-text native** — Markdown files as source of truth, database as index
3. **Zero-I/O core** — `mneme-core` has no filesystem, network, or async runtime
4. **AI-augmented** — semantic intelligence enhances manual organization
5. **Interoperable** — standard Markdown, easy import/export, daimon API integration

## License

AGPL-3.0 — see [LICENSE](LICENSE) for details.

Copyright (C) 2026 Robert MacCracken
