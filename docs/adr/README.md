# Architecture Decision Records

Decisions about mneme — what we chose, the context, and the consequences we accept. Use these when a future reader would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNN-kebab-case-title.md`, zero-padded to three digits. Never renumber.
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to `Superseded by NNNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adr/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

| # | Decision |
|---|---|
| [001](001-rust-tantivy-sqlite.md) | Rust + Tantivy + SQLite |
| [002](002-plain-text-source-of-truth.md) | Plain Markdown files as source of truth |
| [003](003-daimon-delegation.md) | Daimon delegation for AI features |
| [004](004-import-export-architecture.md) | Import/export as a separate crate |
| [005](005-advanced-ai-architecture.md) | Advanced AI feature architecture |
| [006](006-collaboration-architecture.md) | Collaboration and extensibility architecture |
| [007](007-in-process-vectors.md) | In-process vector store |
| [008](008-retrieval-optimizer.md) | Retrieval optimizer |
| [009](009-multi-vault.md) | Multi-vault support |
| [010](010-rag-eval-metrics.md) | RAG evaluation metrics |
| [011](011-note-consolidation.md) | Note consolidation & evolution |
| [012](012-schema-clustering.md) | Schema clustering & emergent structure |
| [013](013-pluggable-embedding-backends.md) | Pluggable embedding backends |
| [014](014-training-feedback-export.md) | Training feedback export |
| [015](015-unterminated-code-fence.md) | Flush an unterminated code fence instead of dropping it |
