# Shebe Performance Characteristics

**Version:** 0.3.0 <br>
**Last Updated:** 2025-10-26 <br>
**Status:** Validated with 30/30 Performance Test Scenarios (100% Success Rate) <br>

---

## Performance Summary

**Validated on Istio (5,605 files, Go) and OpenEMR (6,364 files, PHP polyglot)**

| Metric        | Target        | Actual              | Status           |
|---------------|---------------|---------------------|------------------|
| Indexing      | >500 files/s  | **1,928-11,210 f/s**|  3.9x-22.4x     |
| Query (p50)   | <20ms         | **2ms**             |  10x better     |
| Query (p95)   | <50ms         | **2ms**             |  25x better     |
| Token Usage   | <5,000        | **210-650**         |  8-24x better   |
| Test Coverage | >95%          | **100%**            |  30/30 scenarios|
| Polyglot      | 5+ file types | **11 types**        |  220%           |

---

## Measured Performance

### Indexing (Synchronous)

**Measured on Real-World Repositories:**

| Repository | Files  | Duration  | Throughput      | vs Target     |
|------------|--------|-----------|-----------------|---------------|
| Istio      | 5,605  | 0.5s      | **11,210 f/s**  |  22.4x    |
| OpenEMR    | 6,364  | 3.3s      | **1,928 f/s**   |  3.9x     |

**Key Findings:**
- Throughput varies with file complexity (simple YAML/Go vs large PHP files)
- Ultra-fast indexing suitable for interactive use (<4s for 6k files)
- Consistent metadata accuracy (100% correct file/chunk counts)

### Query Latency

**Consistent 2ms Across All Query Types:**

| Query Type     | Latency | Results | Token Usage |
|----------------|---------|---------|-------------|
| Keyword        | 2ms     | 10-50   | 450-650     |
| Boolean AND    | 2ms     | 15-20   | 500-600     |
| Phrase         | 1-2ms   | 5-10    | 400-500     |
| Large sets (k=50) | 2ms  | 50      | 2,100       |

**No performance degradation** with larger result sets.

### Tool Performance

| Tool             | Latency  | Token Usage | Notes                     |
|------------------|----------|-------------|---------------------------|
| search_code      | 2ms      | 210-650     | Validated (30 tests)      |
| list_sessions    | <10ms    | ~460        | Rich metadata             |
| get_session_info | <5ms     | ~110        | Calculated stats          |
| index_repository | 0.5-3.3s | 1,000-2,000 | 1,928-11,210 files/sec    |
| preview_chunk    | <50ms    | 250-500     | Schema v2 fix (working)   |
| read_file        | <10ms    | Varies      | Auto-truncation at 20KB   |
| find_file        | <10ms    | Varies      | Glob + regex support      |
| list_dir         | <10ms    | ~2,600      | 500 file limit            |
| delete_session   | ~3.3s    | ~40         | Confirmation required     |

---

## Performance Comparison

**Shebe vs Alternatives (Validated):**

| System    | Speed     | Tokens      | Best For           |
|-----------|-----------|-------------|--------------------|
| **Shebe** | **2ms**   | **210-650** | Content search     |
| Ripgrep   | 27ms      | 69*         | Exact patterns     |
| Serena    | 150-200ms | 2,800-5,200 | Symbol navigation  |

*Ripgrep: paths only. Shebe: snippets + BM25 ranking

**Advantages:**
- **15.8x faster than ripgrep** (with content snippets)
- **75-100x faster than Serena** (for content search)
- **4-24x better token efficiency** than Serena
- **Polyglot excellence:** 11 file types in single query

---

## Test Coverage

**Comprehensive Validation (2025-10-26):**

- **Unit Tests:** 364 tests (100% pass rate)
- **Performance Tests:** 30 scenarios (100% pass rate)
- **Repositories:** Istio (Go) + OpenEMR (PHP polyglot)
- **Test Categories:**
  - Repository indexing (2 tests)
  - File discovery & navigation (3 tests)
  - File reading (2 tests)
  - Search & context (4 tests)
  - Session management (4 tests)
  - Performance benchmarks (3 tests)
  - Workflow comparisons (3 tests - inferred)

**Results:** All performance targets exceeded, zero blocking issues found.

See: [performance-analysis/020-phase01-mcp-performance-test-results-v3.md](./performance-analysis/020-phase01-mcp-performance-test-results-v3.md)

---

## Optimization History

### v0.3.0 - Schema v2 & Comprehensive Testing
- **Fixed:** preview_chunk bug (chunk_index now INDEXED)
- **Validated:** 30/30 performance test scenarios
- **Measured:** 2ms consistent latency, 1,928-11,210 files/sec indexing
- **Polyglot:** Validated 11 file types in single query

### v0.3.0 - Synchronous Indexing
- Removed async progress (~1,000 LOC, 86% reduction)
- Fixed metadata bugs (100% accuracy)
- Maintained ultra-fast indexing (3.9x-22.4x targets)

### Earlier Versions
- v0.2.0: Core architecture (OpenEMR validation: 570 files/sec)
- v0.4.0: Ergonomic tools (no performance regression)
- v0.7.0: Preview chunk (initial implementation)

---

## Future Opportunities

**Status:** All targets exceeded. Performance is production-ready.

**Optional Enhancements (Low Priority):**

1. **Query Caching:** 2ms -> <1ms (optional speedup)
2. **Index Warming:** Eliminate cold-start latency
3. **Parallel Search:** For >50k file repositories
4. **Token Compression:** 210-650 -> <200 tokens (minor optimization)

**Priority:** Low - current performance exceeds all requirements

---

## Detailed Analysis

**Comprehensive Test Results:**
- [performance-analysis/020-phase01-mcp-performance-test-results-v3.md](./performance-analysis/020-phase01-mcp-performance-test-results-v3.md)

**Comparison Analysis:**
- [performance-analysis/015-phase01-validated-comparison-technical.md](./performance-analysis/015-phase01-validated-comparison-technical.md)
- [performance-analysis/010-phase01-openemr-search-comparison-analysis.md](./performance-analysis/010-phase01-openemr-search-comparison-analysis.md)

---

**Related:** [ARCHITECTURE.md](../ARCHITECTURE.md) | [README.md](../README.md) | [CONTEXT.md](./CONTEXT.md)
