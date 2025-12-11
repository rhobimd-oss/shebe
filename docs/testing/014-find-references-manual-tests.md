# Manual Test Plan: find_references Tool

**Document:** 014-find-references-manual-tests.md <br>
**Related:** dev-docs/work-plans/014-find-references-tool-01.md (Phase 4.6) <br>
**Status:** Ready for Testing <br>

## Overview

Manual end-to-end tests for the `find_references` MCP tool using three real-world codebases
of varying size, language and complexity:

| Repository       | Language   | Size                      | Complexity     |
|------------------|------------|---------------------------|----------------|
| steveyegge/beads | Go         | Small (~50 files)         | Single package |
| openemr/openemr  | PHP        | Large (~5000 files)       | Enterprise app |
| istio/istio      | Go + YAML  | Very Large (~5700 files)  | Polyglot       |

### Istio Repository Composition

The Istio repo provides an interesting polyglot test case:

| File Type  | Count  | LOC     | Notes                 |
|------------|--------|---------|-----------------------|
| Go         | 1,853  | 500,011 | Core code             |
| YAML       | 2,482  | 154,780 | 56% are release notes |
| Proto      | 69     | -       | API definitions       |
| Markdown   | 88     | -       | Documentation         |

Two test sessions are used for comparative analysis:
- **istio-pilot**: Narrow scope (pilot/ only) - 510 Go + 195 YAML files
- **istio-full**: Broad scope (full repo) - tests polyglot search quality

## Prerequisites

1. Shebe MCP server running
2. Claude Code connected to Shebe
3. Repositories available at `~/github/`

## Test Session Setup

Before running tests, index each repository:

```
# Small repo (beads)
shebe-mcp index_repository path=~/github/steveyegge/beads session=beads-test

# Medium repo (OpenEMR) - use subset for faster indexing
shebe-mcp index_repository path=~/github/openemr/openemr/library session=openemr-lib

# Large repo (Istio) - NARROW scope (pilot/ only, Go-focused)
shebe-mcp index_repository path=~/github/istio/istio/pilot session=istio-pilot

# Large repo (Istio) - BROAD scope (full repo, polyglot)
shebe-mcp index_repository path=~/github/istio/istio session=istio-full
```

**Indexing Time Estimates:**
- beads-test: ~2 seconds
- openemr-lib: ~10 seconds
- istio-pilot: ~30 seconds
- istio-full: ~3 minutes (includes 2400+ YAML files)

---

## Test Cases

### Category 1: Small Repository (beads)

#### TC-1.1: Function with Tests

**Symbol:** `FindDatabasePath`
**Type:** function
**Expected:** Definition in beads.go + test in beads_test.go

```json
{
  "symbol": "FindDatabasePath",
  "session": "beads-test",
  "symbol_type": "function"
}
```

**Verify:**
- [ ] High confidence for function definition in beads.go
- [ ] High confidence for test function TestFindDatabasePath
- [ ] Output groups results by confidence level
- [ ] Session timestamp displayed

#### TC-1.2: Type Reference

**Symbol:** `Storage`
**Type:** type
**Expected:** Interface definition + implementations

```json
{
  "symbol": "Storage",
  "session": "beads-test",
  "symbol_type": "type"
}
```

**Verify:**
- [ ] Type annotation patterns matched (: Storage, Storage interface)
- [ ] Constructor functions matched (NewSQLiteStorage returns Storage)

#### TC-1.3: Short Symbol

**Symbol:** `db`
**Type:** variable
**Expected:** Many matches with varying confidence

```json
{
  "symbol": "db",
  "session": "beads-test",
  "symbol_type": "variable",
  "max_results": 20
}
```

**Verify:**
- [ ] Results limited to max_results
- [ ] Highest confidence results shown first
- [ ] Deduplication working (one result per line)

---

### Category 2: Large Repository (OpenEMR)

#### TC-2.1: PHP Function Search

**Symbol:** `sqlQuery`
**Type:** function
**Expected:** Many references across library files

```json
{
  "symbol": "sqlQuery",
  "session": "openemr-lib",
  "symbol_type": "function",
  "max_results": 50
}
```

**Verify:**
- [ ] Function call pattern matches `sqlQuery(`
- [ ] Results from multiple PHP files
- [ ] Context lines show surrounding code

#### TC-2.2: Comment Detection

**Symbol:** `ADODB`
**Type:** any
**Expected:** Mix of code and comment references

```json
{
  "symbol": "ADODB",
  "session": "openemr-lib"
}
```

**Verify:**
- [ ] References in comments have LOWER confidence
- [ ] References in code have HIGHER confidence
- [ ] Proper confidence grouping (high/medium/low)

#### TC-2.3: No Matches

**Symbol:** `nonexistent_xyz_function_12345`
**Type:** function
**Expected:** No references found message

```json
{
  "symbol": "nonexistent_xyz_function_12345",
  "session": "openemr-lib"
}
```

**Verify:**
- [ ] "No references found" message displayed
- [ ] Session timestamp still shown
- [ ] No error thrown

#### TC-2.4: defined_in Exclusion

**Symbol:** `amcAdd`
**Type:** function
**Defined in:** amc.inc.php

```json
{
  "symbol": "amcAdd",
  "session": "openemr-lib",
  "symbol_type": "function",
  "defined_in": "amc.inc.php",
  "include_definition": false
}
```

**Verify:**
- [ ] Definition file (amc.inc.php) NOT in results
- [ ] Only call sites shown

---

### Category 3: Very Large Repository (Istio)

#### TC-3.1: Go Type Search

**Symbol:** `AuthorizationPolicy`
**Type:** type
**Expected:** Struct definition + usages across pilot package

```json
{
  "symbol": "AuthorizationPolicy",
  "session": "istio-pilot",
  "symbol_type": "type"
}
```

**Verify:**
- [ ] Type definition matched
- [ ] Type annotations matched (: AuthorizationPolicy)
- [ ] Generic type usages matched (<AuthorizationPolicy>)
- [ ] Struct instantiation matched (AuthorizationPolicy{})

#### TC-3.2: Go Method Search

**Symbol:** `DeepCopy`
**Type:** function
**Expected:** Multiple implementations across types

```json
{
  "symbol": "DeepCopy",
  "session": "istio-pilot",
  "symbol_type": "function",
  "max_results": 30
}
```

**Verify:**
- [ ] Method definitions matched (.DeepCopy)
- [ ] Method calls matched
- [ ] Multiple types have DeepCopy methods

#### TC-3.3: Import Pattern

**Symbol:** `cluster`
**Type:** any
**Expected:** Package imports and usages

```json
{
  "symbol": "cluster",
  "session": "istio-pilot"
}
```

**Verify:**
- [ ] Import statements matched with high confidence
- [ ] Package prefix usages matched (cluster.ID)

#### TC-3.4: Test File Boost

**Symbol:** `AddressMap`
**Type:** type
**Expected:** Higher confidence in test files

```json
{
  "symbol": "AddressMap",
  "session": "istio-pilot",
  "symbol_type": "type"
}
```

**Verify:**
- [ ] addressmap_test.go references have +0.05 confidence boost
- [ ] Test file references clearly identified
- [ ] Both definition and test files included

---

### Category 4: Edge Cases

#### TC-4.1: Symbol with Dots

**Symbol:** `context.Context`
**Type:** type

```json
{
  "symbol": "context.Context",
  "session": "istio-pilot",
  "symbol_type": "type"
}
```

**Verify:**
- [ ] Dot is treated literally (not regex wildcard)
- [ ] Matches exact string "context.Context"

#### TC-4.2: Context Lines Boundary

**Symbol:** `FindBeadsDir`
**Context lines:** 0

```json
{
  "symbol": "FindBeadsDir",
  "session": "beads-test",
  "context_lines": 0
}
```

**Verify:**
- [ ] Only the matching line shown (no context)
- [ ] Line numbers still accurate

#### TC-4.3: Maximum Context

**Symbol:** `FindBeadsDir`
**Context lines:** 10

```json
{
  "symbol": "FindBeadsDir",
  "session": "beads-test",
  "context_lines": 10
}
```

**Verify:**
- [ ] Up to 21 lines shown (10 before + match + 10 after)
- [ ] Handles file boundaries gracefully

#### TC-4.4: Single Result Limit

**Symbol:** `AuthorizationPolicies`
**Max results:** 1

```json
{
  "symbol": "AuthorizationPolicies",
  "session": "istio-pilot",
  "max_results": 1
}
```

**Verify:**
- [ ] Exactly 1 result returned
- [ ] Highest confidence result selected

---

### Category 5: Polyglot Comparison (Narrow vs Broad Istio)

Tests comparing the same symbol searches across narrow (Go-focused) and broad (polyglot)
indexing strategies. This informs Shebe's utility as a polyglot search tool.

#### TC-5.1: Go Symbol - Narrow vs Broad

**Symbol:** `AuthorizationPolicy`
**Sessions:** istio-pilot (narrow) vs istio-full (broad)

**Narrow search:**
```json
{
  "symbol": "AuthorizationPolicy",
  "session": "istio-pilot",
  "symbol_type": "type",
  "max_results": 50
}
```

**Broad search:**
```json
{
  "symbol": "AuthorizationPolicy",
  "session": "istio-full",
  "symbol_type": "type",
  "max_results": 50
}
```

**Compare:**
- [ ] Narrow: All results are Go code references
- [ ] Broad: Results include YAML config references (kind: AuthorizationPolicy)
- [ ] Broad: YAML references have LOWER confidence than Go code
- [ ] Record: result count difference (narrow vs broad)
- [ ] Record: performance difference (narrow vs broad)

**Expected Insight:** Broad search finds config files referencing the type,
useful for understanding full usage but with more noise.

#### TC-5.2: Cross-Language Symbol

**Symbol:** `istio`
**Sessions:** istio-pilot vs istio-full

**Narrow search:**
```json
{
  "symbol": "istio",
  "session": "istio-pilot",
  "max_results": 30
}
```

**Broad search:**
```json
{
  "symbol": "istio",
  "session": "istio-full",
  "max_results": 30
}
```

**Compare:**
- [ ] Narrow: References in Go imports, package paths
- [ ] Broad: Also includes YAML metadata, proto packages, markdown docs
- [ ] Record: file type distribution in results
- [ ] Observe: confidence scoring across file types

**Expected Insight:** Common terms appear across all file types;
confidence scoring should prioritize code over config/docs.

#### TC-5.3: YAML-Only Symbol

**Symbol:** `kind: VirtualService`
**Sessions:** istio-pilot vs istio-full

**Narrow search:**
```json
{
  "symbol": "VirtualService",
  "session": "istio-pilot"
}
```

**Broad search:**
```json
{
  "symbol": "VirtualService",
  "session": "istio-full"
}
```

**Compare:**
- [ ] Narrow: Primarily Go struct/type references
- [ ] Broad: Also finds YAML CRD definitions in manifests/samples
- [ ] Record: YAML vs Go result ratio
- [ ] Observe: Are YAML config references useful or noise?

**Expected Insight:** For Kubernetes resources, broad search finds
both the Go implementation AND the YAML usage examples.

#### TC-5.4: Release Notes Noise Test

**Symbol:** `bug-fix`
**Sessions:** istio-full only

```json
{
  "symbol": "bug-fix",
  "session": "istio-full",
  "max_results": 50
}
```

**Verify:**
- [ ] Results dominated by releasenotes/*.yaml files
- [ ] Low confidence due to YAML file type penalty
- [ ] Demonstrates release notes as search noise
- [ ] Consider: Should releasenotes/ be excluded by default?

**Expected Insight:** The 1400 release note YAML files contribute
noise for generic terms; may warrant exclude pattern recommendation.

#### TC-5.5: Performance Comparison

Run the same search on both sessions and compare performance:

**Symbol:** `Service`
**Type:** type

| Metric            | istio-pilot  | istio-full  | Delta  |
|-------------------|--------------|-------------|--------|
| Results count     |              |             |        |
| Search time (ms)  |              |             |        |
| High confidence % |              |             |        |
| Go file results   |              |             |        |
| YAML file results |              |             |        |

**Verify:**
- [ ] Broad search is slower (more files to scan)
- [ ] Broad search returns more results
- [ ] High confidence % is LOWER in broad (more noise)

---

### Category 5 Summary Questions

After completing Category 5 tests, answer:

1. **Signal-to-Noise Ratio:** Does broad indexing hurt search quality?
2. **Cross-Language Value:** Are YAML/config references useful or noise?
3. **Performance Impact:** Is the broad index acceptably fast?
4. **Recommendation:** Should users prefer narrow or broad indexing?

---

## Performance Benchmarks

Track execution time for each test:

| Test                         | Session     | Expected Time  | Actual Time  | Status  |
|------------------------------|-------------|----------------|--------------|---------|
| TC-1.1 (small, simple)       | beads-test  | < 100ms        |              |         |
| TC-2.1 (large, many matches) | openemr-lib | < 500ms        |              |         |
| TC-3.1 (type search)         | istio-pilot | < 500ms        |              |         |
| TC-3.2 (method search)       | istio-pilot | < 500ms        |              |         |
| TC-5.1 narrow                | istio-pilot | < 500ms        |              |         |
| TC-5.1 broad                 | istio-full  | < 2000ms       |              |         |
| TC-5.5 narrow                | istio-pilot | < 500ms        |              |         |
| TC-5.5 broad                 | istio-full  | < 2000ms       |              |         |

**Performance Pass Criteria:**
- Small repo searches: < 200ms
- Narrow scope (pilot): < 500ms
- Broad scope (full): < 2000ms
- No search exceeds 5000ms

---

## Output Format Verification

For each test, verify the output format:

```markdown
## References to `{symbol}` ({count} found)

### High Confidence ({count})

#### {file_path}:{line_number}
```{language}
{context_lines}
```
- **Pattern:** {pattern_name}
- **Confidence:** {score}

### Medium Confidence ({count})
...

### Low Confidence ({count})
...

---

**Summary:**
- High confidence: {n} references
- Medium confidence: {n} references
- Low confidence: {n} references
- Total files: {n}
- Session indexed: {timestamp} ({relative_time})

**Files to update:**
- `{file1}`
- `{file2}`
```

---

## Test Execution Log

| Test ID | Date | Tester | Result | Notes |
|---------|------|--------|--------|-------|
| TC-1.1 | | | | |
| TC-1.2 | | | | |
| TC-1.3 | | | | |
| TC-2.1 | | | | |
| TC-2.2 | | | | |
| TC-2.3 | | | | |
| TC-2.4 | | | | |
| TC-3.1 | | | | |
| TC-3.2 | | | | |
| TC-3.3 | | | | |
| TC-3.4 | | | | |
| TC-4.1 | | | | |
| TC-4.2 | | | | |
| TC-4.3 | | | | |
| TC-4.4 | | | | |
| TC-5.1 (narrow) | | | | |
| TC-5.1 (broad) | | | | |
| TC-5.2 (narrow) | | | | |
| TC-5.2 (broad) | | | | |
| TC-5.3 (narrow) | | | | |
| TC-5.3 (broad) | | | | |
| TC-5.4 | | | | |
| TC-5.5 | | | | |

**Result Legend:** PASS | FAIL | SKIP | BLOCKED

---

## Success Criteria

All tests must pass for Phase 4.6 completion:

1. **Functional (20 test scenarios)**
   - Categories 1-4: 15 basic test cases pass
   - Category 5: 5 polyglot comparison tests completed
   - No crashes or unhandled errors

2. **Output Quality**
   - Markdown format renders correctly
   - Line numbers are accurate
   - Context extraction is correct

3. **Performance**
   - Narrow scope searches: < 500ms
   - Broad scope searches: < 2000ms
   - No search exceeds 5000ms

4. **Accuracy**
   - High confidence results are true positives
   - Comments/strings correctly penalized
   - Test files correctly boosted

5. **Polyglot Assessment (Category 5)**
   - Document signal-to-noise findings
   - Provide narrow vs broad indexing recommendation
   - Identify any exclude pattern recommendations

---

## Known Limitations

Document any discovered limitations:

1. Pattern-based (not AST) - may have false positives
2. Chunk-based search - very long files may have duplicate matches
3. Requires re-indexing if files change
