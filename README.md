# Shebe

**Simple RAG Service for Code Search**

Fast BM25 full-text search for code repositories with MCP integration for Claude Code.


## Table of Contents

- [Quick Start](#quick-start)
- [What is Shebe?](#what-is-shebe)
- [Why Shebe?](#why-shebe)
- [Common Tasks](#common-tasks)
- [Tool Selection Guide](#tool-selection-guide)
- [Configuration](#configuration)
- [Documentation](#documentation)
- [Performance](#performance)
- [Architecture](#architecture)
- [Troubleshooting](#troubleshooting)
- [Project Status](#project-status)
- [License](#license)
- [Contributing](#contributing)

---

## Quick Start

See [INSTALLATION.md](./INSTALLATION.md).

---

## What is Shebe?

Shebe provides **content search** for code - find functions, APIs and patterns across
large codebases using keyword search.

**Key Features:**
- 2ms query latency
- 2k-12k files/sec indexing (6k files in 0.5s)
- 200-700 tokens/query
- BM25 only - no embeddings or GPU
- Full UTF-8 support (emoji, CJK, special characters)
- 14 MCP tools for Claude Code ([reference](./docs/guides/mcp-tools-reference.md))

**Positioning:** Complements structural tools (Serena MCP) with content search.

---

## Why Shebe?

When using AI coding assistants to refactor symbols across large codebases (6k+ files),
developers face a binary choice: precision (LSP-based tools) or efficiency (grep/ripgrep).
Shebe attempts to eliminate this trade-off.

**Benchmark: Refactoring `AuthorizationPolicy` across Istio (~6k files)**

| Approach | Searches | Time | Tokens |
|----------|----------|------|--------|
| Shebe `find_references` | 1 | 2-3s | ~4,500 |
| Claude + Grep | 13 | 15-20s | ~12,000 |
| Claude + Serena MCP | 8 | 25-30s | ~18,000 |

Shebe provides 6-10x faster end-to-end time and 3-4x fewer tokens by returning
confidence-scored, pattern-classified results in a single call.

See [WHY_SHEBE.md](./WHY_SHEBE.md) for detailed benchmarks and tool comparisons.

### Quick Comparison

| Capability | Shebe | grep/ripgrep | Serena MCP |
|------------|-------|--------------|------------|
| Ranked results (BM25) | Yes | No | No |
| Confidence scoring | Yes | No | No |
| Non-code files (YAML, md) | Yes | Yes | No |
| Token efficiency | 200-700 | 2,000-8,000 | 1,000-3,000 |
| Speed (5k+ files) | 2-32ms | 100-1000ms | 500-5000ms |

---

## Common Tasks

Quick links to accomplish specific goals:

| Task                     | Tool                               | Guide                                                                  |
|--------------------------|------------------------------------|------------------------------------------------------------------------|
| Rename a symbol safely   | `find_references`                  | [Reference](./docs/guides/mcp-tools-reference.md#tool-find_references) |
| Search polyglot codebase | `search_code`                      | [Reference](./docs/guides/mcp-tools-reference.md#tool-search_code)     |
| Explore unfamiliar repo  | `index_repository` + `search_code` | [Quick Start](./docs/guides/mcp-quick-start.md)                        |
| Find files by pattern    | `find_file`                        | [Reference](./docs/guides/mcp-tools-reference.md#tool-find_file)       |
| View file with context   | `read_file` or `preview_chunk`     | [Reference](./docs/guides/mcp-tools-reference.md#tool-read_file)       |
| Update stale index       | `reindex_session`                  | [Reference](./docs/guides/mcp-tools-reference.md#tool-reindex_session) |

---

## Tool Selection Guide

### Content Search (Use Shebe)

Best for finding code by keywords, patterns and text content:
- "Find all usages of `authenticate`"
- "Where is rate limiting implemented?"
- "Show me error handling patterns"
- "Find configuration for database connections"

### Structural Navigation (Use Serena/LSP)

Best for precise symbol operations and type information:
- "Go to definition of `UserService`"
- "Find all implementations of `Handler` trait"
- "Rename `oldFunc` to `newFunc` across codebase"
- "Show type hierarchy for this class"

### Simple Pattern Matching (Use grep/ripgrep)

Best for exact string matches in small codebases:
- "Find exact string `TODO:`"
- "Count occurrences of `deprecated`"
- "Quick one-off search in <1,000 files"

### External Information (Use Web Search)

Best for documentation and community knowledge:
- "Latest React 19 migration guide"
- "Community solutions for specific errors"
- "Blog posts about architectural patterns"

### Shebe + Serena Together

For complete codebase exploration without token waste:

```
1. Shebe: "Find usages of authenticate" -> discover files (2ms, 300 tokens)
2. Serena: "Go to definition" -> navigate to implementation (precise)
3. Shebe: "Find similar patterns" -> discover related code (2ms, 300 tokens)
```

---

## Configuration

### Quick Reference

| Variable           | Default                | Description                                  |
|--------------------|------------------------|----------------------------------------------|
| `SHEBE_INDEX_DIR`  | `~/.local/state/shebe` | Session storage location                     |
| `SHEBE_CHUNK_SIZE` | `512`                  | Characters per chunk (100-2000)              |
| `SHEBE_OVERLAP`    | `64`                   | Overlap between chunks                       |
| `SHEBE_DEFAULT_K`  | `10`                   | Default search results count                 |
| `SHEBE_MAX_K`      | `100`                  | Maximum search results allowed               |

### Configuration File

Create `shebe.toml` in your working directory or `~/.config/shebe/shebe.toml`:

```toml
[indexing]
chunk_size = 512
overlap = 64
max_file_size = 10485760  # 10MB

[search]
default_k = 10
max_k = 100
```

See [CONFIGURATION.md](./CONFIGURATION.md) for complete reference.

---

## Documentation

### Getting Started
- **[INSTALLATION.md](./INSTALLATION.md)** - Installation and setup guide
- **[Quick Start Guide](./docs/guides/mcp-quick-start.md)** - 5-minute setup for Claude Code

### Reference
- **[MCP Tools Reference](./docs/guides/mcp-tools-reference.md)** - Complete API for all 14 tools
- **[CONFIGURATION.md](./CONFIGURATION.md)** - All configuration options
- **[Performance Benchmarks](./docs/Performance.md)** - Detailed performance data

### Development
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Developer guide (where/how to change code)
- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - How to contribute
- **[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)** - Community guidelines
- **[SECURITY.md](./SECURITY.md)** - Security policy and reporting

---

## Performance

**Validated on Istio (5,605 files, Go-heavy) and OpenEMR (6,364 files, PHP polyglot):**

| Metric             | Result                                           |
|--------------------|--------------------------------------------------|
| Query latency      | **2ms** (consistent across all query types)      |
| Indexing (Istio)   | **11,210 files/sec** (0.5s for 5,605 files)      |
| Indexing (OpenEMR) | **1,928 files/sec** (3.3s for 6,364 files)       |
| Token usage        | **210-650** tokens/query                         |
| Polyglot coverage  | **11 file types** in single query                |

See [docs/Performance.md](./docs/Performance.md) for detailed benchmarks.

---

## Architecture

### MCP-Only Design

Shebe is accessed exclusively via the MCP protocol, designed for Claude Code integration.
No HTTP server required.

### System Design

```
                    +------------------+
                    |   Claude Code    |
                    +--------+---------+
                             | MCP (stdio)
                    +--------v---------+
                    |   shebe-mcp      |
                    |   (14 tools)     |
                    +--------+---------+
                             |
                    +--------v---------+
                    |  Shared Storage  |
                    | ~/.local/state/  |
                    |  shebe/sessions/ |
                    +------------------+
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for developer guide.

---

## Troubleshooting

| Issue                         | Cause                            | Solution                                           |
|-------------------------------|----------------------------------|----------------------------------------------------|
| "Session not found"           | Session doesn't exist or typo    | Run `list_sessions` to see available sessions      |
| "Schema version mismatch"     | Session from older Shebe version | Run `upgrade_session` to migrate                   |
| Slow indexing                 | Disk I/O or large files          | Exclude `node_modules/`, `target/`, check disk     |
| No search results             | Empty session or wrong query     | Verify with `get_session_info`, check query syntax |
| "File not found" in read_file | File deleted since indexing      | Run `reindex_session` to update                    |
| High token usage              | Too many results                 | Reduce `k` parameter (default: 10)                 |

For detailed troubleshooting, see [docs/guides/mcp-setup-guide.md](./docs/guides/mcp-setup-guide.md).

---

## Project Status

**Version:** 0.6.0
**Status:** Production Ready - MCP-Only Architecture (14 Tools)
**Testing:** 397 tests (86.76% coverage) + 30 performance scenarios (100% pass rate)
**Next:** Stage 3 (CI/CD Pipeline)

See [CHANGELOG.md](./CHANGELOG.md) for version history.

---

## License

See [LICENSE](./LICENSE).

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines.

**Quick checklist:**
1. Read [ARCHITECTURE.md](./ARCHITECTURE.md) for codebase guide
2. All 397 tests must pass (`make test`)
3. Zero clippy warnings (`make clippy`)
4. Max 120 char line length
5. Maintain >85% test coverage (currently 86.76%)
6. Single commit per feature branch

See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community guidelines.
