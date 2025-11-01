# Shebe MCP Tools Reference

Complete API reference for all Shebe MCP tools.

**Version:** 0.7.1
**Protocol:** JSON-RPC 2.0 over stdio
**Format:** Markdown responses

---

## Table of Contents

1. [search_code](#tool-search_code)
2. [list_sessions](#tool-list_sessions)
3. [get_session_info](#tool-get_session_info)
4. [index_repository](#tool-index_repository) **(v0.2.0, simplified in v0.3.0)**
5. [get_server_info](#tool-get_server_info) **(NEW in v0.3.0)**
6. [get_config](#tool-get_config) **(NEW in v0.3.0)**
7. [list_dir](#tool-list_dir) **(NEW in v0.7.0)**
8. [read_file](#tool-read_file) **(NEW in v0.7.0)**
9. [Error Codes](#error-codes)
10. [Performance Characteristics](#performance-characteristics)

---

## Tool: search_code

Search indexed code repositories using BM25 full-text search with
phrase and boolean query support.

### Description

Executes BM25 ranked search across all chunks in a specified session.
Results include code snippets with syntax highlighting, file paths,
chunk metadata, and relevance scores.

### Input Schema

| Parameter | Type    | Required | Default | Constraints       | Description             |
|-----------|---------|----------|---------|-------------------|-------------------------|
| query     | string  | Yes      | -       | 1-500 chars       | Search query            |
| session   | string  | Yes      | -       | ^[a-zA-Z0-9_-]+$  | Session ID              |
| k         | integer | No       | 10      | 1-100             | Max results to return   |

### Query Syntax

**Simple Keywords:**
```
authentication
```
Searches for "authentication" in all indexed code.

**Phrase Queries:**
```
"user authentication function"
```
Searches for exact phrase match (all words in order).

**Boolean Operators:**
```
patient AND authentication
login OR signup
NOT deprecated
patient AND (login OR authentication)
```

Supported operators: `AND`, `OR`, `NOT`
Use parentheses for grouping.

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_code",
    "arguments": {
      "query": "authenticate",
      "session": "openemr-main",
      "k": 10
    }
  }
}
```

### Response Format

```markdown
Found 10 results for query 'authenticate' (42ms):

## Result 1 (score: 12.45)
**File:** `/src/auth/patient_auth.php` (chunk 3, bytes 1024-1536)

```php
function authenticatePatient($username, $password) {
    // Patient authentication logic
    if (empty($username) || strlen($password) < 8) {
        return false;
    }
    return validateCredentials($username, $password);
}
```

## Result 2 (score: 9.32)
**File:** `/src/utils/auth_helpers.php` (chunk 1, bytes 512-1024)

```php
function validateCredentials($user, $pwd) {
    // Credential validation
    return hash_equals(hash('sha256', $pwd), getStoredHash($user));
}
```

...
```

### Response Structure

Each result includes:
- **Score:** BM25 relevance score (higher = more relevant)
- **File Path:** Absolute path to source file
- **Chunk Metadata:** Chunk index and byte offsets
- **Code Snippet:** Actual code with syntax highlighting
- **Language Detection:** Automatic based on file extension

### Performance

**Validated Performance (Production-Scale Codebases):**

**Istio v1.26.0 (5,605 files, 69,904 chunks):**
| Metric | Value | Notes |
|--------|-------|-------|
| Average | **1.7ms** | 7 diverse queries |
| Median | **2ms** | Consistent latency |
| Range | 1-3ms | Minimal variance |
| p95 | **2ms** | 25x better than 50ms target |
| Success Rate | 100% | All queries returned results |

**OpenEMR (6,364 files, 456,992 chunks):**
| Metric | Value | Notes |
|--------|-------|-------|
| Average | 10-80ms | Larger index (191MB vs 49MB) |
| Token Usage | 1,500-3,200 | 40-60% better than alternatives |
| Cold cache | 10ms | No warmup needed |
| Warm cache | 10ms | Minimal difference |

**Performance by Repository Size:**
- Small (<100 files): 1-3ms
- Medium (~1,000 files): 1-5ms
- Large (~5,000-6,000 files): 1-3ms (Istio) or 10-80ms (OpenEMR)
- Very Large (>10,000 files): Target <5ms maintained

**Comparison vs Alternatives:**
- **15.8x faster** than ripgrep (1.7ms vs 27ms avg)
- **4,758x faster** than Serena Pattern Search (1.7ms vs 8,088ms)
- **8,027x faster** than Serena Symbol Search (1.7ms vs 13,646ms)

**Key Insight:** Query complexity has minimal impact on latency. Boolean operators, phrases, and keywords all perform similarly (1-3ms range).

### Error Codes

| Code   | Message               | Cause                        | Solution                   |
|--------|-----------------------|------------------------------|----------------------------|
| -32602 | Invalid params        | Empty query                  | Provide non-empty query    |
| -32602 | Invalid params        | k out of range (1-100)       | Use k between 1 and 100    |
| -32602 | Invalid params        | Query too long (>500 chars)  | Shorten query              |
| -32001 | Session not found     | Invalid session ID           | Use list_sessions to find  |
| -32004 | Search failed         | Query parsing error          | Check query syntax         |
| -32603 | Internal error        | Tantivy error                | Report bug with query      |

### Usage Examples

**Basic keyword search:**
```
You: Search for "database" in openemr-main

Claude: [Executes search_code with query="database", session="openemr-main"]
```

**Phrase search:**
```
You: Find the exact phrase "patient authentication function" in openemr-main

Claude: [Executes search_code with query="\"patient authentication function\""]
```

**Boolean search:**
```
You: Find code with "patient AND (login OR authentication)" in openemr-main

Claude: [Executes search_code with query="patient AND (login OR authentication)"]
```

**Limited results:**
```
You: Show me just the top 3 results for "error handling" in openemr-main

Claude: [Executes search_code with query="error handling", k=3]
```

---

## Tool: list_sessions

List all indexed code sessions with metadata summary.

### Description

Returns a list of all available sessions in the configured
SHEBE_INDEX_DIR with file counts, chunk counts, storage size, and
creation timestamps.

### Input Schema

No parameters required.

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "list_sessions",
    "arguments": {}
  }
}
```

### Response Format

```markdown
Available sessions (3):

## openemr-main
- **Files:** 4,210
- **Chunks:** 12,450
- **Size:** 52.40 MB
- **Created:** 2025-10-20T10:00:00Z

## shebe-dev
- **Files:** 84
- **Chunks:** 256
- **Size:** 1.24 MB
- **Created:** 2025-10-21T08:30:00Z

## test-session
- **Files:** 3
- **Chunks:** 4
- **Size:** 8.57 KB
- **Created:** 2025-10-21T20:17:19Z
```

### Response Fields

- **Files:** Number of source files indexed
- **Chunks:** Total chunks created (depends on chunk_size config)
- **Size:** Total index size on disk (human-readable)
- **Created:** ISO 8601 timestamp of session creation

### Performance

| Metric    | Value   |
|-----------|---------|
| Latency   | <10ms   |
| Memory    | <5MB    |
| I/O       | Minimal |

### Error Codes

| Code   | Message        | Cause                | Solution                     |
|--------|----------------|----------------------|------------------------------|
| -32603 | Internal error | Storage read failure | Check SHEBE_INDEX_DIR perms  |
| -32603 | Internal error | Invalid meta.json    | Re-index affected session    |

### Usage Examples

**List all sessions:**
```
You: What code sessions are available in Shebe?

Claude: [Executes list_sessions]
Available sessions (3): openemr-main, shebe-dev, test-session
```

**Before searching:**
```
You: I want to search my code. What sessions do I have?

Claude: [Executes list_sessions to show available sessions]
```

---

## Tool: get_session_info

Get detailed metadata and statistics for a specific indexed session.

### Description

Returns comprehensive information about a session including overview,
configuration parameters, and computed statistics like average chunks
per file and average chunk size.

### Input Schema

| Parameter | Type   | Required | Constraints      | Description     |
|-----------|--------|----------|------------------|-----------------|
| session   | string | Yes      | ^[a-zA-Z0-9_-]+$ | Session ID      |

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_session_info",
    "arguments": {
      "session": "openemr-main"
    }
  }
}
```

### Response Format

```markdown
# Session: openemr-main

## Overview
- **Status:** Ready
- **Files:** 4,210
- **Chunks:** 12,450
- **Size:** 52.40 MB
- **Created:** 2025-10-20T10:00:00Z

## Configuration
- **Chunk size:** 512 chars
- **Overlap:** 64 chars

## Statistics
- **Avg chunks/file:** 2.96
- **Avg chunk size:** 4.31 KB
```

### Response Fields

**Overview:**
- **Status:** Always "Ready" (future: may include "Indexing", "Error")
- **Files:** Total files indexed
- **Chunks:** Total chunks created
- **Size:** Index size on disk
- **Created:** Session creation timestamp

**Configuration:**
- **Chunk size:** Characters per chunk (set during indexing)
- **Overlap:** Character overlap between chunks

**Statistics:**
- **Avg chunks/file:** Chunks divided by files
- **Avg chunk size:** Total chunk bytes divided by chunk count

### Performance

| Metric  | Value |
|---------|-------|
| Latency | <5ms  |
| Memory  | <5MB  |
| I/O     | 1 read|

### Error Codes

| Code   | Message           | Cause                 | Solution                |
|--------|-------------------|-----------------------|-------------------------|
| -32602 | Invalid params    | Missing session param | Provide session ID      |
| -32001 | Session not found | Invalid session ID    | Use list_sessions first |
| -32603 | Internal error    | Corrupt metadata      | Re-index session        |

### Usage Examples

**Get session details:**
```
You: Tell me about the "openemr-main" session

Claude: [Executes get_session_info with session="openemr-main"]
Shows detailed stats about the session
```

**Before large search:**
```
You: How many files are in my-project session?

Claude: [Executes get_session_info to show file count]
```

---

## Tool: index_repository

**Available since:** v0.2.0 (simplified to synchronous in v0.3.0)

Index a code repository for full-text search directly from Claude Code.
Runs synchronously and returns complete statistics when finished.

### Description

Indexes a repository using FileWalker, Chunker, and Tantivy storage.
The tool runs synchronously, blocking until indexing completes, then
returns actual statistics (files indexed, chunks created, duration).

No progress tracking needed - you get immediate completion feedback.

### Input Schema

| Parameter | Type | Required | Default | Constraints | Description |
|-----------|------|----------|---------|-------------|-------------|
| path | string | Yes | - | Absolute, exists, is dir | Repository path |
| session | string | Yes | - | 1-64 alphanumeric+dash | Session ID |
| include_patterns | array | No | `["**/*"]` | Glob patterns | Files to include |
| exclude_patterns | array | No | [see below] | Glob patterns | Files to exclude |
| chunk_size | integer | No | 512 | 100-2000 | Characters per chunk |
| overlap | integer | No | 64 | 0 to size-1 | Overlap between chunks |
| force | boolean | No | false | - | Force re-indexing |

**Default Exclusions:**
```
**/target/**        # Rust build
**/node_modules/**  # Node.js deps
**/.git/**          # Git metadata
**/dist/**          # Build outputs
**/build/**         # Build dirs
**/*.pyc            # Python bytecode
**/__pycache__/**   # Python cache
```

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "index_repository",
    "arguments": {
      "path": "/home/user/myapp",
      "session": "myapp-main",
      "include_patterns": ["**/*.rs", "**/*.toml"],
      "exclude_patterns": ["**/target/**", "**/tests/**"],
      "chunk_size": 512,
      "overlap": 64,
      "force": false
    }
  }
}
```

### Response Format

```markdown
Indexing complete!

**Session:** myapp-main
**Files indexed:** 448
**Chunks created:** 2,450
**Duration:** 0.8s

You can now search your code with search_code.
```

### Behavior

**Synchronous Execution:**
- Tool blocks until indexing completes
- Returns actual statistics immediately
- No background tasks or progress tracking needed

**Batch Commits:**
- Commits to Tantivy every 100 files
- Reduces I/O overhead for large repositories
- Same throughput as async version (~570 files/sec)

**Error Handling:**
- Continues on file errors (permission, UTF-8, etc.)
- Fails on critical errors (session creation, storage)
- All errors included in completion message

### Performance

**Tested Performance (OpenEMR 6,364 files):**
| Test Run | Duration | Throughput | Files | Notes |
|----------|----------|------------|-------|-------|
| Test 006 (v0.2.0) | 70s | 90.9 files/sec | 6,364 | Original async implementation |
| Test 008 (v0.3.0) | 5.2s | 1,224 files/sec | 6,364 | Synchronous, cold system |
| Test 009 (v0.3.0) | 3.4s | 1,872 files/sec | 6,364 | Synchronous, warm system |

**Performance by Repository Size:**
| Repository Size | Files | Expected Duration | Throughput Range |
|-----------------|-------|-------------------|------------------|
| Small           | <100  | 1-4s              | 1,500-2,000 files/sec |
| Medium          | ~1,000 | 2-4s             | 1,500-2,000 files/sec |
| Large           | ~6,000 | 10-15s           | 1,500-2,000 files/sec |
| Very Large      | ~10,000 | 20-30s          | 1,500-2,000 files/sec |

**Throughput:** 1,500-2,000 files/sec (varies with system load, cache state, I/O performance)

**Key Insights:**
- 20.6x faster than original v0.2.0 implementation
- System cache state affects performance (warm cache = faster indexing)
- Synchronous execution provides accurate statistics immediately
- No background processes or progress tracking needed

### Error Codes

| Code   | Message | Cause | Solution |
|--------|---------|-------|----------|
| -32602 | Invalid params | Path doesn't exist | Check path is correct |
| -32602 | Invalid params | Path not absolute | Use absolute path |
| -32602 | Invalid params | Path not directory | Provide directory path |
| -32602 | Invalid params | Session exists | Use force=true to re-index |
| -32602 | Invalid params | Invalid session name | Use alphanumeric+dash only |
| -32602 | Invalid params | chunk_size out of range | Use 100-2000 |

### Usage Examples

**Basic indexing:**
```
You: Index my Rust project at /home/user/myapp

Claude: [Calls index_repository, waits for completion]
Indexing complete! 448 files, 2,450 chunks in 0.8s
```

**Custom patterns:**
```
You: Index /home/user/myapp but only Python and Rust files, exclude tests

Claude: [Calls with include_patterns=["**/*.py", "**/*.rs"],
         exclude_patterns=["**/tests/**"]]
```

**Re-indexing:**
```
You: Re-index myapp-main with latest code

Claude: [Calls with force=true to overwrite]
```

### Best Practices

1. **Use descriptive session names:** `project-branch` format
2. **Index only needed files:** Use include/exclude patterns
3. **Be patient with large repos:** Indexing 10k+ files may take 30s+
4. **Check completion message:** Review files indexed and any errors
5. **Clean up old sessions:** Use HTTP API to delete unused sessions

---

## Tool: get_server_info

**Available since:** v0.3.0

Get version and build information about the running shebe-mcp server.

### Description

Returns server version, protocol version, Rust version, and a list of available tools.
Use this to verify which version of shebe-mcp is running and check compatibility.

### Input Schema

No parameters required.

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "get_server_info",
    "arguments": {}
  }
}
```

### Response Format

```markdown
# Shebe MCP Server Information

## Version
- **Version:** 0.3.0
- **Rust Version:** 1.88

## Server Details
- **Name:** shebe-mcp
- **Description:** BM25 full-text search MCP server
- **Protocol:** MCP 2024-11-05

## Available Tools
- search_code: Search indexed code
- list_sessions: List all sessions
- get_session_info: Get session details
- index_repository: Index a repository (synchronous)
- get_server_info: Show server version (this tool)
- get_config: Show current configuration
```

### Response Fields

**Version:**
- Server version (semantic versioning)
- Rust compiler version used to build

**Server Details:**
- Server name (shebe-mcp)
- Brief description
- MCP protocol version

**Available Tools:**
- Complete list of all available MCP tools
- Brief description of each tool

### Performance

| Metric  | Value |
|---------|-------|
| Latency | <1ms  |
| Memory  | <1MB  |
| I/O     | None  |

### Error Codes

No tool-specific errors. Uses standard JSON-RPC error codes only.

### Usage Examples

**Check server version:**
```
You: What version of shebe-mcp is running?

Claude: [Executes get_server_info]
Running shebe-mcp v0.3.0 with Rust 1.88
```

**List available tools:**
```
You: What tools are available in Shebe?

Claude: [Executes get_server_info]
Shows 6 available tools with descriptions
```

**Verify compatibility:**
```
You: Is my shebe-mcp version compatible with the latest features?

Claude: [Executes get_server_info to check version]
```

---

## Tool: get_config

**Available since:** v0.3.0

Get the current configuration of the running shebe-mcp server.

### Description

Returns all configuration settings including server, indexing, storage, search,
and limits parameters. Shows both the values currently in use and their sources
(defaults, config file, or environment variables).

### Input Schema

| Parameter | Type    | Required | Default | Description |
|-----------|---------|----------|---------|-------------|
| detailed  | boolean | No       | false   | Show all patterns |

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "get_config",
    "arguments": {
      "detailed": false
    }
  }
}
```

### Response Format

**Basic (detailed=false):**

```markdown
# Shebe MCP Configuration

## Server
- **Host:** 127.0.0.1
- **Port:** 3000
- **Log Level:** info

## Indexing
- **Chunk Size:** 512 chars
- **Overlap:** 64 chars
- **Max File Size:** 10 MB
- **Include Patterns:** 13 patterns
- **Exclude Patterns:** 8 patterns

## Storage
- **Index Directory:** /home/user/.local/state/shebe

## Search
- **Default K:** 10
- **Max K:** 100
- **Max Query Length:** 500

## Limits
- **Max Concurrent Indexes:** 1
- **Request Timeout:** 300s
```

**Detailed (detailed=true):**

Includes all the above plus:

```markdown
## Include Patterns
- `*.rs`
- `*.toml`
- `*.md`
- `*.txt`
- `*.php`
- `*.js`
- `*.ts`
- `*.py`
- `*.go`
- `*.java`
- `*.c`
- `*.cpp`
- `*.h`

## Exclude Patterns
- `**/node_modules/**`
- `**/target/**`
- `**/vendor/**`
- `**/.git/**`
- `**/build/**`
- `**/__pycache__/**`
- `**/dist/**`
- `**/.next/**`
```

### Response Fields

**Server:**
- Host address (bind address for HTTP server)
- Port number
- Logging level

**Indexing:**
- Chunk size (characters per chunk)
- Overlap (characters between chunks)
- Max file size (MB, larger files skipped)
- Include/exclude pattern counts

**Storage:**
- Index directory (where sessions are stored)

**Search:**
- Default K (default result count)
- Max K (maximum allowed results)
- Max query length (character limit)

**Limits:**
- Max concurrent indexes
- Request timeout in seconds

### Performance

| Metric  | Value |
|---------|-------|
| Latency | <1ms  |
| Memory  | <1MB  |
| I/O     | None  |

### Error Codes

No tool-specific errors. Uses standard JSON-RPC error codes only.

### Usage Examples

**Check configuration:**
```
You: What's the current chunk size configuration?

Claude: [Executes get_config]
The chunk size is set to 512 characters with 64 character overlap.
```

**View all patterns:**
```
You: Show me all the file patterns being used for indexing

Claude: [Executes get_config with detailed=true]
Shows all include and exclude patterns
```

**Verify storage location:**
```
You: Where are my indexed sessions stored?

Claude: [Executes get_config]
Sessions are stored in /home/user/.local/state/shebe
```

**Debug configuration:**
```
You: Why aren't my Python files being indexed?

Claude: [Executes get_config with detailed=true]
Checks include/exclude patterns to diagnose issue
```

### Best Practices

1. **Use basic mode for quick checks:** Default is sufficient for most queries
2. **Use detailed mode for debugging:** Shows all patterns when troubleshooting
3. **Verify before indexing:** Check patterns match your repository structure
4. **Document custom configs:** If using custom shebe.toml or env vars

---

## Tool: list_dir

**Available since:** v0.7.0

List all files indexed in a session with automatic truncation for large repositories.

### Description

Returns a list of all indexed files in a session, sorted alphabetically by default. Auto-truncates
to 500 files maximum to stay under the MCP 25k token limit. Shows a clear warning message when
truncation occurs with suggestions for alternative approaches.

### Input Schema

| Parameter | Type    | Required | Default | Constraints | Description |
|-----------|---------|----------|---------|-------------|-------------|
| session   | string  | Yes      | -       | ^[a-zA-Z0-9_-]+$ | Session ID |
| limit     | integer | No       | 100     | 1-500       | Max files to return |
| sort      | string  | No       | "alpha" | alpha/size/indexed | Sort order |

### Auto-Truncation Behavior

**Default Limit:** 100 files (when user doesn't specify `limit`)
**Maximum Limit:** 500 files (enforced even if user requests more)

When a repository has more files than the limit, the tool:
1. Returns only the first N files (sorted alphabetically by default)
2. Shows a clear warning message at the top
3. Provides suggestions for filtering (use `find_file`) or pagination

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "list_dir",
    "arguments": {
      "session": "large-repo",
      "limit": 200,
      "sort": "alpha"
    }
  }
}
```

### Response Format (Without Truncation)

```markdown
**Session:** `small-repo`
**Files:** 50 (showing 50)

| File Path | Chunks |
|-----------|--------|
| `/src/main.rs` | 3 |
| `/src/lib.rs` | 5 |
| `/Cargo.toml` | 1 |
...
```

### Response Format (With Truncation)

```markdown
WARNING: OUTPUT TRUNCATED - MAXIMUM 500 FILES DISPLAYED

Showing: 500 of 5,605 files (first 500, alphabetically sorted)
Reason: Maximum display limit is 500 files (MCP 25k token limit)
Not shown: 5,105 files

💡 SUGGESTIONS:
- Use `find_file` with patterns to filter: find_file(session="large-repo", pattern="*.yaml")
- For pagination support, see: docs/work-plans/011-phase02-mcp-pagination-implementation.md
- For full file list, use bash: find /path/to/repo -type f | sort

---

**Files 1-500 (of 5,605 total):**

| File Path | Chunks |
|-----------|--------|
| `/src/api/auth.rs` | 4 |
| `/src/api/handlers.rs` | 12 |
...
```

### Sort Options

**alpha (default):** Alphabetically by file path
**size:** Largest files first (requires filesystem stat)
**indexed:** Insertion order (order files were indexed)

### Performance

| Metric  | Value   | Notes |
|---------|---------|-------|
| Latency | <50ms   | Small repos (<100 files) |
| Latency | <200ms  | Large repos (5,000+ files) |
| Memory  | <10MB   | Depends on file count |

### Error Codes

| Code   | Message | Cause | Solution |
|--------|---------|-------|----------|
| -32602 | Invalid params | Missing session | Provide session ID |
| -32001 | Session not found | Invalid session | Use list_sessions first |
| -32603 | Internal error | Index read failure | Re-index session |

### Usage Examples

**List files in small repo:**
```
You: List all files in my-project session

Claude: [Executes list_dir with session="my-project"]
Shows all 42 files (no truncation warning)
```

**List files in large repo (truncated):**
```
You: List all files in istio-main session

Claude: [Executes list_dir with session="istio-main"]
WARNING: OUTPUT TRUNCATED - showing 100 of 5,605 files
Suggests using find_file for filtering
```

**Custom limit:**
```
You: Show me the first 250 files in large-repo

Claude: [Executes list_dir with session="large-repo", limit=250]
Shows 250 files with truncation warning (5,605 total)
```

**Sort by size:**
```
You: Show me the largest files in my-project

Claude: [Executes list_dir with session="my-project", sort="size"]
Lists files sorted by size (largest first)
```

### Best Practices

1. **Use find_file for large repos:** Pattern-based filtering is more efficient
2. **Start with default limit:** 100 files is usually enough for exploration
3. **Check the warning:** If truncated, consider filtering approach
4. **Use sort wisely:** `size` sort requires filesystem access (slower)

---

## Tool: read_file

**Available since:** v0.7.0

Read file contents from an indexed session with automatic truncation for large files.

### Description

Retrieves the full contents of a file from an indexed session. Auto-truncates to 20,000
characters maximum to stay under the MCP 25k token limit. Shows a clear warning message
when truncation occurs with the percentage shown and suggestions for alternatives.

### Input Schema

| Parameter | Type   | Required | Constraints | Description |
|-----------|--------|----------|-------------|-------------|
| session   | string | Yes      | ^[a-zA-Z0-9_-]+$ | Session ID |
| file_path | string | Yes      | Absolute path | Path to file (from search results) |

### Auto-Truncation Behavior

**Maximum Characters:** 20,000 (approximately 5,000 tokens with 80% safety margin)

When a file exceeds 20,000 characters, the tool:
1. Reads only the first 20,000 characters
2. Ensures UTF-8 character boundary safety (never splits multi-byte characters)
3. Shows a warning with the percentage shown and suggestions
4. Returns valid, syntax-highlighted code

### Request Example

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "tools/call",
  "params": {
    "name": "read_file",
    "arguments": {
      "session": "openemr-main",
      "file_path": "/src/database/migrations/001_initial.sql"
    }
  }
}
```

### Response Format (Without Truncation)

```markdown
**File:** `/src/auth.rs`
**Session:** `my-project`
**Size:** 5.2 KB (120 lines)
**Language:** rust

```rust
use crate::error::AuthError;

pub fn authenticate(username: &str, password: &str) -> Result<Token, AuthError> {
    // Authentication logic here
    validate_credentials(username, password)?;
    generate_token(username)
}
```
```

### Response Format (With Truncation)

```markdown
WARNING: FILE TRUNCATED - SHOWING FIRST 20000 CHARACTERS

Showing: Characters 1-20000 of 634000 total (3.2%)
Reason: Maximum display limit is 20000 characters (MCP 25k token limit)
Not shown: 614000 characters

💡 SUGGESTIONS:
- Use `search_code` to find specific content in this file
- Use `preview_chunk` to view specific sections
- For full file, use bash: cat /path/to/large-file.sql

---

**File:** `/src/database/migrations/001_initial.sql`
**Showing:** First 20000 characters (~280 lines)

```sql
-- Database initialization
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    ...
[Content continues until 20,000 character limit]
```
```

### UTF-8 Safety

The tool ensures UTF-8 character boundary safety when truncating:
- Never splits multi-byte characters (emoji, CJK, Arabic, etc.)
- Uses `ensure_utf8_boundary()` helper function
- Truncates to last valid UTF-8 character if needed
- All 5 UTF-8 safety tests passing

### Performance

| Metric  | Value | Notes |
|---------|-------|-------|
| Latency | <50ms | Small files (<20KB) |
| Latency | <200ms | Large files (>500KB, truncated) |
| Memory  | <5MB  | Maximum for truncated files |

### Error Codes

| Code   | Message | Cause | Solution |
|--------|---------|-------|----------|
| -32602 | Invalid params | Empty file_path | Provide file path |
| -32001 | Session not found | Invalid session | Use list_sessions first |
| -32001 | Invalid request | File not indexed | Check file_path or re-index |
| -32001 | Invalid request | File not found | File deleted since indexing |
| -32001 | Invalid request | Binary file | File contains non-UTF-8 data |

### Usage Examples

**Read small file:**
```
You: Show me the contents of src/main.rs in my-project

Claude: [Executes read_file with session="my-project", file_path="/src/main.rs"]
Shows full file contents with syntax highlighting (no warning)
```

**Read large file (truncated):**
```
You: Show me the database migration file in openemr-main

Claude: [Executes read_file with file_path="/sql/icd9-codes.sql"]
WARNING: FILE TRUNCATED - showing first 20,000 characters (10.4% of 634KB file)
Suggests using search_code to find specific content
```

**UTF-8 handling:**
```
You: Read the file with Chinese comments in my-project

Claude: [Executes read_file]
Handles multi-byte characters safely, no broken characters at truncation point
```

**Binary file error:**
```
You: Read the image file in my-project

Claude: [Executes read_file]
Error: File contains non-UTF-8 data (binary file). Cannot display in MCP response.
```

### Best Practices

1. **Use for small-to-medium files:** Under 20k characters (no truncation)
2. **Use search_code for large files:** Find relevant sections first
3. **Check the warning:** If truncated, use search_code or preview_chunk
4. **For full content:** Use bash tools (cat, less) for files >20k chars
5. **Verify file exists:** Check search results or list_dir before reading

### Comparison with Alternatives

**When to use read_file:**
- File is under 20,000 characters
- You need syntax-highlighted display
- File was found via search_code or list_dir

**When to use alternatives:**
- **search_code:** Find specific content in large files
- **preview_chunk:** View context around search results
- **bash cat:** Read full content of large files without limits
- **bash less:** Interactive viewing of large files

---

## Error Codes

Complete error code reference for all tools.

### Standard JSON-RPC Errors

| Code   | Name          | Description                   |
|--------|---------------|-------------------------------|
| -32700 | Parse error   | Invalid JSON                  |
| -32600 | Invalid req   | Missing required fields       |
| -32601 | Method N/F    | Method not found              |
| -32602 | Invalid params| Parameter validation failed   |
| -32603 | Internal error| Server-side error             |

### Shebe-Specific Errors

| Code   | Name              | Description                      |
|--------|-------------------|----------------------------------|
| -32001 | Session not found | Requested session doesn't exist  |
| -32002 | Index error       | Failed to read index             |
| -32003 | Config error      | Configuration invalid            |
| -32004 | Search failed     | Query parsing or execution error |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Session not found: nonexistent-session"
  }
}
```

In Claude Code, errors display as:
```
Error: Session not found: nonexistent-session
```

### Error Handling Best Practices

1. **Session not found:** Always call `list_sessions` first
2. **Invalid query:** Check syntax (quotes balanced, operators valid)
3. **Large results:** Reduce k parameter if timeouts occur
4. **Internal errors:** Report with query and session details

---

## Performance Characteristics

### Latency Targets

| Tool              | p50    | p95    | p99    | Notes                 |
|-------------------|--------|--------|--------|-----------------------|
| search_code       | 10ms   | 50ms   | 100ms  | Depends on session    |
| list_sessions     | 5ms    | 10ms   | 20ms   | Lightweight           |
| get_session_info  | 3ms    | 5ms    | 10ms   | Single file read      |

### Tested Performance (OpenEMR 6,364 files)

Based on comprehensive performance testing (doc 009-phase01):

| Tool              | Min | Avg | Max | p95  | Notes |
|-------------------|-----|-----|-----|------|-------|
| search_code       | 2ms | 2.86ms | 4ms | 8ms  | Tested on 7 diverse queries |
| list_sessions     | <5ms | ~8ms | <10ms | <10ms | Lightweight operation |
| get_session_info  | <3ms | ~3ms | <5ms | <5ms | Single file read |
| index_repository  | N/A | 1,872 files/sec | N/A | N/A | 3.4s for 6,364 files |

**Key Findings:**
- **search_code:** Query complexity has minimal impact (2-4ms for all query types)
- **Cache performance:** No measurable difference between cold/warm cache
- **False positives:** 0% across all tests
- **Boolean operators:** 100% accuracy
- **Performance scales:** Large repos (6,000+ files) same 2-4ms latency

### Memory Usage

| Component     | Memory       |
|---------------|--------------|
| MCP Adapter   | <50MB        |
| Per Query     | <5MB         |
| Tantivy Index | Varies*      |

*Tantivy loads segments on demand. Memory usage depends on session size.

### Throughput

| Metric            | Value            |
|-------------------|------------------|
| Concurrent Queries| 1 (stdio limit)  |
| Sequential QPS    | >100             |
| Cold Start        | <100ms           |

---

## Language Detection

Code snippets are automatically syntax-highlighted based on file extension.

### Supported Languages (30+)

| Extension(s)      | Language    | Extension(s) | Language   |
|-------------------|-------------|--------------|------------|
| .rs               | rust        | .go          | go         |
| .py               | python      | .java        | java       |
| .js, .jsx         | javascript  | .kt, .kts    | kotlin     |
| .ts, .tsx         | typescript  | .swift       | swift      |
| .php              | php         | .c           | c          |
| .rb               | ruby        | .cpp, .cc    | cpp        |
| .sh, .bash        | bash        | .h, .hpp     | cpp        |
| .sql              | sql         | .cs          | csharp     |
| .html, .htm       | html        | .css         | css        |
| .json             | json        | .yaml, .yml  | yaml       |
| .xml              | xml         | .md          | markdown   |
| .toml             | toml        | .ini         | ini        |
| .vue              | vue         | .scala       | scala      |
| .clj, .cljs       | clojure     | .ex, .exs    | elixir     |

And more. If language not detected, defaults to plaintext.

---

## Best Practices

### Effective Searching

1. **Start broad, then narrow:**
   ```
   "database" -> "database connection" -> "database connection pool"
   ```

2. **Use boolean operators for precision (100% accurate):**
   ```
   "patient AND authentication" (must have both terms)
   "login OR signup" (either term)
   "auth NOT deprecated" (exclude deprecated code)
   "patient AND (login OR authentication)" (grouping with parentheses)
   ```

3. **Phrase queries for exact code patterns:**
   ```
   "function authenticateUser" (exact sequence)
   "CREATE TABLE users" (SQL patterns)
   "class UserController" (class definitions)
   ```

4. **Optimize k parameter based on use case:**
   ```
   k=5   - Quick exploration, get immediate answers (2-3ms)
   k=10  - Balanced default (2-4ms)
   k=20  - Comprehensive search, find diverse results (2-4ms)
   k=50+ - Thorough analysis (still fast, 3-5ms)
   ```

5. **Expect moderate relevance, zero false positives:**
   - Average relevance: 2.4/5 (tested on semantic queries)
   - False positive rate: 0% (all results contain search terms)
   - Best result may rank #8, not #1 (scan results, don't trust rank alone)
   - Highly relevant code always present in results

6. **When to use Shebe vs alternatives:**
   - **Use search_code for:** Unfamiliar/large codebases (1,000+ files), polyglot searches,
     semantic queries, finding top-N relevant results
   - **Use grep for:** Exact regex patterns, exhaustive searches (need ALL matches),
     small codebases (<100 files)
   - **Use Serena for:** Symbol refactoring, precise symbol lookup, AST-based code editing

### Session Management

1. **Use descriptive session names:**
   - Good: `openemr-v7.0.2`, `backend-auth`, `frontend-ui`
   - Bad: `test`, `temp`, `session1`

2. **Organize by project/branch:**
   ```
   my-app-main
   my-app-feature-auth
   my-app-v1.0
   ```

3. **Clean up old sessions:**
   - Use HTTP API to delete unused sessions
   - Keep session count manageable (<10-20)

### Performance Optimization

1. **Index only relevant files:**
   ```
   Include: *.rs, *.py (actual code)
   Exclude: target/**, node_modules/** (build artifacts)
   ```

2. **Adjust chunk size for file type:**
   - Small chunks (256): Dense code (Python, Ruby)
   - Large chunks (1024): Verbose code (Java, C++)
   - Default (512): Good balance

3. **Use appropriate k values:**
   - k=5: Quick answers
   - k=10: Default, good balance
   - k=50+: Comprehensive analysis (slower)

---

## See Also

- **Setup Guide:** docs/guides/mcp-setup-guide.md
- **Quick Start:** docs/guides/mcp-quick-start.md
- **Troubleshooting:** docs/troubleshooting/mcp-integration-troubleshooting.md
- **Architecture:** ARCHITECTURE.md (MCP Integration section)

---

**Document Version:** 1.3
**Last Updated:** 2025-10-25
**Protocol Version:** 2024-11-05
**New in v0.7.1:** Auto-truncation for list_dir and read_file tools
