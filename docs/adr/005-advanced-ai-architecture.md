# ADR-005: Advanced AI Feature Architecture

## Status
Accepted

## Context

Mneme needs advanced AI capabilities beyond basic RAG: writing assistance, translation, temporal analysis, and multi-modal note support. These features have varying dependency requirements on daimon.

## Decision

All advanced AI features follow the established delegation pattern:

1. **Try daimon first** — leverage the agent runtime for AI inference
2. **Fallback locally** — provide useful (if limited) results when daimon is unavailable

Specific decisions:
- **Writer**: Delegates generation to daimon; local fallback uses simple heuristics (word swaps, sentence templates)
- **Translator**: Delegates to daimon; local fallback returns a placeholder with preserved Markdown structure, clearly marked as pending translation
- **Temporal analysis**: Runs entirely locally using existing concept extraction. No daimon dependency — purely statistical analysis of note metadata and content
- **Multi-modal**: Image description and audio transcription delegate to daimon/Shruti. Attachment metadata management is local-only

## Consequences

- **Positive**: All features work (at reduced capability) without daimon
- **Positive**: No new external dependencies for AI features — reuses existing DaimonClient
- **Positive**: Temporal analysis is fully offline, enabling fast analytics
- **Negative**: Local writing fallbacks are basic heuristics, not true AI generation
- **Negative**: Translation placeholders require a second pass when daimon becomes available
