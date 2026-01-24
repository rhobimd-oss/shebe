# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.6] - 2026-01-24

### Added
- MCP Registry publication support (work plan 019 phase 6)
  - `publish:mcp-registry` CI job with manual trigger for controlled releases
  - DNS-based authentication via `rci mcpb publish`
  - Validates server.json and verifies publication via registry API

### Changed
- CI/CD pipeline migrated to rci tool v0.1.3-rc6 (replaces bash scripts)
  - `rci build` replaces ci-build.sh
  - `rci release gitlab` replaces ci-release.sh
  - `rci release github` replaces ci-github-release.sh
  - `rci mcpb create` replaces ci-mcpb.sh
  - `rci mcpb publish` replaces ci-mcpb-publish.sh
- MCPB filename pattern: `shebe-v{version}.mcpb` -> `shebe-mcp-v{version}.mcpb`
  - Required by MCP Registry URL validation (filename must contain 'mcp')
- Added `filepath` to MCPB asset link for direct download URL pattern
- Manual trigger for `release:shebe` and `publish:mcp-registry` jobs
- Moved `ci-github-build.sh` to `deploy/` directory
- Updated docs/CICD.md to document rci tool usage (v2.0)

### Removed
- `scripts/` directory (~2,200 LOC of bash scripts)
  - ci-build.sh, ci-release.sh, ci-mcpb.sh, ci-mcpb-publish.sh, ci-github-release.sh
  - generate-commit-message.sh (moved to cicd repo)
  - Analysis scripts (analyze-claude-errors.py, analyze-serena-context.py)
- `make commit` target (commit message generation handled by cicd repo)

## [0.5.5] - 2026-01-17

### Added
- Comprehensive CLI usage guide (`docs/guides/cli-usage.md`, 424 lines)
- Release documentation (RELEASE_NOTES.md, RELEASE_CHANGELOG.md)

### Changed
- Updated ARCHITECTURE.md with current tool and feature inventory (14 MCP tools, 10 CLI commands)
- Updated README.md with accurate feature list and positioning
- Updated INSTALLATION.md with clearer setup instructions
- Updated all user-facing guides (mcp-quick-start, mcp-setup-guide, mcp-tools-reference)
- Updated CI/CD release script with preview mode support
- Updated .gitignore for release artifacts

## [0.5.4] - 2026-01-16

### Added
- `shebe` CLI binary for standalone code search operations
- 10 CLI commands: index-repository, search-code, find-references, list-sessions,
  get-session-info, delete-session, reindex-session, show-config, get-server-info, completions
- Human-readable and JSON output modes (`--format json`)
- Shell completion support (bash, zsh, fish, PowerShell, elvish)
- Colored terminal output (respects NO_COLOR environment variable)

### Changed
- Architecture: Two binaries sharing `core/` module (shebe-mcp, shebe)
- Binary count: 1 -> 2 (added shebe CLI)
- Added clap 4, clap_complete and colored dependencies

## [0.5.3] - 2025-12-31

### Removed
- HTTP API and REST server (`src/http/` module, `src/main.rs`)
- HTTP dependencies (axum, tower, tower-http)
- ServerConfig (host, port settings no longer needed)
- 12 HTTP-related tests

### Changed
- Architecture: MCP-only (shebe-mcp is now the sole binary)
- Documentation updated to reflect MCP-only design
- Test count: 409 -> 397 tests (HTTP tests removed)
- Binary count: 2 -> 1 (shebe HTTP binary removed)

### Fixed
- Cleaned up stale HTTP references in source code comments

## [0.5.2] - 2025-12-30

### Added
- C++ benchmark documentation for Eigen codebase comparison
- Increased max_k limit from 100 to 500 for thorough searches

### Changed
- Test count: 392 -> 397 tests

## [0.5.1] - 2025-12-11

### Added
- `find_references` MCP tool for token-efficient symbol discovery
  - Identifies all references to a symbol across indexed codebase (~50-70 tokens per reference)
  - Confidence scoring (high/medium/low) based on pattern matching
  - Language-aware patterns for Rust, Go, Python, TypeScript, JavaScript
  - Reference type classification (definition, call, import, type_annotation)
  - Session freshness warnings for stale indexes
- Helper functions for byte-to-line conversion and context extraction

### Changed
- Tool count: 13 -> 14 MCP tools
- Test count: 350 -> 392 tests

## [0.5.0] - 2025-12-11

### Added
- CI/CD pipeline for automated releases (`scripts/ci-build.sh`, `scripts/ci-release.sh`)
- GitLab release automation using CI_JOB_TOKEN
- `upgrade_session` MCP tool for one-command schema migration
- Query preprocessing module with auto-escaping for Tantivy compatibility
  - Curly braces: `{id}` -> `\{id\}`
  - URL patterns: `/users/{id}` -> `"/users/\{id\}"`
  - Multi-colon identifiers: `pkg:scope:name` -> `"pkg:scope:name"`
- Literal search mode for `search_code` (escapes all special characters)
- Field validation with suggestions (`file` -> `file_path`, `code` -> `content`)
- Schema version and last_indexed_at in `list_sessions` output

### Changed
- Reorganized codebase into `core/`, `http/`, `mcp/` top-level modules
- Created unified `Services` struct replacing duplicate `ShebeServices` and `AppState`
- Renamed `src/api/` to `src/http/` for consistency with adapter naming
- Restructured test directory to mirror source layout (`tests/core/`, `tests/http/`, `tests/mcp/`)
- Default `force=true` for `index_repository` (always re-indexes, eliminates session-exists errors)
- Improved schema mismatch error messages (includes repository path and upgrade instructions)
- Enhanced session-exists error with metadata (file count, schema version, timestamp)
- Build optimization: clippy `--no-deps` flag (only lint shebe code)
- Docker dev container now caches build artifacts via cargo-target volume
- Tool count: 12 -> 13 MCP tools
- Test count: 285 -> 350 tests

## [0.4.1] - 2025-10-28

### Added
- `reindex_session` MCP tool for automated re-indexing with stored repository_path
- Configuration override support (chunk_size, overlap) with validation
- Force flag to bypass config-unchanged check for schema migrations

### Changed
- Tool count: 11 -> 12 MCP tools
- Test count: 384 -> 392 tests

## [0.4.0] - 2025-10-25

### Added
- `read_file` MCP tool - Read file contents from indexed sessions
- `delete_session` MCP tool - Delete sessions via MCP
- `list_dir` MCP tool - List all files in session with sorting options
- `find_file` MCP tool - Pattern-based file search with glob/regex support
- `preview_chunk` MCP tool - Show N lines before/after search result chunk
- Performance documentation (`docs/Performance.md`)
- Testing documentation (`docs/Testing.md`)

### Changed
- Tool count: 6 -> 11 MCP tools (83% increase)
- Test count: 332 -> 364 tests
- Documentation restructured following awesome-architecture-md guidelines
- README.md reduced from 286 to 154 lines
- ARCHITECTURE.md reduced from 1,219 to 361 lines (70% reduction)

## [0.3.0] - 2025-10-23

### Added
- `index_repository` MCP tool - Direct repository indexing without HTTP server
- `get_server_info` MCP tool - Server capabilities and version info
- `get_config` MCP tool - Current configuration retrieval
- Synchronous indexing with immediate metadata updates

### Removed
- `index_status` tool (replaced by synchronous indexing)
- Complex async progress tracking (~1,000 LOC removed, 86% reduction)

### Fixed
- Critical metadata bug - files_indexed and chunks_created now correct

### Changed
- Tool count: 3 -> 6 MCP tools
- Test count: 276 -> 332 tests
- Simplified indexing architecture (synchronous execution)

## [0.2.0] - 2025-10-21

### Added
- MCP (Model Context Protocol) server integration
- `shebe-mcp` binary for Claude Code integration
- `search_code` MCP tool - BM25 search with Markdown formatting
- `list_sessions` MCP tool - Session metadata listing
- `get_session_info` MCP tool - Detailed session statistics
- Stdio transport for Claude Code communication
- McpToolHandler trait and ToolRegistry for dynamic tool registration
- Language detection for 30+ programming languages
- Complete ShebeError to McpError mapping

### Changed
- Test count: 130 -> 276 tests (83 MCP-specific)

### Performance
- p95 latency: 8ms (25x better than 200ms target)
- Query latency: 1.7ms avg (29x better than 50ms target)

## [0.1.0] - 2025-10-21

### Added
- Core RAG service architecture using BM25 full-text search via Tantivy
- UTF-8 safe chunker (character-based, handles emojis and multi-byte chars)
- FileWalker with glob pattern matching (include/exclude filters)
- IndexingPipeline orchestration (walk -> read -> chunk)
- Tantivy storage layer with 7-field schema
- StorageManager with session CRUD operations
- SearchService with BM25 ranking and relevance scoring
- REST API with Axum (5 endpoints: health, index, search, list/delete sessions)
- Configuration management (TOML + environment variables)
- Docker deployment (multi-stage build, 97.3MB image)
- Makefile with 15+ automation targets

### Performance
- Indexing throughput: 570 files/sec (14% above 500/sec target)
- UTF-8 safety: Zero panics on complex Unicode (emoji, CJK, RTL text)

### Testing
- 130 total tests (79 unit, 7 integration, 37 UTF-8, 3 doc)
- OpenEMR validation: 4,210 files indexed successfully

[Unreleased]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.6...HEAD
[0.5.6]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.5...v0.5.6
[0.5.5]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.4...v0.5.5
[0.5.4]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.3...v0.5.4
[0.5.3]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.2...v0.5.3
[0.5.2]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.1...v0.5.2
[0.5.1]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.5.0...v0.5.1
[0.5.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.4.1...v0.5.0
[0.4.1]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.4.0...v0.4.1
[0.4.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.2.0...v0.3.0
[0.2.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.1.0...v0.2.0
[0.1.0]: https://gitlab.com/rhobimd-oss/shebe/-/tags/v0.1.0
