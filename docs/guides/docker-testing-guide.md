# Docker Development and Testing Guide

**Purpose:** All Rust development and testing runs in Docker containers for consistency across environments

**Shebe Version:** 0.3.0 <br>
**Document Version:** 2.0 <br>
**Created:** 2025-11-01 <br>

---

## Overview

Shebe uses a Docker-first development workflow with two specialized containers:

- **shebe-dev:** Interactive development container (local build: registry.gitlab.com/rhobimd-oss/cicd/rust:20251101-local)
- **shebe-test:** CI/CD testing container (registry.gitlab.com/rhobimd-oss/cicd/rust:20251031-b1.88-slim)

All `cargo` commands MUST run through the shebe-dev container via Makefile targets. This ensures
consistency with CI/CD and eliminates "works on my machine" issues.

## Quick Start

IMPORTANT: All commands run from repository root. Never run `cargo` commands directly.

### Development Commands

```bash
# Build project
make build                # Debug build
make build-release        # Release build

# Run tests
make test                 # Uses cargo nextest
make test-coverage        # With coverage (requires 85% minimum)

# Code quality
make fmt                  # Format code
make fmt-check            # Check formatting (CI/CD)
make clippy               # Lint code (zero warnings required)
make check                # Quick compilation check

# Interactive development
make shell                # Open bash in shebe-dev container
```

### MCP Binary Commands

```bash
# Build and install shebe-mcp
make mcp-build            # Build release binary
make mcp-install          # Install to /usr/local/lib + symlink
make mcp-install-config   # Install config template to ~/.config/shebe/
make mcp-uninstall        # Remove binary and symlink
make mcp-test             # Test MCP binary with initialize message
```

### Cleaning Up

```bash
make clean                # Remove Docker volumes
```

---

## Development Workflow

### Typical Development Cycle

```bash
# 1. Edit code in services/shebe-server/src/

# 2. Check compilation
make check

# 3. Run tests
make test

# 4. Format and lint
make fmt
make clippy

# 5. Commit (pre-commit hook runs tests automatically)
git add .
git commit -F tmp/001.txt
```

### Working Inside the Container

For interactive development or debugging:

```bash
# Open shell in shebe-dev container
make shell

# Inside container (working dir is /workspace = services/shebe-server)
cargo build
cargo test test_specific_function
cargo test -- --nocapture
cargo add new-crate
exit
```

The container has:
- Working directory: `/workspace` (maps to `services/shebe-server`)
- Cached volumes: cargo registry, git and build artifacts
- Environment: `RUST_BACKTRACE=1`, `CARGO_HOME=/usr/local/cargo`

---

## Docker Compose Configuration

**File:** `deploy/docker-compose.yml`

### shebe-dev (Development Container)

```yaml
shebe-dev:
  image: registry.gitlab.com/rhobimd-oss/cicd/rust:20251101-local
  container_name: shebe-dev
  working_dir: /workspace
  volumes:
    - ../services/shebe-server:/workspace:rw
    - cargo-registry:/usr/local/cargo/registry
    - cargo-git:/usr/local/cargo/git
  environment:
    RUST_BACKTRACE: 1
    CARGO_HOME: /usr/local/cargo
  command: bash
```

Used by all Makefile development targets (build, test, fmt, clippy, check, shell).

### shebe-test (CI/CD Container)

```yaml
shebe-test:
  image: registry.gitlab.com/rhobimd-oss/cicd/rust:20251031-b1.88-slim
  container_name: shebe-test
  working_dir: /workspace/services/shebe-server
  volumes:
    - ../:/workspace:rw
    - cargo-registry:/usr/local/cargo/registry
    - cargo-git:/usr/local/cargo/git
    - cargo-target:/workspace/services/shebe-server/target
  environment:
    RUST_BACKTRACE: 1
    CARGO_HOME: /usr/local/cargo
  command: cargo nextest run --color=always
```

Used by CI/CD pipelines for automated testing.

### Volume Caching

Three Docker volumes cache build artifacts:

- **cargo-registry:** Downloaded crates (~200MB)
- **cargo-git:** Git dependencies
- **cargo-target:** Compiled artifacts (~1GB)

Volumes persist between runs for faster builds.

**Clean Cache:**

```bash
make clean
```

This removes all cached volumes. Next build will re-download dependencies.

---

## CI/CD Integration

The shebe-test container is designed for GitLab CI/CD pipelines.

### GitLab CI Example

```yaml
test:
  image: docker:latest
  services:
    - docker:dind
  script:
    - cd deploy
    - docker compose run --rm shebe-test
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == "main"
```

### Pre-Commit Hook Integration

A git pre-commit hook runs tests automatically before allowing commits:

1. Detects changes to `.rs` or `.toml` files
2. Runs `make test` (uses shebe-dev container)
3. Blocks commit if tests fail (392 tests must pass)
4. Enforces 85% minimum code coverage

**Bypass hook (not recommended):**
```bash
git commit --no-verify -m "message"
```

---

## Performance Characteristics

All commands use the shebe-dev container via Makefile targets.

| Command            | First Run | Cached Run | Use Case                    |
|--------------------|-----------|------------|-----------------------------|
| `make test`        | 30-60s    | 5-10s      | Regular testing (392 tests) |
| `make test-coverage` | 1-2 min | 20-30s     | Coverage analysis (85% min) |
| `make check`       | 10-20s    | 2-5s       | Quick compilation check     |
| `make clippy`      | 15-25s    | 3-7s       | Lint checking               |
| `make build`       | 1-2 min   | 10-20s     | Debug build                 |
| `make build-release` | 2-3 min | 30-60s     | Release build               |

**First run:** Downloads dependencies and builds from scratch
**Cached run:** Uses Docker volumes for registry, git and build artifacts

**Test suite:** 392 tests across 7 categories (unit, integration, API, session, MCP, UTF-8, doc)

---

## Troubleshooting

### Issue: Docker not found

**Error:**
```
Docker not found or docker compose not available
```

**Solution:**
1. Install Docker: https://docs.docker.com/get-docker/
2. Verify installation: `docker --version && docker compose version`

### Issue: Docker daemon not running

**Error:**
```
Cannot connect to the Docker daemon
```

**Solution:**
```bash
# Linux
sudo systemctl start docker
sudo systemctl enable docker

# macOS/Windows
# Start Docker Desktop application
```

### Issue: Permission denied

**Error:**
```
permission denied while trying to connect to Docker daemon
```

**Solution:**
```bash
# Add user to docker group (Linux)
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker ps
```

### Issue: Image not found

**Error:**
```
Error response from daemon: pull access denied for registry.gitlab.com/rhobimd-oss/cicd/rust
```

**Solution:**
The shebe-dev image (registry.gitlab.com/rhobimd-oss/cicd/rust:20251101-local) is a local build.

1. Check if image exists: `docker images | grep rhobimd-oss/cicd/rust`
2. If missing, you may need to build it or use a public Rust image
3. Alternatively, modify `deploy/docker-compose.yml` to use `rust:1.88-slim`

### Issue: Slow builds

**Cause:** Rebuilding dependencies every run

**Solution:**
1. Verify volumes exist: `docker volume ls | grep cargo`
2. Pre-download dependencies:
   ```bash
   make shell
   # Inside container:
   cargo fetch
   exit
   ```
3. Clean and rebuild cache:
   ```bash
   make clean
   make build
   ```

### Issue: Tests fail unexpectedly

**Debugging steps:**
```bash
# 1. Run with verbose output
make shell
cargo test -- --nocapture

# 2. Run specific test
cargo test test_name -- --nocapture

# 3. Enable backtrace
RUST_BACKTRACE=full cargo test

# 4. Check test coverage
cargo tarpaulin --all-features --workspace --out Xml
```

---

## Advanced Usage

### Run Specific Tests

```bash
make shell
# Inside container:
cargo nextest run test_validate_path
# or with cargo test:
cargo test test_validate_path
```

### Run Tests with Output

```bash
make shell
cargo test -- --nocapture
```

### Run Tests by Category

```bash
make shell
# Run only integration tests
cargo nextest run --test '*'
# Run only unit tests
cargo nextest run --lib
```

### Debug Failed Tests

```bash
make shell
RUST_BACKTRACE=full cargo test test_name -- --nocapture
```

### Adding Dependencies

```bash
make shell
# Inside container, add a dependency:
cargo add tokio
cargo add --dev criterion

# Update dependencies
cargo update

# Check dependency tree
cargo tree
```

### Running CI/CD Container Locally

```bash
cd deploy
docker compose run --rm shebe-test

# With custom command
docker compose run --rm shebe-test cargo clippy -- -D warnings
docker compose run --rm shebe-test cargo fmt -- --check
```

---

## Best Practices

1. **Always use Makefile targets:** Never run `cargo` commands directly
2. **Use `make shell` for interactive work:** Adding dependencies, debugging, exploration
3. **Run tests before commit:** Pre-commit hook enforces this (392 tests must pass)
4. **Clean cache periodically:** Run `make clean` if builds behave strangely
5. **Keep containers updated:** shebe-dev and shebe-test images should match CI/CD
6. **Maintain test coverage:** Minimum 85% required (currently 86.76%)
7. **Zero clippy warnings:** All production code must pass clippy with -D warnings

---

## Container Comparison

| Feature           | shebe-dev                              | shebe-test                         |
|-------------------|----------------------------------------|------------------------------------|
| Purpose           | Interactive development                | CI/CD testing                      |
| Image             | rhobimd-oss/cicd/rust:20251101-local   | rhobimd-oss/cicd/rust:20251031-b1.88-slim |
| Working Dir       | /workspace (services/shebe-server)     | /workspace/services/shebe-server   |
| Mount             | Only shebe-server directory            | Entire repository                  |
| Test Runner       | cargo nextest                          | cargo nextest                      |
| Use Cases         | build, test, fmt, clippy, shell        | CI/CD pipeline                     |
| Cached Volumes    | cargo-registry, cargo-git              | cargo-registry, cargo-git, cargo-target |

---

## Related Documentation

- [ARCHITECTURE.md](../../ARCHITECTURE.md) - System architecture and entry points
- [Performance.md](../Performance.md) - Performance benchmarks and analysis
- [mcp-quick-start.md](./mcp-quick-start.md) - MCP tools usage guide

---

---

## Update Log

| Date | Shebe Version | Document Version | Changes |
|------|---------------|------------------|---------|
| 2025-11-01 | 0.3.0 | 2.0 | Complete rewrite for docker-compose workflow |
| 2025-10-22 | 0.2.0 | 1.0 | Initial docker testing guide |
