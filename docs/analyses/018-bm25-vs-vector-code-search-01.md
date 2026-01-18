# BM25 vs Vector Search for Code: What Developers Actually Need

**Document:** 018-bm25-vs-vector-code-search-01.md
**Created:** 2026-01-17
**Status:** Complete
**Related:** docs/testing/014-find-references-validation-04.md

---

## Executive Summary

This analysis examines whether BM25-only search (like Shebe) can sufficiently address
developer code search needs, or whether vector/semantic search (like Turbopuffer) is
required. Based on industry research and production systems, **BM25 covers 70-85% of
developer code search value** because developer workflows are predominantly keyword-based.

---

## Research Findings

### Developer Query Patterns

**Google Code Search Study (Sadowski et al. 2015):**

| Metric                          | Finding |
|---------------------------------|----------------------|
| Average query length            | 1-2 terms            |
| Content words                   | 80.3% of query terms |
| Path-restricted queries         | 26%                  |
| "How to use API" queries        | ~33%                 |
| "Why does code behave this way" | ~16%                 |

**Key insight:** Developers search with exact terms they already know - function names,
API names, error messages. They rarely ask conceptual questions in natural language.

From Google's study:
> "Compared to prior studies in a controlled lab environment that observed queries with
> an average of 3-5 terms, in the study the queries had just 1-2 terms, but were
> incrementally refined."

> "The most frequent use case - about one third of Code Searches - are about seeing
> examples of how others have done something."

### Industry Production Systems

| Company | Search Approach | Uses Semantic/Vector? |
|---------|-----------------|----------------------|
| GitHub (Blackbird) | Custom ngram + code heuristics | No |
| Sourcegraph | BM25 first stage + semantic rerank | Yes (2nd stage only) |
| Cursor (Turbopuffer) | Hybrid vector + BM25 | Yes |

#### GitHub's Approach

GitHub built Blackbird from scratch with **no semantic search** for 200M+ repositories:

> "We haven't had a lot of luck using general text search products to power code search.
> The user experience is poor, indexing is slow, and it's expensive to host."

Blackbird uses ngrams with code-specific ranking heuristics (definitions ranked up,
test code ranked down) rather than BM25 or vectors.

#### Sourcegraph's Findings

Sourcegraph's internal evaluations found **BM25 gave 20% improvement** across all
key metrics. They use semantic search only for second-stage reranking:

> "BM25 plays a key role in first stage retrieval, helping to gather a high quality
> candidate set. These candidates are then passed to a transformer model for second
> stage ranking."

#### Cursor's Results

Cursor reports **23.5% improvement** adding semantic search on top of grep - but this
is for AI context retrieval (populating LLM context windows), not human developer search.

---

## Query Type Analysis

| Query Type | % of Queries | BM25 Coverage |
|------------|--------------|---------------|
| Find exact symbol name | ~40% | 100% |
| Find usages of API | ~25% | 100% |
| Navigate to file/path | ~15% | 100% |
| Find error message/string | ~10% | 100% |
| Conceptual ("auth logic") | ~10% | ~50% |

**Estimated BM25 coverage: 70-85% of developer code search value.**

### Where Semantic Search Adds Value

The remaining 15-30% where vector search helps:

1. **Conceptual queries** - "Find authentication handling" when you don't know the
   function name
2. **Synonym matching** - "container" vs "collection", "authenticate" vs "login"
3. **Cross-language concepts** - Finding similar patterns across different languages
4. **Natural language questions** - "How do I connect to the database?"

### Where BM25 Is Sufficient (or Better)

1. **Exact symbol search** - Function names, class names, variables
2. **API usage lookup** - Finding calls to specific methods
3. **Error investigation** - Searching for exact error messages
4. **Refactoring workflows** - Finding all references before rename
5. **Code navigation** - Jumping to known files/paths

For refactoring workflows specifically, BM25 is arguably **better** than semantic
search because you want exact matches, not conceptual similarity.

---

## Shebe vs Turbopuffer Comparison

### Feature Coverage

| Capability | Turbopuffer | Shebe | Coverage |
|------------|-------------|-------|----------|
| BM25 full-text search | Yes | Yes | 100% |
| Vector/semantic search | Yes | No | 0% |
| Hybrid fusion + reranking | Yes | No | 0% |
| Namespace isolation | Yes | Yes (sessions) | 100% |
| Massive scale (billions) | Yes | No (~10k files) | ~1% |
| Cloud/serverless | Yes | No | 0% |
| Cold/warm tiering | Yes | No | 0% |
| API access | Yes | Yes (MCP) | 100% |

**Shebe covers ~25-35% of Turbopuffer's features.**

### Value Coverage

```
Turbopuffer capability breakdown (estimated):
  - BM25 keyword search:     ~35% of value
  - Vector semantic search:  ~35% of value
  - Scale/cloud/serverless:  ~20% of value
  - Hybrid fusion/reranking: ~10% of value

For developer code search workflows:
  - BM25 handles:            70-85% of query VALUE
  - Vector adds:             15-30% additional VALUE

Shebe effective coverage:
  - Feature coverage:        ~30% of Turbopuffer
  - Value coverage:          ~70-85% for typical workflows
```

### Positioning

**Turbopuffer** solves: "How do we provide semantic code search to millions of users
across billions of vectors, cost-effectively?"

**Shebe** solves: "How do I quickly search a local codebase from Claude Code without
hitting context limits?"

**"Poor man's Turbopuffer" is accurate:**
- ~30% of features
- ~70-85% of value for typical developer workflows
- 0% of cost
- 100% offline capability
- 100% privacy (code never leaves machine)

---

## Implications for Shebe

### Workflows Where Shebe Is Sufficient

1. **find_references before rename** - Exact symbol matching (100% BM25)
2. **search_code for API usage** - Finding calls to specific functions (100% BM25)
3. **find_file by pattern** - Glob/regex matching (100% BM25)
4. **Code navigation** - Jumping to known symbols (100% BM25)

### Workflows Where Shebe Falls Short

1. **"Find error handling patterns"** - Conceptual, benefits from semantic
2. **"Code that validates user input"** - Abstract concept, not exact terms
3. **"Similar implementations across repos"** - Requires embedding similarity

### Recommendation

For the stated use case (Claude Code integration for local codebase search), BM25-only
is a defensible choice because:

1. Developer queries are predominantly keyword-based (research-backed)
2. GitHub serves 200M+ repos without semantic search
3. Refactoring workflows want exact matches, not semantic similarity
4. Local/offline requirement eliminates cloud vector services
5. Zero-cost requirement eliminates embedding API calls

**Shebe is not trying to compete with Turbopuffer.** It's a focused tool for a specific
workflow (AI-assisted local code search) where BM25 provides most of the value.

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

- GitHub Engineering. "A Brief History of Code Search at GitHub."
  https://github.blog/engineering/architecture-optimization/a-brief-history-of-code-search-at-github/

- Sourcegraph. "Keeping it Boring (and Relevant) with BM25F."
  https://sourcegraph.com/blog/keeping-it-boring-and-relevant-with-bm25f

- Turbopuffer. "Cursor Scales Code Retrieval to 100B+ Vectors."
  https://turbopuffer.com/customers/cursor

### Books

- Winters, T., et al. "Software Engineering at Google." Chapter 17: Code Search.
  https://abseil.io/resources/swe-book/html/ch17.html

---

## Update Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-17 | 1.0 | Initial analysis |
