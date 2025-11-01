# Shebe Architecture

**Content Search for Code - Developer's Guide to the Codebase**

**Version:** 0.3.0 <br>
**Updated:** 2025-10-28 <br>
**Status:** 12 MCP Tools, 392 Tests (Production Ready)


> **Purpose:** This document helps you understand where to find code and how to make changes.
> For performance data, see [docs/Performance.md](./docs/Performance.md).

---

## Bird's Eye View

Shebe is a **dual-binary RAG service** that provides BM25 full-text search for code repositories:

```
     Claude Code (MCP Client)
               │
               │ MCP Protocol (stdio)
               ▼
    ┌────────────────────────────────┐      ┌────────────────────────────────┐
    │   shebe-mcp (MCP Binary)       │      │   shebe (HTTP Server)          │
    │   - 12 MCP tools               │      │   - REST API (5 endpoints)     │
    │   - stdio transport            │      │   - Initial indexing           │
    │   - Independent operation      │      │   - Optional (not required)    │
    └───────────┬────────────────────┘      └────────────┬───────────────────┘
                │                                        │
                │  Direct filesystem access              │ Direct filesystem access
                │                                        │
                └────────────────┬───────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────────────┐
                │  ~/.local/state/shebe/sessions/             │
                │  (Tantivy indexes + session metadata)       │
                │  - Shared Storage (filesystem)              │
                └─────────────────────────────────────────────┘
```

**Key Insight:** Both binaries are **independent peers** accessing shared storage. 
MCP server can index AND search without the HTTP server running.

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

```
shebe/                         # Repository root
├── services/shebe-server/     # Main Rust service
│   ├── src/
│   │   ├── main.rs            # Entry: HTTP server
│   │   ├── lib.rs             # Library root
│   │   ├── bin/
│   │   │   └── shebe_mcp.rs   # Entry: MCP server
│   │   ├── config.rs          # Config (TOML + env)
│   │   ├── types.rs           # Data structures
│   │   ├── error.rs           # Error types
│   │   ├── indexer/           # Indexing pipeline
│   │   │   ├── chunker.rs     # UTF-8 safe chunking
│   │   │   ├── walker.rs      # File traversal
│   │   │   └── pipeline.rs    # Orchestration
│   │   ├── storage/           # Persistence
│   │   │   ├── tantivy.rs     # Index wrapper
│   │   │   └── session.rs     # Session mgmt
│   │   ├── search/            # Search
│   │   │   └── bm25.rs        # BM25 service
│   │   ├── api/               # REST API
│   │   │   └── handlers.rs    # 5 endpoints
│   │   └── mcp/               # MCP integration
│   │       ├── server.rs      # Event loop
│   │       ├── handlers.rs    # Protocol
│   │       ├── services.rs    # Wrapper
│   │       └── tools/         # 11 tools
│   ├── tests/                 # 384 tests
│   └── Cargo.toml             # 17 prod deps
├── docs/
│   ├── CONTEXT.md             # Status tracker
│   ├── Performance.md         # Benchmarks
│   └── guides/                # User guides
├── ARCHITECTURE.md            # This doc
└── README.md                  # Overview
```

---

## Entry Points

### HTTP Server: `src/main.rs`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load config (config.rs)
    // 2. Create StorageManager (storage/session.rs)
    // 3. Create SearchService (search/bm25.rs)
    // 4. Build Axum router (api/handlers.rs)
    // 5. Start server
}
```

**Add REST endpoints:** `api/handlers.rs`

### MCP Server: `src/bin/shebe_mcp.rs`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load config
    // 2. Create ShebeServices
    // 3. Register tools (mcp/handlers.rs)
    // 4. Run stdio loop (mcp/server.rs)
}
```

**Add MCP tools:** `mcp/tools/*.rs`

---

## Key Files by Task

### Adding a New MCP Tool

1. `src/mcp/tools/your_tool.rs` - Create tool
2. `src/mcp/tools/mod.rs` - Export tool
3. `src/mcp/handlers.rs` - Register in ToolRegistry
4. `src/mcp/tools/get_server_info.rs` - Update tool list
5. Add tests in your tool file

**Pattern:** See `preview_chunk.rs` (~420 LOC, 8 tests)

### Modifying Search

- **Query parsing:** `src/search/bm25.rs`
- **Ranking:** Tantivy BM25 (not customizable)
- **Formatting:** `src/mcp/tools/search_code.rs`

### Changing Indexing

- **File walking:** `src/indexer/walker.rs`
- **Chunking:** `src/indexer/chunker.rs`
- **Storage:** `src/storage/session.rs`

**INVARIANT:** Chunker must respect UTF-8 boundaries

### Configuration

- **Struct:** `src/config.rs`
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

### Why Dual-Binary?

**Decision:** Separate `shebe` + `shebe-mcp`

**Rationale:**
- MCP works without HTTP server
- Clean separation
- Lower memory

**Trade-off:** Two binaries, shared lib

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
5. **Tests:** All 392 must pass (100% success rate)
6. **Schema:** v3 with repository_path and last_indexed_at fields

### Storage Layout

```
~/.local/state/shebe/sessions/
├── {session-id}/
│   ├── meta.json      # Metadata
│   └── tantivy/       # Index
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

17 production crates:

| Crate              | Purpose      | Why             |
|--------------------|--------------|-----------------|
| tantivy 0.22       | BM25         | Pure Rust       |
| axum 0.7           | HTTP         | Tokio-native    |
| tower/tower-http   | HTTP         | Middleware      |
| tokio 1.x          | Async        | Standard        |
| serde/serde_json   | JSON         | API             |
| walkdir            | Files        | Simple          |
| glob               | Patterns     | Familiar        |
| regex              | Pattern match| File discovery  |
| thiserror          | Errors       | Derive          |
| tracing*           | Logs         | Async           |
| toml               | Config       | Config files    |
| chrono             | Timestamps   | Metadata        |
| async-trait        | Traits       | MCP             |
| dirs               | XDG paths    | Cross-platform  |

---

## Testing

392 tests in 7 categories:

1. Unit (220): Module logic
2. Integration (109): E2E
3. API (7): REST
4. Session (24): Mgmt
5. MCP (13): Protocol
6. UTF-8 (19): Safety
7. Doc (3 ignored): Examples

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

# Run
cargo run              # HTTP
cargo run --bin shebe-mcp  # MCP
```

---

## MCP Tools (12)

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
| preview_chunk      | Ergonomic | Show chunk context (v0.3.0: schema v2 fix)   | <5ms                        |
| reindex_session    | Ergonomic | Re-index using stored path (v0.3.0: v3 feat) | Same as index_repository    |

**Pattern:** All implement `McpToolHandler`
**Performance:** Validated on 30/30 test scenarios (100% success rate)

---

## Error Handling

```
ShebeError -> McpError -> JSON-RPC error
```

**INVARIANT:** All `ShebeError` must map to `McpError`

---

## Related Docs

- [README.md](./README.md) - Overview
- [docs/CONTEXT.md](./docs/CONTEXT.md) - Status
- [docs/Performance.md](./docs/Performance.md) - Benchmarks
- [docs/guides/](./docs/guides/) - User guides

---

**Document Status:** Living document
**Version:** 0.3.0 (12 tools, 392 tests, schema v3)
**Updated:** 2025-10-28
**Performance:** Validated with 30/30 test scenarios (100% success rate)
- **Indexing:** 1,928-11,210 files/sec (Istio: 5,605 files in 0.5s, OpenEMR: 6,364 files in 3.3s)
- **Search:** 2ms latency, 210-650 tokens/query, 11 file types in single query
- **Test repositories:** Istio (Go-heavy, 5,605 files) and OpenEMR (PHP polyglot, 6,364 files)
- **Coverage:** 86.76% line coverage (44 source files, ~7,500 LOC)
