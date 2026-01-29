# Test Plan 001: Coverage Envelope Expansion

**Document:** 001-coverage-expansion-plan-01.md
**Status:** Accepted
**Created:** 2026-01-29
**Baseline:** 76.70% (1972/2571 lines)
**Target:** 80%+ (2057+ lines, ~85 lines to cover)
**Method:** Test envelope build-up (see 000-test-envelope-philosophy.md)

---

## Coverage Gap Analysis

Sorted by uncovered lines (descending). Files with fewer than
10 uncovered lines are excluded -- they are inside the
demonstrated envelope already.

| # | File | Covered | Total | Gap | % | Zone | Status |
|---|------|---------|-------|-----|---|------|--------|
| 1 | mcp/tools/preview_chunk.rs | 27 | 119 | 92 | 23% | Adapter | Done |
| 2 | mcp/tools/index_repository.rs | 18 | 75 | 57 | 24% | Adapter | Done |
| 3 | mcp/server.rs | 0 | 43 | 43 | 0% | Adapter | Done |
| 4 | core/xdg.rs | 35 | 70 | 35 | 50% | Core | Done |
| 5 | core/storage/validator.rs | 44 | 62 | 18 | 71% | Core | Done |
| 6 | mcp/handlers.rs | 74 | 92 | 18 | 80% | Adapter | Done |
| 7 | mcp/error.rs | 10 | 25 | 15 | 40% | Adapter | Done |
| 8 | core/storage/tantivy.rs | 55 | 70 | 15 | 79% | Core | Done |
| 9 | mcp/transport.rs | 0 | 13 | 13 | 0% | Adapter | Deferred |
| 10 | mcp/tools/read_file.rs | 67 | 84 | 17 | 80% | Adapter | N/A |
| 11 | mcp/tools/list_dir.rs | 69 | 78 | 9 | 88% | Adapter | N/A |
| 12 | mcp/tools/find_file.rs | 59 | 68 | 9 | 87% | Adapter | N/A |

**Total gap across all files: ~341 lines uncovered**

Covering 85 of those reaches 80%. The plan targets ~120 lines
to provide margin and reach closer to 82%.

---

## Expansion Strategy

Following the envelope model, we expand from center outward
within each file. Each section below identifies:

- **Center tests** (happy path -- highest confidence gain)
- **Boundary tests** (edge cases -- next ring out)
- **Beyond-boundary tests** (invalid input -- outermost ring)

Priority order follows the philosophy: core domain first,
then adapters, then plumbing.

---

## Phase 1: Core Domain (target: >90%)

### 1A. core/xdg.rs (35/70 -- 50%)

The resolve_* methods (config, data, state, cache) follow
identical patterns but only some are exercised by existing
tests. The `migrate_legacy_paths`, `ensure_dirs_exist` and
`log_paths` methods are uncovered.

**Center:**
- `test_xdg_resolve_state_dir` -- verify state dir resolves
  to `~/.local/state/shebe/`
- `test_xdg_resolve_cache_dir` -- verify cache dir
- `test_xdg_sessions_dir` -- verify sessions_dir() returns
  state_dir/sessions
- `test_xdg_logs_dir` -- verify logs_dir()
- `test_xdg_progress_dir` -- verify progress_dir()
- `test_xdg_query_cache_dir` -- verify query_cache_dir()

**Boundary:**
- `test_xdg_ensure_dirs_exist` -- call ensure_dirs_exist(),
  verify directories are created on disk
- `test_xdg_ensure_dirs_idempotent` -- call twice, verify
  no error

**Beyond boundary:**
- `test_xdg_migrate_no_legacy_file` -- no legacy config,
  verify no-op
- `test_xdg_migrate_with_legacy_file` -- create a
  `./shebe.toml`, verify it is copied to config_file() path
- `test_xdg_migrate_does_not_overwrite` -- legacy file
  exists AND config_file() exists, verify no overwrite

**Estimated lines covered: ~30**
**Location:** `src/core/xdg.rs` (extend existing `tests` module)

### 1B. core/storage/validator.rs (44/62 -- 71%)

Existing tests cover empty session, nonexistent session,
validate_all, auto_repair, and calculate_directory_size.
The uncovered code is likely in the `validate_session` method
body -- the branches where metadata has inconsistencies.

**Center:**
- `test_validate_indexed_session` -- index a small repo,
  then validate. Expect is_consistent=true.

**Boundary:**
- `test_validate_session_with_tampered_metadata` -- index,
  then manually edit meta.json to change files_indexed to a
  wrong value. Expect is_consistent=false, size_matches
  still true (index untouched).
- `test_validate_session_missing_tantivy_dir` -- create
  meta.json but delete the tantivy/ subdirectory. Expect
  appropriate error or is_consistent=false.

**Estimated lines covered: ~12**
**Location:** `src/core/storage/validator.rs` (extend `tests` module)

### 1C. core/storage/tantivy.rs (55/70 -- 79%)

15 uncovered lines. Likely error paths in index operations.

**Boundary:**
- `test_search_nonexistent_session` -- search against a
  session ID that was never indexed
- `test_search_empty_index` -- index an empty directory,
  search for anything, expect empty results

**Estimated lines covered: ~10**
**Location:** integration test or extend existing tantivy tests

---

## Phase 2: MCP Adapter Layer (target: >80%)

### 2A. mcp/server.rs (0/43 -- 0%)

The entire server loop is untested because handler tests call
handlers directly. This file contains: `new()`,
`run()`, `process_and_respond()`, `process_message()`, and
`create_error_response()`.

Testing `run()` requires stdin/stdout mocking (high effort).
But `process_message()` and `create_error_response()` can be
tested by constructing an `McpServer` with a mock transport.

**Center:**
- `test_process_message_initialize` -- send valid initialize
  JSON, verify response has protocolVersion
- `test_process_message_ping` -- send ping, verify empty
  result response
- `test_process_message_tools_list` -- send tools/list,
  verify tools array in response

**Boundary:**
- `test_process_message_notifications_initialized` -- send
  `notifications/initialized`, verify no error
- `test_process_message_notifications_cancelled` -- send
  `notifications/cancelled`, verify no error

**Beyond boundary:**
- `test_process_message_unknown_method` -- send unknown
  method, verify METHOD_NOT_FOUND error
- `test_process_message_invalid_json` -- send malformed
  JSON, verify ParseError
- `test_create_error_response` -- verify error response
  structure

**Approach:** `McpServer` owns a `StdioTransport`. To test
without real stdio, either:
(a) Make transport generic/trait-based (refactor, higher cost)
(b) Test `process_message()` directly -- it returns
    `Result<JsonRpcResponse>` and does not use transport.
    This is the low-cost approach.

Option (b) is recommended. `process_message` is the routing
core and covers ~20 of the 43 lines. The remaining 23 lines
are in `run()` and `process_and_respond()` which require
transport mocking -- defer to a future phase.

**Estimated lines covered: ~20**
**Location:** new test module `tests/mcp/server_tests.rs`

**Prerequisite:** `process_message` is currently private.
Change visibility to `pub(crate)` to enable testing from the
integration test crate. Alternatively, add unit tests inside
`server.rs` itself.

### 2B. mcp/tools/index_repository.rs (18/75 -- 24%)

Existing handler_tests cover error paths (missing params,
nonexistent path, file-not-directory). The `execute()` success
path and the `schema()` method body are uncovered. The
validation helpers (`validate_session`, `validate_chunk_size`,
`validate_overlap`) are partially covered.

**Center:**
- `test_index_repository_success` -- index a temp directory
  with a few files, verify success response message contains
  "Indexing complete", file count and chunk count

**Boundary:**
- `test_index_repository_force_false_existing_session` --
  index once, then call again with force=false, verify error
  message contains "already exists"
- `test_index_repository_force_true_reindex` -- index once,
  call again with force=true, verify success
- `test_validate_session_invalid_chars` -- session name
  with spaces or special characters, verify error
- `test_validate_session_too_long` -- session name >64
  chars, verify error
- `test_validate_chunk_size_too_small` -- chunk_size=50,
  verify error
- `test_validate_chunk_size_too_large` -- chunk_size=3000,
  verify error
- `test_validate_overlap_negative` -- overlap beyond range,
  verify error

**Estimated lines covered: ~40**
**Location:** `tests/mcp/handler_tests.rs` or new file
`tests/mcp/index_repository_tests.rs`

### 2C. mcp/tools/preview_chunk.rs (27/119 -- 23%)

Existing unit tests cover `offset_to_lines` and constants.
The `get_chunk_metadata`, `extract_context_lines`,
`format_preview`, and `execute` methods are uncovered.

**Center:**
- `test_preview_chunk_execute_success` -- index a repo with
  known content, search for a term, get a chunk_index and
  file_path, then call preview_chunk with those values.
  Verify output contains the expected code lines.

**Boundary:**
- `test_preview_chunk_chunk_index_zero` -- preview first
  chunk of a file (chunk_index=0)
- `test_preview_chunk_last_chunk` -- preview the last chunk
  of a file
- `test_preview_chunk_context_lines_zero` -- request zero
  context lines
- `test_preview_chunk_context_lines_max` -- request 100
  context lines (max)
- `test_extract_context_lines_at_file_start` -- chunk at
  beginning, before-context is truncated
- `test_extract_context_lines_at_file_end` -- chunk at end,
  after-context is truncated

**Beyond boundary:**
- `test_preview_chunk_nonexistent_session` -- invalid
  session ID, verify error
- `test_preview_chunk_nonexistent_file` -- valid session,
  invalid file_path, verify error
- `test_preview_chunk_invalid_chunk_index` -- chunk_index
  beyond the actual number of chunks, verify error or
  empty result

**Estimated lines covered: ~50**
**Location:** extend `tests` module in `preview_chunk.rs`
plus integration tests in `tests/mcp/`

**Note:** The success-path tests require a real indexed
session. Use the same `create_test_handlers` + `TempDir`
pattern from handler_tests.rs, create files in the temp dir,
index them, then call preview_chunk.

### 2D. mcp/handlers.rs (74/92 -- 80%)

18 uncovered lines. Likely the `handle_cancelled` method
(just added) and parts of `handle_tools_call` error handling.

**Center:**
- Already covered by existing test + the two new tests
  added in WP 025.

**Boundary:**
- `test_tools_call_unknown_tool` -- call tools/call with
  name="nonexistent_tool", verify error response

**Estimated lines covered: ~5**
**Location:** `tests/mcp/handler_tests.rs`

### 2E. mcp/error.rs (10/25 -- 40%)

The `From<ShebeError> for McpError` impl and some enum
variants are untested.

**Center:**
- `test_shebe_error_to_mcp_error` -- create each ShebeError
  variant and convert to McpError, verify mapping is correct

**Boundary:**
- `test_mcp_error_display` -- verify Display output for
  each McpError variant

**Estimated lines covered: ~10**
**Location:** unit tests in `src/mcp/error.rs`

### 2F. mcp/transport.rs (0/13 -- 0%)

The `StdioTransport` writes to actual stdout. Testing
`send_response` requires capturing stdout, which is fragile
in test environments.

**Recommendation:** Defer. The transport is thin plumbing
(serialize + write + flush). Risk is low and test cost is
high. The notification skip logic in `send_response` is
partially redundant with the `process_and_respond` guard.

**Estimated lines covered: 0 (deferred)**

---

## Phase Summary

| Phase | File | Est. Lines | Priority |
|-------|------|-----------|----------|
| 1A | core/xdg.rs | ~30 | P1 |
| 1B | core/storage/validator.rs | ~12 | P2 |
| 1C | core/storage/tantivy.rs | ~10 | P2 |
| 2A | mcp/server.rs | ~20 | P1 |
| 2B | mcp/tools/index_repository.rs | ~40 | P1 |
| 2C | mcp/tools/preview_chunk.rs | ~50 | P1 |
| 2D | mcp/handlers.rs | ~5 | P3 |
| 2E | mcp/error.rs | ~10 | P3 |
| 2F | mcp/transport.rs | 0 | Deferred |
| | **Total** | **~177** | |

Covering all P1 items alone (~140 lines) would bring coverage
from 76.70% to approximately **82%**.

---

## Implementation Order

Following the build-up principle -- center first, expand out,
fix before moving on:

```
Phase 1A: xdg.rs center + boundary tests
    |
    v
Phase 2B: index_repository.rs success path + validations
    |
    v   (index_repository needed to create sessions for 2C)
Phase 2C: preview_chunk.rs success path + boundaries
    |
    v
Phase 2A: server.rs process_message tests
    |
    v
Phase 1B: validator.rs consistency checks
    |
    v
Phase 1C: tantivy.rs edge cases
    |
    v   (if time permits)
Phase 2D: handlers.rs unknown tool test
    |
    v
Phase 2E: error.rs conversion tests
```

Dependencies:
- 2C depends on having indexed sessions (uses patterns from 2B)
- 2A may require making `process_message` pub(crate)
- All other phases are independent

---

## Verification

After implementing each phase:

```bash
make test            # All tests pass
make test-coverage   # Coverage increases
```

Final target: 80%+ overall, with no individual P1 file
below 60%.

---

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/core/xdg.rs` | Extend tests module |
| `src/core/storage/validator.rs` | Extend tests module |
| `src/core/storage/tantivy.rs` | Extend tests or new integration test |
| `src/mcp/server.rs` | Make process_message pub(crate), add unit tests |
| `src/mcp/error.rs` | Add unit tests module |
| `src/mcp/tools/preview_chunk.rs` | Extend tests module |
| `tests/mcp/handler_tests.rs` | Add index_repository and handler tests |
| `tests/mcp/mod.rs` | Register new test modules if needed |

---

## References

- Test envelope philosophy: docs/testing/000-test-envelope-philosophy.md
- Coverage baseline: tarpaulin run 2026-01-29 (76.70%)
- Architecture: ARCHITECTURE.md (module structure)
