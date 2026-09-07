# Shebe Installation Guide

**Version:** 0.6.0 (latest release)
**Last Updated:** 2026-07-13

---

## Homebrew (macOS and Linux)

The recommended install method. Installs both `shebe` (CLI) and
`shebe-mcp` (MCP server) binaries.

```bash
brew tap shebe-oss/tap
brew install shebe
```

See the [homebrew-tap repository](https://github.com/shebe-oss/homebrew-tap) for
supported platforms, bottle availability and troubleshooting.

### Upgrade

```bash
brew update && brew upgrade shebe
```

### Uninstall

```bash
# Remove shebe binaries
brew uninstall shebe

# Optional: remove the tap if no longer needed
brew untap shebe-oss/tap
```

---

## Manual Download (GitHub)

Pre-built binaries from GitHub releases:

```bash
export SHEBE_VERSION=v0.6.0
curl -LO "https://github.com/shebe-oss/shebe/releases/download/${SHEBE_VERSION}/shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz"
curl -LO "https://github.com/shebe-oss/shebe/releases/download/${SHEBE_VERSION}/shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256"

sha256sum -c shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256
tar -xzf shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/
```

See [GitHub Releases](https://github.com/shebe-oss/shebe/releases)
for all platforms.

---

## Manual Download (GitLab)

Pre-built binaries from the GitLab package registry:

```bash
export SHEBE_VERSION=v0.6.0
curl -LO "https://gitlab.com/shebe-oss/shebe/-/releases/${SHEBE_VERSION}/downloads/shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz"
curl -LO "https://gitlab.com/shebe-oss/shebe/-/releases/${SHEBE_VERSION}/downloads/shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256"

sha256sum -c shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz.sha256
tar -xzf shebe-${SHEBE_VERSION}-linux-x86_64.tar.gz
sudo mv shebe shebe-mcp /usr/local/bin/
```

See [GitLab Releases](https://gitlab.com/shebe-oss/shebe/-/releases)
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

See the [shebe-oss GitHub organization](https://github.com/shebe-oss)
for VS Code and other editor extensions.

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
git clone https://gitlab.com/shebe-oss/shebe.git
cd shebe

# Build release binaries (uses Docker for consistency)
make shebe-build

# Install to /usr/local/lib/ with symlinks in /usr/local/bin/
make shebe-install

# Optional: install the config template to ~/.config/shebe/
make shebe-install-config

# Verify installation
which shebe-mcp
shebe --version
```

To remove binaries installed this way, run `make shebe-uninstall`.

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

From a source checkout, `make shebe-test` runs the same MCP initialize check.

---

## Next Steps

After installation, configure Shebe for your use case:

| Task                   | Documentation                                                      |
|------------------------|--------------------------------------------------------------------|
| MCP quickstart         | [docs/guides/mcp-quick-start.md](./docs/guides/mcp-quick-start.md) |
| Full MCP setup guide   | [docs/guides/mcp-setup-guide.md](./docs/guides/mcp-setup-guide.md) |
| Configuration options  | [CONFIGURATION.md](./CONFIGURATION.md)                             |
| CLI usage              | [docs/guides/cli-usage.md](./docs/guides/cli-usage.md)             |
| MCP tools reference    | [docs/guides/mcp-tools-reference.md](./docs/guides/mcp-tools-reference.md) |
| Performance benchmarks | [docs/Performance.md](./docs/Performance.md)                       |

---

## Troubleshooting

### Binary not found

```bash
# Check PATH includes install location
echo $PATH | tr ':' '\n' | grep -E '(local/bin|usr/bin)'

# Use absolute path if needed
/usr/local/bin/shebe --help
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
