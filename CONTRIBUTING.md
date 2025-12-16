# Contributing to Shebe

Thank you for your interest in contributing to Shebe! This document provides guidelines and information for contributors.

---

## Quick Start

1. **Fork the repository** on GitLab
2. **Clone your fork:**
   ```bash
   git clone https://gitlab.com/YOUR_USERNAME/shebe.git
   cd shebe
   ```
3. **Create a feature branch:**
   ```bash
   git checkout -b feat/your-feature-name
   ```
4. **Make your changes** and test
5. **Submit a merge request**

---

## Development Setup

### Prerequisites

- **Rust:** 1.88+ (latest stable recommended)
- **Cargo:** Comes with Rust
- **Git:** For version control

### Building from Source

```bash
# Navigate to the Rust service directory
cd services/shebe-server/

# Build the project
cargo build

# Run tests (392 tests must pass)
cargo test

# Format code
cargo fmt

# Lint code (zero warnings required)
cargo clippy
```

---

## Project Structure

```
shebe/
├── services/shebe-server/     # Main Rust service
│   ├── src/                   # Source code
│   ├── tests/                 # Tests
│   └── Cargo.toml             # Dependencies
├── docs/                      # Documentation
├── deploy/                    # Deployment configs
└── scripts/                   # Utility scripts
```

**Key Directories:**
- `services/shebe-server/src/` - All Rust source code
- `services/shebe-server/tests/` - Integration and unit tests
- `docs/` - Project documentation

---

## Code Style

### Line Length
- Maximum 120 characters per line
- Exception: URLs, import paths, syntax requirements

### Formatting
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings

### Testing
- All 392 tests must pass before submitting MR
- Add tests for new features
- Minimum 85% line coverage required

---

## Testing

```bash
cd services/shebe-server/

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Check coverage (requires cargo-llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --summary-only
```

---

## Git Workflow

### Branch Naming

- `feat/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/fixes
- `chore/` - Maintenance tasks

### Commit Messages

Follow Angular commit message format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Example:**
```
feat(mcp): add preview_chunk tool for context display

Changes:
Implements preview_chunk MCP tool to show N lines before and after
search result chunks with visual boundaries and line numbers.

- Configurable context lines (default: 10, max: 100)
- Retrieves chunk metadata from Tantivy
- Converts byte offsets to line numbers

Contributes-to: rhobimd-oss/shebe

Signed-off-by: OSS Contributor <oss.contributor@example.org>

```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation
- `refactor` - Code refactoring
- `test` - Test updates
- `chore` - Maintenance

---

## Merge Request Guidelines

### Before Submitting

- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Coverage meets minimum 85%
- [ ] Documentation updated if needed
- [ ] CHANGELOG.md updated (if applicable)

### MR Description Template

```markdown
## Summary
Brief description of changes

## Changes
- List of specific changes
- Include any breaking changes

## Testing
- How you tested these changes
- Any new tests added

## Related Issues
Closes #123
```

---

## Documentation

### Code Documentation

- Add doc comments to public APIs
- Use `///` for item documentation
- Use `//!` for module documentation
- Include examples in doc comments

**Example:**
```rust
/// Searches the index for the given query.
///
/// # Arguments
///
/// * `query` - The search query string
/// * `session` - The session identifier
/// * `k` - Maximum number of results
///
/// # Returns
///
/// A `Result` containing matching search results or an error
///
/// # Example
///
/// ```
/// let results = service.search("function", "my-session", 10)?;
/// ```
pub fn search(&self, query: &str, session: &str, k: usize) -> Result<Vec<SearchResult>> {
    // Implementation
}
```

### Architecture Documentation

- Update ARCHITECTURE.md for architectural changes
- Update docs/Performance.md for performance impacts
- Keep examples in sync with code

---

## Adding New Features

### MCP Tools

See existing tools in `src/mcp/tools/` as examples:

1. Create new tool file: `src/mcp/tools/your_tool.rs`
2. Implement `McpToolHandler` trait
3. Add to `src/mcp/tools/mod.rs`
4. Register in `src/mcp/handlers.rs`
5. Update `get_server_info.rs` with tool description
6. Add comprehensive tests

### REST API Endpoints

See `src/api/handlers.rs` for patterns:

1. Add handler function
2. Add route in router
3. Update API documentation
4. Add integration tests

---

## Performance Guidelines

- Maintain indexing speed: ~570 files/sec minimum
- Maintain search latency: <5ms p95 for small repos
- No regressions in benchmark tests

Run benchmarks:
```bash
cargo bench
```

---

## Release Process

1. Update `CHANGELOG.md` with changes under `[Unreleased]` section
2. Update version in `services/shebe-server/Cargo.toml`
3. Tag release: `git tag v0.X.Y`
4. Push tag: `git push origin v0.X.Y`
5. GitLab CI extracts `[Unreleased]` from CHANGELOG.md and publishes release

---

## Getting Help

- **Issues:** https://gitlab.com/rhobimd-oss/shebe/-/issues
- **Discussions:** https://gitlab.com/rhobimd-oss/shebe/-/discussions
- **Documentation:** https://gitlab.com/rhobimd-oss/shebe

---

## Code of Conduct

This project adheres to the Contributor Covenant Code of Conduct. By participating, you are expected to uphold this code.

---

## License

By contributing to Shebe, you agree that your contributions will be licensed under the Apache-2.0 License.

---

**Thank you for contributing to Shebe!**
