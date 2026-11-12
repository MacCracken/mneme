# 016 — Confine note paths to the vault, and serve on loopback by default

**Status**: Accepted
**Date**: 2026-11-12

## Context

Two places where the port faithfully reproduced Rust behaviour that is unsafe in the setting mneme now runs in.

**Explicit note paths.** `CreateNote.path` is an optional, caller-supplied path that `mneme_store_vault_create_note`
wrote as given, matching rust-old's `path: Option<String>` (`crates/mneme-store/src/vault.rs`). A path such as
`../../.bashrc` therefore wrote outside the vault. The 2026-07-10 audit (§2) recorded this as inherited, on the
grounds that the only callers were trusted. Since 1.1.0 that is no longer true: the MCP `mneme_create_note` tool
passes `path` straight from a `tools/call` request, and `mneme serve` accepts those over HTTP.

**The served endpoint.** `mneme serve` (1.1.0) bound `INADDR_ANY` and has no authentication. Anyone who could reach
the port could read every note, create and update notes, and list vault paths. rust-old never had this exposure:
its MCP server spoke stdio only, and its HTTP API defaulted to `127.0.0.1:3838` (`MNEME_BIND`). The handler also
accepted any `Content-Type` and any `Host`, so a web page in the user's own browser could reach a loopback-bound
server two ways:
- a cross-origin `text/plain` POST, which needs no CORS preflight;
- DNS rebinding, which makes the page same-origin and lets it read the replies.

The port's rule is "matches what Rust did; diverge only with an ADR". Both changes below are divergences, so they
are recorded here.

## Decision

**Note paths stay inside the notes directory.** `mneme_store_note_path_is_safe` (`src/store_files.cyr`) refuses a
path that is empty, absolute (`/` or `\`), contains a NUL byte, or has a `..` component (split on `/` and `\`).
- `mneme_store_vault_create_note` returns 0 for such an explicit path, which callers already treat as a failed
  create.
- Every FileStore operation (read, write, delete, exists) applies the same check, so a path that reached the
  database some other way cannot escape either.
- A title-derived path is a flat slug and always passes.

**`mneme serve` binds loopback unless told otherwise.**
- The default address is `127.0.0.1`, as rust-old's API server's was.
- `MNEME_BIND=<dotted-quad>` opts into another interface (`0.0.0.0` for all of them); an unparseable value is a
  startup error.
- Every request must carry `Content-Type: application/json` (415 otherwise). This forces a browser into a CORS
  preflight, which the server never grants. daimon's sandhi RPC client always sends this header.
- While the server is loopback-bound, the `Host` header must name loopback (`127.0.0.1`, `localhost` or `[::1]`,
  with any port), or the request is refused with 403. A missing `Host` (HTTP/1.0) is allowed, since browsers always
  send one.
- When `MNEME_BIND` widens the bind, the Host check is off, because the operator has deliberately exposed the
  server.

## Consequences

- **Positive:**
  - A `tools/call` cannot write outside the vault.
  - The default `mneme serve` cannot be reached from the network or driven by a web page.
  - daimon on the same host, the documented deployment, is unaffected: it calls `http://127.0.0.1:<port>` with
    `application/json`.
- **Negative:**
  - A daimon on a different host now needs `MNEME_BIND` set and `MNEME_MCP_CALLBACK` pointed at a reachable
    address. A caller that relied on writing a note to `../somewhere` gets a failed create.
  - The endpoint is still unauthenticated once `MNEME_BIND` widens it. A shared-secret header needs daimon to send
    one, so it is not done here.
- **Neutral:**
  - The audit's §2 item is closed.
  - The 2026-11-12 audit (`docs/audit/2026-11-12-audit.md`) is the re-audit the first one asked for when a
    listening socket was added.

## Alternatives considered

- **Keep `INADDR_ANY` and add a token.** This would leave the default exposed until daimon learns to send a token,
  and is a larger change than the sweep. Rejected for now; it stays the next step if a remote daimon becomes common.
- **Reject only a leading `../`.** That still misses `a/../../x` and absolute paths, and would leave the check in
  one caller instead of in the store. Rejected.
- **Canonicalise with `realpath` and compare prefixes.** That handles symlinks too, but it needs the file to exist
  and a portable `realpath` on AGNOS. The component check is exact for the paths mneme writes. Symlinks inside the
  vault are the user's own and are followed as before.
