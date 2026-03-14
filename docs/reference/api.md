# Mneme API Reference

Base URL: `http://127.0.0.1:3838`

## Health

### GET /health
Returns server status and note count.

**Response:**
```json
{"status": "ok", "version": "2026.3.13", "notes_count": 42}
```

## Notes

### POST /v1/notes
Create a new note.

**Request:**
```json
{
  "title": "Note Title",
  "content": "Markdown content",
  "tags": ["tag1", "tag2"],
  "path": "optional/custom-path.md"
}
```

**Response:** `201 Created` with the full note object.

### GET /v1/notes?limit=50&offset=0
List notes, ordered by most recently updated.

### GET /v1/notes/{id}
Get a note with full content, tags, and backlinks.

### PUT /v1/notes/{id}
Update a note. All fields optional.

**Request:**
```json
{
  "title": "New Title",
  "content": "New content",
  "tags": ["new-tags"]
}
```

### DELETE /v1/notes/{id}
Delete a note. Returns `204 No Content`.

## Search

### GET /v1/search?q=query&limit=20
Full-text search across note titles, bodies, and tags.

**Response:**
```json
[
  {
    "note_id": "uuid",
    "title": "Note Title",
    "path": "note.md",
    "snippet": "...matching text...",
    "score": 1.23
  }
]
```

## Tags

### GET /v1/tags
List all tags.

### DELETE /v1/tags/{id}
Delete a tag.

## AI Endpoints

### GET /v1/ai/rag/query?q=question&top_k=5
Ask a question across all ingested notes (requires daimon).

### GET /v1/ai/rag/stats
RAG pipeline statistics and daimon availability.

### POST /v1/ai/rag/ingest/{id}
Ingest a specific note into the RAG pipeline.

### GET /v1/ai/summarize/{id}
Summarize a note's content.

### GET /v1/ai/suggest-links/{id}?top_k=5
Get auto-link suggestions for a note.

### GET /v1/ai/concepts/{id}
Extract concepts (topics, entities, key terms) from a note.

### GET /v1/ai/suggest-tags/{id}
Suggest tags for a note based on content analysis.

**Response:**
```json
[
  {
    "tag": "rust",
    "confidence": 0.85,
    "reason": "matches existing vocabulary",
    "source": "vocabulary"
  }
]
```

## Templates

### GET /v1/templates
List all available templates.

**Response:**
```json
{
  "templates": [
    {
      "name": "daily",
      "description": "Daily journal / log note",
      "title_pattern": "{{date}} — Daily Note",
      "default_tags": ["daily"],
      "variables": []
    }
  ]
}
```

### POST /v1/templates/render
Render a template with variable substitution, optionally creating the note.

**Request:**
```json
{
  "template_name": "meeting",
  "variables": {"topic": "Sprint Planning", "attendees": "Alice, Bob"},
  "create": false
}
```

**Response:**
```json
{
  "title": "Meeting: Sprint Planning — 2026-03-13",
  "content": "# Meeting: Sprint Planning\n...",
  "tags": ["meeting"],
  "path": null,
  "created": false
}
```

## AI Writing

### POST /v1/ai/write
AI-assisted writing: complete, reword, or expand text.

**Request:**
```json
{
  "action": "complete",
  "text": "Rust is a systems programming language.",
  "context": "optional surrounding context"
}
```

Actions: `complete`, `reword`, `expand`

**Response:**
```json
{
  "original": "Rust is a systems programming language.",
  "result": "Rust is a systems programming language. Furthermore...",
  "action": "complete",
  "source": "daimon"
}
```

## Translation

### GET /v1/ai/translate/{id}?lang=es&source_lang=en
Translate a note's content to the target language.

**Response:**
```json
{
  "original": "Hello world.",
  "translated": "Hola mundo.",
  "source_language": "en",
  "target_language": "es",
  "word_count": 2,
  "source": "daimon"
}
```

### GET /v1/ai/languages
List supported translation languages.

## Temporal Analysis

### GET /v1/ai/temporal
Analyze knowledge evolution across all notes.

**Response:**
```json
{
  "total_notes": 42,
  "date_range": ["2026-01-01", "2026-03-13"],
  "activity_by_month": [{"period": "2026-03", "notes_created": 5, "notes_updated": 3, "total_words": 2500}],
  "concept_trends": [{"term": "rust", "trend": "rising", "total_occurrences": 15}],
  "most_active_month": "2026-03",
  "avg_words_per_note": 150
}
```

## Export

### GET /v1/export/pdf/{id}
Export a note as a PDF file.

Returns binary PDF with `Content-Type: application/pdf` and `Content-Disposition` header.

## Tasks / Kanban

### GET /v1/tasks
Get all tasks extracted from all notes.

**Response:**
```json
{
  "tasks": [{"id": "uuid", "note_id": "uuid", "text": "Do something", "completed": false, "priority": "high", "line_number": 3}],
  "total": 5,
  "completed": 2,
  "pending": 3
}
```

### GET /v1/tasks/{id}
Get tasks from a specific note.

## Calendar

### GET /v1/calendar?year=2026&month=3
Get a calendar month view showing notes with dates in their titles.

**Response:**
```json
{
  "year": 2026,
  "month": 3,
  "entries": [{"date": "2026-03-13", "note_id": "uuid", "title": "2026-03-13 — Daily Note", "entry_type": "daily_note"}],
  "days_with_notes": 5
}
```

## Flashcards

### GET /v1/flashcards/{id}
Extract flashcards from a note's content (definitions, concepts).

**Response:**
```json
[
  {"id": "uuid", "note_id": "uuid", "front": "What is Rust?", "back": "A systems programming language.", "card_type": "definition"}
]
```

## Web Clipper

### POST /v1/clip/html
Clip HTML content to a Markdown note.

**Request:**
```json
{"html": "<html>...</html>", "url": "https://example.com", "create": true}
```

### POST /v1/clip/bookmark
Save a bookmark as a note.

**Request:**
```json
{"url": "https://example.com", "title": "Example", "description": "A test site", "create": true}
```

## Plugins

### GET /v1/plugins
List installed plugins.
