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
- **Fast:** 2ms query latency (10X faster than ripgrep - faster than a
  single frame of 60fps video)
- **Scalable:** 2k-12k files/sec indexing (5500 files in 0.5s - the time it takes to blink)
- **Token-efficient:** 200-700 tokens/query (3-12x fewer tokens than Claude + ripgrep workflows
  at 2,000-8,000 tokens - ripgrep returns paths only, requiring file reads for content)
- **Simple:** BM25 only, no embeddings/GPU needed, no resource-hogging
- **UTF-8 Safe:** Handles emoji, CJK, all Unicode
- **14 MCP Tools:** Direct Claude Code integration ([full reference](./docs/guides/mcp-tools-reference.md))

**Positioning:** Complements structural tools (Serena MCP) with content search.

---

## Why Shebe?

Shebe draws inspiration from [Lethe](https://github.com/sibyllinesoft/lethe), a full-featured enterprise
RAG platform (which is overkill for local development) and addresses two gaps in existing tools:

1. **Serena MCP** provides structural code navigation via LSP but cannot search non-code files
   (markdown, YAML, configs, documentation)
2. **grep/ripgrep** return only file paths, requiring Claude to read entire files (2,000-8,000 tokens)

Shebe fills both gaps as a lightweight, single-binary tool that returns ranked content snippets
directly (200-700 tokens).


### The Shebe Workflow

```bash
# One-time setup (indexes 5,600 files in 0.5s)
shebe-mcp index_repository /path/to/istio istio
```

**search_code** - Discover code by keywords (2ms, 200-700 tokens):

```bash
shebe-mcp search_code "authentication middleware" istio
```
```
# Results ranked by BM25 relevance:
1. security/pkg/server/ca/authenticate.go:45 (score: 12.3)
   func (s *Server) authenticateRequest(ctx context.Context) ...

2. pilot/pkg/security/authz/policy.go:89 (score: 11.8)
   // Authentication middleware for incoming requests...
```

**find_references** - Locate symbol usages with confidence scoring (5-32ms):

```bash
shebe-mcp find_references "AuthorizationPolicy" istio
```
```
## References to `AuthorizationPolicy` (50 found)

### High Confidence (35)
#### pilot/pkg/security/authz/builder.go:123
  func buildPolicy(p *AuthorizationPolicy) *model.Config {
- **Pattern:** type_annotation
- **Confidence:** 0.85

### Medium Confidence (15)
#### tests/integration/security/authz_test.go:45
  // Test AuthorizationPolicy enforcement
- **Pattern:** word_match (+0.05 test boost)
- **Confidence:** 0.65
```

**How shebe stands out:**

| Capability                | Shebe       | grep/ripgrep           | Serena MCP  |
|---------------------------|-------------|------------------------|-------------|
| Ranked results (BM25)     | Yes         | No (all matches equal) | No          |
| Confidence scoring        | Yes (H/M/L) | No                     | No          |
| Non-code files (YAML, md) | Yes         | Yes                    | No          |
| Token efficiency          | 200-700     | 2,000-8,000            | 1,000-3,000 |
| Speed (5k+ files)         | 2-32ms      | 100-1000ms             | 500-5000ms  |
| Deduplication             | Yes         | No                     | Yes         |

### Speed Comparison

```
                          Query Latency (lower is better)
Shebe BM25                |## 2ms
Claude + ripgrep          |################ 50-200ms
Claude + Web Search       |#################################### 1-3s
Serena Pattern Search     |################################################# 8s+
```

### Detailed Comparison

| Approach                   | Speed     | Tokens/Query  | Limitations                                       |
|----------------------------|-----------|---------------|---------------------------------------------------|
| **Shebe BM25 Index**       | **2ms**   | **210-650**   | Keyword search only (no structural queries)       |
| Claude Code + grep/ripgrep | 50-200ms  | 2,000-8,000   | Must read entire files, slow on large repos       |
| Claude Code + Web Search   | 1-3s      | 5,000-15,000  | Rate limits, network latency, incomplete results  |
| Raw GitHub URLs            | 500ms-2s  | 10,000-50,000 | Network overhead, must know exact file paths      |
| Serena MCP (LSP)           | 100-500ms | 1,000-3,000   | Optimized for structural queries, slow for search |

### Why Shebe is Faster

**1. Pre-computed BM25 Index**
- Indexing happens once (0.5-3.3s for 5k-6k files)
- Search queries hit in-memory Tantivy index (2ms)
- No file I/O or regex processing during search

**2. Token Efficiency**
- Returns only relevant snippets (5 lines context)
- No need to read entire files into Claude's context
- 8-24x fewer tokens than web search or raw file reads

**3. Purpose-built for Keyword Search**
- BM25 ranking returns most relevant results first
- Language-agnostic (works across 11+ file types in one query)
- UTF-8 safe (handles emoji, CJK, special characters)

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
| `SHEBE_LOG_LEVEL`  | `info`                 | Logging verbosity (debug, info, warn, error) |
| `SHEBE_HOST`       | `127.0.0.1`            | HTTP server bind address                     |
| `SHEBE_PORT`       | `3000`                 | HTTP server port                             |
| `SHEBE_CHUNK_SIZE` | `512`                  | Characters per chunk (100-2000)              |
| `SHEBE_OVERLAP`    | `64`                   | Overlap between chunks                       |

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

### Two Ways to Use Shebe

| Binary      | Purpose         | When to Use |
|-------------|-----------------|--------------------------------------------------------|
| `shebe`     | HTTP REST API   | Programmatic access, CI/CD integration, web dashboards |
| `shebe-mcp` | Claude Code MCP | Interactive coding sessions, AI-assisted development   |

Both binaries share the same index storage (`~/.local/state/shebe/sessions/`).
Index once, query from anywhere.

### System Design

```
                    ┌─────────────────┐
                    │   Claude Code   │
                    └────────┬────────┘
                             │ MCP (stdio)
                    ┌────────▼────────┐
                    │   shebe-mcp     │
                    └────────┬────────┘
                             │
    ┌────────────────────────┼────────────────────────┐
    │                        │                        │
    │              ┌─────────▼─────────┐              │
    │              │   Shared Storage  │              │
    │              │ ~/.local/state/   │              │
    │              │   shebe/sessions/ │              │
    │              └─────────▲─────────┘              │
    │                        │                        │
    └────────────────────────┼────────────────────────┘
                             │
                    ┌────────┴────────┐
                    │ shebe (HTTP)    │
                    └────────-────────┘
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

**Version:** 0.5.0
**Status:** Production Ready - All 14 MCP Tools Validated
**Testing:** 392 unit tests (86.76% coverage) + 30 performance scenarios (100% pass rate)
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
2. All 392 tests must pass (`make test`)
3. Zero clippy warnings (`make clippy`)
4. Max 120 char line length
5. Maintain >85% test coverage (currently 86.76%)
6. Single commit per feature branch

See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community guidelines.
