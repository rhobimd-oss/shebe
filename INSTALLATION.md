# Shebe Installation Guide

**Version:** 0.5.4
**Last Updated:** 2026-01-17
**Focus:** MCP Server (shebe-mcp) for Claude Code Integration

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Install (Pre-built Binary)](#quick-install-pre-built-binary)
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
- 14 MCP tools for code search, file discovery and session management
- Direct repository indexing from Claude Code conversations
- 2ms search latency (validated on 5-6k file repositories)
- Support for 11+ file types in polyglot codebases
- 397 tests passing (100% success rate)

**No HTTP server required** - shebe-mcp operates independently with filesystem storage.

---

## Quick Install (Pre-built Binary)

The fastest way to get started. Pre-built binaries are available for Linux x86_64.

### Download and Install

```bash
# Download the latest release (v0.5.5)
export SHEBE_VESRION=0.5.5
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VESRION}/shebe-v${SHEBE_VESRION}-linux-x86_64.tar.gz"
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VESRION}/shebe-v${SHEBE_VESRION}-linux-x86_64.tar.gz.sha256"
sha256sum -c shebe-v${SHEBE_VESRION}-linux-x86_64.tar.gz.sha256

# Extract
tar -xzf shebe-v${SHEBE_VESRION}-linux-x86_64.tar.gz

# Install to PATH (choose one)
sudo mv shebe shebe-mcp /usr/local/bin/          # System-wide

# View default shebe config
shebe show-config
```


### Available Binaries

| Platform  | Architecture            | Status            |
|-----------|-------------------------|-------------------|
| Linux     | x86_64 (amd64)          | Available         |
| Linux     | aarch64 (ARM64)         | Planned           |
| macOS     | x86_64                  | Build from source |
| macOS     | aarch64 (Apple Silicon) | Build from source |
| Windows   | x86_64                  | Build from source |

See [All Releases](https://gitlab.com/rhobimd-oss/shebe/-/releases) for the latest binaries.

---

## Building from Source

Required for macOS, Windows, or if you want the latest development version.

### Prerequisites

- **Rust:** 1.88 or later ([install from rustup.rs](https://rustup.rs/))
- **Git:** For cloning the repository
- **Claude Code:** Desktop app with MCP support
- **Platform:** Linux, macOS, or Windows

### 1. Clone Repository

```bash
git clone https://gitlab.com/rhobimd-oss/shebe.git
cd shebe
```

### 2. Build MCP Binary

```bash
# Build release binary (optimized)
make mcp-build

# Binary created at: services/shebe-server/build/release/shebe-mcp 
# `make mcp-build` outputs to `build/` directory and not the standard `target/` 
# because `target/` is a cached Docker volume for faster incremental builds.
```

### 3. Install

Install to your local bin directory (no sudo required):

```bash
# Install shebe-mcp to ~/.local/bin/ or /usr/local/bin/
make mcp-install

# Verify installation
which shebe-mcp
# Expected output: /usr/local/bin/shebe-mcp (or ~/.local/bin/shebe-mcp)

# run test
make mcp-test
```

### 4: Install Configuration File

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

**Find references before renaming:**
```
"Find all references to handleLogin in myapp-main"
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

# Expected: JSON response with server info and 14 MCP tools
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

### Issue: Schema Version Mismatch

**Solution:**
```bash
# Use the upgrade_session tool in Claude Code:
"Upgrade the old-project session to the current schema"

# Or re-index the repository:
"Re-index ~/projects/myapp as myapp-main with force"
```

### Common Error Messages

| Error                 | Meaning                 | Solution                          |
|-----------------------|-------------------------|-----------------------------------|
| "Session not found"   | Session doesn't exist   | Index repository first            |
| "Path does not exist" | Invalid repo path       | Check path is absolute and exists |
| "Permission denied"   | Can't write to data dir | Check directory permissions       |
| "Schema error"        | Old index format        | Use `upgrade_session` or re-index |

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

**Option A: Pre-built Binary (Linux x86_64)**
```bash
# Download, extract, install
curl -L -o shebe.tar.gz \
  "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/0.5.4/shebe-v0.5.4-linux-x86_64.tar.gz"
tar -xzf shebe.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/
```

**Option B: Build from Source**
```bash
git clone https://gitlab.com/rhobimd-oss/shebe.git
cd shebe
make mcp-build
make mcp-install
```

**Configure Claude Code**
```bash
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
```

Restart Claude Code and start searching!

### Common Claude Code Commands

```
"Index ~/projects/myapp as session 'myapp'"
"Search for 'authentication' in myapp"
"Find references to handleLogin in myapp"
"List all Python files in myapp"
"Find files matching *.yaml in myapp"
"Show me 20 lines around that result"
"Delete the old-session session"
"Upgrade myapp session to current schema"
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
4. **Join development** - See [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guide

---

## Additional Resources

- **[README.md](./README.md)** - Project overview
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Developer's guide to the codebase
- **[docs/Performance.md](./docs/Performance.md)** - Performance benchmarks
- **[CHANGELOG.md](./CHANGELOG.md)** - Version history and release notes
- **[All Releases](https://gitlab.com/rhobimd-oss/shebe/-/releases)** - Download pre-built binaries

---

**Last Updated:** 2026-01-17
**Version:** 0.5.4
**Status:** Production Ready (14 MCP tools, 397 tests passing)
**Focus:** MCP Server for Claude Code Integration
