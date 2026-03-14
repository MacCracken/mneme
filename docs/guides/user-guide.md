# Mneme User Guide

## What is Mneme?

Mneme (Greek: μνήμη, "memory") is an AI-native knowledge base and personal notes application. It stores your notes as plain Markdown files, indexes them for fast full-text and semantic search, and uses AI to discover connections between your ideas.

## Getting Started

### Installation

```bash
# From source
git clone https://github.com/agnostos/mneme
cd mneme
make build

# On AGNOS
ark install mneme
```

### Your First Vault

Mneme stores notes in a **vault** — a directory containing your Markdown files and a `.mneme/` metadata folder.

```bash
# Start the API server (creates vault at ~/mneme by default)
mneme-api

# Or specify a custom vault location
MNEME_VAULT_DIR=/path/to/vault mneme-api
```

### Creating Notes

Via the API:
```bash
curl -X POST http://localhost:3838/v1/notes \
  -H 'Content-Type: application/json' \
  -d '{"title": "My First Note", "content": "Hello, Mneme!", "tags": ["getting-started"]}'
```

Via the TUI:
```bash
mneme-ui
```

Via Claude (MCP):
```
Create a note about Rust's ownership model with tags rust and programming
```

## Concepts

### Notes
Notes are Markdown files with optional YAML frontmatter:
```markdown
---
title: My Note
tags: [rust, programming]
---
Your content here.
```

### Tags
Tags organize notes hierarchically using `/` separators:
- `project/agnos/core` — nested under `project/agnos`
- Tags are created automatically when applied to notes

### Links
Notes can reference each other. Backlinks show you what links **to** a note.

### Knowledge Graph
Notes, tags, and concepts form a graph. Query it to discover connections.

## Terminal UI

Launch with `mneme-ui`. Keyboard shortcuts:

| Key | Action |
|-----|--------|
| `q` | Quit |
| `/` | Search |
| `n` | Notes list |
| `t` | Tag browser |
| `j`/`k` or arrows | Navigate |
| `Enter` | Open/select |
| `Esc` | Back |

## AI Features

When connected to daimon (AGNOS agent runtime), Mneme provides:

- **Semantic Search** — find notes by meaning
- **Summarization** — condense long notes
- **Auto-Linking** — discover related notes
- **Concept Extraction** — identify key topics
- **RAG** — ask questions across your knowledge base

These features degrade gracefully — basic note-taking and full-text search work without daimon.

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `MNEME_VAULT_DIR` | `~/mneme` | Vault directory |
| `MNEME_BIND` | `127.0.0.1:3838` | API server address |
| `DAIMON_URL` | `http://127.0.0.1:8090` | Daimon agent runtime |
| `DAIMON_API_KEY` | (none) | Daimon API key |
| `RUST_LOG` | `info` | Log level |

## Data Storage

```
~/mneme/
├── notes/           # Your Markdown files (source of truth)
│   └── *.md
├── attachments/     # Binary attachments
└── .mneme/
    ├── db.sqlite    # Metadata index (rebuildable)
    └── search-index/ # Tantivy index (rebuildable)
```

Your notes are always plain Markdown files. The database and search index are derived and can be rebuilt from the files at any time.
