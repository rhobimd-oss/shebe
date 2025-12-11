# Shebe MCP Quick Start

Get Shebe MCP running with Claude Code in 5 minutes.

**Time:** 5 minutes | **Difficulty:** Beginner

---

## Prerequisites

- [x] Shebe repository cloned
- [x] Rust installed (1.88+)
- [x] Claude Code installed
- [x] Terminal access

---

## Step 1: Build shebe-mcp (1 minute)

```bash
cd /path/to/shebe
make mcp-build
make mcp-install
```

Verify:
```bash
which shebe-mcp
# Should output: /usr/local/bin/shebe-mcp

# Test installation
make mcp-test
```

---

## Step 2: Configure Claude Code (1 minute)

Create `~/.claude/mcp.json`:

```bash
mkdir -p ~/.claude
cat > ~/.claude/mcp.json << 'EOF'
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "env": {
        "SHEBE_INDEX_DIR": "$HOME/.local/state/shebe"
      }
    }
  }
}
EOF
```

Create storage directory:
```bash
mkdir -p ~/.local/state/shebe/sessions
```

---

## Step 3: Index Test Repository (1.5 minutes)

Create test code:
```bash
mkdir -p /tmp/quick-test
cat > /tmp/quick-test/main.rs << 'EOF'
fn authenticate(user: &str, pwd: &str) -> bool {
    !user.is_empty() && pwd.len() >= 8
}

fn main() {
    println!("Authentication system");
}
EOF
```

Start Shebe server (in separate terminal):
```bash
cd /path/to/shebe/services/shebe-server
SHEBE_INDEX_DIR=~/.local/state/shebe cargo run --release
```

Index the code:
```bash
curl -X POST http://localhost:3000/api/v1/index \
  -H "Content-Type: application/json" \
  -d '{
    "path": "~/github/openemr/openemr",
    "session": "openemr"
  }'
```

Expected: `{"files_indexed":1,"chunks_created":1,...}`

---

## Step 4: Restart Claude Code (30 seconds)

1. Close Claude Code completely
2. Reopen Claude Code
3. Start new conversation

---

## Step 5: Search! (1 minute)

In Claude Code conversation:

```
You: What MCP servers are available?

Claude: I have access to:
- shebe: Shebe RAG service for code search using BM25
  Tools: search_code, list_sessions, get_session_info
```

Try a search:
```
You: List available sessions

Claude: [Calls list_sessions]
Available sessions (1):
- quick-test (1 file, 1 chunk, ~100 bytes)
```

```
You: Search for "authenticate" in quick-test

Claude: [Calls search_code]
Found 1 result:

## Result 1 (score: 0.85)
File: /tmp/quick-test/main.rs

```rust
fn authenticate(user: &str, pwd: &str) -> bool {
    !user.is_empty() && pwd.len() >= 8
}
```
```

---

## Success!

You're now using Shebe MCP for code search in Claude Code.

**Total Time:** ~5 minutes

---

## What's Next?

### Direct Indexing (No HTTP Server Required!)

**NEW:** You can now index repositories directly from Claude Code without starting the HTTP server:

```
You: Index my project at /home/user/my-project for searching

Claude: [Calls index_repository tool]
Indexing started for session 'my-project'.
Use index_status to check progress.

[A few seconds later, Claude polls index_status]

Indexing complete! Indexed 450 files with 2,450 chunks.
You can now search this session.
```

See **Direct Indexing Guide** section below for details.

### Index via HTTP API (Traditional Method)

```bash
# Example: Index your actual codebase via HTTP
curl -X POST http://localhost:3000/api/v1/index -d '{
  "path": "/home/user/my-project",
  "session": "my-project",
  "include_patterns": ["*.rs", "*.py", "*.js"],
  "exclude_patterns": ["**/target/**", "**/node_modules/**"]
}'
```

### Try Advanced Queries

```
# Phrase search
You: Search for "user authentication" in my-project

# Boolean search
You: Find "login AND (username OR email)" in my-project

# Get session details
You: What's in the my-project session?
```

### Explore More

- **Full Setup Guide:** docs/guides/mcp-setup-guide.md
- **Tool Reference:** docs/reference/mcp-tools-reference.md
- **Troubleshooting:** docs/troubleshooting/mcp-integration-troubleshooting.md
- **Configuration Examples:** docs/examples/mcp-configs/

---

## Direct Indexing Guide

**Available since:** Shebe v0.2.0
**Tools:** `index_repository`, `index_status`

### Overview

Direct indexing allows you to index repositories directly from Claude Code conversations without running the HTTP server. Claude Code uses MCP tools to index your code in the background and report progress.

### Basic Usage

**1. Ask Claude to index your repository:**

```
You: Index my Rust project at /home/user/myapp
```

Claude will:
- Call `index_repository` tool with the path
- Start async indexing in the background
- Poll `index_status` to show progress
- Report completion with statistics

**2. Check progress anytime:**

```
You: What's the status of indexing myapp?
```

Claude will call `index_status` and show:
- Progress percentage (50.0%)
- Files processed (225/450)
- Chunks created
- Time remaining estimate
- Current file being processed

### Advanced Options

**Custom file patterns:**

```
You: Index /home/user/myapp but only Python and Rust files, excluding tests
```

Claude will add:
```json
{
  "include_patterns": ["**/*.py", "**/*.rs"],
  "exclude_patterns": ["**/tests/**", "**/test_*"]
}
```

**Custom chunk size:**

```
You: Index /home/user/myapp with smaller chunks (256 characters)
```

**Re-indexing:**

```
You: Re-index myapp session with the latest code
```

Claude will use `force: true` to overwrite the existing index.

### Progress Example

```
# Indexing Status: In Progress

**Session:** `myapp`
**Progress:** 50.0% (225/450)
[███████████████░░░░░░░░░░░░░░░]

**Estimated remaining:** 1m 30s
**Chunks created:** 1,250
**Failed files:** 2
**Elapsed time:** 1m 30s

**Currently processing:** `/src/main.rs`
```

### Completion Example

```
# Indexing Status: Complete

**Session:** `myapp`
**Status:**  Indexing complete

## Summary

- **Files indexed:** 448
- **Files failed:** 2
- **Success rate:** 99.6%
- **Chunks created:** 2,450
- **Duration:** 2m 45s

Indexing complete! You can now search this session with `search_code`.
```

### Troubleshooting Direct Indexing

**Indexing appears stuck:**
```
You: Check the status of myapp indexing
```
If no progress in 5+ minutes, ask Claude to force re-index.

**Permission errors:**
```
You: Why did indexing fail for /path/to/repo?
```
Claude will show detailed errors with `verbose: true`.

**Binary files failing:**
This is normal - binary files can't be indexed as text. The indexing will continue and index all text files successfully.

---

## Quick Troubleshooting

**MCP not showing in Claude Code?**
- Check `cat ~/.claude/mcp.json | jq .` (validates JSON)
- Restart Claude Code completely
- Check `which shebe-mcp` returns a path

**"Session not found"?**
- Verify indexing succeeded (check API response)
- Check `ls ~/.local/state/shebe/sessions/` shows session
- Ensure SHEBE_INDEX_DIR matches in mcp.json and server

**Need help?**
- Read: docs/guides/mcp-setup-guide.md
- Check: docs/troubleshooting/mcp-integration-troubleshooting.md

---

**Document Version:** 1.0
**Last Updated:** 2025-10-21
**Tested With:** Shebe v0.1.0
