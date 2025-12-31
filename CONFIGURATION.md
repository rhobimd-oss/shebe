# Shebe MCP Configuration

This guide covers configuration options for `shebe-mcp`, the MCP server that integrates Shebe with Claude Code.

## Quick Start

Shebe MCP works out-of-the-box with sensible defaults. For most users, no configuration is needed:

```bash
# Add to Claude Code MCP settings (~/.config/claude-code/config.json)
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp"
    }
  }
}
```

For advanced use cases, configure via **TOML file** or **environment variables**.

## Configuration Priority

Settings are loaded in this order (later sources override earlier ones):

1. Built-in defaults
2. TOML configuration file
3. Environment variables

## Configuration File Location

Shebe follows the XDG Base Directory specification. Configuration files are searched in this order:

| Priority | Location                         | When Used                                           |
|----------|----------------------------------|-----------------------------------------------------|
| 1        | `$SHEBE_CONFIG` env var          | Custom path set via environment variable            |
| 2        | `~/.config/shebe/config.toml`    | **Recommended** - User configuration (XDG standard) |
| 3        | `./shebe.toml`                   | Legacy fallback - Current directory                 |
| 4        | Built-in defaults                | No configuration file found                         |

**Recommended location:** `~/.config/shebe/config.toml`

Data directory (indexed sessions):

| Location                         | Purpose                     |
|----------------------------------|-----------------------------|
| `~/.local/share/shebe/sessions/` | Indexed repository sessions |

You can override locations with environment variables:

```bash
# Custom config file
export SHEBE_CONFIG="/path/to/custom/config.toml"

# Custom data directory
export SHEBE_DATA_DIR="/path/to/data"
```

## Configuration Reference

All options are organized into logical sections. Each option can be set via TOML configuration or environment variable.

### Indexing Options

These settings control how Shebe chunks and indexes repository files.

| Option                                                    | Type                | Default   | Description                                                                                                                                                                                                                                         |
|-----------------------------------------------------------|---------------------|-----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| toml: `chunk_size`<br>env: `SHEBE_CHUNK_SIZE`             | integer             | `512`     | Number of Unicode characters per chunk. Larger values provide more context per chunk but use<br>more storage. Must be > 0 and > overlap. **Measured in characters, not bytes** to ensure UTF-8<br>safety across emoji, CJK and special characters. |
| toml: `overlap`<br>env: `SHEBE_OVERLAP`                   | integer             | `64`      | Number of characters to overlap between consecutive chunks. Ensures search terms near chunk<br>boundaries are found. Must be < chunk_size. Higher values improve boundary matching but<br>increase index size.                                      |
| toml: `max_file_size_mb`<br>env: `SHEBE_MAX_FILE_SIZE_MB` | integer             | `10`      | Maximum file size in megabytes. Files larger than this are skipped during indexing to prevent<br>memory issues and slow indexing. Common for vendored dependencies or generated files.                                                              |
| toml: `include_patterns`<br>env: N/A                      | array of<br>strings | See below | Glob patterns for files to index (e.g., `*.rs`, `*.py`). Only files matching these patterns<br>are indexed. Use `**` for recursive matching.                                                                                                        |
| toml: `exclude_patterns`<br>env: N/A                      | array of<br>strings | See below | Glob patterns for files to skip (e.g., `**/node_modules/**`). Applied after include patterns.<br>Use to skip build artifacts, dependencies and binary files.                                                                                       |

**Default include patterns:** `*.rs`, `*.toml`, `*.md`, `*.txt`, `*.php`, `*.js`, `*.ts`, `*.py`, `*.go`, `*.java`, `*.c`, `*.cpp`, `*.h`

**Default exclude patterns:** `**/node_modules/**`, `**/target/**`, `**/vendor/**`, `**/.git/**`, `**/build/**`, `**/__pycache__/**`, `**/dist/**`, plus all binary files (images, audio, video, archives, executables, fonts). See complete list in [config.rs](services/shebe-server/src/config.rs).

### Storage Options

Controls where indexed data is stored.

| Option                                     | Type  | Default                                | Description                                                                                                                                                                         |
|--------------------------------------------|-------|----------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| toml: `index_dir`<br>env: `SHEBE_DATA_DIR` | path  | `~/.local/share/`<br>`shebe/sessions/` | Directory where session indexes are stored. Each indexed repository gets a subdirectory here.<br>Uses XDG data directory by default. Set `SHEBE_DATA_DIR` to use a custom location. |

### Search Options

Controls search behavior and result limits.

| Option                                                    | Type    | Default  | Description                                                                                                                                                           |
|-----------------------------------------------------------|---------|----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| toml: `default_k`<br>env: `SHEBE_DEFAULT_K`               | integer | `10`     | Number of search results returned when the MCP client doesn't specify a limit. Balance between<br>result comprehensiveness and token usage. Must be > 0 and <= max_k. |
| toml: `max_k`<br>env: `SHEBE_MAX_K`                       | integer | `100`    | Hard limit on maximum search results per query. Prevents excessive token usage even if client<br>requests more. Enforced server-side for resource protection.         |
| toml: `max_query_length`<br>env: `SHEBE_MAX_QUERY_LENGTH` | integer | `500`    | Maximum length of search query string in characters. Prevents pathologically long queries that<br>could cause performance issues. BM25 works best with 2-10 keywords. |

### Resource Limits

Controls concurrency and timeouts.

| Option                                                                | Type    | Default  | Description                                                                                                                                                                                               |
|-----------------------------------------------------------------------|---------|----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| toml: `max_concurrent_indexes`<br>env: `SHEBE_MAX_CONCURRENT_INDEXES` | integer | `1`      | Maximum number of repositories that can be indexed simultaneously. Set to `1` to prevent<br>CPU/memory exhaustion. Increase only on powerful machines with sufficient RAM (2GB+ per<br>concurrent index). |
| toml: `request_timeout_sec`<br>env: `SHEBE_REQUEST_TIMEOUT_SEC`       | integer | `300`    | Timeout in seconds for indexing and search requests. Indexing large repositories (>10k files)<br>may need longer timeouts. Search queries typically complete in milliseconds.                             |

### Logging Options

Controls diagnostic output (written to stderr, not stdout, to preserve MCP protocol on stdout).

| Option                                      | Type   | Default  | Description                                                                                                                                                                                                         |
|---------------------------------------------|--------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| toml: `log_level`<br>env: `SHEBE_LOG_LEVEL` | string | `"info"` | Logging verbosity level. Options: `trace` (very verbose, development), `debug` (detailed<br>diagnostics), `info` (normal operations), `warn` (problems only), `error` (critical issues only).<br>Logs go to stderr. |

## Example Configurations

### Minimal Configuration (Recommended)

Most users don't need a config file. Defaults work well:

```bash
# No config file needed - just run shebe-mcp
shebe-mcp
```

### Custom Data Directory

Store indexes in a specific location:

```bash
# Environment variable approach
export SHEBE_DATA_DIR="/mnt/ssd/shebe-indexes"
shebe-mcp
```

Or via TOML (`~/.config/shebe/config.toml`):

```toml
[storage]
index_dir = "/mnt/ssd/shebe-indexes"
```

### Large Repository Tuning

For repositories with >10k files or large code files:

```toml
# ~/.config/shebe/config.toml

[indexing]
chunk_size = 1024              # Larger chunks for better context
overlap = 128                  # More overlap for boundary matching
max_file_size_mb = 20          # Allow larger files

[search]
default_k = 20                 # Return more results by default
max_k = 200                    # Allow requesting more results

[limits]
request_timeout_sec = 600      # 10 minute timeout for huge repos
```

### Memory-Constrained Environments

Reduce memory footprint:

```toml
# ~/.config/shebe/config.toml

[indexing]
chunk_size = 256               # Smaller chunks
overlap = 32                   # Less overlap
max_file_size_mb = 5           # Skip large files

[search]
max_k = 50                     # Limit result set size

[limits]
max_concurrent_indexes = 1     # One index at a time
```

### Custom File Types

Index only specific languages:

```toml
# ~/.config/shebe/config.toml

[indexing]
# Only index Python and JavaScript
include_patterns = [
    "*.py",
    "*.js",
    "*.jsx",
    "*.ts",
    "*.tsx",
]

# Skip tests and examples
exclude_patterns = [
    "**/test/**",
    "**/tests/**",
    "**/examples/**",
    "**/node_modules/**",
    "**/__pycache__/**",
]
```

### Debug Logging

Enable verbose logging for troubleshooting:

```bash
export SHEBE_LOG_LEVEL="debug"
shebe-mcp
```

Or via TOML:

```toml
# ~/.config/shebe/config.toml

[server]
log_level = "debug"
```

## Common Configuration Tasks

### Change Where Indexes Are Stored

```bash
# Option 1: Environment variable
export SHEBE_DATA_DIR="/custom/path"

# Option 2: TOML file (~/.config/shebe/config.toml)
[storage]
index_dir = "/custom/path"
```

### Increase Result Limit

```bash
# Environment variable
export SHEBE_DEFAULT_K=20
export SHEBE_MAX_K=200

# Or TOML
[search]
default_k = 20
max_k = 200
```

### Skip Large Files

```bash
# Environment variable
export SHEBE_MAX_FILE_SIZE_MB=5

# Or TOML
[indexing]
max_file_size_mb = 5
```

### Index Additional File Types

You can only set file patterns via TOML (not environment variables):

```toml
# ~/.config/shebe/config.toml

[indexing]
include_patterns = [
    "*.rs",
    "*.py",
    "*.rb",      # Add Ruby
    "*.scala",   # Add Scala
    "*.kt",      # Add Kotlin
]
```

## Validation and Errors

Shebe validates configuration on startup. Invalid settings cause immediate exit with error messages:

| Validation Rule | Error if Violated |
|----------------|-------------------|
| `chunk_size > 0` | "Chunk size must be non-zero" |
| `overlap < chunk_size` | "Overlap must be less than chunk size" |
| `default_k > 0` | "Default k must be non-zero" |
| `default_k <= max_k` | "Default k cannot exceed max k" |
| `max_query_length > 0` | "Max query length must be non-zero" |
| `max_concurrent_indexes > 0` | "Max concurrent indexes must be non-zero" |
| `request_timeout_sec > 0` | "Request timeout must be non-zero" |

## Performance Impact

Configuration affects performance and resource usage:

| Setting | Larger Values | Smaller Values |
|---------|--------------|----------------|
| **chunk_size** | More context per result, larger index size, slower indexing | Less context, smaller index, faster indexing |
| **overlap** | Better boundary matching, larger index | Faster indexing, smaller index |
| **default_k** | More comprehensive results, higher token usage | Faster responses, lower token usage |
| **max_file_size_mb** | Indexes more files | Skips large/generated files, faster indexing |
| **max_concurrent_indexes** | Faster parallel indexing, high memory use | Lower memory, slower when indexing multiple repos |

**Indexing benchmarks with defaults:**
- Istio (5,605 files): 0.5s, 11,210 files/sec
- OpenEMR (6,364 files): 3.3s, 1,928 files/sec

**Search benchmarks with defaults:**
- Query latency: 2ms (median, p95, p99)
- Token usage: 210-650 tokens per query

See [docs/Performance.md](./docs/Performance.md) for detailed benchmarks.

## Troubleshooting

### "Failed to create XDG directories"

Shebe needs write access to `~/.config/shebe/` and `~/.local/share/shebe/`. Check directory permissions.

### "Overlap must be less than chunk size"

Your `overlap` setting is >= `chunk_size`. Reduce overlap or increase chunk_size.

### Indexing Times Out

Large repositories may need longer timeout:

```bash
export SHEBE_REQUEST_TIMEOUT_SEC=600  # 10 minutes
```

### Out of Memory During Indexing

Reduce concurrent indexing or skip large files:

```toml
[indexing]
max_file_size_mb = 5

[limits]
max_concurrent_indexes = 1
```

## See Also

- [INSTALLATION.md](./INSTALLATION.md) - Setup and installation guide
- [docs/guides/mcp-setup-guide.md](./docs/guides/mcp-setup-guide.md) - Claude Code MCP integration
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture and internals
- [docs/Performance.md](./docs/Performance.md) - Performance benchmarks and tuning
