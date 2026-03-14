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
| `mneme-core` | None | Pure types: Note, Link, Tag, Graph, Frontmatter |
| `mneme-store` | SQLite, filesystem | Persistence: DB operations, file I/O, Vault |
| `mneme-search` | Tantivy, HTTP | Full-text index, semantic search, hybrid merge |
| `mneme-ai` | HTTP (daimon) | RAG, summarization, auto-linking, concepts |
| `mneme-api` | HTTP (axum) | REST API server |
| `mneme-ui` | Terminal (crossterm) | TUI application |
| `mneme-mcp` | Stdio | MCP server for Claude |

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
Client → API → SearchEngine.search()  → Tantivy BM25
          │
          └──→ DaimonClient.rag_query() → daimon /v1/rag/query
          │
          └──→ hybrid_merge() → combined ranked results
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
