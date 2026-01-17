# Shebe Server

**BM25 Code Search Engine with MCP Integration**

The main Rust-based RAG (Retrieval-Augmented Generation)
service providing BM25 full-text search capabilities for code
repositories.

## Quick Start

```bash
# Build from repository root (uses docker-dev container)
make build-release

# Install CLI and MCP binaries
make shebe-install

# Test installation
shebe get-server-info
shebe-mcp --version
```

## Architecture

See [ARCHITECTURE.md](/ARCHITECTURE.md) in the repository root
for complete system design.

## Key Features

- **BM25 Full-Text Search** via Tantivy (2ms latency)
- **UTF-8 Safe Chunking** (character-based, never panics)
- **Session-Based Indexing** (isolated indexes)
- **MCP Server** (14 tools for Claude Code integration)
- **CLI** (10 commands for scripting and manual operations)
- **Production Ready** (Docker, logging)

## Binaries

Shebe provides two binaries:

| Binary      | Purpose                                 | Usage                                   |
|-------------|-----------------------------------------|-----------------------------------------|
| `shebe`     | CLI for scripting and manual operations | `shebe search-code "query" -s session`  |
| `shebe-mcp` | MCP server for Claude Code              | Configured in `~/.claude/settings.json` |

## CLI Usage

```bash
# Index a repository
shebe index-repository /path/to/repo --session myproject

# Search for code
shebe search-code "authentication" --session myproject

# List sessions
shebe list-sessions

# Get session details
shebe get-session-info myproject

# Delete a session
shebe delete-session myproject --confirm

# Show configuration
shebe show-config

# Show version info
shebe get-server-info

# Generate shell completions
shebe completions bash > ~/.local/share/bash-completion/completions/shebe
```

For complete CLI documentation, see [docs/guides/cli-usage.md](/docs/guides/cli-usage.md).

## MCP Integration

Configure in `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "args": []
    }
  }
}
```

For MCP setup details, see [docs/guides/mcp-setup-guide.md](/docs/guides/mcp-setup-guide.md).

## Configuration

Configuration via `~/.config/shebe/config.toml` or environment variables:

```toml
[indexing]
chunk_size = 512
overlap = 64

[storage]
index_dir = "~/.local/state/shebe"

[search]
default_k = 10
max_k = 100
```

## Development

**Working Directory:** All cargo commands via Makefile from repository root.

```bash
# From repository root
make build          # Build debug
make test           # Run tests (397 tests)
make clippy         # Lint
make fmt            # Format
make shell          # Interactive container shell
```

See [CLAUDE.md](/.claude/CLAUDE.md) for development workflows and conventions.

## Documentation

- **Root:** `/docs/` - User-facing documentation
- **Guides:** `/docs/guides/` - CLI, MCP setup guides
- **Performance:** `/docs/Performance.md` - Benchmarks
- **Dev Docs:** `/dev-docs/` - Development planning (gitignored)

## License

See LICENSE file in repository root.
