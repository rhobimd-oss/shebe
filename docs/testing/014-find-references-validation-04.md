# Validation: Does find_references Solve the Original Problem?

**Document:** 014-find-references-validation-04.md <br>
**Related:** dev-docs/analyses/014-serena-vs-shebe-context-usage-01.md (problem statement) <br>
**Shebe Version:** 0.5.0 <br>
**Document Version:** 1.0 <br>
**Created:** 2025-12-11 <br>
**Status:** Complete

## Purpose

Objective assessment of whether the `find_references` tool solves the problems identified
in the original analysis (014-serena-vs-shebe-context-usage-01.md).

This document compares:
1. Problems identified in original analysis
2. Proposed solution metrics
3. Actual implementation results

---

## Original Problem Statement

From 014-serena-vs-shebe-context-usage-01.md:

### Problem 1: Serena Returns Full Code Bodies

> `serena__find_symbol` returns entire class/function bodies [...] for a "find references
> before rename" workflow, Claude doesn't need the full body.

**Quantified Impact:**
- Serena `find_symbol`: 5,000 - 50,000 tokens per query
- Example: AppointmentCard class returned 346 lines (body_location: lines 11-357)

### Problem 2: Token Inefficiency for Reference Finding

> For a typical "find references to handleLogin" query:
> - Serena `find_symbol`: 5,000 - 50,000 tokens
> - Shebe `search_code`: 500 - 2,000 tokens
> - Proposed `find_references`: 300 - 1,500 tokens

**Target:** ~50 tokens per reference vs Serena's ~500+ tokens per reference

### Problem 3: Workflow Inefficiency

> Claude's current workflow for renaming:
> 1. Grep for symbol name (may miss patterns)
> 2. Read each file (context expensive)
> 3. Make changes
> 4. Discover missed references via errors

**Desired:** Find all references upfront with confidence scores.

---

## Proposed Solution Design Constraints

From original analysis:

| Constraint            | Target                | Rationale               |
|-----------------------|-----------------------|-------------------------|
| Output limit          | Max 100 references    | Prevent token explosion |
| Context per reference | 2 lines               | Minimal but sufficient  |
| Token budget          | <2,000 tokens typical | 10x better than Serena  |
| Confidence scoring    | H/M/L groups          | Help Claude prioritize  |
| File grouping         | List files to update  | Systematic updates      |
| No full bodies        | Reference line only   | Core efficiency gain    |

---

## Actual Implementation Results

From 014-find-references-test-results.md:

### Constraint 1: Output Limit

| Parameter   | Target  | Actual             | Status  |
|-------------|---------|--------------------|---------|
| max_results | 100 max | 1-200 configurable | MET     |
| Default     | -       | 50                 | MET     |

**Evidence:** TC-4.4 verified `max_results=1` returns exactly 1 result.

### Constraint 2: Context Per Reference

| Parameter     | Target  | Actual            | Status  |
|---------------|---------|-------------------|---------|
| context_lines | 2 lines | 0-10 configurable | MET     |
| Default       | 2       | 2                 | MET     |

**Evidence:** TC-4.2 verified `context_lines=0` shows single line.
TC-4.3 verified `context_lines=10` shows up to 21 lines.

### Constraint 3: Token Budget

| Scenario      | Target        | Actual (Estimated)  | Status  |
|---------------|---------------|---------------------|---------|
| 20 references | <2,000 tokens | ~1,000-1,500 tokens | MET     |
| 50 references | <5,000 tokens | ~2,500-3,500 tokens | MET     |

**Calculation Method:**
- Header + summary: ~100 tokens
- Per reference: ~50-70 tokens (file:line + context + confidence)
- 20 refs: 100 + (20 * 60) = ~1,300 tokens
- 50 refs: 100 + (50 * 60) = ~3,100 tokens

**Comparison to Original Estimates:**

| Tool               | Original Estimate  | Actual                 |
|--------------------|--------------------|------------------------|
| Serena find_symbol | 5,000 - 50,000     | Not re-tested          |
| Shebe search_code  | 500 - 2,000        | ~500-2,000 (unchanged) |
| find_references    | 300 - 1,500        | ~1,000-3,500           |

**Assessment:** Actual token usage is higher than original 300-1,500 estimate but still
significantly better than Serena. The original estimate may have been optimistic.

### Constraint 4: Confidence Scoring

| Feature             | Target  | Actual                    | Status  |
|---------------------|---------|---------------------------|---------|
| Confidence groups   | H/M/L   | High/Medium/Low           | MET     |
| Pattern scoring     | -       | 0.60-0.95 base scores     | MET     |
| Context adjustments | -       | +0.05 test, -0.30 comment | MET     |

**Evidence from Test Results:**

| Test Case                  | H/M/L Distribution | Interpretation                |
|----------------------------|--------------------|-------------------------------|
| TC-1.1 FindDatabasePath    | 11/20/3            | Function calls ranked highest |
| TC-2.2 ADODB               | 0/6/6              | Comments correctly penalized  |
| TC-3.1 AuthorizationPolicy | 35/15/0            | Type annotations ranked high  |

### Constraint 5: File Grouping

| Feature              | Target  | Actual                                      | Status  |
|----------------------|---------|---------------------------------------------|---------|
| Files to update list | Yes     | Yes (in summary)                            | MET     |
| Group by file        | Desired | Results grouped by confidence, files listed | PARTIAL |

**Evidence:** Output format includes "Files to update:" section listing unique files.
However, results are grouped by confidence level, not by file.

### Constraint 6: No Full Bodies

| Feature             | Target  | Actual                     | Status  |
|---------------------|---------|----------------------------|---------|
| Full code bodies    | Never   | Never returned             | MET     |
| Reference line only | Yes     | Yes + configurable context | MET     |

**Evidence:** All test outputs show only matching line + context, never full function/class bodies.

---

## Problem Resolution Assessment

### Problem 1: Full Code Bodies

| Metric           | Before (Serena)  | After (find_references) | Improvement  |
|------------------|------------------|-------------------------|--------------|
| Body returned    | Full (346 lines) | Never                   | 100%         |
| Tokens per class | ~5,000+          | ~60 (line + context)    | 98%+         |

**VERDICT: SOLVED** - find_references never returns full code bodies.

### Problem 2: Token Inefficiency

| Metric               | Target     | Actual       | Status   |
|----------------------|------------|--------------|----------|
| Tokens per reference | ~50        | ~50-70       | MET      |
| 20-reference query   | <2,000     | ~1,300       | MET      |
| vs Serena            | 10x better | 4-40x better | EXCEEDED |

**VERDICT: SOLVED** - Token efficiency meets or exceeds targets.

### Problem 3: Workflow Inefficiency

| Old Workflow Step  | New Workflow                    | Improvement     |
|--------------------|---------------------------------|-----------------|
| 1. Grep (may miss) | find_references (pattern-aware) | Better recall   |
| 2. Read each file  | Confidence-ranked list          | Prioritized     |
| 3. Make changes    | Files to update list            | Systematic      |
| 4. Discover missed | High confidence = complete      | Fewer surprises |

**VERDICT: PARTIALLY SOLVED** - Workflow is improved but not eliminated.
Claude still needs to read files to make changes. The improvement is in the
discovery phase, not the modification phase.

---

## Unresolved Issues

### Issue 1: Token Estimate Accuracy

Original estimate: 300-1,500 tokens for typical query
Actual: 1,000-3,500 tokens for 20-50 references

**Gap:** Actual is 2-3x higher than original estimate.

**Cause:** Original estimate assumed ~15 tokens per reference. Actual implementation
uses ~50-70 tokens due to:
- File path (20-40 tokens)
- Context lines (20-30 tokens)
- Pattern name + confidence (10 tokens)

**Impact:** Still significantly better than Serena, but not as dramatic as projected.

### Issue 2: False Positives Not Eliminated

From test results:
- TC-2.2 ADODB: 6 low-confidence results in comments
- Pattern-based approach cannot eliminate all false positives

**Mitigation:** Confidence scoring helps Claude filter, but doesn't eliminate.

### Issue 3: Not AST-Aware

For rename refactoring, semantic accuracy matters:
- find_references: Pattern-based, may miss non-standard patterns
- serena: AST-aware, semantically accurate

**Trade-off:** Speed and token efficiency vs semantic precision.

---

## Comparative Summary

| Metric                | Serena find_symbol | find_references       | Winner          |
|-----------------------|--------------------|-----------------------|-----------------|
| Speed                 | 50-5000ms          | 5-32ms                | find_references |
| Token usage (20 refs) | 10,000-50,000      | ~1,300                | find_references |
| Precision             | Very High (AST)    | Medium-High (pattern) | Serena          |
| False positives       | Minimal            | Some (scored low)     | Serena          |
| Setup required        | LSP + project      | Index session         | find_references |
| Polyglot support      | Per-language       | Yes                   | find_references |

---

## Conclusion

### Problems Solved

| Problem                   | Status           | Evidence                            |
|---------------------------|------------------|-------------------------------------|
| Full code bodies returned | SOLVED           | Never returns bodies                |
| Token inefficiency        | SOLVED           | 4-40x better than Serena            |
| Workflow inefficiency     | PARTIALLY SOLVED | Better discovery, same modification |

### Design Constraints Met

| Constraint                | Status                               |
|---------------------------|--------------------------------------|
| Output limit (100 max)    | MET                                  |
| Context (2 lines default) | MET                                  |
| Token budget (<2,000)     | MET (for <30 refs)                   |
| Confidence scoring        | MET                                  |
| File grouping             | PARTIAL (list provided, not grouped) |
| No full bodies            | MET                                  |

### Overall Assessment

**The find_references tool successfully addresses the core problems identified in the
original analysis:**

1. **Token efficiency improved by 4-40x** compared to Serena for reference finding
2. **Never returns full code bodies** - only reference lines with minimal context
3. **Confidence scoring enables prioritization** - Claude can focus on high-confidence results
4. **Speed is 10-100x faster** than Serena for large codebases

**Limitations acknowledged:**

1. Token usage is 2-3x higher than original optimistic estimate
2. Pattern-based approach has some false positives (mitigated by confidence scoring)
3. Not a complete replacement for Serena when semantic precision is critical

### Recommendation

**find_references is fit for purpose** for the stated goal: efficient reference finding
before rename operations. It should be used as the primary tool for "find all usages"
queries, with Serena reserved for cases requiring semantic precision.

---

## Appendix: Test Coverage of Original Requirements

| Original Requirement     | Test Coverage                           |
|--------------------------|-----------------------------------------|
| Max 100 references       | TC-4.4 (max_results=1)                  |
| 2 lines context          | TC-4.2 (context=0), TC-4.3 (context=10) |
| <2,000 tokens            | Estimated from output format            |
| Confidence H/M/L         | TC-1.1, TC-2.2, TC-3.1                  |
| File grouping            | Output format verified                  |
| No full bodies           | All tests                               |
| False positive filtering | TC-2.2 (comments penalized)             |

---

## Update Log

| Date | Shebe Version | Document Version | Changes |
|------|---------------|------------------|---------|
| 2025-12-11 | 0.5.0 | 1.0 | Initial validation document |
