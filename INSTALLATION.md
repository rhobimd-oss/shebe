# Shebe Installation Guide

**Version:** 0.3.0
**Last Updated:** 2025-10-26
**Focus:** MCP Server (shebe-mcp) for Claude Code Integration

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Building from Source](#building-from-source)
4. [Installation](#installation)
5. [Claude Code Configuration](#claude-code-configuration)
6. [Verification](#verification)
7. [Troubleshooting](#troubleshooting)

---

## Overview

Shebe provides **shebe-mcp**, a Model Context Protocol server that integrates with Claude Code for
real-time code search during development sessions.

**What you get:**
- 11 MCP tools for code search, file discovery, and session management
- Direct repository indexing from Claude Code conversations
- 2ms search latency (validated on 5-6k file repositories)
- Support for 11+ file types in polyglot codebases
- 384 tests passing (100% success rate)

**No HTTP server required** - shebe-mcp operates independently with filesystem storage.

---

## Prerequisites

- **Rust:** 1.88 or later ([install from rustup.rs](https://rustup.rs/))
- **Git:** For cloning the repository
- **Claude Code:** Desktop app with MCP support
- **Platform:** Linux, macOS, or Windows

---

## Building from Source

### 1. Clone Repository

```bash
git clone https://gitlab.com/rhobimd-oss/shebe.git
cd shebe
```

### 2. Build MCP Binary

```bash
# Build release binary (optimized)
make mcp-build

# Binary created at: services/shebe-server/target/release/shebe-mcp
```

**Alternative (manual build):**
```bash
cd services/shebe-server
cargo build --release --bin shebe-mcp
```

### 3. Verify Build

```bash
# Run tests to ensure everything works
make mcp-test

# Check binary exists
ls -lh services/shebe-server/target/release/shebe-mcp
```

---

## Installation

### Recommended: User Installation

Install to your local bin directory (no sudo required):

```bash
# Install shebe-mcp to ~/.local/bin/ or /usr/local/bin/
make mcp-install

# Verify installation
which shebe-mcp
# Expected output: /usr/local/bin/shebe-mcp (or ~/.local/bin/shebe-mcp)
```

### Optional: Install Configuration File

Shebe works with built-in defaults, but you can customize settings:

```bash
# Using Makefile (recommended)
make mcp-install-config

# Or manually
mkdir -p ~/.config/shebe
cp shebe.toml ~/.config/shebe/config.toml

# Edit configuration (optional)
# See CONFIGURATION.md for available options
nano ~/.config/shebe/config.toml
```

The `mcp-install-config` target will:
- Create `~/.config/shebe/` directory if it doesn't exist
- Copy `shebe.toml` to `~/.config/shebe/config.toml`
- Skip if config file already exists (preserves your existing config)

**Note:** Most users don't need a config file. Only create one if you need to:
- Change chunk size or overlap settings
- Customize file patterns
- Adjust search result limits
- Use a custom data directory

See [CONFIGURATION.md](./CONFIGURATION.md) for all available options.

### Manual Installation

If you prefer manual installation:

```bash
# Build binary
cd services/shebe-server
cargo build --release --bin shebe-mcp

# Copy to a directory in your PATH
sudo cp target/release/shebe-mcp /usr/local/bin/

# Or for user-only installation
mkdir -p ~/.local/bin
cp target/release/shebe-mcp ~/.local/bin/

# Ensure ~/.local/bin is in your PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"

# Optional: Install config file template
mkdir -p ~/.config/shebe
cp shebe.toml ~/.config/shebe/config.toml
```

---

## Claude Code Configuration

### 1. Create MCP Configuration

Create or edit `~/.claude/mcp.json`:

```bash
mkdir -p ~/.claude
cat > ~/.claude/mcp.json <<'EOF'
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {
        "SHEBE_DATA_DIR": "$HOME/.local/state/shebe"
      }
    }
  }
}
EOF
```

**Configuration breakdown:**
- `command: "shebe-mcp"` - Uses the installed binary from PATH
- `SHEBE_DATA_DIR` - Where indexes are stored (defaults to ~/.local/state/shebe)

### 2. Restart Claude Code

After creating the configuration:
1. Quit Claude Code completely
2. Restart Claude Code
3. MCP server will be auto-discovered

### 3. Usage in Claude Code

Once configured, you can use Shebe directly in conversations:

**Index a repository:**
```
"Index the repository at ~/projects/myapp as session 'myapp-main'"
```

**Search indexed code:**
```
"Search for 'authentication' in the myapp-main session"
```

**Discover files:**
```
"List all YAML files in the myapp-main session"
```

**Get context around results:**
```
"Show me 20 lines of context around that search result"
```

Claude Code will automatically use the appropriate MCP tools.

---

## Verification

### Test MCP Server Manually

```bash
# Test initialize method
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  shebe-mcp | jq '.'

# Expected: JSON response with server info and 11 MCP tools
```

### Test in Claude Code

1. **Open Claude Code**
2. **Start a new conversation**
3. **Ask:** "What Shebe sessions are available?"
4. **Expected:** Claude Code calls `list_sessions` tool and shows results

### Verify Directory Structure

```bash
# Check that data directory exists
ls -la ~/.local/state/shebe/

# After indexing, you should see:
# ~/.local/state/shebe/sessions/{session-name}/
```

---

## Troubleshooting

### Issue: Claude Code Can't Find shebe-mcp

**Solution:**
```bash
# 1. Verify binary is in PATH
which shebe-mcp

# 2. Test binary manually
echo '{"jsonrpc":"2.0","id":1,"method":"ping"}' | shebe-mcp

# 3. Check MCP config
cat ~/.claude/mcp.json | jq '.mcpServers.shebe'

# 4. Ensure PATH is correct in config
# Use absolute path if needed:
{
  "mcpServers": {
    "shebe": {
      "command": "/usr/local/bin/shebe-mcp"
    }
  }
}
```

### Issue: Session Not Found

**Solution:**
```bash
# Check available sessions
ls ~/.local/state/shebe/sessions/

# Verify SHEBE_DATA_DIR matches in config
# Index a test repository to create first session
```

### Issue: No Search Results

**Solution:**
```bash
# Enable debug logging in MCP config
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {
        "SHEBE_DATA_DIR": "$HOME/.local/state/shebe",
        "RUST_LOG": "debug"
      }
    }
  }
}

# Check logs (location varies by platform)
# Linux: ~/.config/claude/logs/mcp.log
# macOS: ~/Library/Logs/Claude/mcp.log

tail -f ~/.config/claude/logs/mcp.log
```

### Issue: Indexing Fails

**Checklist:**
1. Ensure path exists and is readable
2. Check disk space: `df -h ~/.local/state/shebe/`
3. Verify permissions: `ls -la ~/.local/state/shebe/`
4. Enable debug logging (see above)

### Issue: Slow Performance

**Solutions:**
- **Indexing:** Normal for large repos (3.3s for 6k files)
- **Search:** Should be <10ms; check debug logs
- **Large files:** Auto-truncates at 20KB (expected behavior)

### Common Error Messages

| Error                 | Meaning                 | Solution                          |
|-----------------------|-------------------------|-----------------------------------|
| "Session not found"   | Session doesn't exist   | Index repository first            |
| "Path does not exist" | Invalid repo path       | Check path is absolute and exists |
| "Permission denied"   | Can't write to data dir | Check directory permissions       |
| "Schema error"        | Old index format        | Delete session and re-index       |

---

## Advanced Configuration

### Custom Data Directory

```json
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {
        "SHEBE_DATA_DIR": "/custom/path/to/data"
      }
    }
  }
}
```

### Debug Logging

```json
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {
        "SHEBE_DATA_DIR": "$HOME/.local/state/shebe",
        "RUST_LOG": "shebe=debug,tantivy=info"
      }
    }
  }
}
```

### Multiple MCP Servers

You can run shebe alongside other MCP servers:

```json
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp"
    },
    "serena": {
      "command": "serena-mcp"
    }
  }
}
```

---

## Quick Reference

### Installation Summary

```bash
# 1. Clone and build
git clone https://gitlab.com/rhobimd-oss/shebe.git
cd shebe
make mcp-build
make mcp-install

# 2. Configure Claude Code
cat > ~/.claude/mcp.json <<'EOF'
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {"SHEBE_DATA_DIR": "$HOME/.local/state/shebe"}
    }
  }
}
EOF

# 3. Restart Claude Code and start searching!
```

### Common Claude Code Commands

```
"Index ~/projects/myapp as session 'myapp'"
"Search for 'authentication' in myapp"
"List all Python files in myapp"
"Find files matching *.yaml in myapp"
"Show me 20 lines around that result"
"Delete the old-session session"
```

---

## Performance Expectations

Based on comprehensive testing:

| Operation                        | Performance  | Notes                             |
|----------------------------------|--------------|-----------------------------------|
| Indexing (small repo <1k files)  | <1s          | Very fast                         |
| Indexing (medium repo ~6k files) | 3-4s         | OpenEMR: 3.3s                     |
| Indexing (large repo ~10k files) | 5-10s        | Istio: 0.5s                       |
| Search query                     | 2ms          | Consistent across all query types |
| File discovery                   | <10ms        | list_dir, find_file               |
| Context preview                  | <50ms        | preview_chunk                     |

See [docs/Performance.md](./docs/Performance.md) for detailed benchmarks.

---

## Next Steps

1. **Index your first repository** - Try with a small project
2. **Explore MCP tools** - See [docs/guides/mcp-tools-reference.md](./docs/guides/mcp-tools-reference.md)
3. **Read architecture docs** - Understand how it works: [ARCHITECTURE.md](./ARCHITECTURE.md)
4. **Join development** - See [CLAUDE.md](./CLAUDE.md) for contribution guide

---

## Additional Resources

- **[README.md](./README.md)** - Project overview
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Developer's guide to the codebase
- **[docs/Performance.md](./docs/Performance.md)** - Performance benchmarks
- **[docs/CONTEXT.md](./docs/CONTEXT.md)** - Project status and roadmap

---

**Last Updated:** 2025-10-28
**Version:** 0.3.0
**Status:** Production Ready (11 MCP tools, 384 tests passing)
**Focus:** MCP Server for Claude Code Integration
