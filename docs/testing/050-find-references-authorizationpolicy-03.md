# Serena MCP Comparison Test: AuthorizationPolicy in Istio 1.28

**Date:** 2025-12-28
**Repository:** /home/orodha/github/istio/istio
**Symbol:** AuthorizationPolicy
**Comparison:** Shebe find_references vs Serena MCP (Claude-assisted)

## Serena Search Statistics

### Search Iterations Required

| # | Tool                     | Parameters                                 | Results      | Purpose                     |
|---|--------------------------|--------------------------------------------|--------------|-----------------------------|
| 1 | find_symbol              | name_path=AuthorizationPolicy, depth=1     | 6 symbols    | Find all symbol definitions |
| 2 | find_referencing_symbols | pilot/pkg/model/authorization.go           | 37 refs      | Refs to struct type |
| 3 | find_referencing_symbols | pkg/config/schema/gvk/resources.gen.go     | 59 refs      | Refs to GVK constant |
| 4 | find_referencing_symbols | pkg/config/schema/kind/resources.gen.go    | 20 refs      | Refs to kind constant |
| 5 | search_for_pattern       | securityclient\.AuthorizationPolicy        | 41 matches   | Client alias refs |
| 6 | search_for_pattern       | clientsecurityv1beta1\.AuthorizationPolicy | 14 matches   | v1beta1 alias refs |
| 7 | search_for_pattern       | security_beta\.AuthorizationPolicy         | 100+ matches | Proto alias refs |
| 8 | search_for_pattern       | kind: AuthorizationPolicy (YAML)           | 60+ matches  | YAML declarations |

**Total searches required:** 8

## E2E Time Comparison

| Approach              | Searches | Wall Time | Token Usage    |
|-----------------------|----------|-----------|----------------|
| Shebe find_references | 1        | ~2-3s     | ~4,500 tokens  |
| Claude + Grep         | 13       | ~15-20s   | ~12,000 tokens |
| Claude + Serena       | 8        | ~25-30s   | ~18,000 tokens |

### Time Breakdown (Serena Approach)

| Phase                              | Duration    |
|------------------------------------|-------------|
| find_symbol (initial discovery)    | ~3s         |
| find_referencing_symbols (3 calls) | ~12s        |
| search_for_pattern (4 calls)       | ~8s         |
| Claude processing between calls    | ~5s         |
| **Total E2E**                      | **~25-30s** |

## Token Usage Comparison

### Shebe find_references (Single Call)

| Component         | Tokens     |
|-------------------|------------|
| Tool call (input) | ~50        |
| Response (output) | ~4,500     |
| **Total**         | **~4,550** |

### Grep-Based Search (13 Calls)

| Component | Tokens |
|-----------|--------|
| 13 tool calls (input) | ~650 |
| 13 responses (output) | ~8,500 |
| Claude reasoning between calls | ~3,000 |
| **Total** | **~12,150** |

### Serena-Based Search (8 Calls)

| Component | Tokens |
|-----------|--------|
| 8 tool calls (input) | ~800 |
| 8 responses (output) | ~14,000 |
| Claude reasoning between calls | ~3,200 |
| **Total** | **~18,000** |

## Symbol Definitions Found (Serena find_symbol)

| Symbol | Kind | File | Line |
|--------|------|------|------|
| AuthorizationPolicy | Struct | pilot/pkg/model/authorization.go | 24-29 |
| AuthorizationPolicy | Constant | pkg/config/schema/kind/resources.gen.go | 7 |
| AuthorizationPolicy | Variable | pkg/config/schema/gvk/resources.gen.go | 13 |
| AuthorizationPolicy | Variable | pkg/config/schema/gvr/resources.gen.go | 9 |
| AuthorizationPolicy | Variable | pkg/config/schema/collections/collections.gen.go | 40-56 |
| AuthorizationPolicy | Variable | pkg/config/schema/collections/collections.agent.gen.go | 23-39 |

Serena immediately identified 6 distinct symbol definitions with their kinds (Struct, Constant, Variable).

## Files to Update (Serena-Derived)

### Core Type Definition
- pilot/pkg/model/authorization.go (struct + methods)

### Schema/Registry Files
- pkg/config/schema/gvk/resources.gen.go
- pkg/config/schema/gvr/resources.gen.go
- pkg/config/schema/kind/resources.gen.go
- pkg/config/schema/collections/collections.gen.go
- pkg/config/schema/collections/collections.agent.gen.go
- pkg/config/schema/kubetypes/resources.gen.go

### Implementation Files
- pilot/pkg/config/kube/crdclient/types.gen.go
- pilot/pkg/security/authz/builder/builder.go
- pilot/pkg/networking/grpcgen/lds.go
- pilot/pkg/networking/core/networkfilter_test.go
- pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex.go
- pilot/pkg/serviceregistry/kube/controller/ambient/authorization.go
- pilot/pkg/serviceregistry/kube/controller/ambient/policies.go
- pilot/pkg/serviceregistry/kube/controller/ambient/multicluster.go
- pkg/config/analysis/analyzers/authz/authorizationpolicies.go
- pkg/config/analysis/analyzers/conditions/conditions.go
- pkg/config/analysis/analyzers/k8sgateway/workloadselector.go
- pilot/pkg/model/push_context.go
- pilot/pkg/model/sidecar.go
- pilot/pkg/xds/cds.go
- pilot/pkg/xds/eds.go
- pilot/pkg/xds/nds.go
- pilot/pkg/xds/rds.go
- pilot/pkg/xds/workload.go

### Test Files
- pilot/pkg/model/authorization_test.go
- pilot/pkg/model/sidecar_test.go
- pilot/pkg/networking/core/gateway_test.go
- pilot/pkg/networking/core/listener_test.go
- pilot/pkg/networking/grpcgen/grpcgen_test.go
- pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_test.go
- pilot/pkg/serviceregistry/kube/controller/ambient/authorization_test.go
- pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_workloadentry_test.go
- pilot/pkg/serviceregistry/kube/controller/ambient/ambientindex_multicluster_test.go
- pilot/pkg/xds/ecds_test.go
- pilot/pkg/xds/proxy_dependencies_test.go
- pilot/pkg/xds/workload_test.go
- pkg/config/validation/validation.go
- pkg/config/validation/validation_test.go
- pkg/config/analysis/analyzers/analyzers_test.go
- tests/fuzz/config_validation_fuzzer.go

### YAML Test Data (44 files)
- pilot/pkg/security/authz/builder/testdata/http/*.yaml (20+ files)
- pilot/pkg/security/authz/builder/testdata/tcp/*.yaml (8 files)
- pilot/pkg/serviceregistry/kube/controller/ambient/testdata/*.yaml (3 files)
- tests/integration/pilot/testdata/*.yaml (3 files)
- pkg/config/validation/testdata/crds/*.yaml (2 files)
- pkg/config/analysis/analyzers/testdata/*.yaml (3 files)
- pkg/test/datasets/validation/dataset/*.yaml (2 files)
- manifests/charts/base/files/crd-all.gen.yaml
- pkg/config/schema/metadata.yaml
- operator/cmd/mesh/testdata/manifest-generate/output/all_on.golden-show-in-gh-pull-request.yaml

## Serena Advantages

1. **Semantic understanding**: Identified symbol kinds (Struct, Constant, Variable, Method)
2. **Hierarchical view**: find_symbol with depth=1 shows struct fields
3. **Contextual references**: find_referencing_symbols shows containing function/method
4. **LSP-based accuracy**: Uses Go language server for precise symbol resolution

## Serena Limitations

1. **Multiple calls required**: Each symbol definition needs separate find_referencing_symbols call
2. **No cross-file aggregation**: Can't search for references across all definitions at once
3. **Pattern search needed**: Import aliases require search_for_pattern (not semantic)
4. **No YAML support**: YAML files require pattern search, not semantic analysis
5. **Higher token usage**: Verbose JSON responses consume more tokens

## Comparison Summary

| Metric | Shebe | Grep | Serena |
|--------|-------|------|--------|
| Searches | 1 | 13 | 8 |
| E2E Time | ~2-3s | ~15-20s | ~25-30s |
| Tokens | ~4,500 | ~12,000 | ~18,000 |
| Symbol kinds | No | No | Yes |
| Confidence scores | Yes | No | No |
| YAML support | Yes | Yes | Pattern only |
| Semantic context | BM25 | None | LSP |
| Actionable output | Immediate | Manual | Semi-manual |

## Key Observations

### Serena Strengths
- LSP-based semantic understanding of Go code
- Accurate symbol kind identification (Struct vs Constant vs Variable)
- Hierarchical symbol exploration (struct fields, methods)
- find_referencing_symbols provides containing function context

### Serena Weaknesses for This Task
- Symbol with same name in multiple files requires multiple find_referencing_symbols calls
- Import aliases (securityclient., security_beta.) not detected semantically
- YAML files not analyzed semantically
- Higher token consumption due to verbose JSON responses
- Slower E2E time due to multiple round trips

### Why Shebe Performed Better
1. **Single operation**: One call covers all definitions and usages
2. **Cross-file aggregation**: Finds all references regardless of import alias
3. **YAML support**: Indexes and searches YAML files natively
4. **BM25 ranking**: Confidence scores filter noise automatically
5. **Token efficiency**: Compact output format
6. **Pattern awareness**: Detects type_instantiation, type_annotation, word_match

## Conclusion

For refactoring a type like `AuthorizationPolicy` with multiple definitions and import aliases:

| Tool | Best For |
|------|----------|
| **Shebe** | Discovery and enumeration of all references |
| **Serena** | Precise symbol manipulation and editing |
| **Grep** | Exhaustive text search when patterns are known |

**Recommendation:** Use Shebe for discovery phase, Serena for editing phase.
- Shebe find_references: "What needs to change?" (1 call, ~4.5k tokens)
- Serena replace_symbol_body: "Make the changes" (semantic editing)
