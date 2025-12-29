# C++ Symbol Reference Discovery Test Plan: Eigen Codebase

**Document:** 015-shebe-cpp-accuracy-test-plan-01.md <br>
**Created:** 2025-12-28 <br>
**Purpose:** Comparative evaluation of refactoring tools for C++ symbol discovery <br>

## Research Question

**How does Shebe's `find_references` refactoring approach compare to alternatives?**

Specifically, when a developer needs to rename or modify a C++ symbol:
- Which tool finds the most complete set of references to update?
- Which tool has the fewest false positives?
- Which tool provides the most useful output for the refactoring workflow?

## Hypothesis

**Even if Shebe misses semantic references (templates, macros, type aliases, ADL), concise
file:line output enables faster iteration than alternatives.**

The bet: What Shebe lacks in semantic completeness, it compensates for with:

1. **Token efficiency** - ~50-70 tokens per reference vs verbose grep output
2. **Confidence ranking** - High-confidence results first, reducing review burden
3. **Iteration speed** - Claude can quickly read flagged locations and find related refs

This hypothesis predicts:
- Shebe will have lower recall than grep on first pass
- But Shebe + Claude iteration will reach equivalent coverage faster (fewer tokens consumed)
- Serena may have higher precision but slower setup/query overhead

## Executive Summary

This test plan evaluates reference discovery tools on their ability to answer the core
refactoring question: **"What are all the references I need to update?"**

The test uses the Eigen C++ library as a challenging benchmark due to its extensive use of
templates, macros and type aliases. The same methodology will be applied to three approaches:

| Approach | Tool | Method |
|----------|------|--------|
| **Shebe** | `mcp__shebe__find_references` | BM25 text search + pattern heuristics |
| **grep** | `grep -rn` / `rg` via Bash | Exact text matching |
| **Serena** | `mcp__serena__find_referencing_symbols` | LSP-based semantic analysis |

Results will be documented separately for each tool.

## Tool Under Test: find_references

### Purpose

The `find_references` tool is a **discovery** tool for the pre-refactoring phase. It
enumerates locations efficiently (~50-70 tokens per reference) so developers know what
needs to change before making modifications.

### Key Parameters

| Parameter | Description | Test Values |
|-----------|-------------|-------------|
| `symbol` | Symbol name to find references for | See test symbols |
| `session` | Indexed session ID | `eigen` |
| `symbol_type` | Hint for filtering (function, type, variable, constant, any) | Varies by symbol |
| `defined_in` | File where symbol is defined (excluded from results) | Optional |
| `max_results` | Maximum references to return | 50, 100, 200 |
| `context_lines` | Lines of context around each reference | 2 |

### Output Structure

The tool returns:
- Confidence levels: High (>=0.80), Medium (0.50-0.79), Low (<0.50)
- Pattern classifications: function_call, generic_type, type_annotation, variable
- "Files to update" list with high-confidence references grouped first
- Code context around each reference

## Test Codebase: Eigen

- **Repository:** /home/orodha/gitlab/libeigen/eigen
- **Session:** `eigen`
- **Files:** 1,919
- **Chunks:** 40,458
- **Index Size:** 15.40 MB

### Why Eigen Tests find_references

Eigen challenges reference discovery with:

1. **Template parameters** - `Matrix<Scalar, Rows, Cols>` uses `Scalar` as both type and value
2. **Macro-generated symbols** - `MatrixXd` created by `EIGEN_MAKE_TYPEDEFS`
3. **CRTP base classes** - `PlainObjectBase<Derived>` referenced through inheritance
4. **Generic names** - `traits`, `Index`, `Scalar` appear in many unrelated contexts
5. **Namespaced symbols** - `Eigen::internal::traits` vs `std::traits`

## Test Categories

### Category A: Distinct Symbols (Low Ambiguity)

Symbols with unique names unlikely to cause false positives.

| Symbol | Type | symbol_type | Expected Challenge |
|--------|------|-------------|-------------------|
| `MatrixXd` | typedef | type | Macro-generated, many usages |
| `CwiseBinaryOp` | class template | type | Expression template, technical |
| `PlainObjectBase` | class template | type | CRTP base, inheritance refs |
| `EIGEN_DEVICE_FUNC` | macro | any | Attribute macro, high frequency |

### Category B: Generic Symbols (High Ambiguity)

Symbols with common names likely to match unrelated code.

| Symbol | Type | symbol_type | Expected Challenge |
|--------|------|-------------|-------------------|
| `traits` | struct template | type | Generic name, many contexts |
| `Index` | typedef | type | Common word, namespace collision |
| `Scalar` | template param | type | Ubiquitous in math code |
| `Dynamic` | constant | constant | Common word |

### Category C: Hierarchical Symbols

Symbols that participate in type hierarchies.

| Symbol | Type | symbol_type | Expected Challenge |
|--------|------|-------------|-------------------|
| `DenseBase` | class template | type | Base class, inherited members |
| `Vector3d` | typedef | type | Derived from Matrix, less common |

## Test Execution Plan

### Prerequisites

Verify Eigen session exists:

```
MCP Tool: mcp__shebe__list_sessions
Expected: eigen session with ~1,919 files, ~40,458 chunks
```

### Phase 1: Ground Truth Collection

For each symbol, establish grep baseline:

```bash
grep -rn "SYMBOL" /home/orodha/gitlab/libeigen/eigen \
    --include="*.h" --include="*.cpp" --include="*.hpp"
```

Record:
- Total lines matching
- Unique files matching
- Sample of match contexts

### Phase 2: find_references Tests

#### Test 2.1: Basic Reference Discovery

For each Category A symbol:

```
MCP Tool: mcp__shebe__find_references
Parameters:
  - symbol: "MatrixXd"
  - session: eigen
  - symbol_type: type
  - max_results: 100
  - context_lines: 2
```

Record:
- Total references found
- Confidence distribution (High/Medium/Low counts)
- Pattern distribution (function_call, generic_type, type_annotation, variable)
- Unique files in results

#### Test 2.2: Ambiguity Handling

For each Category B symbol:

```
MCP Tool: mcp__shebe__find_references
Parameters:
  - symbol: "traits"
  - session: eigen
  - symbol_type: type
  - max_results: 100
  - context_lines: 2
```

Evaluate:
- False positive rate (references to unrelated `traits`)
- Confidence calibration (do low-confidence results correlate with false positives?)
- Pattern classification accuracy

#### Test 2.3: Definition Exclusion

Test the `defined_in` parameter:

```
MCP Tool: mcp__shebe__find_references
Parameters:
  - symbol: "MatrixXd"
  - session: eigen
  - symbol_type: type
  - defined_in: "Eigen/src/Core/Matrix.h"
  - max_results: 100
```

Verify:
- Definition file is excluded from results
- Reference count drops appropriately

#### Test 2.4: symbol_type Filtering

Compare results with different symbol_type hints:

```
# As type
mcp__shebe__find_references(symbol="Index", symbol_type="type", ...)

# As variable
mcp__shebe__find_references(symbol="Index", symbol_type="variable", ...)

# As any
mcp__shebe__find_references(symbol="Index", symbol_type="any", ...)
```

Measure:
- Result count differences
- Precision improvements with correct hint
- False positive reduction

#### Test 2.5: max_results Scaling

Test result completeness at different limits:

```
mcp__shebe__find_references(symbol="EIGEN_DEVICE_FUNC", max_results=50, ...)
mcp__shebe__find_references(symbol="EIGEN_DEVICE_FUNC", max_results=100, ...)
mcp__shebe__find_references(symbol="EIGEN_DEVICE_FUNC", max_results=200, ...)
```

Evaluate:
- Are results ranked by confidence?
- Does increasing max_results add mostly low-confidence results?

#### Test 2.6: Iteration Efficiency (Hypothesis Test)

Test whether concise output enables faster coverage through iteration:

**Scenario:** Find all references to `MatrixXd` including semantic relationships

**Shebe iteration workflow:**
1. Run `find_references(symbol="MatrixXd", max_results=50)`
2. Record: tokens consumed, files identified
3. From high-confidence results, identify related symbols (e.g., `Matrix`, `EIGEN_MAKE_TYPEDEFS`)
4. Run follow-up queries for related symbols
5. Record: cumulative tokens, cumulative files discovered
6. Repeat until no new files found

**grep workflow:**
1. Run `grep -rn "MatrixXd" ...`
2. Record: output size (tokens), files identified
3. Parse output to identify related patterns
4. Run follow-up greps
5. Record: cumulative tokens, cumulative files

**Metrics to compare:**
- Tokens consumed to reach X% file coverage
- Number of tool invocations to reach X% coverage
- Time to actionable "files to update" list

### Phase 3: Precision Validation

For each symbol, validate a sample of results:

1. **Select 5 high-confidence results randomly**
2. **Read the referenced file** using `mcp__shebe__read_file` or `Read` tool
3. **Manually verify** if the match is a true reference to the symbol
4. **Calculate sampled precision** = true positives / 5

Validation criteria:
- True Positive: Reference actually uses the symbol being searched
- False Positive: Match is coincidental (e.g., substring, different namespace)

### Phase 4: Coverage Analysis

Compare find_references results to grep baseline:

1. **Extract unique files** from find_references results
2. **Extract unique files** from grep results
3. **Calculate file coverage** = find_references_files / grep_files
4. **Identify gaps** - files in grep but not in find_references

## Metrics Framework

### Comparison Dimensions

The three approaches will be compared on:

1. **Completeness** - Does the tool find all references that need updating?
2. **Precision** - Are the returned results actually references (not false positives)?
3. **Usability** - Is the output actionable for the refactoring workflow?

### Primary Metrics

| Metric | Formula | Measures |
|--------|---------|----------|
| **Recall (File Coverage)** | tool_files / grep_files | Completeness |
| **Sampled Precision** | true_positives / sampled_results | Precision |
| **Confidence Calibration** | correlation(confidence, is_true_positive) | Usability |

### Secondary Metrics

| Metric | Description | Measures |
|--------|-------------|----------|
| **Output Efficiency** | Tokens per useful reference | Usability |
| **Ranking Quality** | True positives ranked higher? | Usability |
| **Setup Overhead** | Time/effort to enable the tool | Usability |

### Iteration Efficiency Metrics (Hypothesis Test)

| Metric | Description |
|--------|-------------|
| **Tokens to 80% coverage** | Cumulative tokens consumed to find 80% of grep baseline files |
| **Queries to 80% coverage** | Number of tool invocations to reach 80% coverage |
| **First-pass coverage** | % of files found in initial query (before iteration) |
| **Iteration multiplier** | Final coverage / first-pass coverage |

### Approach-Specific Considerations

| Approach | Unique Strengths | Unique Weaknesses |
|----------|------------------|-------------------|
| **Shebe** | Confidence scoring, concise output (~50-70 tokens/ref) | Requires indexing, text-only |
| **grep** | No setup, exhaustive, exact matching | Verbose output, no ranking |
| **Serena** | True semantic analysis, type-aware | Requires LSP server, setup overhead |

### Hypothesis Predictions

| Metric | Shebe | grep | Serena |
|--------|-------|------|--------|
| First-pass recall | Lower | Highest | Medium |
| Tokens per reference | Lowest | Highest | Medium |
| Queries to 80% coverage | Medium | Fewest | Most |
| Tokens to 80% coverage | **Lowest** | Highest | Medium |

## Test Results Template

For each symbol:

```markdown
## Symbol: [NAME]

### Configuration
- symbol_type: ___
- max_results: ___
- defined_in: ___ (if used)

### Ground Truth (grep)
- Lines matching: ___
- Files matching: ___

### find_references Results
- Total references: ___
- Confidence distribution:
  - High (>=0.80): ___
  - Medium (0.50-0.79): ___
  - Low (<0.50): ___
- Pattern distribution:
  - function_call: ___
  - generic_type: ___
  - type_annotation: ___
  - variable: ___
- Unique files: ___

### Precision Validation (5 samples)
| # | File | Line | Confidence | True Positive? |
|---|------|------|------------|----------------|
| 1 | | | | |
| 2 | | | | |
| 3 | | | | |
| 4 | | | | |
| 5 | | | | |

Sampled precision: ___/5 = ___%

### Calculated Metrics
- File coverage: ___ / ___ = ___%
- Ranking quality: ___
```

## Test Symbols Summary

| Symbol | Category | symbol_type | Ground Truth Files | Notes |
|--------|----------|-------------|-------------------|-------|
| `MatrixXd` | A | type | 125 | Primary test case |
| `CwiseBinaryOp` | A | type | 44 | Expression template |
| `PlainObjectBase` | A | type | 15 | CRTP base |
| `EIGEN_DEVICE_FUNC` | A | any | 246 | High frequency macro |
| `Vector3d` | C | type | 31 | Derived typedef |
| `DenseBase` | C | type | 53 | Hierarchy base |
| `traits` | B | type | 214 | Generic name |
| `Index` | B | type | 499 | Common word |
| `Scalar` | B | type | 540 | Ubiquitous |
| `Dynamic` | B | constant | 375 | Common word |

## Appendix A: Confidence Level Interpretation

From tool documentation:

- **High (>=0.80):** Very likely a real reference, should be updated
- **Medium (0.50-0.79):** Probable reference, review before updating
- **Low (<0.50):** Possible false positive (comments, strings, docs)

## Appendix B: Pattern Classifications

| Pattern | Matches | Example |
|---------|---------|---------|
| `function_call` | symbol(), .symbol() | `MatrixXd()`, `m.transpose()` |
| `generic_type` | <symbol>, template args | `Matrix<Scalar, ...>` |
| `type_annotation` | : symbol, type position | `const MatrixXd&` |
| `variable` | Assignments, property access | `MatrixXd m = ...` |

## Appendix C: Eigen Type Hierarchy Reference

```
EigenBase<Derived>
    |
    +-- DenseBase<Derived>
            |
            +-- DenseCoeffsBase<Derived>
                    |
                    +-- MatrixBase<Derived>
                    |       |
                    |       +-- PlainObjectBase<Matrix<...>>
                    |               |
                    |               +-- Matrix<Scalar, Rows, Cols, ...>
                    |
                    +-- ArrayBase<Derived>

Expression Types:
    CwiseBinaryOp<BinaryOp, Lhs, Rhs>
    CwiseUnaryOp<UnaryOp, Xpr>
    Block<Xpr, Rows, Cols>
    Transpose<Xpr>
```

## Test Execution Order

1. **Shebe:** Execute tests, document in `015-shebe-cpp-accuracy-results-02.md`
2. **grep/ripgrep:** Execute tests, document in `015-grep-cpp-accuracy-results-03.md`
3. **Serena:** Execute tests, document in `015-serena-cpp-accuracy-results-04.md`
4. **Comparison:** Summarize findings in `015-cpp-accuracy-comparison-05.md`
