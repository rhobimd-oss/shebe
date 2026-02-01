# Shebe Installation Guide

**Version:** 0.5.7
**Last Updated:** 2026-02-01

---

## Homebrew (macOS and Linux)

The recommended install method. Installs both `shebe` (CLI) and
`shebe-mcp` (MCP server) binaries.

```bash
brew tap rhobimd-oss/shebe-releases \
  https://github.com/rhobimd-oss/shebe-releases
brew install shebe
```

Upgrade after a new release:

```bash
brew update && brew upgrade shebe
```

---

## Manual Download (GitHub)

Pre-built binaries from GitHub releases:

```bash
export SHEBE_VERSION=0.5.7
curl -LO "https://github.com/rhobimd-oss/shebe-releases/releases/download/v${SHEBE_VERSION}/shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz"
tar -xzf shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/
```

See [GitHub Releases](https://github.com/rhobimd-oss/shebe-releases/releases)
for all platforms.

---

## Manual Download (GitLab)

Pre-built binaries from the GitLab package registry:

```bash
export SHEBE_VERSION=0.5.7
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VERSION}/shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz"
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VERSION}/shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256"

sha256sum -c shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256
tar -xzf shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/
```

See [GitLab Releases](https://gitlab.com/rhobimd-oss/shebe/-/releases)
for all platforms.

---

## Editor Extensions

### Zed

Search for "Shebe" in Zed's extension panel, or add to
`.zed/settings.json`:

```json
{
  "context_servers": {
    "shebe-mcp": {
      "command": {
        "path": "shebe-mcp"
      }
    }
  }
}
```

### Other Editors

See the [shebe-releases](https://github.com/rhobimd-oss/shebe-releases)
repository for VS Code and other editor extensions.

---

## Available Binaries

| Platform | Architecture           | Status    |
|----------|------------------------|-----------|
| Linux    | x86_64 (glibc)         | Available |
| Linux    | x86_64-musl (static)   | Available |
| macOS    | ARM (Apple Silicon)    | Available |
| macOS    | x86_64 (Intel)         | Available |
| Linux    | aarch64 (ARM64)        | Planned   |

---

## Building from Source

Required for macOS, Windows, or the latest development version.

### Prerequisites

- **Rust:** 1.88+ ([rustup.rs](https://rustup.rs/))
- **Git:** For cloning the repository
- **Docker:** For containerized builds (recommended)

### Build Steps

```bash
# Clone repository
git clone https://gitlab.com/rhobimd-oss/shebe.git
cd shebe

# Build release binaries (uses Docker for consistency)
make mcp-build

# Install to /usr/local/bin/ (or ~/.local/bin/)
make mcp-install

# Verify installation
which shebe-mcp
shebe --version
```

### Alternative: Direct Cargo Build

If you prefer building without Docker:

```bash
cd services/shebe-server
cargo build --release

# Binaries at: target/release/shebe and target/release/shebe-mcp
sudo cp target/release/shebe target/release/shebe-mcp /usr/local/bin/
```

---

## Verification

Test that the installation works:

```bash
# Test CLI
shebe show-config

# Test MCP server (sends initialize request)
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | shebe-mcp | head -1
```

Expected: JSON response with server capabilities and 14 MCP tools.

---

## Next Steps

After installation, configure Shebe for your use case:

| Task | Documentation |
|------|---------------|
| Configure Claude Code integration | [docs/guides/mcp-quick-start.md](./docs/guides/mcp-quick-start.md) |
| Full MCP setup guide | [docs/guides/mcp-setup-guide.md](./docs/guides/mcp-setup-guide.md) |
| Configuration options | [CONFIGURATION.md](./CONFIGURATION.md) |
| CLI usage | [docs/guides/cli-usage.md](./docs/guides/cli-usage.md) |
| MCP tools reference | [docs/guides/mcp-tools-reference.md](./docs/guides/mcp-tools-reference.md) |
| Performance benchmarks | [docs/Performance.md](./docs/Performance.md) |

---

## Troubleshooting

### Binary not found

```bash
# Check PATH includes install location
echo $PATH | tr ':' '\n' | grep -E '(local/bin|usr/bin)'

# Use absolute path if needed
/usr/local/bin/shebe-mcp --help
```

### Permission denied

```bash
# Ensure binary is executable
chmod +x /usr/local/bin/shebe /usr/local/bin/shebe-mcp
```

### Build fails

```bash
# Ensure Rust 1.88+
rustc --version

# Update Rust if needed
rustup update stable
```

For MCP-specific issues, see [docs/guides/mcp-setup-guide.md](./docs/guides/mcp-setup-guide.md#troubleshooting).

---

## Related Documentation

- [README.md](./README.md) - Project overview
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture
- [CONFIGURATION.md](./CONFIGURATION.md) - All configuration options
- [CHANGELOG.md](./CHANGELOG.md) - Version history
