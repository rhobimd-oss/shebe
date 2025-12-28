# Why Shebe?

**The Problem with Current AI-Assisted Code Search**

When using AI coding assistants to refactor symbols across large codebases (6k+ files),
developers face a choice: semantic precision (LSP tools, multiple round-trips) or raw speed
(grep, unranked results). Shebe provides both - single-call discovery with confidence-scored,
pattern-classified output.

**What about indexing time?** Shebe requires a one-time index (0.5s for ~6k files). Even
including this cost, index + search (0.5s + 2ms) completes faster than a single grep-based
workflow iteration (15-20s). The index persists across sessions, so subsequent searches
incur only the 2ms query cost.

## The Refactoring Challenge

Consider renaming `AuthorizationPolicy` across the Istio codebase (~6k files). This symbol
appears in multiple contexts:

- Go struct definition (`type AuthorizationPolicy struct`)
- Pointer types (`*AuthorizationPolicy`)
- Slice types (`[]AuthorizationPolicy`)
- Type instantiations (`AuthorizationPolicy{}`)
- GVK constants (`gvk.AuthorizationPolicy`)
- Kind constants (`kind.AuthorizationPolicy`)
- Multiple import aliases (`securityclient.`, `security_beta.`, `clientsecurityv1beta1.`)
- YAML manifests (`kind: AuthorizationPolicy`)

Each context matters for a safe refactor. Missing even one reference creates
runtime failures or broken builds.

## Tool Comparison: Benchmarks

Consider the following three approaches on this scenario - refactoring `AuthorizationPolicy`
across Istio 1.28:

- [Claude + Grep/Ripgrep](#approach-1-claude--grepripgrep)
- [Claude + Serena MCP (LSP-based)](#approach-2-claude--serena-mcp-lsp-based)
- [Claude + Shebe (BM25 index)](#approach-3-shebe-find_references-bm25-based)


### Approach 1: Claude + Grep/Ripgrep

The standard ClaudeCode approach requires iterative searching:

| Search   | Pattern                                     | Results         | Purpose |
|:---------|:--------------------------------------------|:----------------|---------|
| 1        | `AuthorizationPolicy` (Go files)            | 57 files        | Initial discovery |
| 2        | `AuthorizationPolicy` (YAML files)          | 54 files        | YAML declarations |
| 3        | `type AuthorizationPolicy struct`           | 1 match         | Type definition |
| 4        | `*AuthorizationPolicy`                      | 1 match         | Pointer usages |
| 5        | `[]AuthorizationPolicy`                     | 27 matches      | Slice usages |
| 6        | `AuthorizationPolicy{`                      | 30+ matches     | Instantiations |
| 7        | `gvk.AuthorizationPolicy`                   | 52 matches      | GVK references |
| 8        | `kind: AuthorizationPolicy`                 | 30+ matches     | YAML kinds |
| 9        | `kind.AuthorizationPolicy`                  | 19 matches      | Kind package refs |
| 10       | `securityclient.AuthorizationPolicy`        | 41 matches      | Client refs |
| 11       | `clientsecurityv1beta1.AuthorizationPolicy` | 14 matches      | v1beta1 refs |
| 12       | `security_beta.AuthorizationPolicy`         | 30+ matches     | Proto refs |
| 13       | Total count query                           | 470 occurrences | Verification |

**Results:**
- 13 searches required
- 15-20 seconds end-to-end
- ~12,000 tokens consumed
- Manual synthesis needed to produce actionable file list

### Approach 2: Claude + Serena MCP (LSP-based)

Serena provides semantic understanding but requires multiple round-trips:

| Search # | Tool | Results | Purpose |
|----------|------|---------|---------|
| 1 | find_symbol | 6 symbols | All definitions |
| 2 | find_referencing_symbols (struct) | 37 refs | Struct references |
| 3 | find_referencing_symbols (GVK) | 59 refs | GVK references |
| 4 | find_referencing_symbols (kind) | 20 refs | Kind references |
| 5 | search_for_pattern (client alias) | 41 matches | Import aliases |
| 6 | search_for_pattern (v1beta1) | 14 matches | More aliases |
| 7 | search_for_pattern (proto) | 100+ matches | Proto aliases |
| 8 | search_for_pattern (YAML) | 60+ matches | YAML files |

**Results:**
- 8 searches required
- 25-30 seconds end-to-end
- ~18,000 tokens consumed
- YAML files require fallback to pattern search
- Import aliases not detected semantically

### Approach 3: Shebe find_references (BM25-based)

A single call produces comprehensive output:

```bash
shebe-mcp find_references "AuthorizationPolicy" istio
```

**Results:**
- 1 search required
- 2-3 seconds end-to-end
- ~4,500 tokens consumed
- 100 references with confidence scores (H/M/L)
- 27 unique files identified
- Pattern classification (type_instantiation, type_annotation, word_match)

## Comparison Summary

| Metric | Shebe | Grep | Serena |
|--------|-------|------|--------|
| Searches required | 1 | 13 | 8 |
| End-to-end time | 2-3s | 15-20s | 25-30s |
| Tokens consumed | ~4,500 | ~12,000 | ~18,000 |
| Actionable output | Immediate | Manual synthesis | Semi-manual |
| Confidence scoring | Yes | No | No |
| Pattern classification | Yes | No | Partial (symbol kinds) |
| YAML support | Native | Native | Pattern fallback |
| Cross-file aggregation | Yes | Manual | Per-definition |

**Measured differences:**
- 6-10x faster end-to-end than grep or Serena workflows
- 2.7-4x fewer tokens consumed per refactoring task
- Single operation vs 8-13 iterative searches

## Tool Limitations

### Grep/Ripgrep

Ripgrep executes in 24ms, but the workflow overhead adds up:

1. **No semantic understanding**: `AuthorizationPolicy` matches documentation,
   comments, variable names and actual type references equally
2. **Multiple patterns required**: Each usage context (pointer, slice, alias)
   requires a separate search
3. **Manual synthesis**: 13 searches produce raw matches requiring analysis
   to identify actionable files
4. **Token overhead**: Returns file paths only, requiring Claude to read
   entire files (2,000-8,000 tokens per file)

### Serena MCP

Serena provides LSP-based semantic analysis, but has constraints for this use case:

1. **Multiple definitions require multiple calls**: `AuthorizationPolicy` exists
   as a struct, constant, variable and in collections - each needs separate
   `find_referencing_symbols`
2. **Import aliases not detected**: `securityclient.AuthorizationPolicy` and
   `security_beta.AuthorizationPolicy` require pattern search fallback
3. **YAML not analyzed semantically**: Falls back to pattern search for
   Kubernetes manifests
4. **Token overhead**: Verbose JSON responses consume 3-4x more tokens
5. **Optimized for editing**: Serena is designed for precise symbol operations,
   not broad discovery

## How Shebe Addresses These

### Pre-computed BM25 Index

Indexing happens once when starting work with a codebase:

```bash
# Index 5,965 files in 0.5 seconds
shebe-mcp index_repository ~/github/istio/istio istio
```

Subsequent searches hit an in-memory Tantivy index - no file I/O or regex
processing during queries.

### Confidence Scoring

Shebe's `find_references` classifies matches by confidence:

| Confidence | Pattern | Example |
|------------|---------|---------|
| High (0.85-0.90) | type_instantiation | `&AuthorizationPolicy{}` |
| High (0.90) | type_annotation | `kind: AuthorizationPolicy` |
| Medium (0.65-0.75) | word_match + test boost | `// Test AuthorizationPolicy` |
| Low (<0.50) | word_match | Documentation mentions |

This enables prioritization - high-confidence references first, medium-confidence
for edge cases, low-confidence (docs, comments) for review if needed.

### Cross-File Aggregation

A single call finds all references regardless of:
- Import aliases
- File types (Go, YAML, Markdown, JSON)
- Symbol context (definition, usage, test, documentation)

The output is a file list with line numbers and context, without manual synthesis.

### Compact Output Format

Shebe returns 5 lines of context per match:

```
pilot/pkg/model/authorization.go:24 (score: 12.3)
  type AuthorizationPolicy struct {
      // Policy configuration...
  }
```

Compare to Serena's JSON format:
```json
{
  "file": "pilot/pkg/model/authorization.go",
  "symbol": "AuthorizationPolicy",
  "kind": "Struct",
  "range": {"start": {"line": 24, "character": 5}, "end": {...}},
  "containing_symbol": "...",
  ...
}
```

Compact output means fewer tokens per result.

## Recommended Workflow

Shebe and Serena serve different purposes:

1. **Discovery (Shebe)**: "What files contain this symbol?"
   - Single call, ~4,500 tokens
   - Confidence-scored, pattern-classified
   - YAML and non-code files included

2. **Editing (Serena)**: "Apply the change semantically"
   - `replace_symbol_body` for precise edits
   - LSP-based refactoring
   - Rename propagation

Use Shebe for the discovery phase, Serena for the editing phase.

## Tool Selection Guide

| Task | Tool | Reason |
|------|------|--------|
| Find all usages of a symbol | Shebe `find_references` | Single call, confidence scores |
| Rename a symbol across codebase | Shebe (discover) + Serena (edit) | Discovery + precision |
| Search YAML/Markdown/configs | Shebe `search_code` | Native non-code support |
| Go to definition | Serena `find_symbol` | LSP precision |
| Find implementations of interface | Serena | Semantic analysis |
| Keyword search | Shebe `search_code` | 2ms latency, ranked results |
| Exact string match | grep/ripgrep | Simplest tool for simple tasks |

## Summary

Shebe addresses the gap between grep's raw speed and Serena's semantic precision:

- **Token efficiency**: 3-4x fewer tokens than alternative workflows
- **Time efficiency**: 6-10x faster end-to-end
- **Single-operation discovery**: One call vs 8-13 iterative searches
- **Structured output**: Confidence-scored, pattern-classified results
- **Polyglot support**: Go, YAML, Markdown, JSON and 11+ file types in one query

For AI-assisted workflows where context window tokens and response latency
affect productivity, Shebe reduces the overhead of large codebase discovery tasks.
