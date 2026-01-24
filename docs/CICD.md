# Shebe CI/CD Pipeline

**Version:** 2.0
**Updated:** 2026-01-24

This document describes the CI/CD pipeline for building, testing and releasing Shebe.

---

## Overview

Shebe uses a dual-platform CI/CD strategy:

- **GitLab CI:** Primary CI/CD for testing, Linux builds and GitLab releases
- **GitHub Actions:** macOS builds (requires native Apple runners)

All CI/CD automation is handled by the `rci` tool (v0.1.3-rc6), which replaces
the previous bash scripts with a centralized, tested binary.

```
                            Tag Push (v*.*.*)
                                  |
                                  v
+------------------------------------------------------------------+
|                         GitLab CI                                |
|                                                                  |
|  STAGE 1: test                                                   |
|  +-----------+                                                   |
|  | test:shebe|  cargo fmt, clippy, nextest                       |
|  +-----------+                                                   |
|        |                                                         |
|        v                                                         |
|  STAGE 2: build (parallel matrix)                                |
|  +------------------+     +------------------+                    |
|  | build:linux      |     | build:linux      |                    |
|  | (glibc)          |     | (musl + mcpb)    |                    |
|  | rci build        |     | rci build        |                    |
|  +--------+---------+     | rci mcpb create  |                    |
|           |               +--------+---------+                    |
|           +-------+----------------+                              |
|                   |                                               |
|                   v                                               |
|           +---------------+                                       |
|           | build:macos   |---> triggers GitHub Actions           |
|           +-------+-------+                                       |
|                   |                                               |
|                   v                                               |
|  STAGE 3: release (manual)                                       |
|  +---------------+                                                |
|  | release:shebe | rci release gitlab                             |
|  | [play button] | rci release github (draft)                     |
|  +-------+-------+                                                |
|          |                                                        |
|          v                                                        |
|  STAGE 4: publish (manual)                                        |
|  +--------------------+                                           |
|  | publish:mcp-registry | rci mcpb publish                        |
|  | [play button]        |                                         |
|  +--------------------+                                           |
+------------------------------------------------------------------+
                                  |
                    repository_dispatch (from build:macos)
                                  |
                                  v
+------------------------------------------------------------------+
|                       GitHub Actions                             |
|                                                                  |
|  +-------------------+     +-------------------+                  |
|  | build (matrix)    |     | release           |                  |
|  | - x86_64-darwin   | --> | - Upload artifacts|                  |
|  | - aarch64-darwin  |     | - Publish release |                  |
|  +-------------------+     +-------------------+                  |
+------------------------------------------------------------------+
                                  |
                                  v
                  +-------------------------------+
                  | Published Releases            |
                  | GitLab: linux-x86_64, musl    |
                  | GitHub: darwin-x86_64, arm64  |
                  +-------------------------------+
```

---

## rci Tool

All CI/CD automation uses the `rci` tool from `registry.gitlab.com/rhobimd-oss/cicd`.

| Command | Purpose | Previous Script |
|---------|---------|-----------------|
| `rci build` | Build binaries, create tarballs | `scripts/ci-build.sh` |
| `rci mcpb create` | Create MCP Bundle | `scripts/ci-mcpb.sh` |
| `rci release gitlab` | Create GitLab release | `scripts/ci-release.sh` |
| `rci release github` | Create GitHub release | `scripts/ci-github-release.sh` |
| `rci mcpb publish` | Publish to MCP Registry | `scripts/ci-mcpb-publish.sh` |
| `rci cache restore/save` | S3-based build cache | N/A |

The rci tool is pre-installed in the CI Docker images.

---

## Pipeline Stages

### Stage 1: prep

**Job:** `dummy:job`

Placeholder job for merge request pipelines. Ensures pipelines always have at least one job.

| Attribute | Value |
|-----------|-------|
| Trigger | MR pipelines only |
| Duration | < 1 second |
| Purpose | Prevent empty pipeline errors |

---

### Stage 2: test

**Job:** `test:shebe`

Runs on merge requests and main branch pushes when Rust files change.

| Check | Tool | Threshold |
|-------|------|-----------|
| Format | `cargo fmt --check` | No differences |
| Lint | `cargo clippy` | Zero warnings |
| Tests | `cargo nextest` | All passing |

**Triggers:**
- Merge request with changes to `Cargo.toml` or `Cargo.lock`
- Push to main with Rust file changes
- Tag push

---

### Stage 3: build

**Job:** `build:linux`

Builds Linux release binaries using parallel matrix strategy.

| Variant | Image | Output | Use Case |
|---------|-------|--------|----------|
| glibc | rust-debian | `shebe-vX.Y.Z-linux-x86_64.tar.gz` | Standard Linux |
| musl | rust-alpine | `shebe-vX.Y.Z-linux-x86_64-musl.tar.gz` | Alpine, MCPB |

**Commands:**
```bash
rci build --service-dir services/shebe-server --suffix ${ARTIFACT_SUFFIX} --publish-package-registry

# For musl variant only:
rci mcpb create --service-dir services/shebe-server --publish-package-registry
```

**Artifacts:**
- `releases/*.tar.gz` - Binary tarballs
- `releases/*.sha256` - Checksums
- `releases/*.mcpb` - MCP Bundle (musl only)

---

**Job:** `build:macos`

Triggers GitHub Actions to build macOS binaries. Runs after `build:linux`.

**Actions:**
- Extracts version from `Cargo.toml`
- Triggers `repository_dispatch` event on GitHub
- Passes version and ref to GitHub Actions

**Required Variables:**
- `SHEBE_GITHUB_TOKEN` - GitHub Personal Access Token with repo scope
- `SHEBE_GITHUB_REPO` - Target repository (default: `rhobimd-oss/shebe`)

---

### Stage 4: release

**Job:** `release:shebe`

Creates releases on both GitLab and GitHub.

**Commands:**
```bash
rci release gitlab --service-dir services/shebe-server --force
rci release github --service-dir services/shebe-server --force
```

**Features:**
- Extracts changelog from `CHANGELOG.md`
- Uploads artifacts to GitLab Package Registry
- Creates release with asset links
- Creates draft GitHub release with Linux artifacts

---

### Stage 5: publish

**Job:** `publish:mcp-registry` (manual trigger)

Publishes to official MCP Registry.

**Command:**
```bash
rci mcpb publish --from-registry
```

**Required Variables:**
- `MCP_PRIVATE_KEY` - DNS-based authentication key

---

## GitHub Actions Workflow

**File:** `.github/workflows/release-macos.yml`

Builds macOS binaries on native Apple runners.

### Triggers

| Trigger | Source | Use Case |
|---------|--------|----------|
| `repository_dispatch` | GitLab CI | Production releases |
| `workflow_dispatch` | Manual | Testing |

### Build Matrix

| Target | Runner | Architecture |
|--------|--------|--------------|
| `x86_64-apple-darwin` | macos-15 | Intel |
| `aarch64-apple-darwin` | macos-15 | Apple Silicon |

### Jobs

**Job:** `build`

1. Checkout source at ref/tag
2. Install Rust toolchain
3. Cache cargo registry
4. Build and package via `deploy/ci-github-build.sh`
5. Upload as GitHub Actions artifact

**Script:** `deploy/ci-github-build.sh --target <target>`

The script:
- Extracts version from `Cargo.toml`
- Builds release binary for target
- Creates tarball with SHA256 checksum
- Sets GitHub Actions outputs (`version`, `artifact_name`)

**Job:** `release`

1. Checkout source (for Cargo.toml)
2. Extract version from `Cargo.toml`
3. Download build artifacts
4. Upload to GitHub release (create if manual run)
5. Publish release (remove draft status)

---

## CI/CD Variables

### GitLab CI/CD Variables

| Variable | Type | Scope | Description |
|----------|------|-------|-------------|
| `SHEBE_GITHUB_TOKEN` | Masked, Protected | Tags | GitHub PAT with `repo` scope |
| `SHEBE_GITHUB_REPO` | Variable | All | Target GitHub repository |
| `MCP_PRIVATE_KEY` | Masked, Protected | Tags | MCP Registry auth key |
| `SCCACHE_AWS_ACCESS_KEY_ID` | Masked | All | S3 cache credentials |
| `SCCACHE_AWS_SECRET_ACCESS_KEY` | Masked | All | S3 cache credentials |

### Creating GitHub PAT

1. Go to GitHub Settings > Developer settings > Personal access tokens > Fine-grained tokens
2. Create token with:
   - **Repository access:** `rhobimd-oss/shebe`
   - **Permissions:** Contents (read/write), Actions (read/write)
3. Add to GitLab: Settings > CI/CD > Variables
   - Key: `SHEBE_GITHUB_TOKEN`
   - Masked: Yes
   - Protected: Yes

---

## Local Testing

### macOS Build Script

The macOS build script can be tested locally:

```bash
# Preview macOS build (no actual build)
./deploy/ci-github-build.sh --target aarch64-apple-darwin --preview
./deploy/ci-github-build.sh --target x86_64-apple-darwin --preview

# Actual build (requires Rust toolchain)
./deploy/ci-github-build.sh --target aarch64-apple-darwin
```

### rci Commands

The rci tool can be used locally via the development container:

```bash
# Build with rci
make rci-build

# Create MCPB
make rci-mcpb
```

---

## Artifact Naming Convention

```
shebe-{version}-{os}-{arch}[-{variant}].tar.gz
```

| Platform | Artifact Name |
|----------|---------------|
| Linux x86_64 (glibc) | `shebe-v0.6.0-linux-x86_64.tar.gz` |
| Linux x86_64 (musl) | `shebe-v0.6.0-linux-x86_64-musl.tar.gz` |
| macOS Intel | `shebe-v0.6.0-darwin-x86_64.tar.gz` |
| macOS Apple Silicon | `shebe-v0.6.0-darwin-aarch64.tar.gz` |
| MCP Bundle | `shebe-mcp-v0.6.0.mcpb` |

### Tarball Contents

```
shebe-v0.6.0-linux-x86_64/
  shebe-mcp    # MCP server binary
  shebe        # CLI binary
```

---

## Release Flow

### Creating a Release

1. **Update version** in `services/shebe-server/Cargo.toml`
2. **Update CHANGELOG.md** with release notes
3. **Create and push tag:**
   ```bash
   git tag v0.6.0
   git push origin v0.6.0
   git push github v0.6.0  # Mirror to GitHub
   ```

4. **Pipeline executes:**
   - GitLab builds Linux binaries (`rci build`)
   - GitLab creates releases (`rci release gitlab/github`)
   - GitHub Actions builds macOS binaries
   - GitHub Actions publishes final release

5. **Verify releases:**
   - GitLab: https://gitlab.com/rhobimd-oss/shebe/-/releases
   - GitHub: https://github.com/rhobimd-oss/shebe/releases

### Release Checklist

- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG.md updated with [Unreleased] section
- [ ] All tests passing on main
- [ ] Tag created and pushed to both remotes
- [ ] GitLab pipeline completed successfully
- [ ] GitHub Actions completed successfully
- [ ] Both releases contain all platform artifacts

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| GitHub release not created | Missing `SHEBE_GITHUB_TOKEN` | Add variable to GitLab CI/CD |
| macOS build not triggered | PAT missing Actions scope | Regenerate PAT with Actions permission |
| 401 on GitHub API | PAT expired | Regenerate and update variable |
| Empty releases dir | Build job failed | Check build:linux logs |
| Version mismatch | Cargo.toml not updated | Ensure version in Cargo.toml matches tag |

### Debug Commands

```bash
# Check GitLab CI variables
gitlab-ci-lint .gitlab-ci.yml

# Test GitHub API access (requires SHEBE_GITHUB_TOKEN env var)
curl -H "Authorization: Bearer $SHEBE_GITHUB_TOKEN" \
  https://api.github.com/repos/rhobimd-oss/shebe/releases

# Verify tag exists on both remotes
git ls-remote origin --tags v0.6.0
git ls-remote github --tags v0.6.0

# Test version extraction locally
grep '^version' services/shebe-server/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
```

---

## Architecture Decisions

### Why Dual CI/CD?

**Problem:** macOS cannot run in Docker containers (Apple EULA, kernel requirements)

**Solution:** Use GitHub Actions for macOS builds

- GitLab CI: Linux builds, orchestration, GitLab releases
- GitHub Actions: macOS builds only (triggered by GitLab)

### Why Both GitLab and GitHub Releases?

| Platform | Purpose |
|----------|---------|
| GitLab | Primary release, full changelog, package registry |
| GitHub | Zed extension compatibility (`zed::latest_github_release()`) |

### Why musl Builds?

- **Static linking:** No glibc dependency
- **MCPB bundles:** Single portable binary
- **Alpine containers:** Native compatibility

### Why rci Tool?

- **Centralized logic:** Single source of truth for CI/CD operations
- **Tested:** The tool itself has tests, reducing CI script bugs
- **Consistent:** Same behavior across GitLab, local dev, and any CI system
- **Maintainable:** Changes to CI logic happen in one place

---

## References

- [GitLab CI/CD Documentation](https://docs.gitlab.com/ee/ci/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [GitHub REST API: Releases](https://docs.github.com/en/rest/releases)
- [Zed Extension API](https://docs.rs/zed_extension_api)
- [rci Tool](https://gitlab.com/rhobimd-oss/cicd) (internal)
