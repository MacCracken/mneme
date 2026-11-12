# Changelog

All notable changes to Mneme will be documented in this file.

## [Unreleased]

## [1.2.1] — 2026-11-12

**Toolchain and vendored stdlib refreshed: Cyrius `6.7.9` → `6.7.18`, with every file in `lib/` matching the
`6.7.18` stdlib byte-for-byte. The one source edit is a rename off two newly deprecated bayan TOML names. 62 files
/ 1,133 assertions, all green, with identical per-suite counts at both pins.**

### Changed
- **Toolchain pin `6.7.9` → `6.7.18`.** CI installs from the `cyrius.cyml [package].cyrius` pin, so this needs no
  workflow edit.
- **`lib/` re-resolved via `cyrius deps`.** 86 locked files: 56 changed, 23 unchanged, and 7 new under
  `lib/unicode/` (`casefold`, `categories`, `normalize`, `_decode` and three data tables), which bayan 1.11.1
  includes itself. Folded libs:
  - bayan `1.5.13` → `1.11.1`
  - patra `1.16.0` → `1.16.6`
  - sandhi `1.10.9` → `1.13.7`
  - sakshi `2.5.8` → `2.5.10`
  - sigil `3.13.11` → `3.13.15`
- **The kept modules `lib/hashmap_fast.cyr` and `lib/ws_server.cyr` were refreshed by hand from the `6.7.18`
  stdlib.** `hashmap_fast` is byte-identical between `6.7.9` and `6.7.18`; `ws_server` changed. 6.7.18's
  `cyrius deps` refuses to re-lock across a pin move while an undeclared `lib/` file differs from the new snapshot.
  Its suggested fixes are to delete the file or to vendor the whole snapshot (`cyrius lib sync --full`); neither
  was used. Copying the two modules first lets `cyrius deps` lock them and keep them as "outside the declared
  footprint". The order is recorded in `docs/development/state.md`.
- **`bayan_toml_get` → `bayan_toml_get_by_cstr` and `bayan_toml_get_sections` → `bayan_toml_get_sections_by_cstr`**
  (`src/core_config.cyr`, 3 + 2 sites). bayan 1.11.0 deprecated the bare names because they don't say what kind of
  key they take. Each forwards to its `_by_cstr` form, so this is a pure rename and changes no answers: every key is
  a C-string literal, including the ones that pass through `_cfg_toml_str`. The build no longer prints the five
  deprecation warnings, and the binary size is unchanged by the rename.
- **`mneme serve` now refuses more malformed JSON** (bayan 1.11.1, no mneme source change). The JSON tree parser
  returns 0 for a duplicate object key, a leading-zero number (`01`), a number with no digits, or an out-of-range
  integer. The request body at `src/mcp_server.cyr:104` is mneme's only bayan JSON parse, so such a body now gets
  the existing `-32700 Parse error` reply instead of being dispatched. No test sends one. The embedding and daimon
  HTTP responses are parsed by hand and are unaffected.
- **`cyrius.lock` re-locked** for the new pin: `cyrius deps --verify` reports 86 verified, 0 failed.

### Checked, no change needed
A read of cyrius's `ecosystem-migration.md` for 6.7.10–6.7.18, checked against mneme's source, found nothing else
that applies:
- no float literals or `: f64` annotations, so the 6.7.11–6.7.16 float-typing changes can't reach mneme;
- no structs, closures, generics, tuples or multi-value returns;
- no enum members separated without `;`;
- no newly refused top-level statements;
- no name colliding with the new stdlib names.

The suite also compiles with no undefined-function or duplicate-definition warnings, and the tests that include
`lib/bayan.cyr` by hand pick up its unicode modules without help.

### Verified at the new pin
- `cyrius build src/main.cyr build/mneme`: OK, 2,595,568 → 2,899,832 B (+304,264). The folded libs' source grew
  (bayan +259 KB plus 353 KB of unicode tables, patra +78 KB, sandhi +67 KB, sigil +45 KB), and mneme builds
  without dead-code elimination. The build and test warnings are the `6.7.9` set minus sigil's static-buffer
  warning and the "large static data" warning, which no longer appear.
- `cyrius test`: 62 passed, 0 failed (1,133 assertions), the same count in every suite as at `6.7.9`, both before
  and after the rename.
- `scripts/pdfcheck.sh --strict`: 7/7 under bayan 1.11.1's validator.
- `tests/mneme.fcyr`: ok.
- `tests/mneme.bcyr` and `benches/mneme_bench.bcyr` both run (`6.7.9` run in brackets):
  - `core_uuid_v4` 550 ns (567)
  - `ai_tokenize` 3.96 µs (4.00)
  - `squared_distance_3d` 37 ns (38)
  - `bm25_search_200docs` 309 µs (297)

  These are single runs taken at different times, not an interleaved A/B.

## [1.2.0] — 2026-11-12

**Toolchain onto the Cyrius 6.7.x language minor: `6.6.20` → `6.7.9`, with every file in `lib/` matching the
`6.7.9` stdlib byte-for-byte. No source change. 62 files / 1,133 assertions, all green, with identical per-suite
counts at both pins.**

### Changed
- **Toolchain pin `6.6.20` → `6.7.9`.** 6.7.x is a language minor: checked traits, `bool`/`true`/`false` and a real
  `!`, `const`, `loop`/`do … while`, tuples, default and named arguments, `dyn`, and an opt-in bounds-checked mode.
  CI installs from the `cyrius.cyml [package].cyrius` pin, so this needs no workflow edit.
- **`lib/` re-resolved via `cyrius deps`.** 79 modules: 54 changed, 25 unchanged, none added or removed. Folded libs:
  - bayan `1.5.12` → `1.5.13`
  - patra `1.15.2` → `1.16.0`
  - sandhi `1.10.8` → `1.10.9`
  - sakshi `2.5.7` → `2.5.8`
  - sigil `3.13.10` → `3.13.11`
- **`lib/hashmap_fast.cyr` and `lib/ws_server.cyr` refreshed by hand from the `6.7.9` stdlib**, as at 1.1.5. They are
  kept on purpose and `cyrius deps` doesn't manage them. The `6.7.9` `hashmap_fast` carries the upstream `fhm_set`
  fix: overwriting a key that is already present no longer rebuilds the table, and no longer drops the value when the
  allocator is exhausted.
- **`cyrius.lock` re-locked** for the new pin (79 hashes): `cyrius deps --verify` reports 79 verified, 0 failed.

### Checked, no change needed
Every 6.7.0–6.7.9 entry in cyrius's `ecosystem-migration.md` that can silently change a computed value was checked
against mneme's source. None applies:
- **A lone `!` was silently dropped before 6.7.3**, so `if (!ok)` ran as `if (ok)`. mneme uses no `!` operator.
- **`self` in an `impl` is now a `*T`** (6.7.0), narrow `*p` reads and writes the element (6.7.8), and a pointer's step
  no longer leaks through other operators (6.7.9). mneme has no `impl`, no `self` and no `*T` typed pointer.
- **New reserved words** (`trait`, `const`, `true`, `false`, `do`, `dyn`) and **new stdlib names** (`f64_parse_n`,
  `sock_set_nodelay`, `sys_ppoll`). mneme declares none of them.
- **patra 1.16.0 refuses `ORDER BY` on a TEXT/BYTES chain column**, which used to sort by on-disk position. mneme
  orders only by `notes.updated_at` (INT) and `tags.name` (STR, an inline comparable type), so both queries are
  unaffected.

### Verified at the new pin
- `cyrius build src/main.cyr build/mneme`: OK, 2,593,824 → 2,595,568 B. The build warnings are the same set; sigil's
  static-buffer warning only moved line (28046 → 28103).
- `cyrius test`: 62 passed, 0 failed (1,133 assertions), the same count in every suite as at `6.6.20`.
- `scripts/pdfcheck.sh --strict`: 7/7 under bayan 1.5.13's validator.
- `tests/mneme.fcyr`: ok.
- `tests/mneme.bcyr` and `benches/mneme_bench.bcyr` both run (`6.6.20` run in brackets):
  - `core_uuid_v4` 567 ns (595)
  - `ai_tokenize` 4.00 µs (4.43)
  - `squared_distance_3d` 38 ns (42)
  - `bm25_search_200docs` 297 µs (310)

  These are single runs taken at different times, not an interleaved A/B, so the small gains are not a measured
  speed-up.

## [1.1.5] — 2026-11-12

**Toolchain and vendored stdlib refreshed: Cyrius `6.6.2` → `6.6.20`, and every file in `lib/` now matches the
`6.6.20` stdlib byte-for-byte. The bump needed no behavioral source change. Its one source edit is a rename off a
newly deprecated bayan name. 62 files / 1,133 assertions, all green, with identical per-suite counts at both pins.**

### Changed
- **Toolchain pin `6.6.2` → `6.6.20`.** CI installs from the `cyrius.cyml [package].cyrius` pin, so this needs no
  workflow edit.
- **`lib/` re-resolved via `cyrius deps`.** 79 modules: 70 changed, 5 unchanged (`alloc_agnos`, `args_macos`,
  `args_win`, `ct`, `tls_native_keysched`), and 4 newly pulled in as transitive includes of declared leaves
  (`alloc_cx` and `poison` via `alloc`, `sys` via `sigil`, `tls_hostid` via `tls`). Folded libs:
  - bayan `1.5.5` → `1.5.12`
  - patra `1.14.1` → `1.15.2`
  - sandhi `1.9.16` → `1.10.8`
  - sakshi `2.5.1` → `2.5.7`
  - sigil `3.12.16` → `3.13.10`
- **`lib/math.cyr` re-vendored in the same change, as the roadmap required for any pin ≥ 6.6.13.** 6.6.13 made
  `f64_le`, `f64_ge` and `f64_trunc` compiler builtins (reserved names). The 6.6.2 `math.cyr` still defined all
  three; the 6.6.20 copy defines none.
- **`f64_parse` is correctly rounded since 6.6.13** (cyrius I4), so parsed values can move by up to 2 ulp. mneme
  calls it in 29 files (11 src, 18 tests). No asserted value moved: every suite reports the same count at `6.6.2`
  and `6.6.20`, so there was no golden numeric output to re-baseline.
- **`bayan_json_v_obj_get` → `bayan_json_v_obj_get_by_cstr`** (`src/mcp_server.cyr` ×5, `tests/mcp_server.tcyr`
  ×3). bayan 1.5.10 deprecated the old name because it does not say what kind of key it takes. It forwards to
  `_by_cstr`, so this is a pure rename and changes no answers: every key at these sites is a C-string literal. The
  build no longer prints the five deprecation warnings, and the binary size is unchanged (2,593,824 B).

### Added
- **`cyrius.lock`** (79 SHA-256 hashes plus the `cyrius 6.6.20` row). `cyrius deps` now writes a lock for a
  stdlib-only project, and `cyrius deps --verify` checks every vendored file against it: 79 verified, 0 failed.

### Fixed
- **Two vendored modules had gone stale: `lib/hashmap_fast.cyr` and `lib/ws_server.cyr`.** They are not declared in
  `[deps].stdlib` and nothing in mneme includes them, so `cyrius deps` never refreshed them. They already differed
  from the 6.6.2 stdlib. Both were copied from the `6.6.20` stdlib and re-locked with `cyrius deps --lock`. They
  are kept on purpose, so each pin bump refreshes them by hand (the procedure is in `docs/development/state.md`).

### Verified at the new pin
- `cyrius build src/main.cyr build/mneme`: OK.
- `cyrius test`: 62 passed, 0 failed (1,133 assertions).
- `scripts/pdfcheck.sh --strict`: 7/7 under bayan 1.5.12's validator.
- `tests/mneme.fcyr`: ok.
- `tests/mneme.bcyr` and `benches/mneme_bench.bcyr` both run:
  - `core_uuid_v4` 595 ns
  - `ai_tokenize` 4.43 µs
  - `squared_distance_3d` 42 ns
  - `bm25_search_200docs` 310 µs

  These are single runs and were not compared against a `6.6.2` bench run.

## [1.1.4] — 2026-09-11

### Changed

- **Toolchain `6.5.35` → `6.6.2`.** No source change; the value form needed none.
  Build, tests, and any bench/fuzz/distlib target the repo ships re-verified at the new pin.

## [1.1.3] — 2026-08-22

**Toolchain and vendored stdlib refreshed: Cyrius `6.5.33` → `6.5.35`, carrying bayan `1.4.2` → `1.5.2` (eight fixed
security/correctness defects) and patra `1.13.9` → `1.13.10`. The upgrade itself needed no mneme source change —
every suite reported identical counts across it. Reading the fold then surfaced a port regression worth fixing:
`core_config.cyr` wrote TOML values unescaped, where `rust-old` escaped them. Also repairs the release-tag
workflow, which two Rust-era leftovers had made incapable of completing a release, and updates every GitHub Action.
62 files, 1,122 → 1,133 assertions, all green.**

### Fixed
- **`mneme_core_vault_entry_to_toml` / `mneme_core_config_to_toml` wrote TOML values UNESCAPED**
  (`src/core_config.cyr`). Five string values — vault `name`, `path`, `description`, plus `default_vault` and the
  embedding `backend` — were interpolated straight between literal quotes, so a value holding a `"`, a newline or a
  backslash emitted **malformed TOML that silently lost data on the way back in**. `rust-old` got this right for
  free: it serializes with `toml::to_string` (`crates/mneme-core/src/config.rs:166,202`), which escapes basic
  strings. This is a **parity repair, not a divergence** — no ADR needed.
  - Values are now escaped on write (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, and `\u00XX` for any other
    control byte, which TOML forbids bare inside a basic string).
  - ⚠ **The matching decoder is not optional.** bayan does **not** unescape TOML basic strings — `_toml_parse_str`
    returns the raw slice between the quotes with the backslashes still in it (bayan's only unescape,
    `_jp_token_unescape`, is JSON's). So mneme must decode exactly what mneme encoded; all 8 string-value read
    sites now go through `_cfg_toml_str` (= `bayan_toml_get` + unescape, with absent-is-0 preserved). The two
    halves must be changed together, and the new tests fail loudly if bayan ever starts unescaping.
  - Non-ASCII is untouched: `load8` is zero-extended, so a UTF-8 lead/continuation byte (128–255) never trips the
    `c < 32` control-character arm. Asserted directly (`Résumé — notes` round-trips).
  - **`tests/core_config.tcyr` 13 → 24 assertions**, and mutation-proven: **6 of the 11 new ones fail against the
    pre-fix writer**. The other 5 pin behavior that previously only *looked* right because writing raw and reading
    raw cancelled out — the emitted file was still invalid TOML that a conforming parser (Rust's included) would
    have read differently, so those assertions check the emitted bytes, not just the round-trip.

- **`.github/workflows/release-automation.yml` could never complete a release.** Two independent defects, both
  Rust-era leftovers the 1.1.2 sweep missed while fixing the identical bug in `scripts/bump-version.sh`:
  - It ran `git add VERSION Cargo.toml Cargo.lock`. **No `Cargo.toml` or `Cargo.lock` has existed since the port**,
    and `git add` on a missing path is fatal — so the job died at the commit step every time. Now `git add VERSION`
    alone, which is all that changes: `cyrius.cyml` reads `${file:VERSION}`.
  - Its version gate validated **CalVer** (`^YYYY.M.D$`) while every other moving part of the release is semver —
    `VERSION` is `1.1.x`, `scripts/bump-version.sh` rejects anything but `MAJOR.MINOR.PATCH`, and `release.yml`
    triggers on the tag glob `[0-9]+.[0-9]+.[0-9]+*`. A CalVer input passed this gate and then died one step later
    in `bump-version.sh`; a real semver input was rejected outright. Now `^[0-9]+\.[0-9]+\.[0-9]+$`, matching the
    tag trigger and the script.

### Changed
- **Toolchain pin `6.5.33` → `6.5.35`.** `6.5.35` and `6.5.34` are compiler-internal — band F finally lets the
  linear-scan register allocator time-share a register (loop-aware liveness via `RA_SCAN_LOOPS`, plus the `picked`
  lifetime cap that was silently blocking every interval after the fifth), and band E made `CYRIUS_IR=3` agree with
  the default backend on all 282 corpus files. mneme builds on the default backend, so the observable effect here is
  a smaller binary, not new behavior. CI installs from the `cyrius.cyml [package].cyrius` pin, so this flows through
  with no workflow edit.
- **`lib/` re-resolved via `cyrius deps`.** Exactly **two** of the 73 vendored modules changed — `bayan.cyr` and
  `patra.cyr`; the other 71 are byte-identical between `6.5.33` and `6.5.35`. Every file in `lib/` now matches the
  `6.5.35` stdlib byte-for-byte.
  ⚠ Use **`cyrius deps`**, not `cyrius update`. `deps` is manifest-driven and resolves only what
  `[deps].stdlib` declares; `cyrius update` copies the toolchain's *entire* `lib/` — it adds 29 modules mneme never
  declared (`regex`, `mabda`, `yukti`, `niyama`, `sankoch`, `unicode/`, …), silently widening the dependency surface.
  (`cyrius update --help` is also not a recognised flag: it *performs* the update rather than printing help.)
- **bayan `1.4.2` → `1.5.2`** (215,533 → 641,083 B), folded in by Cyrius 6.5.34. Carries real security work from
  three upstream releases: two heap buffer overflows in `csv` (CWE-787), a one-byte remote crash in `base64`, an
  unchecked allocation in `yaml`, an out-of-bounds read in `cyml`, a **remote memory-exhaustion DoS in `json`**,
  dropped carries in `bigint`'s `u256_mul`, and `bayan_u64_mulmod` taking **SIGFPE on ordinary inputs**. mneme's
  exposure is the `json` DoS: `src/mcp_server.cyr` parses JSON-RPC envelopes straight off a socket in
  `mneme serve`, so this hardens a remote-facing path. All **19** `bayan_*` symbols mneme calls (13 `json_v_*`,
  5 `toml_*`, `json_last_error`) are present and signature-unchanged — the `1.5.0` PDF writer/reader is additive.
- **patra `1.13.9` → `1.13.10`.** The whole diff is one hunk: `patra_init` no longer calls
  `sakshi_set_level(SK_WARN)`, a process-global call that reset the host's configured log level the moment it opened
  a database. No effect on mneme, which sets no level and makes no sakshi call — but it means `mneme serve` can no
  longer have its logging silenced by `patra_open` if a level is ever configured. All 10 `patra_*` symbols mneme
  calls are unchanged.

- **GitHub Actions updated and consistently SHA-pinned.** `actions/checkout` **v4 → v7.0.1**,
  `softprops/action-gh-release` **v2 → v3.0.2**, `actions/dependency-review-action` **v4 → v5.0.0**. `sbom.yml` and
  `release-automation.yml` were tracking floating `@v4` tags while `ci.yml`/`release.yml` pinned by SHA; all six
  references are now pinned to a verified commit SHA with the tag in a trailing comment, matching the repo's
  existing "pinned — CVE-21" convention.
- **Removed `.github/actions/setup-rust-env/`** — a Rust composite action (rustup, `cargo install cross`, a cache
  keyed on `**/Cargo.lock`) left from the original workspace. Referenced by **no** workflow since the port, and it
  pulled `dtolnay/rust-toolchain@master` — a floating, mutable ref, i.e. the one genuine supply-chain exposure in
  `.github/`. Deleted rather than version-bumped: there is no Rust here to set up.

### Measured — the binary grew 199 KB, which is the incoming PDF implementation
- `build/mneme` **2,460,904 → 2,660,040 B (+199,136, +8.1 %)**, and it is *not* the toolchain: 6.5.35's release gate
  records cycc itself getting 4,096 B **smaller**. It is bayan's PDF stack — unreachable functions in the link went
  **2,757 → 3,126 (+369)**, the shape of the 152 `bayan_pdf_*` functions.
- **Not a regression to triage.** `bayan_pdf_*` is the designated replacement for the hand-rolled writer in
  `src/io_export_pdf.cyr` (bayan's own module was shaped around auditing it — see
  [`roadmap.md`](docs/development/roadmap.md)'s now-unblocked P2 item). These bytes are that implementation arriving
  ahead of its consumer; the migration makes them reachable and retires mneme's hand-roll in exchange. Recorded as a
  measurement, not a cost.

### Verified
- **Build + full suite green on `6.5.35`**, and identical to the `6.5.33` baseline: all **62** `.tcyr` files report
  the *same* per-suite pass counts before and after (**1,122 assertions**, 0 failures). The toolchain-drift warning
  the build printed against the stale pin is gone.
- **`scripts/pdfcheck.sh --strict` passes 7/7** against bayan `1.5.2`'s validator, including the 5-page/13-object
  fixture — the tripled bayan did not make the strict PDF oracle reject mneme's hand-rolled writer.

## [1.1.2] — 2026-08-21

**Fix: the PDF exporter silently lost note content — an unterminated ``` fence dropped everything after it, and
every write failure reported success. Toolchain moved to Cyrius `6.5.33`.**

### Fixed
- **`mneme_io_parse_markdown_blocks` (`src/io_export_pdf.cyr`) dropped an unterminated code fence entirely.**
  When the input ended while `in_code` was still 1 — a plain typo — the parse loop exited without flushing
  `code_buf`, so **every line after the stray fence was absent from the exported PDF**: no error, no warning, and a
  structurally valid file that opens fine. `"intro\n```\ncode\nmore code\nEVERYTHING AFTER IS LOST\n"` produced
  exactly one block (`TEXT "intro"`) and discarded the other four lines. The parser now flushes a trailing open
  fence as an `MD_CODE` block — the same shape a closing fence produces — so no input line can be dropped, and
  records the malformed input in a new **`mneme_io_md_unterminated_fence()`** (module-global + accessor, bayan's
  `bayan_json_last_error()` idiom) so callers can warn rather than accept it silently. A bare trailing ```` ``` ````
  swallows nothing and so adds no empty block. This is a **deliberate divergence from `rust-old/`**, which has the
  identical bug at `crates/mneme-io/src/export_pdf.rs:247-291` — see
  [`docs/adr/015-unterminated-code-fence.md`](docs/adr/015-unterminated-code-fence.md).
- **`mneme_io_export_note_to_pdf` reported success for every failed write.** It discarded `file_write_all`'s
  result and returned 0 unconditionally, so a full disk, an unwritable path, and a missing directory were all
  indistinguishable from a successful export while nothing reached disk. It now returns 0 only when the whole
  buffer was written, else the negative errno from open/write, or `-5` (EIO) for a short write — a truncated,
  unopenable PDF must not read as success. Rust returned `Result<(), IoError>` here.
- **`mneme_io_export_notes_to_pdf` counted every attempt as a success.** It discarded `sys_mkdir`'s result and
  incremented `count` per note regardless of outcome, so an uncreatable output directory reported every note as
  exported with zero files written. It now propagates the first failure (directory creation or any note's write)
  as a negative return, matching Rust's `?` on `create_dir_all` + `export_note_to_pdf`, and returns the count only
  on full success. The bare one-level `sys_mkdir` is replaced by the stdlib's **`xmkdir_p`** (6.5.7+), which is
  `create_dir_all`'s equivalent — so a **nested** output directory now works (it previously failed) and
  re-exporting into an existing directory is not an error.
- **`tests/mcp_server.tcyr` could not fail the build.** It ended `return 0` instead of `return assert_summary()`
  — the only test file that did — so its 16 assertions ran but a failure was never reported to the harness.
- **`scripts/bump-version.sh` was broken.** It sed-ed `Cargo.toml`, left over from the Rust original; no
  `Cargo.toml` has existed since the port, so `set -euo pipefail` made the script fail outright. Rewritten for the
  Cyrius layout, where `VERSION` is the single source of truth (`cyrius.cyml` reads `${file:VERSION}`), with
  `major`/`minor`/`patch` bumps.

### Changed
- **Toolchain pin `6.4.58` → `6.5.33`**, and `lib/` re-resolved via `cyrius deps` (51 stdlib modules updated).
  CI installs from the `cyrius.cyml [package].cyrius` pin, so this flows through with no workflow edit.
- **bayan renamed `bayan_json_v_parse_str` → `bayan_json_v_parse_buf`** (bayan 1.3.0): the `X_str` suffix is
  reserved for "the Str-taking variant of X", and a cstr+len form occupying that name was being silently
  mis-dispatched from 1-arg `bayan_json_v_parse(someStr)` call sites across the ecosystem. Updated the two mneme
  call sites (`src/mcp_server.cyr`, `tests/mcp_server.tcyr`); both pass a raw pointer + length, so this is a
  straight rename with no behavior change.

### Added
- **[`docs/adr/015-unterminated-code-fence.md`](docs/adr/015-unterminated-code-fence.md)** — records the
  deliberate divergence from the `rust-old/` parity oracle, per the "diverge only with an ADR" rule in `CLAUDE.md`.
  The ADR index in `docs/adr/README.md` (which still read "No ADRs yet" alongside 14 committed ADRs) is now
  populated, and its stated filename convention corrected to the three-digit form every file on disk already uses.
- **`scripts/pdfcheck.sh`** — an acceptance gate over bayan's strict, dependency-free PDF validator
  (`bayan/scripts/pdfcheck.py`), reused rather than vendored so both ports are gated by one oracle. Unlike
  poppler/mupdf — which rebuild a broken xref by scanning for `N 0 obj` and so hide exactly this class of bug — it
  checks every offset, `/Length` and `/Size` against the actual bytes. All 7 fixtures the test suite emits pass,
  including the 5-page/13-object case. Warns and skips when bayan is not checked out; `--strict` makes that fatal.
- **`tests/io_export_pdf.tcyr` strengthened from 21 assertions to 49.** The previous tests asserted only that the
  file existed, exceeded 100 bytes, and began with `%PDF` — true of almost any garbage, and unable to catch either
  bug above. Added: regression cover for both fixes (blocks, the diagnostic flag, the recovered text in the
  exported PDF bytes, and both error-propagation paths); structural assertions (`/Type/Catalog`, `/Type/Page`,
  font, `trailer`, `%%EOF`); the note's own text surviving into the content stream; and a **byte-accurate xref
  check** that `startxref` points exactly at the `xref` keyword. Verified to actually catch the regressions: the
  new suite fails 7 assertions against the pre-fix code and passes 49/49 after. Failure paths use a regular file as
  a blocked parent (ENOTDIR on every platform, and valid even when tests run as root) rather than a
  permission-based or presumed-missing path.

## [1.1.1] — 2026-07-13

**Fix: MCP self-registration now sends each tool's `inputSchema` (it was omitted, so daimon advertised a bare `{}`).**

### Fixed
- **`_mcp_self_register` / `_mcp_reg_body` (`src/mcp_server.cyr`) omitted the tool `inputSchema`** from the
  registration payload it POSTs to daimon (`/v1/mcp/tools`) — it sent only `name`/`description`/`callback_url`.
  So daimon advertised every `mneme_*` tool with an **empty `{}`** schema, even though mneme defines real ones
  (`mcp_protocol.cyr`). An empty `{}` is **not** a valid Anthropic `input_schema` (it requires `{"type":"object",…}`),
  so an Anthropic-backed consumer (thoth → hoosh) had its **whole** tools request rejected — surfacing downstream as
  an empty completion (streaming) or HTTP 502 (block mode), making the agentic loop unusable whenever mneme was in
  the registry. Registration now serializes the schema via a new **`_mcp_schema_json`**: `{"type":"object"}` plus,
  for a tool with required fields, a `properties` map (each required arg typed as a string — mneme's note-tool args
  are all strings) and the `required` array — e.g. `mneme_create_note` →
  `{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"}},"required":["title","content"]}`.
  The registration body also **orders `callback_url` before the nested `inputSchema`** — daimon's registration
  parser scans flatly for `callback_url` and returns `400 missing callback_url` if a nested object precedes it
  (matches the field order `daimon`/`stack.sh` registrations already use; a more robust daimon parser is a separate
  upstream nicety). Unit-tested (`tests/mcp_server.tcyr`: the schema serializer + the registration body/ordering)
  and **live-verified** end to end — daimon now advertises typed `mneme_*` schemas and a thoth agentic turn against
  the full registry completes (was empty / 502). See
  `docs/development/issues/2026-07-13-mcp-tools-advertise-empty-inputschema.md`.

## [1.1.0] — 2026-07-12

**Served MCP endpoint — `mneme serve` — so mneme can be HOSTED by daimon (and consumed by thoth's memory seam).**

Mneme's MCP tools were in-process only (no socket). This adds a network endpoint over the *existing* dispatch so
daimon can register mneme as an external tool (callback_url) and clients reach `mneme_*` through daimon — mneme as
a served domain, like hoosh.

### Added
- **`mneme serve [port]`** (default 8100) — a new subcommand (same binary as the TUI, resolved via `argv`) that
  runs an MCP endpoint. `src/mcp_server.cyr`: the socket/accept/recv + HTTP wire is sandhi's
  (`sandhi_server_run_opts` / `sandhi_server_body_offset` / `sandhi_server_send_response`); the tool logic is the
  existing `mneme_mcp_handle_tool_call`. New: a JSON-RPC 2.0 envelope parse, a JSON-arguments → mneme args-vec
  converter, and the request handler. Serves `tools/call`; other methods → JSON-RPC "method not found" (daimon
  serves `tools/list` from its own registry). Vault resolved via `_mneme_vault_dir()` (moved here from `main.cyr`;
  shared with the TUI).
- **Self-registration with daimon** — on serve start, if `MNEME_DAIMON_URL` is set, mneme POSTs its 8 tool defs to
  daimon `/v1/mcp/tools` with `callback_url` = `MNEME_MCP_CALLBACK` or `http://127.0.0.1:<port>`. Best-effort: a
  daimon that is down just means mneme serves standalone.
- Unit test `tests/mcp_server.tcyr` (the pure parse: JSON-RPC id token, the args converter, edge cases); the
  socket/dispatch/self-register path is verified live (thoth `/remember` → daimon → mneme round-trips a real note).

### Fixed
- **`args_init()` on startup** — `main()` now calls it after `alloc_init()` so `argv`/`argc` work (without it the
  `serve` subcommand check saw no args and fell through to the TUI).
- **Hardened the request handler against a remote DoS** (found in adversarial review): a non-string JSON-RPC
  `method` or `params.name` made `bayan_json_v_str` return 0, then `str_eq_cstr(0, …)` null-dereferenced and
  SIGSEGV'd the single-threaded serve loop. Now type-guarded (`bayan_json_v_is_str`) → a clean JSON-RPC error.
  Also: a string JSON-RPC `id` is now JSON-escaped in the echoed response.

### Changed
- **Toolchain pin `6.4.57` → `6.4.58`**; vendored `lib/` refreshed via `cyrius lib sync`.

## [1.0.1] — 2026-07-12

**Launch repair.** A bare `mneme` launch off-AGNOS died immediately at patra open
(`[ERROR] patra: cannot open or create file`) because `main.cyr` hardcoded the vault
root to `/mneme` — an absolute path under `/` that a normal user cannot create.

### Fixed
- **Portable vault root** — `main.cyr` now resolves the vault to a writable location:
  `MNEME_VAULT` (explicit) → `$HOME/.mneme` (portable default) → `/mneme` (the AGNOS
  canonical mount, last resort). The store layer already `mkdir -p`'d the vault dir;
  only the entrypoint's hardcoded root was wrong. The models dir moves with it
  (`<vault>/models`). Verified: a bare launch now creates the vault and runs.

### Changed
- **Toolchain pin** `6.4.42` → `6.4.57` (`cyrius.cyml [package].cyrius`).
- **`cyrius.cyml` version** now reads `${file:VERSION}` — the `VERSION` file is the
  single source of truth (no inlined number to drift).

## [1.0.0] — 2026-07-10

**Full Rust → Cyrius port.** The entire 21,014-LOC Rust workspace (`rust-old/`, the
frozen parity oracle) is reimplemented in pure Cyrius, targeting AgnosticOS. Every
test-bearing Rust module is mirrored 1:1; **61 `.tcyr` files, all green**.

### Ported subsystems
- **mneme-core** — tag, frontmatter, note, link, graph, plugin, calendar, task,
  config (+ a `core_uuid` v4/v5 helper). TOML/JSON via `bayan`.
- **mneme-store** — `patra` (embedded SQL) DB, file store with real SHA-256 (`sigil`),
  versioning, sharing, registry, vault, multi-vault manager.
- **mneme-search** — built-from-scratch BM25 inverted index (no Tantivy analog),
  brute-force cosine vector store (no usearch analog), query DSL, RRF hybrid merge,
  Thompson-sampling retrieval optimizer, context buffer, cross-vault, semantic engine.
- **mneme-ai** — concepts, tagger, temporal, templates, training-export, deterministic
  K-means++ clustering, RAG-eval, consolidation, summarizer, linker, event bus, rag,
  writer, flashcards (SM-2), translator, qa-bridge, multimodal, creative, daimon client.
- **mneme-mcp** — MCP 2.0 protocol (JSON-RPC + 8 tool schemas) + full tool dispatch,
  tested end-to-end against a real on-disk vault.
- **mneme-api** — the complete HTTP surface as an in-process router (notes CRUD,
  search, tags, AI endpoints, templates, tasks, calendar, flashcards, clip, plugins,
  PDF export); all 30 integration cases pass via `tower::oneshot`-style dispatch.
- **mneme-ui (TUI)** — app state, all 9 panel renderers (line-buffer; no ratatui
  analog), key-event dispatch, and a real terminal adapter. `src/main.cyr` builds to
  a working TUI binary.

### Live bridges (over the sovereign stack)
- **HTTP via `sandhi`** — daimon REST client (`/health`, `/v1/rag/*`) + remote
  embeddings (`POST /v1/embeddings`), wired into the embedding backend via an fnptr
  hook; runs on AGNOS over `sock_connect`/`send`/`recv`; graceful-degrades.
- **Terminal** — ANSI console render + stdin key decode over `sys_write`/`sys_read`.

### Hand-rolled (pending sovereign-library migration)
- Markdown→HTML subset and a valid multi-page PDF writer (`io_export_pdf`,
  catalog/pages/font + byte-accurate xref) — migrate to `bayan_markdown_*` /
  `bayan_pdf_*` when they ship (both on bayan's roadmap).

### Not yet ported (P2 backlog)
- Local in-process ONNX inference — lands with the sovereign ML stack (akshara →
  rosnet → rupantara → tula → anukulana). Remote embeddings cover the path meanwhile.

### Notes
- Toolchain pinned to Cyrius `6.4.42`. Benchmarks in `docs/benchmarks.md`; security
  audit in `docs/audit/2026-07-10-audit.md`.

## [2026.3.18] — 2026-03-18

### Build & CI
- Eliminated `openssl-sys` dependency by switching `ort` to `tls-rustls` backend
- CI builds no longer require system OpenSSL/libssl-dev packages
- Improved `bump-version.sh` to update all Cargo.toml files consistently (modeled on nazar patterns)
- Release automation workflow now uses bump script for consistent version management

### Quality
- 450 tests across 8 crates (up from 382)
- 65% test coverage (threshold maintained)
- Zero clippy warnings across all crates
- Fixed all clippy lints: loop variables, clamp patterns, type complexity, range checks
- New test suites: `db.rs` (20 tests), `files.rs` (6 tests), additional MCP/search/manager tests

## [2026.3.15] — 2026-03-15

### Phase 13 — Synapse Direct Embedding Pipeline
- `mneme-search/src/embedding_backend.rs`: `EmbeddingBackend` trait with `LocalOnnxBackend` and `RemoteHttpBackend`
- `RemoteHttpBackend`: OpenAI-compatible `/v1/embeddings` client (works with Synapse, Ollama, OpenAI)
- `build_backend()` factory: "auto" tries remote first, falls back to local ONNX
- `SemanticEngine::open_with_config()` accepts `EmbeddingConfig` for backend selection
- `mneme-core/src/config.rs`: `EmbeddingSection` in `MnemeConfig` (backend, remote_url, model, api_key, dimensions)
- Health endpoint reports `embedding_backend` name and `embedding_dimension`
- Dimension probe on remote connect to auto-detect vector size

### Phase 14 — Training Feedback Export
- `mneme-ai/src/training_export.rs`: `TrainingRecord` enum (SearchClick, EditAfterSearch, TrustOverride, NoteContent)
- `TrainingLog`: append-only JSONL at `.mneme/training.jsonl` per vault
- `GET /v1/export/training-data` endpoint with `?type=`, `?since=`, `?include_notes=` filters
- Search feedback handler logs clicks with query, note title, arm name, position
- `SearchFeedbackRequest` extended with optional `query` and `position` fields

### Phase 8 — Note Consolidation & Evolution
- `mneme-search/src/semantic_engine.rs`: `find_similar_to()` for embedding-based duplicate detection
- `mneme-ai/src/consolidation.rs`: `detect_duplicates_semantic()`, `MergeSuggestion` type, `DuplicatePair.detection_method` field
- `mneme-store/migrations/002_last_accessed.sql`: `last_accessed` column + index
- `mneme-store/src/db.rs`: `touch_note()` for lightweight access tracking
- `mneme-store/src/vault.rs`: `get_note()` bumps `last_accessed` on full content reads
- Content freshness scoring: `days_since_update / max(days_since_access, 1)` in `StaleNote`
- `POST /v1/ai/consolidate/merge` endpoint for LLM merge suggestions via daimon
- `GET /v1/notes/duplicates?method=semantic|jaccard|both` query param
- TUI: Stale notes panel (`!` keybinding) showing days stale + freshness score

### Phase 9 — Schema Clustering & Emergent Structure
- `mneme-ai/src/clustering.rs`: K-means++ with deterministic init, Lloyd's iteration, elbow heuristic
- `mneme-search/src/semantic_engine.rs`: `embed()`, `embed_batch()`, `indexed_note_ids()` methods
- `mneme-search/src/vector_store.rs`: `note_ids()` method
- `mneme-ai/src/client.rs`: `label_cluster()` for LLM-generated cluster labels/summaries
- `GET /v1/ai/clusters?k=&max_k=&label=` endpoint
- TUI: Clusters panel (`c` keybinding) with expand/collapse navigation
- Types: `ClusteringResult`, `Cluster`, `NoteEmbedding`

### Phase 10 — Context-Aware Retrieval
- `mneme-search/src/context_buffer.rs`: `ContextBuffer` with bounded deque, recency-weighted averaging, `fuse_embeddings()`
- `mneme-search/src/semantic_engine.rs`: `context_search()` fuses query + context embeddings
- `mneme-core/src/config.rs`: `ContextRetrievalConfig` with `enabled`, `query_weight` (λ=0.7), `buffer_capacity`
- `mneme-api/src/state.rs`: `ContextBuffer` per vault in `VaultEngines`
- `mneme-api/src/handlers.rs`: `GET /v1/notes/{id}` pushes to context buffer; search uses it automatically; `?context=false` opt-out
- TUI: `select_note()` pushes to context buffer, `run_search()` merges context-aware semantic results

### Phase 11 — Document Provenance & Trust Scoring
- `mneme-core/src/note.rs`: `Provenance` enum (Manual 1.0, Import 0.8, WebClip 0.6, Generated 0.4), `trust_score()` method
- `mneme-core/src/frontmatter.rs`: `trust:` field parsing (high/medium/low or float), `parse_trust_value()`
- `mneme-store/migrations/003_provenance.sql`: `provenance TEXT`, `trust_override REAL` columns
- `mneme-api/src/handlers.rs`: trust as multiplicative boost on hybrid search scores, `SearchResultItem.trust` field
- `mneme-api/src/advanced_handlers.rs`: web clipper auto-sets `Provenance::WebClip`
- `CreateNote.provenance` optional field for origin tracking
- TUI: [H]/[M]/[L] trust indicators (green/yellow/red) in search results

### Phase 5 — In-Process Vector Store
- `mneme-search/src/embedder.rs`: ONNX Runtime wrapper for all-MiniLM-L6-v2 (384-dim)
- `mneme-search/src/vector_store.rs`: usearch ANN index with persistent metadata
- `mneme-search/src/semantic_engine.rs`: Facade combining embedder + vector store, graceful daimon fallback
- Feature-gated behind `local-vectors` (default on)
- Vector index persists in `.mneme/vectors/`, models from `MNEME_MODELS_DIR`

### Phase 6 — Retrieval Quality & Feedback Loop
- `mneme-search/src/retrieval_optimizer.rs`: Thompson Sampling bandit with 4 arms (balanced, fulltext_heavy, semantic_heavy, recency_boost)
- `hybrid_merge` now accepts `BlendWeights` via `weighted_hybrid_merge`
- Search response includes `search_id` for feedback correlation
- `/v1/search/feedback` endpoint records clicks, improves future ranking
- `/v1/search/optimizer` endpoint shows arm stats
- MCP `mneme_search_feedback` tool
- Optimizer state persists in `.mneme/optimizer.json`

### Phase 12 — Multi-Vault Support
- `mneme-core/src/config.rs`: MnemeConfig, VaultConfigEntry, VaultInfo types
- `mneme-store/src/registry.rs`: VaultRegistry with TOML persistence
- `mneme-store/src/manager.rs`: VaultManager managing multiple vaults
- `mneme-search/src/cross_vault.rs`: Cross-vault RRF merge with vault weights
- `mneme-api/src/state.rs`: VaultState + VaultWithEngines
- `mneme-api/src/vault_handlers.rs`: /v1/vaults CRUD + switch endpoints
- All handlers support `?vault=` parameter
- MCP: `mneme_list_vaults`, `mneme_switch_vault`, `mneme_search_feedback` tools (8 total now)
- TUI: VaultPicker panel (press `v`)
- Config: `mneme.toml` with `[vaults]` table

### Phase 7 — RAG Evaluation Metrics
- `mneme-ai/src/rag_eval.rs`: local-only token-overlap scoring (no LLM required)
- Faithfulness, answer relevance, and chunk utilization scores (0.0–1.0)
- Weighted overall score: 50% faithfulness, 30% relevance, 20% utilization
- `RagEvalAggregates` for running averages across queries per vault
- `/v1/ai/rag/stats` returns eval aggregates

### Quality
- 362 tests across 8 crates (up from 313)
- 3 new DB migrations (001–003)
- New modules: clustering, context_buffer, embedding_backend, training_export
- 4 new ADRs (011–014): consolidation, clustering, embedding backends, training export
- New deps: usearch 2, ort 2.0.0-rc.12, ndarray 0.17, tokenizers 0.21

## [2026.3.13] — 2026-03-13

### Core
- Note, Link, Tag, Graph types with Markdown frontmatter parsing
- Task extraction from Markdown checkboxes with priority markers and due dates
- Calendar types: daily notes, month views, entry type detection
- Plugin system: registry with capability-based lookup and hook points

### Store
- SQLite persistence with CRUD operations, file-backed Markdown storage, content hashing (SHA-256)
- Note versioning: version history with line-level diffs, Delta integration ready
- Vault sharing: multi-user with Owner/Editor/Viewer roles, conflict detection

### Search
- Tantivy full-text search with BM25 ranking
- Hybrid semantic search via daimon vector endpoints
- Tag-based filtering, search result pagination

### AI
- RAG pipeline: ingest, query, and Q&A over knowledge base
- Note summarization (extractive + daimon abstractive)
- Auto-linking: suggest connections between related notes
- Concept extraction: entities, topics, key terms via TF-IDF
- Auto-tagging with confidence scores and vocabulary matching
- Note templates: daily, meeting, project with variable substitution
- AI writing assistant: complete, reword, expand with daimon/local fallback
- Translation pipeline: per-note and batch, 12+ supported languages
- Temporal analysis: monthly activity tracking, concept trend detection
- Multi-modal notes: image/audio/video attachments, Shruti transcription
- Creative suite: Tazama shot lists, Rasa design assets, Shruti show notes, BullShift trade journal
- Spaced repetition: SM-2 scheduling, flashcard extraction from definitions and concepts

### API
- Axum HTTP server (port 3838) with 30+ endpoints
- Note CRUD, search, tags, health check
- AI: RAG query/ingest/stats, summarize, suggest-links, concepts, suggest-tags, write, translate, languages, temporal
- Templates: list, render (with optional note creation)
- Export: PDF
- Tasks/Kanban, Calendar, Flashcards
- Web Clipper: HTML-to-Markdown, bookmarks
- Plugins: list installed

### UI
- Terminal interface (ratatui + crossterm)
- Note list, Markdown viewer, search, backlinks panel, tag browser
- Knowledge graph visualization (force-directed layout, Canvas rendering, node cycling, zoom/pan)
- Split-pane / multi-note view (side-by-side, pane switching, note picker)
- Keyboard-driven navigation

### MCP
- JSON-RPC 2.0 server with 5 tools for Claude integration
- create_note, search, get_note, update_note, query_graph

### Import / Export
- Import: Obsidian vaults (wikilinks, frontmatter, inline tags)
- Import: Notion export directories (UUID cleanup, link conversion)
- Import: Plain Markdown directories
- Export: Static HTML site with navigation and styling
- Export: PDF (printpdf, A4, Helvetica/Courier, page breaks)
- Web clipper: HTML-to-Markdown conversion, bookmark saving

### Quality
- 257 tests across 8 crates (unit + integration)
- 77.69% test coverage
- Criterion benchmark suite for search and store operations
- Zero clippy warnings

### Documentation
- User guide covering all features
- API reference for all 30+ endpoints
- Architecture documentation with crate dependency graph and data flow
- 6 Architecture Decision Records (ADR-001 through ADR-006)

### AGNOS Integration
- Systemd service unit (`deploy/mneme.service`)
- Marketplace recipe (`deploy/mneme.agnos-agent`)
- 5 agnoshi voice/agent intents (`deploy/agnoshi-intents.toml`)
