# Grep/Ripgrep Comparison Test: AuthorizationPolicy in Istio 1.28

**Repository:** ~/github/istio/istio <br>
**Symbol:** AuthorizationPolicy <br>
**Comparison:** Shebe find_references vs Grep (Claude-assisted) <br>
**Shebe Version:** 0.5.0 <br>
**Document Version:** 1.0 <br>
**Created:** 2025-12-28 <br>

## Grep Search Statistics

### Raw Search Performance

| Metric                      | Value  |
|-----------------------------|--------|
| Ripgrep execution time      | 0.024s |
| Files with matches (Go)     | 57     |
| Files with matches (YAML)   | 54     |
| Total files with matches    | 111    |
| Total occurrences (Go only) | 470    |

### Claude + Grep Search Iterations

To produce actionable refactoring output, the following searches were required:

| # | Search Pattern                                | Type         | Results         | Purpose                |
|---|-----------------------------------------------|--------------|-----------------|------------------------|
| 1 | `AuthorizationPolicy`                         | Go files     | 57 files        | Find all Go files      |
| 2 | `AuthorizationPolicy`                         | YAML files   | 54 files        | Find all YAML files    |
| 3 | `AuthorizationPolicy`                         | Go count     | 470 occurrences | Count total matches    |
| 4 | `type AuthorizationPolicy struct`             | Go content   | 1 match         | Find type definition   |
| 5 | `\*AuthorizationPolicy`                       | Go content   | 1 match         | Find pointer usages    |
| 6 | `\[\]AuthorizationPolicy`                     | Go content   | 27 matches      | Find slice usages      |
| 7 | `AuthorizationPolicy\{`                       | Go content   | 30+ matches     | Find instantiations    |
| 8 | `gvk\.AuthorizationPolicy`                    | Go content   | 52 matches      | Find GVK references    |
| 9 | `kind: AuthorizationPolicy`                   | YAML content | 30+ matches     | Find YAML declarations |
| 10 | `kind\.AuthorizationPolicy`                  | Go content   | 19 matches      | Find kind package refs |
| 11 | `securityclient\.AuthorizationPolicy`        | Go content   | 41 matches      | Find client refs       |
| 12 | `clientsecurityv1beta1\.AuthorizationPolicy` | Go content   | 14 matches      | Find v1beta1 refs      |
| 13 | `security_beta\.AuthorizationPolicy`         | Go content   | 30+ matches     | Find proto refs        |

**Total searches required:** 13

## E2E Time Comparison

| Approach              | Searches | Wall Time | Token Usage    |
|-----------------------|----------|-----------|----------------|
| Shebe find_references | 1        | ~2-3s     | ~4,500 tokens  |
| Claude + Grep         | 13       | ~15-20s   | ~12,000 tokens |

### Time Breakdown (Grep Approach)

| Phase | Duration |
|-------|----------|
| Initial file listing (2 searches) | ~1s |
| Count occurrences | ~0.5s |
| Type definition search | ~0.5s |
| Pattern-specific searches (9 searches) | ~10s |
| Claude processing between searches | ~5-8s |
| **Total E2E** | **~15-20s** |

## Token Usage Comparison

### Shebe find_references (Single Call)

| Component | Tokens |
|-----------|--------|
| Tool call (input) | ~50 |
| Response (output) | ~4,500 |
| **Total** | **~4,550** |

### Grep-Based Search (Multiple Calls)

| Component | Tokens |
|-----------|--------|
| 13 tool calls (input) | ~650 |
| 13 responses (output) | ~8,500 |
| Claude reasoning between calls | ~3,000 |
| **Total** | **~12,150** |

## Actionable Output Comparison

### Shebe find_references Output

Provided directly:
- 100 references with file paths and line numbers
- Confidence scores (high/medium/low)
- Pattern classification (type_instantiation, type_annotation, word_match)
- 27 unique files to update
- Ready for refactoring

### Grep-Based Output (After 13 Searches)

Required manual synthesis to identify:
- Type definition location: `pilot/pkg/model/authorization.go:25`
- Type aliases in different packages:
  - `gvk.AuthorizationPolicy`
  - `kind.AuthorizationPolicy`
  - `securityclient.AuthorizationPolicy`
  - `clientsecurityv1beta1.AuthorizationPolicy`
  - `security_beta.AuthorizationPolicy`
- YAML `kind: AuthorizationPolicy` declarations
- 111 total files (but many are noise - release notes, docs, etc.)

## Files to Update (Grep-Derived)

### Core Implementation Files

| File | Occurrences | Type |
|------|-------------|------|
| pilot/pkg/model/authorization.go | 20 | Type definition |
| pilot/pkg/model/authorization_test.go | 25 | Tests |
| pkg/config/validation/validation.go | 13 | Validation |
| pkg/config/validation/validation_test.go | 102 | Tests |
| pilot/pkg/serviceregistry/kube/controller/ambient/authorization_test.go | 54 | Tests |
| pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_test.go | 30 | Tests |
| pilot/pkg/config/kube/crdclient/types.gen.go | 17 | Generated |

### Generated/Schema Files

| File | Occurrences |
|------|-------------|
| pkg/config/schema/collections/collections.gen.go | 10 |
| pkg/config/schema/collections/collections.agent.gen.go | 10 |
| pkg/config/schema/gvk/resources.gen.go | 10 |
| pkg/config/schema/kubetypes/resources.gen.go | 4 |
| pkg/config/schema/kind/resources.gen.go | 5 |
| pkg/config/schema/gvr/resources.gen.go | 4 |
| pkg/config/schema/kubeclient/resources.gen.go | 5 |

### Integration Test YAML Files

| File | Kind Declarations |
|------|-------------------|
| pilot/pkg/security/authz/builder/testdata/http/multiple-policies-in.yaml | 9 |
| tests/integration/pilot/testdata/authz-a.yaml | 2 |
| tests/integration/pilot/testdata/authz-b.yaml | 2 |
| pilot/pkg/security/authz/builder/testdata/http/*.yaml | 20+ |
| pilot/pkg/security/authz/builder/testdata/tcp/*.yaml | 7 |

## Key Differences

| Aspect | Shebe find_references | Grep + Claude |
|--------|----------------------|---------------|
| Single operation | Yes | No (13 iterations) |
| Confidence scoring | Yes (0.0-1.0) | No |
| Pattern classification | Yes | Manual |
| False positive filtering | Automatic | Manual |
| Context per match | 2 lines (configurable) | Variable |
| Token efficiency | High (~4.5k) | Low (~12k) |
| Time efficiency | High (~2-3s) | Low (~15-20s) |
| Actionable output | Immediate | Requires synthesis |

## Observations

### Grep Advantages

1. **Raw speed**: Ripgrep executes in 24ms
2. **Exhaustive**: Found all 470 occurrences vs 100 limited by find_references
3. **Flexibility**: Can search any pattern with regex
4. **Familiar**: Standard Unix tooling

### Shebe find_references Advantages

1. **Single call**: One operation vs 13 iterations
2. **Intelligent filtering**: Removes noise (docs, release notes)
3. **Confidence scoring**: Prioritizes actual code references
4. **Pattern detection**: Understands type_instantiation vs word_match
5. **Token efficient**: 2.7x fewer tokens used
6. **Time efficient**: 6-8x faster E2E
7. **Refactoring-ready**: Output directly usable

### Why Grep Required Multiple Iterations

The symbol `AuthorizationPolicy` appears in multiple contexts:
1. As a Go struct type (`type AuthorizationPolicy struct`)
2. As a pointer (`*AuthorizationPolicy`)
3. As a slice (`[]AuthorizationPolicy`)
4. As a type instantiation (`AuthorizationPolicy{}`)
5. As a GVK constant (`gvk.AuthorizationPolicy`)
6. As a kind constant (`kind.AuthorizationPolicy`)
7. With different import aliases (`securityclient.`, `security_beta.`, `clientsecurityv1beta1.`)
8. In YAML as `kind: AuthorizationPolicy`

Each context required a separate grep pattern to fully understand the refactoring scope.

## Conclusion

For refactoring a type like `AuthorizationPolicy` in a large codebase:

| Metric | Shebe | Grep |
|--------|-------|------|
| E2E Time | ~2-3s | ~15-20s |
| Searches | 1 | 13 |
| Tokens | ~4,500 | ~12,000 |
| Actionable? | Yes | Requires synthesis |

**Shebe find_references** provides a 6-8x speedup and 2.7x token reduction while
producing immediately actionable output with confidence scoring and pattern
classification.

---

## Update Log

| Date | Shebe Version | Document Version | Changes |
|------|---------------|------------------|---------|
| 2025-12-28 | 0.5.0 | 1.0 | Initial comparison test document |
