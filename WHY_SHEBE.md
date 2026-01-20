# Shebe Benchmarks

Two detailed benchmarks comparing Shebe against grep/ripgrep and Serena MCP
for large-scale symbol refactoring. For overview, capabilities and quick start,
see [README.md](./README.md).

---

## Benchmark 1: Go/YAML Symbol Refactoring (Istio)

**Scenario:** Rename `AuthorizationPolicy` across the Istio codebase (~6k files)

This symbol appears in multiple contexts that all need updating:
- Go struct definition (`type AuthorizationPolicy struct`)
- Pointer and slice types (`*AuthorizationPolicy`, `[]AuthorizationPolicy`)
- Type instantiations (`AuthorizationPolicy{}`)
- GVK and kind constants (`gvk.AuthorizationPolicy`, `kind.AuthorizationPolicy`)
- Multiple import aliases (`securityclient.`, `security_beta.`, `clientsecurityv1beta1.`)
- YAML manifests (`kind: AuthorizationPolicy`)

Missing even one reference creates runtime failures or broken builds.

### Approach 1: Claude + Grep/Ripgrep

The standard approach requires iterative searching:

| Search | Pattern                                     | Results         | Purpose            |
|:-------|:--------------------------------------------|:----------------|:-------------------|
| 1      | `AuthorizationPolicy` (Go files)            | 57 files        | Initial discovery  |
| 2      | `AuthorizationPolicy` (YAML files)          | 54 files        | YAML declarations  |
| 3      | `type AuthorizationPolicy struct`           | 1 match         | Type definition    |
| 4      | `*AuthorizationPolicy`                      | 1 match         | Pointer usages     |
| 5      | `[]AuthorizationPolicy`                     | 27 matches      | Slice usages       |
| 6      | `AuthorizationPolicy{`                      | 30+ matches     | Instantiations     |
| 7      | `gvk.AuthorizationPolicy`                   | 52 matches      | GVK references     |
| 8      | `kind: AuthorizationPolicy`                 | 30+ matches     | YAML kinds         |
| 9      | `kind.AuthorizationPolicy`                  | 19 matches      | Kind package refs  |
| 10     | `securityclient.AuthorizationPolicy`        | 41 matches      | Client refs        |
| 11     | `clientsecurityv1beta1.AuthorizationPolicy` | 14 matches      | v1beta1 refs       |
| 12     | `security_beta.AuthorizationPolicy`         | 30+ matches     | Proto refs         |
| 13     | Total count query                           | 470 occurrences | Verification       |

**Results:** 13 searches, 15-20s end-to-end, ~12,000 tokens, manual synthesis needed

### Approach 2: Claude + Serena MCP (LSP-based)

Serena provides semantic understanding but requires multiple round-trips:

| Search # | Tool                              | Results      | Purpose           |
|----------|-----------------------------------|--------------|-------------------|
| 1        | find_symbol                       | 6 symbols    | All definitions   |
| 2        | find_referencing_symbols (struct) | 37 refs      | Struct references |
| 3        | find_referencing_symbols (GVK)    | 59 refs      | GVK references    |
| 4        | find_referencing_symbols (kind)   | 20 refs      | Kind references   |
| 5        | search_for_pattern (client alias) | 41 matches   | Import aliases    |
| 6        | search_for_pattern (v1beta1)      | 14 matches   | More aliases      |
| 7        | search_for_pattern (proto)        | 100+ matches | Proto aliases     |
| 8        | search_for_pattern (YAML)         | 60+ matches  | YAML files        |

**Results:** 8 searches, 25-30s end-to-end, ~18,000 tokens, YAML requires fallback

### Approach 3: Shebe find_references (BM25-based)

A single call produces comprehensive output:

```bash
shebe-mcp find_references "AuthorizationPolicy" istio
```

**Results:** 1 search, 2-3s end-to-end, ~4,500 tokens, 100 refs with confidence scores

### Comparison Summary

| Metric                 | Shebe   | Grep       | Serena          |
|------------------------|---------|------------|-----------------|
| Searches required      | 1       | 13         | 8               |
| End-to-end time        | 2-3s    | 15-20s     | 25-30s          |
| Tokens consumed        | ~4,500  | ~12,000    | ~18,000         |
| Actionable output      | Immediate | Manual synthesis | Semi-manual |
| Confidence scoring     | Yes     | No         | No              |
| Pattern classification | Yes     | No         | Partial         |
| YAML support           | Native  | Native     | Pattern fallback |

**Measured differences:** 6-10x faster, 2.7-4x fewer tokens, single operation vs 8-13 searches

---

## Benchmark 2: C++ Symbol Refactoring (Eigen Library)

A second benchmark validates Shebe's accuracy advantage for substring-collision scenarios.

**Scenario:** Rename `MatrixXd` -> `MatrixPd` across the Eigen C++ library (~6k files)

**Challenge:** The symbol `MatrixXd` appears as a substring in other symbols:
- `ColMatrixXd` (different type)
- `MatrixXdC`, `MatrixXdR` (different types)

Grep matches all of these, creating false positives that would introduce bugs.

### Results Summary

| Metric             | grep/ripgrep | Serena         | Shebe (optimized) |
|--------------------|--------------|----------------|-------------------|
| **Completion**     | Complete     | Blocked        | Complete          |
| **Discovery Time** | 31ms         | ~2 min         | **16ms**          |
| **Total Time**     | 74ms         | >60 min (est.) | ~15s              |
| **Token Usage**    | ~13,700      | ~506,700 (est.)| ~7,000            |
| **Files Modified** | 137          | 0 (blocked)    | 135               |
| **False Positives**| 2            | N/A            | 0                 |
| **Accuracy**       | 98.5%        | N/A            | **100%**          |

### Key Findings

**grep/ripgrep (74ms):**
- Fastest execution
- Renamed 2 files incorrectly (false positives):
  - `test/is_same_dense.cpp` - Contains `ColMatrixXd`
  - `Eigen/src/QR/ColPivHouseholderQR_LAPACKE.h` - Contains `MatrixXdC`, `MatrixXdR`
- Would have introduced bugs without manual review

**Serena (blocked):**
- C++ macros (`EIGEN_MAKE_TYPEDEFS`) not visible to LSP
- Symbolic approach found only 6 references vs 522 actual occurrences
- Required pattern search fallback, making it slowest overall

**Shebe optimized (16ms discovery, 100% accuracy):**
- Configuration: `max_k=500`, `context_lines=0`
- Single-pass discovery of all 135 files in 16ms (4.6x faster than grep)
- Zero false positives due to confidence scoring
- ~52 tokens per file (vs grep's ~100)

### Accuracy vs Speed

```
Work Efficiency (higher = faster)
     ^
     |            Shebe (16ms discovery, 0 errors)
     |                 *
     |   grep/ripgrep (74ms total, 2 errors)
     |        *
     |
     +-------------------------------------------------> Accuracy
```

Shebe discovery is 4.6x faster than grep AND more accurate (100% vs 98.5%).

---

## Optimized Configuration

For bulk refactoring, use these settings:

```
find_references:
  max_results: 500    # Eliminates iteration (default: 100)
  context_lines: 0    # Reduces tokens ~50% (default: 2)
```

**Results with optimized config:**
- 135 files in 1 pass, 16ms discovery (vs 4 passes with defaults)
- ~7,000 tokens total (vs ~15,000 with defaults)
- ~15 seconds end-to-end (discovery + batch rename)

---

## Tool Limitations (Details)

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

Serena provides LSP-based semantic analysis, but has constraints for discovery:

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

---

## Sources

### Academic Research

- Sadowski, C., et al. (2015). "How Developers Search for Code: A Case Study."
  FSE 2015. https://research.google/pubs/how-developers-search-for-code-a-case-study/

- Hora, A., et al. (2021). "What Developers Search For and What They Find."
  MSR 2021. https://homepages.dcc.ufmg.br/~andrehora/pub/2021-msr-googling-for-development.pdf

### Industry Blog Posts

- GitHub Engineering. "The Technology Behind GitHub's New Code Search."
  https://github.blog/engineering/architecture-optimization/the-technology-behind-githubs-new-code-search/

- Sourcegraph. "Keeping it Boring (and Relevant) with BM25F."
  https://sourcegraph.com/blog/keeping-it-boring-and-relevant-with-bm25f

- Turbopuffer. "Cursor Scales Code Retrieval to 100B+ Vectors."
  https://turbopuffer.com/customers/cursor

### Books

- Winters, T., et al. "Software Engineering at Google." Chapter 17: Code Search.
  https://abseil.io/resources/swe-book/html/ch17.html
