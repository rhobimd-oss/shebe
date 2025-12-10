# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline for automated releases (`scripts/ci-build.sh`, `scripts/ci-release.sh`)
- GitLab release automation using CI_JOB_TOKEN

### Changed
- Reorganized codebase into `core/`, `http/`, `mcp/` top-level modules
- Created unified `Services` struct replacing duplicate `ShebeServices` and `AppState`
- Renamed `src/api/` to `src/http/` for consistency with adapter naming
- Restructured test directory to mirror source layout (`tests/core/`, `tests/http/`, `tests/mcp/`)
- Test count: 392 -> 285 (consolidated duplicate test modules)

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

[Unreleased]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.4.1...HEAD
[0.4.1]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.4.0...v0.4.1
[0.4.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.2.0...v0.3.0
[0.2.0]: https://gitlab.com/rhobimd-oss/shebe/-/compare/v0.1.0...v0.2.0
[0.1.0]: https://gitlab.com/rhobimd-oss/shebe/-/tags/v0.1.0
