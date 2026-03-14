# ADR-001: Rust + Tantivy + SQLite

## Status
Accepted

## Context

Mneme is an AI-native knowledge base and notes application for the AGNOS ecosystem. We need to choose the primary language, search engine, and persistence layer.

## Decision

- **Language**: Rust (2024 edition), consistent with all AGNOS Tier 1 applications (Tazama, Rasa, Shruti, Delta, Synapse).
- **Full-text search**: Tantivy — a Rust-native search engine library. Embeds directly into the application with no external service dependency. Supports BM25 ranking, faceted search, and custom tokenizers.
- **Database**: SQLite via `sqlx` — lightweight, file-based, consistent with ecosystem. Stores note metadata, tags, links, and graph edges. Note content stays as plain Markdown files.
- **Vector/semantic search**: Delegated to daimon's `/v1/vectors/*` API rather than embedding a vector database. This leverages existing AGNOS infrastructure and avoids duplicating Synapse/daimon capabilities.

## Consequences

- Tantivy provides fast full-text search without an external service (unlike Elasticsearch/Meilisearch).
- SQLite keeps the deployment simple — single binary + data directory.
- Daimon dependency for semantic search means Mneme needs a running AGNOS instance for AI features, but basic note-taking and full-text search work standalone.
- Plain Markdown files as source of truth means notes are portable and human-readable outside Mneme.
