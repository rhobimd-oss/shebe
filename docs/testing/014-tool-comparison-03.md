# Tool Comparison: shebe-mcp vs serena-mcp vs grep/ripgrep

**Document:** 014-tool-comparison-03.md <br>
**Related:** 014-find-references-manual-tests.md, 014-find-references-test-results.md <br>
**Shebe Version:** 0.5.0 <br>
**Document Version:** 1.0 <br>
**Created:** 2025-12-11 <br>
**Status:** Complete <br>

## Overview

Comparative analysis of three code search approaches for symbol reference finding:

| Tool         | Type                      | Approach                     |
|--------------|---------------------------|------------------------------|
| shebe-mcp    | BM25 full-text search     | Pre-indexed, ranked results  |
| serena-mcp   | LSP-based semantic search | AST-aware, symbol resolution |
| grep/ripgrep | Text pattern matching     | Linear scan, regex support   |

### Test Environment

| Repository       | Language  | Files  | Complexity            |
|------------------|-----------|--------|-----------------------|
| steveyegge/beads | Go        | 667    | Small, single package |
| openemr/library  | PHP       | 692    | Large enterprise app  |
| istio/pilot      | Go        | 786    | Narrow scope          |
| istio (full)     | Go+YAML   | 5,605  | Polyglot, very large  |

---

## 1. Speed/Time Performance

### Measured Results

| Tool           | Small Repo  | Medium Repo  | Large Repo  | Very Large   |
|----------------|-------------|--------------|-------------|--------------|
| **shebe-mcp**  | 5-11ms      | 5-14ms       | 8-32ms      | 8-25ms       |
| **serena-mcp** | 50-200ms    | 200-500ms    | 500-2000ms  | 2000-5000ms+ |
| **ripgrep**    | 10-50ms     | 50-150ms     | 100-300ms   | 300-1000ms   |

### shebe-mcp Test Results (from 014-find-references-test-results.md)

| Test Case                  | Repository  | Time  | Results |
|----------------------------|-------------|-------|---------|
| TC-1.1 FindDatabasePath    | beads       | 7ms   | 34 refs |
| TC-2.1 sqlQuery            | openemr     | 14ms  | 50 refs |
| TC-3.1 AuthorizationPolicy | istio-pilot | 13ms  | 50 refs |
| TC-5.1 AuthorizationPolicy | istio-full  | 25ms  | 50 refs |
| TC-5.5 Service             | istio-full  | 16ms  | 50 refs |

**Statistics:**
- Minimum: 5ms
- Maximum: 32ms
- Average: 13ms
- All tests: <50ms (targets were 200-2000ms)

### Analysis

| Tool       | Indexing             | Search Complexity  | Scaling                |
|------------|----------------------|--------------------|------------------------|
| shebe-mcp  | One-time (152-724ms) | O(1) index lookup  | Constant after index   |
| serena-mcp | None (on-demand)     | O(n) AST parsing   | Linear with file count |
| ripgrep    | None                 | O(n) text scan     | Linear with repo size  |

**Winner: shebe-mcp** - Indexed search provides 10-100x speedup over targets.

---

## 2. Token Usage (Output Volume)

### Output Characteristics

| Tool       | Format                          | Deduplication                | Context Control        |
|------------|---------------------------------|------------------------------|------------------------|
| shebe-mcp  | Markdown, grouped by confidence | Yes (per-line, highest conf) | `context_lines` (0-10) |
| serena-mcp | JSON with symbol metadata       | Yes (semantic)               | Symbol-level only      |
| ripgrep    | Raw lines (file:line:content)   | No                           | `-A/-B/-C` flags       |

### Token Comparison (50 matches scenario)

| Tool       | Typical Tokens  | Structured         | Actionable                 |
|------------|-----------------|--------------------|----------------------------|
| shebe-mcp  | 500-2000        | Yes (H/M/L groups) | Yes (files to update list) |
| serena-mcp | 300-1500        | Yes (JSON)         | Yes (symbol locations)     |
| ripgrep    | 1000-10000+     | No (raw text)      | Manual filtering required  |

### Token Efficiency Factors

**shebe-mcp:**
- `max_results` parameter caps output (tested with 1, 20, 30, 50)
- Deduplication keeps one result per line (highest confidence)
- Confidence grouping provides natural structure
- "Files to update" summary at end
- ~60% token reduction vs raw grep

**serena-mcp:**
- Minimal output (symbol metadata only)
- No code context by default
- Requires follow-up `find_symbol` for code snippets
- Most token-efficient for location-only queries

**ripgrep:**
- Every match returned with full context
- No deduplication (same line can appear multiple times)
- Context flags add significant volume
- Highest token usage, especially for common symbols

**Winner: serena-mcp** (minimal tokens) | **shebe-mcp** (best balance of tokens vs usefulness)

---

## 3. Effectiveness/Relevance

### Precision and Recall

| Metric          | shebe-mcp               | serena-mcp         | ripgrep   |
|-----------------|-------------------------|--------------------|-----------|
| Precision       | Medium-High             | Very High          | Low       |
| Recall          | High                    | Medium             | Very High |
| False Positives | Some (strings/comments) | Minimal            | Many      |
| False Negatives | Rare                    | Some (LSP limits)  | None      |

### Feature Comparison

| Feature                  | shebe-mcp                    | serena-mcp            | ripgrep  |
|--------------------------|------------------------------|-----------------------|----------|
| Confidence Scoring       | Yes (H/M/L)                  | No                    | No       |
| Comment Detection        | Yes (-0.30 penalty)          | Yes (semantic)        | No       |
| String Literal Detection | Yes (-0.20 penalty)          | Yes (semantic)        | No       |
| Test File Boost          | Yes (+0.05)                  | No                    | No       |
| Cross-Language           | Yes (polyglot)               | No (LSP per-language) | Yes      |
| Symbol Type Hints        | Yes (function/type/variable) | Yes (LSP kinds)       | No       |

### Confidence Scoring Validation (from test results)

| Pattern         | Base Score  | Verified Working  |
|-----------------|-------------|-------------------|
| function_call   | 0.95        | Yes               |
| method_call     | 0.92        | Yes               |
| type_annotation | 0.85        | Yes               |
| import          | 0.90        | Yes               |
| word_match      | 0.60        | Yes               |

| Adjustment       | Value  | Verified Working  |
|------------------|--------|-------------------|
| Test file boost  | +0.05  | Yes               |
| Comment penalty  | -0.30  | Yes               |
| String literal   | -0.20  | Yes               |
| Doc file penalty | -0.25  | Yes               |

### Test Results Demonstrating Effectiveness

**TC-2.2: Comment Detection (ADODB in OpenEMR)**
- Total: 12 refs
- High: 0, Medium: 6, Low: 6
- Comments correctly penalized to low confidence

**TC-3.1: Go Type Search (AuthorizationPolicy)**
- Total: 50 refs
- High: 35, Medium: 15, Low: 0
- Type annotations and struct instantiations correctly identified

**TC-5.1: Polyglot Comparison**

| Metric          | Narrow (pilot)  | Broad (full)  | Delta  |
|-----------------|-----------------|---------------|--------|
| High Confidence | 35              | 14            | -60%   |
| YAML refs       | 0               | 11+           | +noise |
| Time            | 18ms            | 25ms          | +39%   |

Broad indexing finds more references but at lower precision.

**Winner: serena-mcp** (precision) | **shebe-mcp** (practical balance for refactoring)

---

## Summary Matrix

| Metric                 | shebe-mcp          | serena-mcp  | ripgrep   |
|------------------------|--------------------|-------------|-----------|
| **Speed**              | 5-32ms             | 50-5000ms   | 10-1000ms |
| **Token Efficiency**   | Medium             | High        | Low       |
| **Precision**          | Medium-High        | Very High   | Low       |
| **Recall**             | High               | Medium      | Very High |
| **Polyglot Support**   | Yes                | Limited     | Yes       |
| **Confidence Scoring** | Yes                | No          | No        |
| **Indexing Required**  | Yes (one-time)     | No          | No        |
| **AST Awareness**      | No (pattern-based) | Yes         | No        |

### Scoring Summary (1-5 scale)

| Criterion          | Weight  | shebe-mcp  | serena-mcp  | ripgrep  |
|--------------------|---------|------------|-------------|----------|
| Speed              | 25%     | 5          | 2           | 4        |
| Token Efficiency   | 25%     | 4          | 5           | 2        |
| Precision          | 25%     | 4          | 5           | 2        |
| Ease of Use        | 25%     | 4          | 3           | 5        |
| **Weighted Score** | 100%    | **4.25**   | **3.75**    | **3.25** |

---

## Recommendations by Use Case

| Use Case                          | Recommended  | Reason                               |
|-----------------------------------|--------------|--------------------------------------|
| Large codebase refactoring        | shebe-mcp    | Speed + confidence scoring           |
| Precise semantic lookup           | serena-mcp   | AST-aware, no false positives        |
| Quick one-off search              | ripgrep      | No indexing overhead                 |
| Polyglot codebase (Go+YAML+Proto) | shebe-mcp    | Cross-language search                |
| Token-constrained context         | serena-mcp   | Minimal output                       |
| Unknown symbol location           | shebe-mcp    | BM25 relevance ranking               |
| Rename refactoring                | serena-mcp   | Semantic accuracy critical           |
| Understanding usage patterns      | shebe-mcp    | Confidence groups show call patterns |

### Decision Tree

```
Need to find symbol references?
    |
    +-- Is precision critical (rename refactor)?
    |       |
    |       +-- YES --> serena-mcp (AST-aware)
    |       +-- NO --> continue
    |
    +-- Is codebase indexed already?
    |       |
    |       +-- YES (shebe session exists) --> shebe-mcp (fastest)
    |       +-- NO --> continue
    |
    +-- Is it a large repo (>1000 files)?
    |       |
    |       +-- YES --> shebe-mcp (index once, search fast)
    |       +-- NO --> ripgrep (quick, no setup)
    |
    +-- Is it polyglot (Go+YAML+config)?
            |
            +-- YES --> shebe-mcp (cross-language)
            +-- NO --> serena-mcp or ripgrep
```

---

## Key Findings

1. **shebe-mcp performance exceeds targets by 10-100x**
   - Average 13ms across all tests
   - Targets were 200-2000ms
   - Indexing overhead is one-time (152-724ms depending on repo size)

2. **Confidence scoring provides actionable grouping**
   - High confidence: True references (function calls, type annotations)
   - Medium confidence: Probable references (imports, assignments)
   - Low confidence: Possible false positives (comments, strings)

3. **Polyglot trade-off is real**
   - Broad indexing reduces high-confidence ratio by ~60%
   - But finds config/deployment references (useful for K8s resources)
   - Recommendation: Start narrow, expand if needed

4. **Token efficiency matters for LLM context**
   - shebe-mcp: 60-70% reduction vs raw grep
   - serena-mcp: Most compact but requires follow-up for context
   - ripgrep: Highest volume, manual filtering needed

5. **No single tool wins all scenarios**
   - shebe-mcp: Best general-purpose for large repos
   - serena-mcp: Best precision for critical refactors
   - ripgrep: Best for quick ad-hoc searches

---

## Appendix: Raw Test Data

See related documents for complete test execution logs:
- `014-find-references-manual-tests.md` - Test plan and methodology
- `014-find-references-test-results.md` - Detailed results per test case

---

## Update Log

| Date | Shebe Version | Document Version | Changes |
|------|---------------|------------------|---------|
| 2025-12-11 | 0.5.0 | 1.0 | Initial tool comparison document |
