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
