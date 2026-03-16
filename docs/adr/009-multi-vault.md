# ADR-009: Multi-Vault Support

## Status
Accepted

## Context

Users often maintain separate knowledge bases for different domains (personal, work, per-project). Mneme previously supported a single vault directory. Supporting multiple vaults requires a registry, per-vault isolation, and cross-vault operations.

## Decision

### VaultRegistry
`mneme-store/src/registry.rs` implements a `VaultRegistry` backed by TOML persistence. Each vault entry stores name, path, description, and last-opened timestamp. The registry file lives at a user-level path (not inside any single vault).

### VaultManager
`mneme-store/src/manager.rs` provides `VaultManager`, which manages the lifecycle of multiple vaults — opening, closing, and switching between them. Each vault is fully independent with its own `.mneme/` directory containing SQLite DB, Tantivy index, vector store, and optimizer state.

### Config Types
`mneme-core/src/config.rs` defines `MnemeConfig`, `VaultConfigEntry`, and `VaultInfo` as zero-I/O types. Configuration is read from `mneme.toml` with a `[vaults]` table specifying default vault and per-vault entries.

### Cross-Vault Search
`mneme-search/src/cross_vault.rs` fans out queries to multiple vaults in parallel and merges results using Reciprocal Rank Fusion with per-vault weight multipliers. This allows users to search across all vaults or a selected subset.

### API Integration
`mneme-api/src/state.rs` introduces `VaultState` and `VaultWithEngines` to hold per-vault search engines and stores. `mneme-api/src/vault_handlers.rs` provides `/v1/vaults` CRUD and `/v1/vaults/{id}/switch`. All existing handlers accept an optional `?vault=` query parameter.

### MCP and TUI
MCP adds `mneme_list_vaults` and `mneme_switch_vault` tools (8 total). The TUI adds a VaultPicker panel accessible via `v`.

## Consequences

- **Positive**: Full vault isolation — each vault can be backed up, moved, or shared independently
- **Positive**: Cross-vault search provides a unified view when needed
- **Positive**: `?vault=` parameter is backward-compatible — existing single-vault users are unaffected
- **Negative**: Per-vault resources (search engines, DB connections) multiply memory usage
- **Negative**: Cross-vault linking (`[[vault:note]]`) is resolved at render time, not enforced referentially
