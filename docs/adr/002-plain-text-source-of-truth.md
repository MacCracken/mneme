# ADR-002: Plain Markdown Files as Source of Truth

## Status
Accepted

## Context

Knowledge base applications face a fundamental tension: database-first storage (fast queries, structured) vs. file-first storage (portable, human-readable, version-controllable).

## Decision

Note content is stored as plain Markdown files on disk. SQLite and Tantivy serve as derived indexes that can be rebuilt from the file system at any time.

Directory structure:
```
~/mneme/
├── notes/
│   ├── 2026/
│   │   └── 03/
│   │       └── my-note.md
│   └── projects/
│       └── agnos-roadmap.md
├── attachments/
└── .mneme/
    ├── db.sqlite
    └── search-index/
```

## Consequences

- Notes are portable — copy/move/edit with any text editor.
- Version control friendly — git integration for note history.
- Index rebuild is always possible — no data loss if SQLite is corrupted.
- Slightly more complex write path (file + DB + index update).
- File-watching needed to detect external edits.
