# Contributing to Mneme

## Getting Started

1. Clone the repo and install Rust 1.85+
2. Run `make build` to verify the build
3. Run `make check` to run the full quality suite

## Architecture Rules

- **`mneme-core` must have zero I/O** — no tokio, no filesystem, no network calls. Pure types and logic only.
- **Search stays in `mneme-search`** — Tantivy indexes, vector queries, and ranking logic are isolated.
- **AI inference stays in `mneme-ai`** — model calls, RAG pipelines, and embedding generation are isolated.
- **`mneme-store` owns persistence** — SQLite access, file I/O, and migration logic live here only.
- All API handlers are thin wrappers — business logic lives in core.

## Code Quality

Before submitting:

```bash
make check    # fmt + clippy + tests
```

- `cargo fmt --all` — consistent formatting
- `cargo clippy -- -D warnings` — warnings are errors
- `cargo test --workspace` — all tests pass
- Target 65%+ test coverage

## Commit Messages

Use concise, descriptive commit messages:
- `add: semantic search pipeline` (new feature)
- `fix: backlink resolution for renamed notes` (bug fix)
- `refactor: extract knowledge graph module` (restructure)

## Crate Guidelines

| Crate | Can depend on | Cannot depend on |
|-------|--------------|-----------------|
| `mneme-core` | std only | anything with I/O |
| `mneme-store` | `mneme-core` | `mneme-ai`, `mneme-ui` |
| `mneme-search` | `mneme-core`, `mneme-store` | `mneme-ui`, `mneme-mcp` |
| `mneme-ai` | `mneme-core`, `mneme-search` | `mneme-ui`, `mneme-mcp` |
| `mneme-api` | `mneme-core`, `mneme-store`, `mneme-search`, `mneme-ai` | `mneme-ui` |
| `mneme-ui` | all internal crates | — |
| `mneme-mcp` | all internal crates | — |

## Git Workflow

- Branch from `main`
- Branch names: `feature/*`, `bugfix/*`, `docs/*`, `refactor/*`
- Conventional commits: `feat(core): add note linking`
- PR into `main`, squash merge
