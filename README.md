# Shebe

**Simple RAG Service for Code Search**

Fast BM25 full-text search for code repositories with MCP integration for Claude Code.


## Table of Contents

- [What is Shebe?](#what-is-shebe)
- [Why Shebe?](#why-shebe)
- [Documentation](#documentation)
- [Performance](#performance)
- [Architecture](#architecture)
- [Development](#development)
- [Project Status](#project-status)
- [License](#license)
- [Contributing](#contributing)

---

## What is Shebe?

Shebe provides **content search** for code - find functions, APIs, and patterns across 
large codebases using keyword search.

**Key Features:**
- **Fast:** 2ms query latency (10x better than 20ms target)
- **Scalable:** 1,928-11,210 files/sec indexing (3.9x-22.4x faster than target)
- **Token-efficient:** 210-650 tokens/query (8-24x better than 5,000 target)
- **Simple:** BM25 only, no embeddings/GPU needed
- **UTF-8 Safe:** Handles emoji, CJK, all Unicode
- **12 MCP Tools:** Direct Claude Code integration

**Positioning:** Complements structural tools (Serena MCP) with content search.
**Validated:** 30/30 performance test scenarios + 384 unit tests passed

---

## Why Shebe?

When working with large reference codebases (Istio, OpenEMR, Django, etc.), you need fast keyword search without
burning tokens or waiting for slow searches. Shebe's workflow is dramatically faster and more efficient than
alternatives:

### The Shebe Workflow

```bash
# One-time setup
git clone https://github.com/istio/istio
shebe-mcp index_repository /path/to/istio session_name

# Fast searches (2ms each, 210-650 tokens)
shebe-mcp search "authentication middleware" session_name
shebe-mcp search "rate limiting config" session_name
```

### Comparison: Shebe vs Alternatives

| Approach                   | Speed     | Tokens/Query  | Limitations                                                           |
|----------------------------|-----------|---------------|-----------------------------------------------------------------------|
| **Shebe BM25 Index**       | **2ms**   | **210-650**   | Keyword search only (no structural queries)                           |
| Claude Code + grep/ripgrep | 50-200ms  | 2,000-8,000   | Must read entire files, slow on large repos                           |
| Claude Code + Web Search   | 1-3s      | 5,000-15,000  | Rate limits, network latency, incomplete results                      |
| Raw GitHub URLs            | 500ms-2s  | 10,000-50,000 | Network overhead, must know exact file paths                          |
| Serena MCP (LSP)           | 100-500ms | 1,000-3,000   | Optimized for structural queries (go-to-def), slow for keyword search |

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

### When to Use Each Tool

**Use Shebe when:**
- Searching for keywords, APIs, patterns across large codebases
- Working with reference repos you don't have locally (clone + index once)
- Need fast, token-efficient content search
- Looking for usage examples or implementation patterns

**Use Serena MCP when:**
- Need structural queries (go-to-definition, find references)
- Working with TypeScript/JavaScript projects
- Need type information or symbol navigation
- Local codebase is already open in your editor

**Use grep/ripgrep when:**
- Single search in local codebase
- Don't need ranking or relevance scoring
- Repository is small (<1,000 files)

**Use web search when:**
- Need latest documentation or blog posts
- Searching for concepts, not specific code
- Looking for community discussions or issues

### Shebe Complements Other Tools

Shebe focuses on **content search** and works alongside structural tools:
- **Serena MCP:** Structural navigation (go-to-def, references)
- **Shebe:** Keyword search (find usage patterns, APIs, examples)
- **Together:** Complete codebase exploration without token waste

---

## Documentation

- **[INSTALLATION.md](./INSTALLATION.md)** - Installation and setup guide for shebe-mcp
- **[CONFIGURATION.md](./CONFIGURATION.md)** - Complete configuration reference (server, indexing, search, limits)
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Developer guide (where/how to change code)
- **[docs/Performance.md](./docs/Performance.md)** - Performance benchmarks and analysis
- **[docs/guides/](./docs/guides/)** - User guides (setup, quick start, Docker)
- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - How to contribute to the project
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

**Dual-binary design:**
- `shebe` - HTTP server (indexing)
- `shebe-mcp` - MCP server (Claude Code search)

Both share filesystem storage (no network coordination needed).

See [ARCHITECTURE.md](./ARCHITECTURE.md) for developer guide.

---

## Project Status

**Version:** 0.3.0 <br>
**Status:** Production Ready - All 11 MCP Tools Validated <br>
**Recent:** Comprehensive performance testing complete (30/30 scenarios, 100% pass rate) <br>
**Performance:** 2ms search, 1,928-11,210 files/sec indexing, 11 file types in single query <br>
**Testing:** 384 unit tests (86.76% coverage) + 30 performance scenarios (100% success rate) <br>
**Next:** Stage 3 (CI/CD Pipeline) <br>

See [docs/Performance.md](./docs/Performance.md) for detailed performance benchmarks.

---

## License

See [LICENSE](./LICENSE).

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines.

**Quick checklist:**
1. Read [ARCHITECTURE.md](./ARCHITECTURE.md) for codebase guide
2. All 384 tests must pass
3. Zero clippy warnings
4. Max 120 char line length
5. Maintain >85% test coverage (currently 86.76%)
6. Single commit per feature branch

See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community guidelines.
