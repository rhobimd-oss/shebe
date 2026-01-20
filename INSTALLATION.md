# Shebe Installation Guide

**Version:** 0.5.7
**Last Updated:** 2026-01-18

---

## Quick Install (Pre-built Binary)

Pre-built binaries are available for Linux x86_64.

```bash
# Download the latest release
export SHEBE_VERSION=0.5.6-rc3
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VERSION}/shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz"
curl -LO "https://gitlab.com/api/v4/projects/75748935/packages/generic/shebe/${SHEBE_VERSION}/shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256"

# Verify checksum
sha256sum -c shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256

# Extract and install
tar -xzf shebe-v${SHEBE_VERSION}-linux-x86_64.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/

# Verify installation
shebe --version
shebe-mcp --help
```

### Available Binaries

| Platform | Architecture            | Status            |
|----------|-------------------------|-------------------|
| Linux    | x86_64 (glibc)          | Available         |
| Linux    | x86_64-musl (static)    | Available         |
| Linux    | aarch64 (ARM64)         | Planned           |
| macOS    | x86_64 / Apple Silicon  | Build from source |
| Windows  | x86_64                  | Build from source |

See [All Releases](https://gitlab.com/rhobimd-oss/shebe/-/releases) for download links.

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
