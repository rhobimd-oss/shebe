# find_references Integration Test Plan

**Document:** FIND_REFERENCES_TEST_PLAN.md <br>
**Related:** dev-docs/work-plans/014-find-references-tool-01.md (Phase 4.5) <br>
**Status:** Implementation <br>

## Test Envelope Methodology

Tests follow the aerospace "flight test envelope" approach:

1. **Nominal conditions** - Center of parameter space (standard usage)
2. **Boundary conditions** - Edges of each parameter
3. **Corner cases** - Combinations of multiple boundaries
4. **Off-nominal** - Error conditions and failure modes

Reference: https://en.wikipedia.org/wiki/Flight_envelope

## Parameter Space

| Parameter          | Min      | Nominal    | Max       | Notes                                     |
|--------------------|----------|------------|-----------|-------------------------------------------|
| symbol             | 2 chars  | 8-15 chars | 200 chars | <2 returns InvalidParams                  |
| session            | valid ID | -          | -         | nonexistent returns error                 |
| symbol_type        | -        | "any"      | -         | enum: function/type/variable/constant/any |
| defined_in         | none     | -          | path      | Excludes definition file                  |
| include_definition | false    | false      | true      | Boolean flag                              |
| context_lines      | 0        | 2          | 10        | Lines around each match                   |
| max_results        | 1        | 50         | 200       | Maximum references returned               |

## Test Categories (28 tests total)

### 1. Nominal Tests (5 tests)

Tests at the center of the parameter envelope - standard usage patterns.

| Test                                    | Symbol            | Parameters           | Expected                           |
|-----------------------------------------|-------------------|----------------------|------------------------------------|
| `test_find_function_references`         | `calculate_total` | symbol_type=function | Find across multiple files         |
| `test_find_type_references`             | `UserConfig`      | symbol_type=type     | Find type annotations              |
| `test_find_with_default_params`         | `myFunc`          | defaults only        | context_lines=2, max=50            |
| `test_find_references_returns_markdown` | any               | -                    | Output has ## headers, code blocks |
| `test_find_includes_session_timestamp`  | any               | -                    | Output shows "Session indexed:"    |

### 2. Boundary Tests (8 tests)

Tests at the edges of each parameter.

| Test                               | Boundary      | Value           | Expected                 |
|------------------------------------|---------------|-----------------|--------------------------|
| `test_symbol_min_length_2_chars`   | symbol min    | "fn" (2 chars)  | Success                  |
| `test_symbol_max_length_200_chars` | symbol max    | 200 char string | Success                  |
| `test_context_lines_zero`          | context min   | 0               | Only matching line       |
| `test_context_lines_max_10`        | context max   | 10              | 21 lines per match       |
| `test_max_results_one`             | results min   | 1               | Single best match        |
| `test_max_results_200`             | results max   | 200             | Truncated at 200         |
| `test_no_references_found`         | results count | 0               | "No references found"    |
| `test_many_references_found`       | results count | 50+             | All returned (up to max) |

### 3. Corner Tests (4 tests)

Combinations of multiple boundary conditions.

| Test                                       | Boundaries Combined                     | Expected            |
|--------------------------------------------|-----------------------------------------|---------------------|
| `test_short_symbol_min_results_no_context` | 2 chars + max=1 + context=0             | Single line result  |
| `test_defined_in_excludes_definition_file` | defined_in set                          | Excludes that file  |
| `test_include_definition_true`             | include_definition=true                 | Includes definition |
| `test_multiple_boundaries_combined`        | long symbol + max context + max results | Handles all         |

### 4. Error Tests (4 tests)

Off-nominal conditions - invalid inputs.

| Test                               | Input             | Expected Error  |
|------------------------------------|-------------------|-----------------|
| `test_empty_symbol_rejected`       | symbol=""         | InvalidParams   |
| `test_single_char_symbol_rejected` | symbol="a"        | InvalidParams   |
| `test_whitespace_symbol_rejected`  | symbol="   "      | InvalidParams   |
| `test_nonexistent_session`         | session="invalid" | Error           |

### 5. Confidence Scoring Tests (4 tests)

End-to-end validation of confidence scoring.

| Test                                 | Context             | Expected Confidence  |
|--------------------------------------|---------------------|----------------------|
| `test_function_call_high_confidence` | `symbol()` pattern  | >= 0.80              |
| `test_comment_low_confidence`        | `// symbol`         | < 0.70               |
| `test_doc_file_low_confidence`       | Symbol in .md file  | < 0.55               |
| `test_test_file_confidence_boost`    | Symbol in test file | +0.05 boost          |

### 6. Multi-Language Tests (3 tests)

Pattern matching across different languages.

| Test                      | Language  | Pattern                     |
|---------------------------|-----------|-----------------------------|
| `test_rust_use_statement` | Rust      | `use crate::module::symbol` |
| `test_python_import`      | Python    | `from module import symbol` |
| `test_go_function_call`   | Go        | `symbol(args)`              |

## Test Fixtures

### Rust Fixture
```rust
const RUST_FIXTURE: &[(&str, &str)] = &[
    ("src/lib.rs", "pub fn calculate_total(items: &[Item]) -> f64 { items.len() as f64 }"),
    ("src/handlers.rs", "use crate::calculate_total;\nlet total = calculate_total(&cart);"),
    ("tests/lib_test.rs", "#[test]\nfn test_calculate_total() { calculate_total(&[]); }"),
    ("README.md", "# API\n\nThe `calculate_total` function computes the sum."),
];
```

### Multi-Language Fixture
```rust
const MULTILANG_FIXTURE: &[(&str, &str)] = &[
    ("main.go", "package main\n\nfunc processData(input []byte) error { return nil }"),
    ("handler.py", "from utils import processData\n\ndef main():\n    processData(data)"),
    ("app.ts", "import { processData } from './utils';\n\nprocessData(data);"),
];
```

## Implementation Notes

- All tests are async using `#[tokio::test]`
- Each test creates isolated temp directory for session storage
- Tests use `TestRepo::with_files()` for creating test data
- Helper functions handle handler setup and result extraction
- Tests should complete in < 60 seconds total

## Success Criteria

1. All 28 tests pass with `make test`
2. Full parameter envelope coverage (boundaries + corners)
3. Error paths validated (4 conditions)
4. Confidence scoring end-to-end verified
5. Multi-language patterns confirmed
