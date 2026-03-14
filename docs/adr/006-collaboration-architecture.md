# ADR-006: Collaboration and Extensibility Architecture

## Status
Accepted

## Context

Mneme needs collaboration features (versioning, sharing) and an extensibility framework (plugins). These cut across multiple crates.

## Decision

### Versioning
In-memory version store in `mneme-store` that tracks note snapshots and computes line-level diffs. Designed for future Delta (AGNOS VCS) integration — the `VersionStore` interface can be backed by Delta's API without changing consumer code.

### Sharing
Vault sharing is managed through `SharingConfig` with three role levels (Owner, Editor, Viewer). Conflict detection compares content hashes — when two users edit the same base version, an `EditConflict` is raised. Resolution strategies: LastWrite, KeepBoth, or Manual.

### Plugin System
A capability-based `PluginRegistry` in `mneme-core` (zero-I/O) defines what plugins can do: NoteProcessor, Importer, Exporter, SearchProvider, AiPipeline. Hook points (PreCreate, PostUpdate, etc.) allow plugins to intercept operations. The registry is intentionally simple — no dynamic loading yet, just a typed registration API.

## Consequences

- **Positive**: Versioning is decoupled from Delta — works standalone with in-memory store
- **Positive**: Sharing roles are simple and auditable
- **Positive**: Plugin system is extensible without modifying core crates
- **Negative**: In-memory version store loses history on restart (Delta integration resolves this)
- **Negative**: No real-time WebSocket sync yet — sharing is file-level, not character-level
