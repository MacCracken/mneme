# ADR-011: Note Consolidation & Evolution

**Status:** Accepted
**Date:** 2026-03-15

## Context

As vaults grow, notes accumulate duplicates, go stale, and diverge in quality. The existing Jaccard token-overlap duplicate detection is O(n²) and misses semantic similarity. There is no mechanism for tracking content freshness or suggesting merges.

## Decision

### Content Freshness

Add `last_accessed` column to notes. Bump it on full-content reads (not list views). Compute freshness as `days_since_update / max(days_since_access, 1)`:
- Score >> 1.0: accessed recently but old content (priority refresh)
- Score ~1.0: normal staleness
- High absolute age + low access: archive candidate

### Embedding-Based Duplicate Detection

Add `detect_duplicates_semantic()` alongside existing Jaccard detection. The API accepts a `method` parameter: "jaccard", "semantic", or "both". Semantic detection uses `SemanticEngine::find_similar_to()` to query the vector store per note, then deduplicates symmetric pairs.

### LLM Merge Suggestions

Delegate merge reasoning to daimon via `POST /v1/knowledge/merge`. Returns which note to keep, merged title/content, rationale, and confidence score. User must approve before any merge is applied.

## Consequences

- DB migration 002 adds `last_accessed` column with index
- `touch_note()` is fire-and-forget (non-blocking)
- Semantic duplicate detection requires an available embedding backend
- Merge suggestions require daimon connectivity; the endpoint returns 502 if unavailable
