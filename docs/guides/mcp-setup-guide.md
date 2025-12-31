# Shebe MCP Server Setup Guide

**Estimated Time:** 10 minutes
**Difficulty:** Beginner
**Version:** 0.5.3

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Verification](#verification)
5. [First Search](#first-search)
6. [Troubleshooting](#troubleshooting)
7. [Next Steps](#next-steps)

---

## Prerequisites

### System Requirements

- **Operating System:** Linux (tested on Debian/Ubuntu), macOS
- **Rust:** 1.88+ (for building from source)
- **Claude Code:** Latest version with MCP support
- **Disk Space:** 1GB+ free (for session storage)
- **Memory:** 2GB+ RAM recommended

### Knowledge Requirements

- Basic command line usage (cd, mkdir, ls)
- Understanding of file paths (absolute vs relative)
- JSON syntax basics (for configuration)

### Verify Prerequisites

```bash
# Check Rust installation
rustc --version
# Expected: rustc 1.88.0 or higher

# Check available disk space
df -h ~
# Need at least 1GB free

# Verify you have write permissions
touch ~/.test && rm ~/.test
# Should complete without errors
```

---

## Installation

### Option A: Build from Source (Current Method)

```bash
# Clone the repository
git clone <repository-url>
cd shebe

# Build and install
make mcp-build
make mcp-install

# Verify installation
which shebe-mcp
# Expected: /usr/local/bin/shebe-mcp

# Test the installation
make mcp-test
```

### Option B: Pre-built Binary (Future)

```bash
# Once releases are available:
# curl -L https://github.com/shebe/releases/latest/shebe-mcp \
#     -o shebe-mcp
# chmod +x shebe-mcp
# sudo mv shebe-mcp /usr/local/bin/
```

### Test Binary Installation

```bash
# Send a simple initialize request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
  "protocolVersion":"2024-11-05","capabilities":{"tools":{}},
  "clientInfo":{"name":"test","version":"1.0"}}}' | shebe-mcp

# Expected output (JSON response):
# {"jsonrpc":"2.0","id":1,"result":{...}}
# Press Ctrl+C to exit
```

---

## Configuration

### Step 1: Create Claude Code MCP Configuration

```bash
# Create Claude Code config directory (if not exists)
mkdir -p ~/.claude

# Create MCP configuration file
touch ~/.claude/mcp.json
```

### Step 2: Add Shebe MCP Server Configuration

Edit `~/.claude/mcp.json` with your favorite editor:

```bash
nano ~/.claude/mcp.json
# or
vim ~/.claude/mcp.json
# or
code ~/.claude/mcp.json  # if using VS Code
```

Add the following configuration:

```json
{
  "mcpServers": {
    "shebe": {
      "command": "shebe-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "SHEBE_INDEX_DIR": "/home/USERNAME/.local/state/shebe"
      },
      "description": "Shebe RAG service for code search using BM25"
    }
  }
}
```

**IMPORTANT:** Replace `USERNAME` with your actual username, or use absolute path.

**Pro Tip:** Use `$HOME` or `~` if Claude Code supports environment variable expansion:

```json
"SHEBE_INDEX_DIR": "$HOME/.local/state/shebe"
```

### Step 3: Create Sessions Storage Directory

```bash
# Create directory specified in SHEBE_INDEX_DIR
mkdir -p ~/.local/state/shebe/sessions

# Verify directory exists
ls -ld ~/.local/state/shebe/sessions
# Expected: drwxr-xr-x (readable and writable by you)
```

### Step 4: Validate Configuration

```bash
# Validate JSON syntax
cat ~/.claude/mcp.json | jq .

# If jq not installed:
# sudo apt install jq      # Ubuntu/Debian
# brew install jq          # macOS

# Should output formatted JSON without errors
```

---

## Verification

### Step 1: Test MCP Binary Manually

```bash
# Set environment variable
export SHEBE_INDEX_DIR=~/.local/state/shebe

# Send initialize request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
  "protocolVersion":"2024-11-05","capabilities":{"tools":{}},
  "clientInfo":{"name":"test","version":"1.0"}}}' | shebe-mcp 2>/dev/null

# Expected: JSON response with server info
# {"jsonrpc":"2.0","id":1,"result":{"capabilities":...}}
```

### Step 2: List Available Tools

```bash
# Request tools list
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | \
  SHEBE_INDEX_DIR=~/.local/state/shebe shebe-mcp 2>/dev/null | \
  tail -1 | jq -r '.result.tools[].name'

# Expected output:
# search_code
# list_sessions
# get_session_info
```

### Step 3: Restart Claude Code

**Important:** Claude Code must be restarted to load MCP configuration.

```bash
# Close Claude Code completely
# Then reopen Claude Code

# In a new conversation, you can ask:
# "What MCP servers are available?"
```

### Step 4: Verify in Claude Code

Start a new Claude Code conversation and ask:

```
You: What MCP servers do you have access to?

Claude: I have access to the following MCP servers:
- shebe: Shebe RAG service for code search using BM25
  Status: Connected
  Tools: search_code, list_sessions, get_session_info
```

If shebe doesn't appear, check [Troubleshooting](#troubleshooting).

---

## First Search

### Step 1: Create Test Repository

```bash
# Create simple test code
mkdir -p /tmp/test-code
cd /tmp/test-code

# Create test files
cat > auth.rs << 'EOF'
fn authenticate_user(username: &str, password: &str) -> bool {
    // User authentication logic
    if username.is_empty() || password.len() < 8 {
        return false;
    }
    true
}

fn validate_token(token: &str) -> Result<String, String> {
    // Token validation
    if token.is_empty() {
        return Err("Empty token".to_string());
    }
    Ok(token.to_string())
}
EOF

cat > patient.rs << 'EOF'
fn login_patient(id: u32, credentials: &str) -> bool {
    // Patient login logic
    !credentials.is_empty()
}

fn authenticate_patient(username: &str) -> bool {
    // Patient authentication
    username.starts_with("patient_")
}
EOF

cat > database.rs << 'EOF'
struct Database {
    connection_string: String,
}

impl Database {
    fn query(&self, sql: &str) -> Vec<String> {
        // Database query execution
        vec![]
    }
}
EOF
```

### Step 2: Index Test Repository in Claude Code

In your Claude Code conversation:

```
You: Index /tmp/test-code as session test-session

Claude: [Calls index_repository tool]
Indexing /tmp/test-code...

Indexing complete!
- Session: test-session
- Files indexed: 3
- Chunks created: 4
- Duration: 50ms
```

### Step 3: Search in Claude Code

In your Claude Code conversation:

```
You: List available Shebe sessions

Claude: [Calls list_sessions tool]

Available sessions (1):

## test-session
- Files: 3
- Chunks: 4
- Size: 8-9 KB
- Created: 2025-10-21T...

---

You: Search for "authenticate" in test-session

Claude: [Calls search_code tool]

Found 3 results for query 'authenticate' (6ms):

## Result 1 (score: 0.75)
File: /tmp/test-code/auth.rs (chunk 0, bytes 0-...)

```rust
fn authenticate_user(username: &str, password: &str) -> bool {
    // User authentication logic
    ...
}
```

## Result 2 (score: 0.58)
File: /tmp/test-code/patient.rs (chunk 0, bytes 0-...)

```rust
fn authenticate_patient(username: &str) -> bool {
    // Patient authentication
    ...
}
```
```

**Success!** Your Shebe MCP setup is working correctly.

---

## Troubleshooting

### Issue 1: "Command not found: shebe-mcp"

**Symptoms:** Claude Code can't find shebe-mcp binary

**Solutions:**

```bash
# Option 1: Use absolute path in mcp.json
{
  "mcpServers": {
    "shebe": {
      "command": "/usr/local/bin/shebe-mcp",
      ...
    }
  }
}

# Option 2: Verify binary location
which shebe-mcp
ls -l /usr/local/bin/shebe-mcp

# Option 3: Check permissions
chmod +x /usr/local/bin/shebe-mcp
```

### Issue 2: "Session not found"

**Symptoms:** Search returns "Session not found: test-session"

**Solutions:**

```bash
# Verify SHEBE_INDEX_DIR matches in mcp.json
cat ~/.claude/mcp.json | jq '.mcpServers.shebe.env.SHEBE_INDEX_DIR'

# List sessions directory
ls ~/.local/state/shebe/sessions/

# Verify session was indexed
cat ~/.local/state/shebe/sessions/test-session/meta.json 2>/dev/null || \
  echo "Session not found - reindex repository"
```

### Issue 3: No MCP Server in Claude Code

**Symptoms:** Claude Code doesn't show shebe in available servers

**Solutions:**

1. **Validate mcp.json syntax:**
   ```bash
   cat ~/.claude/mcp.json | jq .
   ```

2. **Check Claude Code logs:**
   ```bash
   tail -f ~/.claude/logs/mcp-*.log
   # Look for errors during initialization
   ```

3. **Restart Claude Code completely:**
   - Close all windows
   - Restart application
   - Start new conversation

4. **Test binary manually:**
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize",...}' | shebe-mcp
   ```

### Issue 4: Slow Performance

**Symptoms:** Searches take >1 second

**Solutions:**

```bash
# Check session size
du -sh ~/.local/state/shebe/sessions/*

# For large sessions:
# - Reduce k parameter (use k=5 instead of k=10)
# - Index smaller file sets with better patterns
# - Use SSD for session storage (not HDD)
```

### More Help

- **Troubleshooting Guide:** docs/troubleshooting/mcp-integration-troubleshooting.md
- **Test Results:** docs/testing/003-phase04-integration-test-results.md
- **GitHub Issues:** (coming soon)

---

## Next Steps

Now that Shebe MCP is set up, explore these features:

### 1. Index Real Codebases

In Claude Code:

```
You: Index /home/user/projects/my-app as session my-app-main,
     include only Rust, Python and JavaScript files,
     exclude target, .git and node_modules directories

Claude: [Calls index_repository tool]
Indexing complete!
- Session: my-app-main
- Files indexed: 450
- Chunks created: 2,450
- Duration: 3.2s
```

### 2. Try Advanced Queries

```
# Phrase search
You: Search for "user authentication function" in my-app-main

# Boolean search
You: Find code with "patient AND (login OR signup)" in my-app-main

# Top-K parameter
You: Show me the top 5 results for "database query" in my-app-main
```

### 3. Explore All Tools

```
# List all sessions
You: What code sessions are available?

# Get detailed session info
You: Tell me about the "my-app-main" session

# Search specific patterns
You: Find all error handling code in my-app-main
```

### 4. Advanced Configuration

See example configurations for:
- Multiple session directories (work vs personal)
- Custom chunk sizes and overlap
- Debug logging
- Performance tuning

Examples: `docs/examples/mcp-configs/`

### 5. Read Full Documentation

- **Quick Start:** docs/guides/mcp-quick-start.md (5 minutes)
- **Tool Reference:** docs/reference/mcp-tools-reference.md
- **Architecture:** ARCHITECTURE.md (MCP Integration section)

---

## Summary

Congratulations! You've successfully set up Shebe MCP for Claude Code.

**What you've accomplished:**
- Built and installed shebe-mcp binary
- Configured Claude Code MCP settings
- Created session storage directory
- Indexed your first repository
- Executed successful searches via Claude Code

**Performance:**
- Search latency: <10ms for small sessions (tested)
- Works with sessions up to 4,000+ files
- Markdown-formatted results with syntax highlighting

**Support:**
- Documentation: docs/guides/
- Troubleshooting: docs/troubleshooting/
- GitHub: (coming soon)

Happy coding with Shebe MCP!

---

**Document Version:** 1.0
**Last Updated:** 2025-10-21
**Tested With:** Shebe v0.1.0, Claude Code (latest)
