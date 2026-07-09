# mneme — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**0.1.0** — ported from Rust (2026-07-09) via `cyrius port`. **21,014 lines** of
Rust preserved at `rust-old/` (the full original workspace) for parity reference.

## Toolchain

- **Cyrius pin**: `6.4.33` (in `cyrius.cyml [package].cyrius`)

## Source

- Rust reference: 21,014 lines at `rust-old/` (frozen, do not edit — parity oracle).
- Cyrius port in progress:
  - **M1 `mneme-core` — COMPLETE.** All 9 modules ported into `src/core_*.cyr`
    (tag, frontmatter, note, link, graph, plugin, calendar, task, config) + a
    `core_uuid` helper (v4 random / v5 sha1).
  - **M2 `mneme-io` / `mneme-store` — IN PROGRESS.** Pure-logic modules first:
    `io_wikilink`, `io_notion` done. Next: web_clipper, export_html, then the
    store side (versioning, sharing, registry) and the I/O + `patra` tier.

## Tests

**12 `.tcyr` files, ~118 assertions — all green.** Each mirrors the corresponding
Rust `#[cfg(test)]` module 1:1 against `rust-old/`. (`#[tokio::test]` async I/O
tests are ported alongside the fs/patra tier.)

## Dependencies

Direct (declared in `cyrius.cyml [deps].stdlib`):

- string, fmt, alloc, vec, str, hashmap, math, chrono, random, sha1, tagged,
  fnptr, bayan, syscalls, io, args, assert
- `bayan` (TOML/JSON) is a carve — also explicitly `include "lib/bayan.cyr"`.
- Planned: `patra` (embedded SQL) for `mneme-store`; sibling deps (`bote`, daimon
  HTTP) in later milestones.

## Consumers

_None yet._

## Next

See [`roadmap.md`](roadmap.md). Finish M2's pure-logic modules, then the
`fs` + `patra` + async I/O tier (store/db, files, vault, manager; io imports).
