#!/usr/bin/env bash
#----------------------------------------------------------
# Generate a commit message using Claude
#
# Follows commit conventions from .claude/CLAUDE.md:
# - Angular style format (type(scope): description)
# - No emojis or special characters
# - No Oxford commas
# - No superlatives or emotional language
# - Max 120 character line length
# - Includes Contributes-to and Signed-off-by trailers
#
# Usage:
#   ./generate-commit-message.sh --dry-run          # Preview without saving
#   ./generate-commit-message.sh --all              # Include unstaged changes
#   ./generate-commit-message.sh --model sonnet     # Use a different model
#
# Output:
#   Saves commit message to tmp/<N>.txt (next available number)
#----------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TMP_DIR="${REPO_ROOT}/tmp"

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Generate a commit message using Claude."
    echo ""
    echo "Options:"
    echo "  --dry-run            Print output without saving to file"
    echo "  --all                Include unstaged changes in diff"
    echo "  --output FILE        Specify output file path"
    echo "  --model, -m NAME     Model to use: haiku, sonnet, opus (default: haiku)"
    echo "  -h, --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --dry-run              # Preview commit message"
    echo "  $0 --model sonnet         # Use sonnet model"
    echo "  $0 --all                  # Include unstaged changes"
}

# Show help if no arguments
if [[ $# -eq 0 ]]; then
    show_help
    exit 0
fi

# Parse arguments
DRY_RUN=false
INCLUDE_UNSTAGED=false
OUTPUT_FILE=""
MODEL="haiku"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --all)
            INCLUDE_UNSTAGED=true
            shift
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --model|-m)
            MODEL="$2"
            if [[ ! "$MODEL" =~ ^(haiku|sonnet|opus)$ ]]; then
                echo "Error: Model must be one of: haiku, sonnet, opus" >&2
                exit 1
            fi
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

#----------------------------------------------------------
# Functions
#----------------------------------------------------------

log() {
    echo "[commit-msg] $*" >&2
}

error() {
    echo "[commit-msg] ERROR: $*" >&2
    exit 1
}

get_next_tmp_number() {
    mkdir -p "${TMP_DIR}"
    local max=0
    shopt -s nullglob
    for f in "${TMP_DIR}"/[0-9]*.txt; do
        if [[ -f "$f" ]]; then
            local num
            num=$(basename "$f" .txt)
            if [[ "$num" =~ ^[0-9]+$ ]] && [[ "$num" -gt "$max" ]]; then
                max="$num"
            fi
        fi
    done
    shopt -u nullglob
    echo $((max + 1))
}

get_git_diff() {
    local diff=""

    # Get staged changes stat
    diff+="=== STAGED CHANGES ===
"
    diff+="$(git diff --cached --stat 2>/dev/null || true)
"

    # Get staged diff (truncated if too long)
    local staged_diff
    staged_diff=$(git diff --cached 2>/dev/null | head -c 6000 || true)
    if [[ ${#staged_diff} -ge 6000 ]]; then
        staged_diff+="
... (diff truncated)"
    fi
    diff+="
${staged_diff}
"

    # Include unstaged if requested
    if [[ "${INCLUDE_UNSTAGED}" == "true" ]]; then
        local unstaged
        unstaged=$(git diff --stat 2>/dev/null || true)
        if [[ -n "$unstaged" ]]; then
            diff+="
=== UNSTAGED CHANGES ===
${unstaged}
"
        fi
    fi

    # Get status
    diff+="
=== GIT STATUS ===
$(git status --short 2>/dev/null || true)
"

    echo "$diff"
}

get_recent_commits() {
    git log -5 --pretty=format:"%s" 2>/dev/null || true
}

#----------------------------------------------------------
# System prompt for commit message generation
#----------------------------------------------------------

SYSTEM_PROMPT='You are a commit message generator. Generate commit messages following these strict conventions:

FORMAT:
- Use Angular style: type(scope): short description
- Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build
- Scope is optional but recommended (e.g., mcp, core, http, indexer, search)
- Short description: imperative mood, lowercase, no period at end
- Body: explain what and why (not how), wrap at 72 characters
- Max 120 characters per line

STYLE RULES (CRITICAL):
- NO emojis or special Unicode characters anywhere
- NO Oxford commas (write "a, b and c" not "a, b, and c")
- NO superlatives or emotional language (no "great", "excellent", "amazing")
- Use objective, factual descriptions only
- Imperative mood for subject line ("add feature" not "added feature")

STRUCTURE:
```
type(scope): short description (max 50 chars ideal, 72 max)

Longer description if needed. Explain what changed and why.
Wrap lines at 72 characters. Leave blank line after subject.

- Bullet points for multiple changes
- Each bullet is a complete thought

Contributes-to: rhobimd-oss/shebe

Signed-off-by: RHOBIMD HEALTH
```

EXAMPLES OF GOOD COMMIT MESSAGES:
```
feat(mcp): add find_references tool for symbol discovery

Implement token-efficient symbol reference finding with confidence
scoring. Supports Rust, Go, Python, TypeScript and JavaScript with
pattern-based matching for definitions, calls and imports.

- Add find_references tool handler with configurable context
- Implement confidence scoring (high/medium/low) based on patterns
- Add session freshness warnings for stale indexes

Contributes-to: rhobimd-oss/shebe

Signed-off-by: RHOBIMD HEALTH
```

```
fix(core): handle UTF-8 boundaries in text chunker

Prevent panics when chunk boundaries fall within multi-byte UTF-8
sequences by using char_indices() instead of byte slicing.

Contributes-to: rhobimd-oss/shebe

Signed-off-by: RHOBIMD HEALTH
```

```
docs: update README with performance benchmarks

Add search speed comparison (13x faster than ripgrep) and token
efficiency metrics. Include real-world examples from Istio indexing.

Contributes-to: rhobimd-oss/shebe

Signed-off-by: RHOBIMD HEALTH
```

Generate ONLY the commit message text. No explanations or markdown code fences.'

#----------------------------------------------------------
# Main
#----------------------------------------------------------

cd "${REPO_ROOT}"

# Check for staged changes
if ! git diff --cached --quiet 2>/dev/null; then
    : # Has staged changes
elif [[ "${INCLUDE_UNSTAGED}" == "true" ]] && ! git diff --quiet 2>/dev/null; then
    : # Has unstaged changes and --all flag
else
    error "No changes to commit. Stage changes with 'git add' first."
fi

# Get diff and recent commits
log "Gathering git diff..."
DIFF=$(get_git_diff)
RECENT=$(get_recent_commits)

# Build prompt
USER_PROMPT="Generate a commit message for these changes:

${DIFF}

Recent commits in this repo (for style reference):
${RECENT}

Generate a commit message following the conventions exactly. Include the trailers:
Contributes-to: rhobimd-oss/shebe

Signed-off-by: RHOBIMD HEALTH"

# Generate commit message using Claude
log "Generating commit message with Claude ${MODEL}..."
COMMIT_MSG=$(claude --print --model "${MODEL}" --system-prompt "${SYSTEM_PROMPT}" "${USER_PROMPT}")

# Handle dry run
if [[ "${DRY_RUN}" == "true" ]]; then
    echo ""
    echo "========================================================================"
    echo "GENERATED COMMIT MESSAGE:"
    echo "========================================================================"
    echo "${COMMIT_MSG}"
    echo "========================================================================"
    exit 0
fi

# Determine output file
if [[ -z "${OUTPUT_FILE}" ]]; then
    NUM=$(get_next_tmp_number)
    OUTPUT_FILE="${TMP_DIR}/${NUM}.txt"
fi

# Save to file
mkdir -p "$(dirname "${OUTPUT_FILE}")"
echo "${COMMIT_MSG}" > "${OUTPUT_FILE}"

log "Commit message saved to: ${OUTPUT_FILE}"
log ""
log "To commit: git commit -F ${OUTPUT_FILE}"
log "To amend:  git commit --amend -F ${OUTPUT_FILE}"
echo ""
echo "========================================================================"
echo "GENERATED COMMIT MESSAGE:"
echo "========================================================================"
echo "${COMMIT_MSG}"
