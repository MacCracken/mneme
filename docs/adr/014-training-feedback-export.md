# ADR-014: Training Feedback Export

**Status:** Accepted
**Date:** 2026-03-15

## Context

Mneme captures user signals that are valuable for fine-tuning domain-specific models:
- Search feedback (which results users click)
- Note edits after searches (implicit relevance signals)
- Trust/provenance overrides (quality corrections)
- Note content itself (embedding training pairs)

These signals are currently used only for the Thompson Sampling retrieval optimizer and not exported for model training.

## Decision

### Training Record Types

Define a `TrainingRecord` tagged enum with four variants:
- **SearchClick**: query, clicked_note_id, title, search_arm, position
- **EditAfterSearch**: query, note_id, title, latency_secs
- **TrustOverride**: note_id, old_provenance, new_trust
- **NoteContent**: note_id, title, content, tags

### Append-Only JSONL Log

Store training records in `.mneme/training.jsonl` per vault. The `TrainingLog` type provides:
- `append()` — write a single record (atomic line write)
- `read_all()` / `read_filtered()` — read with optional type and date filters
- `export_jsonl()` — raw JSONL string for streaming export
- `clear()` — reset the log after export

### API Endpoint

`GET /v1/export/training-data` with query params:
- `type` — filter by record type
- `since` — ISO 8601 date filter
- `include_notes` — include all note content as NoteContent records

### Integration Points

The search feedback handler (`POST /v1/search/feedback`) now logs SearchClick records when the request includes the optional `query` field. The `SearchFeedbackRequest` is backward-compatible — existing clients that don't send `query` still work for optimizer feedback, they just don't generate training records.

## Alternatives Considered

- **SQLite table for training data**: Rejected — JSONL is simpler, append-only, and directly consumable by training pipelines without serialization overhead.
- **Streaming export via WebSocket**: Deferred — the JSONL file approach is sufficient for batch fine-tuning. Real-time export can be added via the daimon event bus (Phase 15).

## Consequences

- Training log grows unbounded — users should periodically export and clear
- JSONL format is compatible with Synapse, HuggingFace, and standard ML tooling
- No PII scrubbing yet — Phase 14 roadmap item for future privacy controls
- The optional `query` field on feedback means training data quality depends on client cooperation
