//! Integration tests for find_references MCP tool
//!
//! Tests follow the "test envelope" methodology from aerospace flight testing:
//! - Nominal: Standard usage patterns (center of parameter space)
//! - Boundary: Edge values of each parameter
//! - Corner: Combinations of boundary conditions
//! - Error: Invalid inputs and failure modes
//!
//! See FIND_REFERENCES_TEST_PLAN.md for detailed test plan.

use crate::common::{create_test_services, index_test_repository, TestRepo};
use serde_json::json;
use shebe::core::services::Services;
use shebe::mcp::error::McpError;
use shebe::mcp::protocol::ToolResult;
use shebe::mcp::tools::find_references::FindReferencesHandler;
use shebe::mcp::tools::handler::McpToolHandler;
use std::sync::Arc;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a handler with an indexed test session
/// Returns the handler, services, and TestRepo (kept alive for file access)
async fn setup_handler_with_session(
    files: &[(&str, &str)],
    session_id: &str,
) -> (FindReferencesHandler, Arc<Services>, TestRepo) {
    let services = Arc::new(create_test_services());
    let repo = TestRepo::with_files(files);
    let _stats = index_test_repository(&services, repo.path(), session_id).await;
    (
        FindReferencesHandler::new(Arc::clone(&services)),
        services,
        repo,
    )
}

/// Extract text content from ToolResult
fn extract_text(result: &ToolResult) -> &str {
    let shebe::mcp::protocol::ContentBlock::Text { text } = &result.content[0];
    text
}

// =============================================================================
// Test Fixtures
// =============================================================================

/// Rust fixture for nominal and confidence tests
const RUST_FIXTURE: &[(&str, &str)] = &[
    (
        "src/lib.rs",
        r#"pub fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price).sum()
}

pub struct Item {
    pub name: String,
    pub price: f64,
}
"#,
    ),
    (
        "src/handlers.rs",
        r#"use crate::calculate_total;

pub fn handle_checkout(cart: &[Item]) -> f64 {
    let total = calculate_total(cart);
    total
}
"#,
    ),
    (
        "tests/lib_test.rs",
        r#"use mylib::calculate_total;

#[test]
fn test_calculate_total() {
    let items = vec![];
    let result = calculate_total(&items);
    assert_eq!(result, 0.0);
}
"#,
    ),
    (
        "README.md",
        r#"# Shopping Cart API

The `calculate_total` function computes the sum of all item prices.

## Usage

Call calculate_total with a slice of items.
"#,
    ),
];

/// Multi-language fixture for pattern matching tests
#[allow(dead_code)]
const MULTILANG_FIXTURE: &[(&str, &str)] = &[
    (
        "main.go",
        r#"package main

func processData(input []byte) ([]byte, error) {
    return processData_internal(input)
}

func processData_internal(input []byte) ([]byte, error) {
    return input, nil
}
"#,
    ),
    (
        "handler.py",
        r#"from utils import processData

def main():
    data = b"test"
    result = processData(data)
    return result
"#,
    ),
    (
        "app.ts",
        r#"import { processData } from './utils';

export function main(): void {
    const data = new Uint8Array([1, 2, 3]);
    processData(data);
}
"#,
    ),
];

/// Fixture with TypeScript types
const TYPESCRIPT_TYPES_FIXTURE: &[(&str, &str)] = &[
    (
        "src/types.ts",
        r#"export interface UserConfig {
    name: string;
    age: number;
    email: string;
}
"#,
    ),
    (
        "src/services.ts",
        r#"import { UserConfig } from './types';

export function validateConfig(config: UserConfig): boolean {
    return config.name.length > 0;
}

export function createConfig(name: string): UserConfig {
    return { name, age: 0, email: '' };
}
"#,
    ),
    (
        "src/handlers.ts",
        r#"import { UserConfig } from './types';

export class ConfigHandler {
    private config: UserConfig;

    constructor(config: UserConfig) {
        this.config = config;
    }
}
"#,
    ),
];

// =============================================================================
// Nominal Tests (5 tests) - Center of Envelope
// =============================================================================

#[tokio::test]
async fn test_find_function_references() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "rust-project").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "rust-project",
        "symbol_type": "function"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should find references in multiple files
    assert!(
        text.contains("calculate_total"),
        "Output should contain symbol name"
    );
    assert!(
        text.contains("found"),
        "Output should indicate references found"
    );
    // Should find: lib.rs definition, handlers.rs use, tests/lib_test.rs use, README mention
    assert!(
        text.contains("lib.rs") || text.contains("handlers.rs"),
        "Should find references in source files"
    );
}

#[tokio::test]
async fn test_find_type_references() {
    let (handler, _services, _repo) =
        setup_handler_with_session(TYPESCRIPT_TYPES_FIXTURE, "ts-project").await;

    let args = json!({
        "symbol": "UserConfig",
        "session": "ts-project",
        "symbol_type": "type"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    assert!(
        text.contains("UserConfig"),
        "Output should contain type name"
    );
    assert!(
        text.contains("found"),
        "Output should indicate references found"
    );
}

#[tokio::test]
async fn test_find_with_default_params() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "default-test").await;

    // Only required params, all others use defaults
    let args = json!({
        "symbol": "calculate_total",
        "session": "default-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // With defaults: symbol_type=any, context_lines=2, max_results=50
    assert!(
        text.contains("calculate_total"),
        "Should find symbol with defaults"
    );
}

#[tokio::test]
async fn test_find_references_returns_markdown() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "markdown-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "markdown-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Check markdown structure
    assert!(text.contains("##"), "Output should have markdown headers");
    assert!(text.contains("```"), "Output should have code blocks");
    assert!(
        text.contains("**"),
        "Output should have bold text for labels"
    );
}

#[tokio::test]
async fn test_find_includes_session_timestamp() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "timestamp-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "timestamp-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should include session indexed timestamp
    assert!(
        text.contains("Session indexed") || text.contains("indexed"),
        "Output should show session timestamp"
    );
}

// =============================================================================
// Boundary Tests (8 tests) - Edges of Parameters
// =============================================================================

#[tokio::test]
async fn test_symbol_min_length_2_chars() {
    // Create fixture with 2-char symbol "fn"
    let files = &[(
        "src/main.rs",
        "fn main() { fn_helper(); }\nfn fn_helper() {}",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "min-symbol-test").await;

    let args = json!({
        "symbol": "fn",
        "session": "min-symbol-test"
    });

    // 2 chars is the minimum, should succeed
    let result = handler.execute(args).await;
    assert!(result.is_ok(), "2-char symbol should be accepted");
}

#[tokio::test]
async fn test_symbol_max_length_200_chars() {
    // Create a 200-char symbol
    let long_symbol = "a".repeat(200);
    let content = format!("fn {}() {{}}", long_symbol);
    let files = vec![("src/main.rs", content.as_str())];
    let (handler, _services, _repo) = setup_handler_with_session(&files, "max-symbol-test").await;

    let args = json!({
        "symbol": long_symbol,
        "session": "max-symbol-test"
    });

    // 200 chars is the maximum, should succeed
    let result = handler.execute(args).await;
    assert!(result.is_ok(), "200-char symbol should be accepted");
}

#[tokio::test]
async fn test_context_lines_zero() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "context-zero-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "context-zero-test",
        "context_lines": 0
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // With context_lines=0, output should be minimal per reference
    assert!(
        text.contains("calculate_total"),
        "Should still find the symbol"
    );
}

#[tokio::test]
async fn test_context_lines_max_10() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "context-max-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "context-max-test",
        "context_lines": 10
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // With context_lines=10, output should include more surrounding lines
    assert!(
        text.contains("calculate_total"),
        "Should find the symbol with max context"
    );
}

#[tokio::test]
async fn test_max_results_one() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "max-one-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "max-one-test",
        "max_results": 1
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should return exactly one result (the highest confidence match)
    assert!(
        text.contains("1 found") || text.contains("(1)"),
        "Should return only 1 result: {}",
        text
    );
}

#[tokio::test]
async fn test_max_results_200() {
    // Create fixture with many occurrences
    let mut files = Vec::new();
    for i in 0..50 {
        let filename = format!("src/mod_{}.rs", i);
        let content = format!("fn process() {{ my_func(); }}\nfn my_func() {{}}");
        files.push((filename, content));
    }
    let files_ref: Vec<(&str, &str)> = files
        .iter()
        .map(|(f, c)| (f.as_str(), c.as_str()))
        .collect();
    let (handler, _services, _repo) = setup_handler_with_session(&files_ref, "max-200-test").await;

    let args = json!({
        "symbol": "my_func",
        "session": "max-200-test",
        "max_results": 200
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should handle large max_results
    assert!(text.contains("my_func"), "Should find symbol");
}

#[tokio::test]
async fn test_no_references_found() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "no-references-test").await;

    let args = json!({
        "symbol": "nonexistent_symbol_xyz",
        "session": "no-references-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should return "No references found" message
    assert!(
        text.contains("No references found"),
        "Should indicate no references: {}",
        text
    );
}

#[tokio::test]
async fn test_many_references_found() {
    // Create fixture with many occurrences of the same symbol
    let mut files_vec = Vec::new();
    for i in 0..20 {
        let filename = format!("src/file_{}.rs", i);
        let content = format!(
            "use crate::common_func;\nfn test_{}() {{ common_func(); common_func(); }}",
            i
        );
        files_vec.push((filename, content));
    }
    files_vec.push((
        "src/common.rs".to_string(),
        "pub fn common_func() {}".to_string(),
    ));

    let files_ref: Vec<(&str, &str)> = files_vec
        .iter()
        .map(|(f, c)| (f.as_str(), c.as_str()))
        .collect();
    let (handler, _services, _repo) =
        setup_handler_with_session(&files_ref, "many-refs-test").await;

    let args = json!({
        "symbol": "common_func",
        "session": "many-refs-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should find multiple references
    assert!(
        text.contains("found"),
        "Should indicate references found: {}",
        text
    );
}

// =============================================================================
// Corner Tests (4 tests) - Combined Boundaries
// =============================================================================

#[tokio::test]
async fn test_short_symbol_min_results_no_context() {
    let files = &[("src/main.rs", "fn ok() { ok(); ok(); }")];
    let (handler, _services, _repo) = setup_handler_with_session(files, "corner-test-1").await;

    let args = json!({
        "symbol": "ok",
        "session": "corner-test-1",
        "max_results": 1,
        "context_lines": 0
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should handle combined minimum boundaries
    assert!(
        text.contains("ok") || text.contains("found"),
        "Should find the symbol"
    );
}

#[tokio::test]
async fn test_defined_in_excludes_definition_file() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "exclude-def-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "exclude-def-test",
        "defined_in": "src/lib.rs",
        "include_definition": false
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should not include lib.rs in results (where symbol is defined)
    // Note: This depends on whether the output mentions the definition file
    assert!(
        !text.contains("src/lib.rs:1") && !text.contains("src/lib.rs:2"),
        "Definition file should be excluded from results"
    );
}

#[tokio::test]
async fn test_include_definition_true() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "include-def-test").await;

    let args = json!({
        "symbol": "calculate_total",
        "session": "include-def-test",
        "defined_in": "src/lib.rs",
        "include_definition": true
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should include definition file when flag is true
    assert!(
        text.contains("lib.rs"),
        "Definition file should be included: {}",
        text
    );
}

#[tokio::test]
async fn test_multiple_boundaries_combined() {
    // Long symbol + max context + large max_results
    let long_symbol = "very_long_function_name_for_testing";
    let content = format!(
        "fn {}() {{}}\nfn caller() {{ {}(); }}",
        long_symbol, long_symbol
    );
    let files = vec![("src/main.rs", content.as_str())];
    let (handler, _services, _repo) =
        setup_handler_with_session(&files, "multi-boundary-test").await;

    let args = json!({
        "symbol": long_symbol,
        "session": "multi-boundary-test",
        "context_lines": 10,
        "max_results": 200
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    assert!(
        text.contains(long_symbol),
        "Should handle combined boundaries"
    );
}

// =============================================================================
// Error Tests (4 tests) - Off-Nominal Conditions
// =============================================================================

#[tokio::test]
async fn test_empty_symbol_rejected() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "empty-symbol-test").await;

    let args = json!({
        "symbol": "",
        "session": "empty-symbol-test"
    });

    let result = handler.execute(args).await;
    assert!(result.is_err(), "Empty symbol should be rejected");

    if let Err(McpError::InvalidParams(msg)) = result {
        assert!(
            msg.contains("empty") || msg.contains("2 characters"),
            "Error should mention empty/length: {}",
            msg
        );
    }
}

#[tokio::test]
async fn test_single_char_symbol_rejected() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "single-char-test").await;

    let args = json!({
        "symbol": "a",
        "session": "single-char-test"
    });

    let result = handler.execute(args).await;
    assert!(result.is_err(), "Single-char symbol should be rejected");

    if let Err(McpError::InvalidParams(msg)) = result {
        assert!(
            msg.contains("2 characters"),
            "Error should mention minimum length: {}",
            msg
        );
    }
}

#[tokio::test]
async fn test_whitespace_symbol_rejected() {
    let (handler, _services, _repo) =
        setup_handler_with_session(RUST_FIXTURE, "whitespace-symbol-test").await;

    let args = json!({
        "symbol": "   ",
        "session": "whitespace-symbol-test"
    });

    let result = handler.execute(args).await;
    assert!(result.is_err(), "Whitespace-only symbol should be rejected");

    if let Err(McpError::InvalidParams(msg)) = result {
        assert!(
            msg.contains("empty") || msg.contains("cannot"),
            "Error should indicate invalid: {}",
            msg
        );
    }
}

#[tokio::test]
async fn test_nonexistent_session() {
    let services = Arc::new(create_test_services());
    let handler = FindReferencesHandler::new(Arc::clone(&services));

    let args = json!({
        "symbol": "test_func",
        "session": "nonexistent-session-xyz"
    });

    let result = handler.execute(args).await;
    assert!(result.is_err(), "Nonexistent session should return error");
}

// =============================================================================
// Confidence Scoring Tests (4 tests)
// =============================================================================

#[tokio::test]
async fn test_function_call_high_confidence() {
    let files = &[(
        "src/main.rs",
        "fn my_function() {}\nfn caller() { my_function(); }",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "high-conf-test").await;

    let args = json!({
        "symbol": "my_function",
        "session": "high-conf-test",
        "symbol_type": "function"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Function call pattern should result in high confidence
    assert!(
        text.contains("High Confidence") || text.contains("0.9"),
        "Function call should have high confidence: {}",
        text
    );
}

#[tokio::test]
async fn test_comment_low_confidence() {
    let files = &[(
        "src/main.rs",
        "// my_symbol is used for testing\nfn other() {}",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "comment-conf-test").await;

    let args = json!({
        "symbol": "my_symbol",
        "session": "comment-conf-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Symbol in comment should have reduced confidence
    if text.contains("found") && !text.contains("No references") {
        assert!(
            text.contains("Low Confidence") || text.contains("Medium Confidence"),
            "Comment should reduce confidence: {}",
            text
        );
    }
}

#[tokio::test]
async fn test_doc_file_low_confidence() {
    let files = &[(
        "README.md",
        "# API\n\nThe `my_api_function` is the main entry point.",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "doc-conf-test").await;

    let args = json!({
        "symbol": "my_api_function",
        "session": "doc-conf-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Symbol in .md file should have low confidence
    if text.contains("found") && !text.contains("No references") {
        assert!(
            text.contains("Low Confidence"),
            "Doc file should have low confidence: {}",
            text
        );
    }
}

#[tokio::test]
async fn test_test_file_confidence_boost() {
    let files = &[(
        "tests/integration_test.rs",
        "#[test]\nfn test_my_function() { my_function(); }",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "test-boost-test").await;

    let args = json!({
        "symbol": "my_function",
        "session": "test-boost-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Test file should get +0.05 confidence boost
    // The function_call pattern (0.95) + test boost (0.05) = 1.0 (clamped)
    if text.contains("found") && !text.contains("No references") {
        assert!(
            text.contains("High Confidence") || text.contains("0.9") || text.contains("1.00"),
            "Test file should boost confidence: {}",
            text
        );
    }
}

// =============================================================================
// Multi-Language Tests (3 tests)
// =============================================================================

#[tokio::test]
async fn test_rust_use_statement() {
    let files = &[
        ("src/lib.rs", "pub mod utils;\npub use utils::helper_func;"),
        ("src/utils.rs", "pub fn helper_func() {}"),
    ];
    let (handler, _services, _repo) = setup_handler_with_session(files, "rust-use-test").await;

    let args = json!({
        "symbol": "helper_func",
        "session": "rust-use-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should match Rust use statement
    assert!(
        text.contains("helper_func"),
        "Should find Rust use statement: {}",
        text
    );
}

#[tokio::test]
async fn test_python_import() {
    let files = &[(
        "main.py",
        "from utils import process_data\n\ndef main():\n    process_data()",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "python-import-test").await;

    let args = json!({
        "symbol": "process_data",
        "session": "python-import-test"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should match Python from...import pattern
    assert!(
        text.contains("process_data"),
        "Should find Python import: {}",
        text
    );
}

#[tokio::test]
async fn test_go_function_call() {
    let files = &[(
        "main.go",
        "package main\n\nfunc HandleRequest(r *Request) {\n    HandleRequest_internal(r)\n}",
    )];
    let (handler, _services, _repo) = setup_handler_with_session(files, "go-func-test").await;

    let args = json!({
        "symbol": "HandleRequest",
        "session": "go-func-test",
        "symbol_type": "function"
    });

    let result = handler.execute(args).await.expect("Execute failed");
    let text = extract_text(&result);

    // Should match Go function calls
    assert!(
        text.contains("HandleRequest"),
        "Should find Go function: {}",
        text
    );
}
