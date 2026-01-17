# Shebe CLI Usage Guide

Complete reference for the `shebe` command-line interface.

**Time:** 10 minutes | **Difficulty:** Beginner

**Shebe Version:** 0.5.4 <br>
**Document Version:** 1.0 <br>
**Created:** 2026-01-16 <br>

---

## Prerequisites

- [x] Shebe built and installed (`make shebe-install`)
- [x] Terminal access

Verify installation:
```bash
shebe --version
shebe --help
```

---

## Quick Reference

| Command | Description |
|--------------------------|-------------------------------|
| `shebe index-repository` | Index a repository for search |
| `shebe search-code`      | Search indexed code           |
| `shebe find-references`  | Find symbol references        |
| `shebe list-sessions`    | List all sessions             |
| `shebe get-session-info` | Show session details          |
| `shebe delete-session`   | Delete a session              |
| `shebe reindex-session`  | Re-index a session            |
| `shebe show-config`      | Show configuration            |
| `shebe get-server-info`  | Show version info             |
| `shebe completions`      | Generate shell completions    |

---

## Global Options

All commands support these options:

| Option | Description |
|--------|-------------|
| `--format human` | Human-readable output (default) |
| `--format json` | JSON output for scripting |
| `--help` | Show command help |
| `--version` | Show version |

---

## Commands

### index-repository

Index a repository for BM25 search.

```bash
# Basic usage
shebe index-repository /path/to/repo --session myproject

# With custom chunking
shebe index-repository /path/to/repo \
  --session myproject \
  --chunk-size 1024 \
  --overlap 128

# Include/exclude patterns
shebe index-repository /path/to/repo \
  --session myproject \
  --include "*.rs" "*.go" \
  --exclude "**/target/**" "**/vendor/**"

# Force re-index existing session
shebe index-repository /path/to/repo --session myproject --force
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--session, -s` | required | Session ID (alphanumeric, hyphens) |
| `--chunk-size` | 512 | Characters per chunk (100-2000) |
| `--overlap` | 64 | Overlap between chunks (0-500) |
| `--include` | all | Glob patterns to include |
| `--exclude` | build dirs | Glob patterns to exclude |
| `--force, -f` | false | Re-index if session exists |

**Output (human):**
```
Indexing /home/user/myproject as 'myproject'...
Indexed 1,234 files (5,678 chunks) in 2.3s
Throughput: 536 files/sec
```

**Output (JSON):**
```json
{
  "session": "myproject",
  "files_indexed": 1234,
  "chunks_created": 5678,
  "duration_secs": 2.3
}
```

---

### search-code

Search indexed code with BM25 ranking.

```bash
# Basic search
shebe search-code "authentication" --session myproject

# Boolean queries
shebe search-code "user AND login" --session myproject
shebe search-code "auth OR session" --session myproject

# Limit results
shebe search-code "error handling" --session myproject --limit 20

# JSON output for scripting
shebe search-code "config" --session myproject --format json
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--session, -s` | required | Session ID to search |
| `--limit, -k` | 10 | Maximum results (1-100) |

**Output (human):**
```
Found 5 results in 'myproject':

[1] src/auth/handler.rs (score: 0.89)
    15: fn authenticate_user(credentials: &Credentials) -> Result<User> {
    16:     // Validate user credentials against database
    17:     let user = db.find_user(&credentials.username)?;

[2] src/middleware/auth.rs (score: 0.76)
    42:     if !session.is_authenticated() {
    43:         return Err(AuthError::NotAuthenticated);
```

---

### find-references

Find all references to a symbol across the indexed codebase.

```bash
# Find function references
shebe find-references "handleLogin" --session myproject

# Specify symbol type for better accuracy
shebe find-references "UserService" --session myproject --symbol-type type

# Exclude definition file
shebe find-references "processData" \
  --session myproject \
  --defined-in src/utils/data.rs
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--session, -s` | required | Session ID to search |
| `--symbol-type` | any | Type hint: function, type, variable, constant, any |
| `--defined-in` | none | File where symbol is defined (excluded from results) |
| `--max-results` | 50 | Maximum references to return |
| `--context-lines` | 2 | Lines of context around each reference |

---

### list-sessions

List all indexed sessions.

```bash
# List all sessions
shebe list-sessions

# JSON output
shebe list-sessions --format json
```

**Output (human):**
```
Sessions (3):
  myproject     1,234 files   5,678 chunks   12.3 MB   2h ago
  openemr       6,364 files  28,123 chunks   45.6 MB   1d ago
  istio         5,605 files  21,456 chunks   38.2 MB   3d ago
```

---

### get-session-info

Show detailed information about a session.

```bash
shebe get-session-info myproject
shebe get-session-info myproject --format json
```

**Output (human):**
```
Session: myproject
  Repository: /home/user/projects/myproject
  Files: 1,234
  Chunks: 5,678
  Size: 12.3 MB
  Indexed: 2026-01-15 10:30:45
  Schema: v3
  Config:
    chunk_size: 512
    overlap: 64
```

---

### delete-session

Delete a session and all associated data.

```bash
# Delete with confirmation flag
shebe delete-session myproject --confirm

# JSON output
shebe delete-session myproject --confirm --format json
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--confirm` | required | Confirm deletion (safety flag) |

---

### reindex-session

Re-index a session using its stored repository path.

```bash
# Re-index with same config
shebe reindex-session myproject

# Override chunk settings
shebe reindex-session myproject --chunk-size 1024

# Force even if config unchanged
shebe reindex-session myproject --force
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--chunk-size` | stored | Override chunk size |
| `--overlap` | stored | Override overlap |
| `--force, -f` | false | Force even if config unchanged |

---

### show-config

Display current Shebe configuration.

```bash
shebe show-config
shebe show-config --format json
```

**Output (human):**
```
Shebe Configuration

Storage
  Index directory: /home/user/.local/state/shebe

Indexing Defaults
  Chunk size: 512 characters
  Overlap: 64 characters

Search Defaults
  Default results (k): 10
  Maximum results: 100
```

---

### get-server-info

Show version and server information.

```bash
shebe get-server-info
shebe get-server-info --format json
```

**Output (human):**
```
Shebe Code Search Engine

Version: 0.5.4
Protocol: MCP 2024-11-05
Tools: 14

Storage: /home/user/.local/state/shebe
```

---

### completions

Generate shell completion scripts.

```bash
# Bash
shebe completions bash > ~/.local/share/bash-completion/completions/shebe

# Zsh
shebe completions zsh > ~/.zfunc/_shebe

# Fish
shebe completions fish > ~/.config/fish/completions/shebe.fish
```

After installing completions, restart your shell or source the file.

---

## Scripting Examples

### Index and Search Pipeline

```bash
#!/bin/bash
SESSION="myproject"
REPO="/path/to/repo"

# Index repository
shebe index-repository "$REPO" --session "$SESSION" --force --format json

# Search and process results
shebe search-code "TODO" --session "$SESSION" --format json | \
  jq -r '.results[] | "\(.file_path):\(.line_start): \(.text)"'
```

### Session Maintenance

```bash
#!/bin/bash
# List sessions older than 7 days and delete them
shebe list-sessions --format json | \
  jq -r '.[] | select(.age_days > 7) | .id' | \
  while read session; do
    echo "Deleting old session: $session"
    shebe delete-session "$session" --confirm
  done
```

### CI/CD Integration

```bash
#!/bin/bash
# Index on every build, search for security issues
shebe index-repository . --session ci-scan --force --format json

# Search for potential security issues
RESULTS=$(shebe search-code "password AND plaintext" --session ci-scan --format json)
COUNT=$(echo "$RESULTS" | jq '.total')

if [ "$COUNT" -gt 0 ]; then
  echo "Found $COUNT potential security issues"
  echo "$RESULTS" | jq '.results[]'
  exit 1
fi
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `NO_COLOR` | Disable colored output when set |
| `SHEBE_INDEX_DIR` | Override default index directory |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

---

## Related Documentation

- [MCP Setup Guide](./mcp-setup-guide.md) - Configure MCP server
- [MCP Quick Start](./mcp-quick-start.md) - Get started with MCP
- [ARCHITECTURE.md](/ARCHITECTURE.md) - System architecture

---

## Update Log

| Date | Shebe Version | Document Version | Changes |
|------|---------------|------------------|---------|
| 2026-01-16 | 0.5.4 | 1.0 | Initial CLI usage guide |
