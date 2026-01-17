# Shebe Architecture

**Content Search for Code - Developer's Guide to the Codebase**

**Version:** 0.5.4 <br>
**Updated:** 2026-01-16 <br>
**Status:** 14 MCP Tools, 10 CLI Commands, 397 Tests (Production Ready)


> **Purpose:** This document helps you understand where to find code and how to make changes.
> For performance data, see [docs/Performance.md](./docs/Performance.md).

---

## Bird's Eye View

Shebe is an **MCP-first RAG service** that provides BM25 full-text search for code repositories:

```
     Claude Code (MCP Client)              Shell / Scripts
               |                                  |
               | MCP Protocol (stdio)             | Direct invocation
               v                                  v
    +--------------------------------+    +-------------------------+
    |   shebe-mcp (MCP Server)       |    |   shebe (CLI)           |
    |   - 14 MCP tools               |    |   - 10 commands         |
    |   - stdio transport            |    |   - Human/JSON output   |
    +---------------+----------------+    +------------+------------+
                    |                                  |
                    +----------------+-----------------+
                                     |
                                     v
                    +---------------------------------------+
                    |       core/ (Domain Logic)            |
                    |  - Indexing, search, storage          |
                    +---------------------------------------+
                                     |
                                     v
                    +---------------------------------------+
                    |  ~/.local/state/shebe/sessions/       |
                    |  (Tantivy indexes + session metadata) |
                    +---------------------------------------+
```

**Key Insight:** Two binaries sharing core logic. No HTTP server required.

---

## Code Map

### Where to Run Commands

**IMPORTANT:** All `cargo` commands run from `services/shebe-server/`

```bash
cd services/shebe-server/
cargo build    # Run from here
cargo test     # Run from here
```

### Repository Structure

The codebase is organized into three top-level modules: `core/`, `mcp/` and `cli/`.
This separation provides clear boundaries between protocol-agnostic domain logic
and the adapter layers.

```
shebe/                         # Repository root
+-- services/shebe-server/     # Main Rust service
|   +-- src/
|   |   +-- lib.rs             # Library root (exports core, mcp, cli)
|   |   +-- bin/
|   |   |   +-- shebe_mcp.rs   # Entry: MCP server
|   |   |   +-- shebe_cli.rs   # Entry: CLI
|   |   |
|   |   +-- core/              # Domain logic (protocol-agnostic)
|   |   |   +-- mod.rs         # Core module root
|   |   |   +-- config.rs      # Config (TOML + env)
|   |   |   +-- error.rs       # Error types
|   |   |   +-- types.rs       # Data structures
|   |   |   +-- services.rs    # Unified Services struct
|   |   |   +-- xdg.rs         # XDG directory handling
|   |   |   +-- storage/       # Persistence
|   |   |   |   +-- session.rs # Session management
|   |   |   |   +-- tantivy.rs # Index wrapper
|   |   |   |   +-- validator.rs # Metadata validation
|   |   |   +-- search/        # Search
|   |   |   |   +-- bm25.rs    # BM25 service
|   |   |   +-- indexer/       # Indexing pipeline
|   |   |       +-- chunker.rs # UTF-8 safe chunking
|   |   |       +-- walker.rs  # File traversal
|   |   |       +-- pipeline.rs # Orchestration
|   |   |
|   |   +-- mcp/               # MCP adapter (depends on core)
|   |   |   +-- mod.rs         # MCP module root
|   |   |   +-- server.rs      # Stdio event loop
|   |   |   +-- handlers.rs    # Protocol routing
|   |   |   +-- protocol.rs    # JSON-RPC types
|   |   |   +-- transport.rs   # Stdio transport
|   |   |   +-- error.rs       # MCP error types
|   |   |   +-- tools/         # 14 tool handlers
|   |   |
|   |   +-- cli/               # CLI adapter (depends on core)
|   |       +-- mod.rs         # CLI entry, Cli/Commands structs
|   |       +-- output.rs      # Colors, formatting, print helpers
|   |       +-- commands/      # 10 command handlers
|   |           +-- index.rs       # index-repository
|   |           +-- search.rs      # search-code
|   |           +-- references.rs  # find-references
|   |           +-- session.rs     # list/info/delete/reindex
|   |           +-- config.rs      # show-config
|   |           +-- info.rs        # get-server-info
|   |           +-- completions.rs # Shell completions
|   |
|   +-- tests/                 # Integration tests
|   +-- Cargo.toml             # 16 prod deps (incl. clap, colored)
+-- docs/
|   +-- Performance.md         # Benchmarks
|   +-- guides/                # User guides
+-- dev-docs/
|   +-- CONTEXT.md             # Status tracker (dev only)
+-- ARCHITECTURE.md            # This doc
+-- README.md                  # Overview
```

**Module Dependencies (one-way):**
```
              +------------------+
              |     core/        |
              |  (domain logic)  |
              +--------+---------+
                       |
          +------------+------------+
          |                         |
          v                         v
+------------------+      +------------------+
|      mcp/        |      |      cli/        |
| (stdio adapter)  |      | (clap adapter)   |
+------------------+      +------------------+
```

**Rules:**
- `mcp/` and `cli/` can import from `core/`, but `core/` never imports from adapters
- `mcp/` and `cli/` do not import from each other

---

## Entry Points

### MCP Server: `src/bin/shebe_mcp.rs`

```rust
use shebe::core::{Config, Services};
use shebe::mcp::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load config (core/config.rs)
    // 2. Create Services (core/services.rs)
    // 3. Register tools (mcp/handlers.rs)
    // 4. Run stdio loop (mcp/server.rs)
}
```

**Add MCP tools:** `src/mcp/tools/*.rs`

### CLI: `src/bin/shebe_cli.rs`

```rust
use clap::Parser;
use shebe::cli::{run, Cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

**Add CLI commands:** `src/cli/commands/*.rs`

---

## Key Files by Task

### Adding a New MCP Tool

1. `src/mcp/tools/your_tool.rs` - Create tool
2. `src/mcp/tools/mod.rs` - Export tool
3. `src/mcp/handlers.rs` - Register in ToolRegistry
4. `src/mcp/tools/get_server_info.rs` - Update tool list
5. Add tests in your tool file

**Pattern:** See `preview_chunk.rs` (~420 LOC, 8 tests)

### Adding a New CLI Command

1. `src/cli/commands/your_cmd.rs` - Create command handler
2. `src/cli/commands/mod.rs` - Export args struct
3. `src/cli/mod.rs` - Add to Commands enum, add match arm in `run()`
4. Add tests (manual testing or integration tests)

**Pattern:** See `search.rs` or `session.rs` for examples

### Modifying Search

- **Query parsing:** `src/core/search/bm25.rs`
- **Ranking:** Tantivy BM25 (not customizable)
- **Formatting:** `src/mcp/tools/search_code.rs`

### Changing Indexing

- **File walking:** `src/core/indexer/walker.rs`
- **Chunking:** `src/core/indexer/chunker.rs`
- **Storage:** `src/core/storage/session.rs`

**INVARIANT:** Chunker must respect UTF-8 boundaries

### Configuration

- **Struct:** `src/core/config.rs`
- **Env vars:** `SHEBE_*` prefix

---

## Design Decisions

### Why BM25 (Not Vector Search)?

**Decision:** Tantivy BM25 only

**Rationale:**
- Developers know keywords
- No GPU/embedding complexity
- Fast: 2ms search latency (validated on 12k+ files)
- Indexing: 1,928-11,210 files/sec (3.9x-22.4x faster than target)
- Validated: 30/30 performance tests passed (100% success rate)

**Trade-off:** Misses semantic similarity

### Why MCP-Only?

**Decision:** Single binary with MCP interface only (v0.6.0)

**Rationale:**
- MCP provides all required functionality (14 tools)
- Single binary deployment, fewer dependencies
- Primary use case is Claude Code integration
- Less code to maintain and test

**Trade-off:** No HTTP/REST API access

### Why Synchronous Indexing?

**Decision:** Removed async progress

**Rationale:**
- Fast enough: 0.5-3.3s for 5-6k files (measured)
- Simpler: -1,000 LOC (86% code reduction)
- Accurate metadata: 100% correct file/chunk counts

**Trade-off:** Blocks during indexing (acceptable for <4s operations)

### Why Character-Based Chunking?

**Decision:** Use `char_indices()`

**Rationale:**
- UTF-8 safety (never splits)
- 19 safety tests

**Trade-off:** Slightly more memory

### Why Session Isolation?

**Decision:** Separate indexes per session

**Rationale:**
- Branch switching support
- Parallel indexing
- Easy cleanup

**Trade-off:** More disk space

---

## Platform Invariants

### Requirements

**Developers must respect:**

1. **Rust:** 1.88+
2. **UTF-8:** Never split multi-byte chars
3. **Sessions:** All ops scoped to session
4. **Line length:** Max 120 chars
5. **Tests:** All 397 must pass (100% success rate)
6. **Schema:** v3 with repository_path and last_indexed_at fields

### Storage Layout

```
~/.local/state/shebe/sessions/
+-- {session-id}/
    +-- meta.json      # Metadata
    +-- tantivy/       # Index
```

**INVARIANT:** `meta.json` and Tantivy must sync

### Tantivy Schema (v2)

```rust
Schema {
    text: TEXT | STORED,
    file_path: STRING | STORED,
    session: STRING | STORED,
    offset_start: i64 | STORED,
    offset_end: i64 | STORED,
    chunk_index: i64 | INDEXED | STORED,  // v0.3.0: Now indexed for preview_chunk
    indexed_at: Date | STORED,
}
```

**INVARIANTS:**
- `file_path + chunk_index` = unique key
- `chunk_index` must be INDEXED for preview_chunk queries
- Schema version tracked in SessionMetadata

---

## Dependencies

16 production crates:

| Crate               | Purpose       | Why              |
|---------------------|---------------|------------------|
| tantivy 0.22        | BM25          | Pure Rust        |
| tokio 1.x           | Async         | Standard         |
| serde/serde_json    | JSON          | API              |
| walkdir             | Files         | Simple           |
| glob                | Patterns      | Familiar         |
| regex               | Pattern match | File discovery   |
| thiserror           | Errors        | Derive           |
| tracing*            | Logs          | Async            |
| toml                | Config        | Config files     |
| chrono              | Timestamps    | Metadata         |
| async-trait         | Traits        | MCP              |
| dirs                | XDG paths     | Cross-platform   |
| once_cell           | Lazy statics  | Patterns         |
| clap 4              | CLI parsing   | Derive, env      |
| clap_complete       | Completions   | bash/zsh/fish    |
| colored             | Terminal      | NO_COLOR aware   |

---

## Testing

397 tests in 6 categories:

1. Unit (~215): Module logic
2. Integration (~102): E2E
3. Session (24): Mgmt
4. MCP (13): Protocol
5. UTF-8 (19): Safety
6. Doc (3 ignored): Examples

**Focus:** UTF-8, errors, protocol, isolation

**Coverage:** 86.76% line coverage (validated with cargo-llvm-cov)

---

## Common Tasks

```bash
cd services/shebe-server/

# Tests
cargo test
cargo test preview_chunk
cargo test -- --nocapture

# Quality
cargo fmt
cargo clippy  # Zero warnings
cargo check

# Run MCP server
cargo run --bin shebe-mcp
```

---

## MCP Tools (14)

| Tool               | Category  | Description                                  | Performance                 |
|--------------------|-----------|----------------------------------------------|-----------------------------|
| search_code        | Core      | BM25 full-text search                        | 2ms latency, 210-650 tokens |
| list_sessions      | Core      | List all indexed sessions                    | <10ms                       |
| get_session_info   | Core      | Detailed session metadata and stats          | <5ms                        |
| index_repository   | Core      | Index repository for search                  | 1,928-11,210 files/sec      |
| get_server_info    | Core      | Server version and capabilities              | <5ms                        |
| show_shebe_config  | Core      | Display current configuration                | <5ms                        |
| read_file          | Ergonomic | Read file with auto-truncation               | <10ms, 20KB limit           |
| delete_session     | Ergonomic | Delete session with confirmation             | <10ms                       |
| list_dir           | Ergonomic | List directory contents with pagination      | <10ms, 500 file limit       |
| find_file          | Ergonomic | Find files by glob/regex patterns            | <10ms                       |
| find_references    | Ergonomic | Find symbol references with confidence       | <500ms typical              |
| preview_chunk      | Ergonomic | Show chunk context (v0.3.0: schema v2 fix)   | <5ms                        |
| reindex_session    | Ergonomic | Re-index using stored path (v0.3.0: v3 feat) | Same as index_repository    |
| upgrade_session    | Ergonomic | Upgrade session schema to latest version     | <100ms                      |

**Pattern:** All implement `McpToolHandler`
**Performance:** Validated on 30/30 test scenarios (100% success rate)

---

## CLI Commands (10)

| Command            | Description                              | MCP Equivalent     |
|--------------------|------------------------------------------|--------------------|
| index-repository   | Index a repository for search            | index_repository   |
| search-code        | BM25 full-text search                    | search_code        |
| find-references    | Find symbol references                   | find_references    |
| list-sessions      | List all indexed sessions                | list_sessions      |
| get-session-info   | Show session details                     | get_session_info   |
| delete-session     | Delete a session                         | delete_session     |
| reindex-session    | Re-index using stored path               | reindex_session    |
| show-config        | Display current configuration            | show_shebe_config  |
| get-server-info    | Version and capabilities                 | get_server_info    |
| completions        | Generate shell completions               | -                  |

**Output formats:** Human-readable (default), JSON (`--format json`)
**Colored output:** Respects NO_COLOR environment variable
**Documentation:** See [docs/guides/cli-usage.md](./docs/guides/cli-usage.md)

---

## Error Handling

```
ShebeError -> McpError -> JSON-RPC error
```

**INVARIANT:** All `ShebeError` must map to `McpError`

---

## Related Docs

- [README.md](./README.md) - Overview
- [dev-docs/CONTEXT.md](./dev-docs/CONTEXT.md) - Status
- [docs/Performance.md](./docs/Performance.md) - Benchmarks
- [docs/guides/](./docs/guides/) - User guides

---

**Document Status:** Living document
**Version:** 0.5.4 (14 MCP tools, 10 CLI commands, 397 tests)
**Updated:** 2026-01-16
**Performance:** Validated with 30/30 test scenarios (100% success rate)
- **Indexing:** 1,928-11,210 files/sec (Istio: 5,605 files in 0.5s, OpenEMR: 6,364 files in 3.3s)
- **Search:** 2ms latency, 210-650 tokens/query, 11 file types in single query
- **Test repositories:** Istio (Go-heavy, 5,605 files) and OpenEMR (PHP polyglot, 6,364 files)
- **Coverage:** 86.76% line coverage (44 source files, ~7,500 LOC)
