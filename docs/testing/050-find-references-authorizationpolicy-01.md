# find_references Performance Test: AuthorizationPolicy in Istio 1.28

**Date:** 2025-12-28
**Session:** istio-1-28
**Repository:** ~/github/istio/istio
**Symbol:** AuthorizationPolicy
**Symbol Type:** type

## Session Statistics

| Metric            | Value  |
|-------------------|--------|
| Files indexed     | 5,965  |
| Chunks created    | 74,589 |
| Indexing duration | 0.5s   |

## find_references Results

| Metric                        | Value |
|-------------------------------|-------|
| Total references found        | 100   |
| High confidence (0.80+)       | 13    |
| Medium confidence (0.50-0.79) | 71    |
| Low confidence (<0.50)        | 16    |
| Unique files                  | 27    |
| Max results requested         | 200   |

## E2E Time Taken

| Operation            | Duration                                   |
|----------------------|--------------------------------------------|
| find_references call | ~2-3 seconds (estimated from MCP response) |

Note: Exact timing not available from MCP response. The find_references tool
returns results synchronously without explicit timing metadata.

## Token Usage

| Category                                 | Count         |
|------------------------------------------|---------------|
| Output tokens (find_references response) | ~4,500 tokens |
| Context lines per reference              | 2 (default)   |

The response includes:
- 100 reference entries with file paths, line numbers, code context
- Confidence scores and pattern types for each reference
- Summary statistics

## High Confidence References (Sample)

| File                      | Line | Pattern            | Confidence |
|---------------------------|------|--------------------|------------|
| grpcgen_test.go           | 421  | type_instantiation | 0.90       |
| grpcgen_test.go           | 444  | type_instantiation | 0.90       |
| deny-and-allow-in.yaml    | 2    | type_annotation    | 0.90       |
| multiple-policies-in.yaml | 78   | type_annotation    | 0.90       |
| ambientindex_test.go      | 1562 | type_instantiation | 0.90       |
| ambientindex_test.go      | 2596 | type_instantiation | 0.90       |
| authorization_test.go     | 56   | type_instantiation | 0.90       |
| workload_test.go          | 342  | type_instantiation | 0.90       |
| authz-a.yaml              | 2    | type_annotation    | 0.90       |
| types.gen.go              | 50   | type_instantiation | 0.85       |
| types.gen.go              | 187  | type_instantiation | 0.85       |
| types.gen.go              | 464  | type_instantiation | 0.85       |
| authorization.go          | 48   | type_instantiation | 0.85       |

## Files to Update (for refactoring)

1. `~/github/istio/istio/tests/integration/pilot/testdata/authz-a.yaml`
2. `~/github/istio/istio/pilot/pkg/config/kube/crdclient/types.gen.go`
3. `~/github/istio/istio/pilot/pkg/networking/grpcgen/grpcgen_test.go`
4. `~/github/istio/istio/pilot/pkg/model/authorization.go`
5. `~/github/istio/istio/pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_test.go`
6. `~/github/istio/istio/pilot/pkg/serviceregistry/kube/controller/ambient/authorization_test.go`
7. `~/github/istio/istio/pilot/pkg/security/authz/builder/testdata/http/multiple-policies-in.yaml`
8. `~/github/istio/istio/pilot/pkg/security/authz/builder/testdata/http/deny-and-allow-in.yaml`
9. `~/github/istio/istio/pilot/pkg/xds/workload_test.go`
10. `~/github/istio/istio/pkg/config/validation/validation_test.go`
11. `~/github/istio/istio/pkg/config/validation/validation.go`
12. `~/github/istio/istio/pkg/config/schema/gvk/resources.gen.go`
13. `~/github/istio/istio/pkg/config/schema/kind/resources.gen.go`
14. `~/github/istio/istio/pkg/config/schema/collections/collections.gen.go`
15. `~/github/istio/istio/pkg/config/schema/collections/collections.agent.gen.go`
16. `~/github/istio/istio/pkg/config/schema/kubetypes/resources.gen.go`
17. `~/github/istio/istio/pilot/pkg/model/authorization_test.go`
18. `~/github/istio/istio/pilot/pkg/model/sidecar_test.go`
19. `~/github/istio/istio/pilot/pkg/networking/core/gateway_test.go`
20. `~/github/istio/istio/pilot/pkg/networking/core/listener_test.go`
21. `~/github/istio/istio/pilot/pkg/networking/core/networkfilter_test.go`
22. `~/github/istio/istio/pilot/pkg/security/authz/builder/builder.go`
23. `~/github/istio/istio/pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex.go`
24. `~/github/istio/istio/pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_workloadentry_test.go`
25. `~/github/istio/istio/pkg/config/analysis/analyzers/analyzers_test.go`
26. `~/github/istio/istio/istioctl/pkg/authz/authz.go`
27. `~/github/istio/istio/releasenotes/notes/remote-ip.yaml`

## Pattern Distribution

| Pattern            | Count | Description                                        |
|--------------------|-------|----------------------------------------------------|
| type_instantiation | 45    | Direct type usage (e.g., `&AuthorizationPolicy{}`) |
| type_annotation    | 12    | YAML kind declarations                             |
| word_match         | 43    | General text matches in code/docs                  |

## Observations

1. **High-quality type detection**: The tool correctly identified Go type instantiations
   with high confidence (0.85-0.90)

2. **YAML support**: Successfully found `kind: AuthorizationPolicy` declarations in
   Kubernetes YAML manifests

3. **Generated code handling**: Detected references in generated files (types.gen.go,
   resources.gen.go, collections.gen.go)

4. **Test file coverage**: Comprehensive coverage of test files where AuthorizationPolicy
   is used extensively

5. **Documentation references**: Found references in release notes and documentation

## Comparison Notes

For a type like `AuthorizationPolicy` in a large Go codebase (5,965 files):
- BM25-based find_references provides semantic context understanding
- Confidence scoring helps prioritize actual type usages vs mentions
- Pattern classification aids in understanding usage context
