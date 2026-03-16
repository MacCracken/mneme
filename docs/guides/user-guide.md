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
| `v` | Vault picker |
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

## Templates

Mneme includes built-in templates for common note types:

| Template | Description |
|----------|-------------|
| `daily` | Daily journal with tasks, notes, and log sections |
| `meeting` | Meeting notes with attendees, agenda, and action items |
| `project` | Project overview with goals, status, and links |

### Using Templates

Via the API:
```bash
# List available templates
curl http://localhost:3838/v1/templates

# Create a daily note from template
curl -X POST http://localhost:3838/v1/templates/render \
  -H 'Content-Type: application/json' \
  -d '{"template_name": "daily", "variables": {}, "create": true}'

# Render a meeting note (preview without creating)
curl -X POST http://localhost:3838/v1/templates/render \
  -H 'Content-Type: application/json' \
  -d '{"template_name": "meeting", "variables": {"topic": "Sprint Planning", "attendees": "Alice, Bob"}}'
```

Templates support variable substitution with `{{variable}}` syntax. Built-in variables include `date`, `time`, `datetime`, `year`, `month`, `day`, and `weekday`.

## Auto-Tagging

Mneme can suggest tags for notes based on content analysis:

```bash
# Get tag suggestions for a note
curl http://localhost:3838/v1/ai/suggest-tags/{note-id}
```

The tagger analyzes note content, matches against existing tags in your vault, and suggests new tags based on key concepts found in the text. Each suggestion includes a confidence score and reason.

## Import / Export

### Importing Notes

**From Obsidian vaults:**
Mneme can import Obsidian vaults, converting wikilinks (`[[note]]`) to standard Markdown links and preserving frontmatter metadata and tags.

**From Markdown directories:**
Import plain Markdown files from any directory, preserving the directory structure as note paths.

**From Notion exports:**
Import from Notion's exported directories — UUID-suffixed filenames are cleaned, Notion-style links are converted to standard Markdown, and CSV database files are detected.

### Exporting Notes

**To HTML:**
Export your vault as a static HTML site with rendered Markdown, navigation, and styling.

## AI Writing Assistant

Mneme can help with writing through three actions:

- **Complete** — continue writing from where you left off
- **Reword** — rephrase text while preserving meaning
- **Expand** — elaborate with more detail and examples

```bash
curl -X POST http://localhost:3838/v1/ai/write \
  -H 'Content-Type: application/json' \
  -d '{"action": "expand", "text": "Rust ensures memory safety."}'
```

When daimon is available, writing uses AI generation. Otherwise, it falls back to local heuristics.

## Translation

Translate notes to any of 12+ supported languages:

```bash
# Translate a note to Spanish
curl http://localhost:3838/v1/ai/translate/{note-id}?lang=es

# List supported languages
curl http://localhost:3838/v1/ai/languages
```

## Temporal Analysis

Track how your knowledge base evolves over time:

```bash
curl http://localhost:3838/v1/ai/temporal
```

Returns monthly activity, concept trends (rising/stable/declining), and growth metrics.

## Multi-Modal Notes

Mneme supports binary attachments alongside Markdown notes:

- **Images** (jpg, png, svg, etc.) — embedded with `![alt](attachments/file.jpg)`
- **Audio** (mp3, wav, flac, etc.) — transcribed via Shruti when available
- **Video** and **Documents** — linked as attachments

Attachments are stored in the `attachments/` directory within your vault.

## PDF Export

Export any note as a PDF:

```bash
curl http://localhost:3838/v1/export/pdf/{note-id} -o note.pdf
```

The PDF includes the note title, tags, and formatted content with proper typography.

## Tasks & Kanban

Mneme extracts tasks from Markdown checkbox syntax in your notes:

```markdown
- [ ] Regular task
- [x] Completed task
- [ ] Urgent fix !urgent
- [ ] Submit report @2026-03-15
```

Priority markers: `!urgent`, `!high`, `!low`. Due dates: `@YYYY-MM-DD`.

```bash
# Get tasks from a note
curl http://localhost:3838/v1/tasks/{note-id}

# Get all tasks across your vault
curl http://localhost:3838/v1/tasks
```

## Calendar

Notes with dates in their titles (e.g., "2026-03-13 — Daily Note") appear in the calendar view:

```bash
curl "http://localhost:3838/v1/calendar?year=2026&month=3"
```

## Flashcards

Generate study flashcards from note content:

```bash
curl http://localhost:3838/v1/flashcards/{note-id}
```

Cards are extracted from:
- **Definitions**: `**Term**: definition text`
- **Concepts**: `## Heading` followed by explanatory content

Includes SM-2 spaced repetition scheduling for optimal review intervals.

## Web Clipper

Save web pages and bookmarks as notes:

```bash
# Clip HTML content
curl -X POST http://localhost:3838/v1/clip/html \
  -H 'Content-Type: application/json' \
  -d '{"html": "<html>...</html>", "url": "https://example.com", "create": true}'

# Save a bookmark
curl -X POST http://localhost:3838/v1/clip/bookmark \
  -H 'Content-Type: application/json' \
  -d '{"url": "https://example.com", "title": "Example", "create": true}'
```

## Multi-Vault

Mneme supports managing multiple knowledge bases (vaults) with unified search across them.

### Setting Up Multiple Vaults

Configure vaults in `mneme.toml`:
```toml
[vaults]
default = "personal"

[vaults.personal]
path = "/home/user/notes"
description = "Personal knowledge base"

[vaults.work]
path = "/home/user/work-notes"
description = "Work projects"
```

Or manage vaults via the API:
```bash
# Create a new vault
curl -X POST http://localhost:3838/v1/vaults \
  -H 'Content-Type: application/json' \
  -d '{"name": "work", "path": "/home/user/work-notes", "description": "Work projects"}'

# List vaults
curl http://localhost:3838/v1/vaults

# Switch active vault
curl -X POST http://localhost:3838/v1/vaults/{id}/switch
```

Any endpoint accepts `?vault=<name>` to target a specific vault without switching.

### Cross-Vault Search

Searches can span all vaults. Results are merged using Reciprocal Rank Fusion with per-vault weight multipliers.

### TUI Vault Switcher

Press `v` in the TUI to open the VaultPicker panel and switch between vaults.

## Search Feedback

Mneme learns from your search behavior to improve ranking over time.

When you click a search result, the feedback is recorded and used by a Thompson Sampling bandit to optimize the blend weights between full-text, semantic, and recency ranking signals.

```bash
# Submit feedback after clicking a search result
curl -X POST http://localhost:3838/v1/search/feedback \
  -H 'Content-Type: application/json' \
  -d '{"search_id": "uuid-from-search-response", "note_id": "clicked-note-uuid", "action": "click"}'

# View optimizer statistics
curl http://localhost:3838/v1/search/optimizer
```

The optimizer state persists in `.mneme/optimizer.json` and improves with use.

## Local Vector Search

Mneme includes an in-process vector search engine that works without daimon.

- Uses ONNX Runtime with the all-MiniLM-L6-v2 model (384-dimensional embeddings)
- ANN index powered by usearch, persisted in `.mneme/vectors/`
- Feature-gated behind `local-vectors` (enabled by default)
- Falls back to daimon when `local-vectors` is disabled or for advanced operations

Set the model directory:
```bash
export MNEME_MODELS_DIR=/path/to/models
```

When daimon is available, Mneme uses it. When it is not, local vector search provides semantic capabilities offline.

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `MNEME_VAULT_DIR` | `~/mneme` | Vault directory |
| `MNEME_BIND` | `127.0.0.1:3838` | API server address |
| `DAIMON_URL` | `http://127.0.0.1:8090` | Daimon agent runtime |
| `DAIMON_API_KEY` | (none) | Daimon API key |
| `MNEME_MODELS_DIR` | (none) | Directory for ONNX embedding models |
| `RUST_LOG` | `info` | Log level |

## Data Storage

```
~/mneme/
├── notes/           # Your Markdown files (source of truth)
│   └── *.md
├── attachments/     # Binary attachments
└── .mneme/
    ├── db.sqlite    # Metadata index (rebuildable)
    ├── search-index/ # Tantivy index (rebuildable)
    ├── vectors/      # usearch ANN index (rebuildable)
    └── optimizer.json # Retrieval optimizer state
```

Your notes are always plain Markdown files. The database and search index are derived and can be rebuilt from the files at any time.
