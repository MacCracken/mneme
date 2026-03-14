# ADR-004: Import/Export as Separate Crate

## Status
Accepted

## Context

Mneme needs to import notes from external sources (Obsidian vaults, Markdown directories) and export to other formats (HTML static sites). These operations are distinct from core note management and involve format-specific parsing logic.

## Decision

Create a dedicated `mneme-io` crate for all import/export operations:

- **Import Obsidian**: Parse wikilinks (`[[note]]`, `[[note|alias]]`), convert to standard Markdown links, extract inline tags, preserve frontmatter.
- **Import Markdown**: Recursively scan directories for `.md` files, preserving directory structure as note paths.
- **Export HTML**: Render Markdown to HTML using comrak (GFM-compatible), generate navigation index, produce a self-contained static site.

The crate depends only on `mneme-core` for type definitions and uses `comrak` for Markdown rendering, `regex` for wikilink parsing, and `glob` for file discovery.

## Consequences

- **Positive**: Import/export logic is isolated from core CRUD operations, making it independently testable.
- **Positive**: Format-specific dependencies (comrak, regex) don't pollute other crates.
- **Positive**: New import/export formats can be added without modifying core crates.
- **Negative**: Import operations produce `CreateNote` structs that must then be persisted through the normal vault pipeline, adding a second step.
