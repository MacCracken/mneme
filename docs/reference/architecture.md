# Mneme Architecture

## Crate Dependency Graph

```
                    mneme-ui    mneme-mcp
                      │  │        │  │
                      │  └────┬───┘  │
                      │       │      │
                      ▼       ▼      │
                    mneme-api        │
                      │  │           │
              ┌───────┘  └────┐     │
              ▼               ▼     │
          mneme-ai      mneme-search│
              │  │          │  │    │
              │  └──────┐   │  │    │
              ▼         ▼   ▼  │    │
          mneme-store ◄─────┘  │    │
              │                │    │
              ▼                ▼    ▼
          mneme-core ◄─────────────┘
```

## Crate Responsibilities

| Crate | I/O | Responsibility |
|-------|-----|---------------|
| `mneme-core` | None | Pure types: Note, Link, Tag, Graph, Frontmatter, tasks, calendar, plugins, config (`MnemeConfig`, `VaultConfigEntry`, `VaultInfo`) |
| `mneme-store` | SQLite, filesystem | Persistence: DB operations, file I/O, Vault, versioning, sharing, `VaultRegistry` (TOML), `VaultManager` (multi-vault lifecycle) |
| `mneme-search` | Tantivy, ONNX, usearch | Full-text index, `Embedder` (ONNX all-MiniLM-L6-v2), `VectorStore` (usearch ANN), `SemanticEngine` (facade), `RetrievalOptimizer` (Thompson Sampling), `CrossVaultSearch` (RRF merge), hybrid merge with `BlendWeights` |
| `mneme-ai` | HTTP (daimon) | RAG, summarization, auto-linking, concepts, writing, translation, temporal, multi-modal, creative suite, flashcards |
| `mneme-api` | HTTP (axum) | REST API server, `VaultState` + `VaultWithEngines`, vault handlers |
| `mneme-ui` | Terminal (crossterm) | TUI application |
| `mneme-mcp` | Stdio | MCP server for Claude |
| `mneme-io` | Filesystem | Import (Obsidian, Markdown, Notion), export (HTML, PDF), web clipper |

## Data Flow

### Note Creation
```
Client → API → Vault.create_note()
                ├── FileStore.write_note()     → notes/path.md
                ├── Database.insert_note()     → db.sqlite
                ├── Database.tag_note()        → db.sqlite
                └── SearchEngine.index_note()  → search-index/
```

### Search Query
```
Client → API → RetrievalOptimizer.select_arm() → BlendWeights
          │
          ├──→ SearchEngine.search()       → Tantivy BM25
          ├──→ SemanticEngine.search()     → usearch ANN (local) or daimon fallback
          └──→ weighted_hybrid_merge()     → combined ranked results + search_id
          │
          └──→ CrossVaultSearch (if multi) → fan-out + RRF merge with vault weights
```

### RAG Pipeline
```
Ingest: Note → DaimonClient.rag_ingest() → daimon chunks + indexes
Query:  Question → DaimonClient.rag_query() → context + source chunks
```

## Design Decisions

See `docs/adr/` for Architecture Decision Records:

- **ADR-001**: Rust + Tantivy + SQLite
- **ADR-002**: Plain Markdown files as source of truth
- **ADR-003**: Daimon delegation for AI features
- **ADR-006**: Collaboration and extensibility architecture
- **ADR-007**: In-process vector store (usearch + ONNX Runtime)
- **ADR-008**: Retrieval optimizer (Thompson Sampling feedback loop)
- **ADR-009**: Multi-vault support (VaultRegistry, VaultManager, cross-vault search)

## AGNOS Integration

```
┌─────────────────────────────────────────┐
│                  AGNOS                    │
│                                           │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │  Mneme   │  │  Daimon  │  │Synapse │ │
│  │  :3838   │──│  :8090   │──│(local) │ │
│  │          │  │ RAG/Vec  │  │ models │ │
│  └──────────┘  └──────────┘  └────────┘ │
│       │                                   │
│  ┌──────────┐                            │
│  │SecureYeo │                            │
│  │(sandbox) │                            │
│  └──────────┘                            │
└─────────────────────────────────────────┘
```

- **Daimon**: RAG, vector store, knowledge endpoints
- **Synapse**: Local LLM inference for AI features
- **SecureYeoman**: Sandbox enforcement, MCP tool registration
- **Marketplace**: `.agnos-agent` bundle distribution
