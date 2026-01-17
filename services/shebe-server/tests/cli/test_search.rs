//! Tests for search-code CLI command
//!
//! Tests the search command handler with various scenarios:
//! - Valid queries with results
//! - Empty results
//! - Session not found errors
//! - Output format variations

use crate::cli::test_helpers::{create_cli_test_services, create_test_repo, setup_indexed_session};
use shebe::cli::commands::search::{execute, SearchArgs};
use shebe::cli::OutputFormat;

/// Test search with valid query returning results
#[tokio::test]
async fn test_search_valid_query_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() { println!(\"hello\"); }"),
        ("src/lib.rs", "pub fn helper() { println!(\"world\"); }"),
    ]);

    setup_indexed_session(&services, repo.path(), "search-test").await;

    let args = SearchArgs {
        query: "println".to_string(),
        session: "search-test".to_string(),
        limit: 10,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Search should succeed: {:?}", result.err());
}

/// Test search with valid query in JSON format
#[tokio::test]
async fn test_search_valid_query_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn main() { println!(\"test\"); }")]);

    setup_indexed_session(&services, repo.path(), "json-test").await;

    let args = SearchArgs {
        query: "main".to_string(),
        session: "json-test".to_string(),
        limit: 5,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Json).await;
    assert!(
        result.is_ok(),
        "JSON search should succeed: {:?}",
        result.err()
    );
}

/// Test search with no matches
#[tokio::test]
async fn test_search_empty_results() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn main() {}")]);

    setup_indexed_session(&services, repo.path(), "empty-test").await;

    let args = SearchArgs {
        query: "nonexistent_symbol_xyz".to_string(),
        session: "empty-test".to_string(),
        limit: 10,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Search with no results should succeed");
}

/// Test search on non-existent session
#[tokio::test]
async fn test_search_session_not_found() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = SearchArgs {
        query: "test".to_string(),
        session: "nonexistent-session".to_string(),
        limit: 10,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Search on missing session should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error should mention 'not found': {}",
        err_msg
    );
}

/// Test search with --files-only flag
#[tokio::test]
async fn test_search_files_only() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() { test_function(); }"),
        ("src/lib.rs", "pub fn test_function() {}"),
    ]);

    setup_indexed_session(&services, repo.path(), "files-only-test").await;

    let args = SearchArgs {
        query: "test_function".to_string(),
        session: "files-only-test".to_string(),
        limit: 10,
        files_only: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Files-only search should succeed");
}

/// Test search limit clamping (values outside 1-100 should be clamped)
#[tokio::test]
async fn test_search_limit_clamping() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn main() {}")]);

    setup_indexed_session(&services, repo.path(), "limit-test").await;

    // Test with limit > 100 (should be clamped to 100)
    let args = SearchArgs {
        query: "main".to_string(),
        session: "limit-test".to_string(),
        limit: 500,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Search with high limit should succeed (clamped)"
    );

    // Test with limit = 0 (should be clamped to 1)
    let args_zero = SearchArgs {
        query: "main".to_string(),
        session: "limit-test".to_string(),
        limit: 0,
        files_only: false,
    };

    let result_zero = execute(args_zero, &services, OutputFormat::Human).await;
    assert!(
        result_zero.is_ok(),
        "Search with zero limit should succeed (clamped to 1)"
    );
}

/// Test search with boolean operators (AND, OR)
#[tokio::test]
async fn test_search_boolean_query() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() { start_server(); }"),
        ("src/server.rs", "pub fn start_server() { listen(); }"),
        ("src/client.rs", "pub fn connect() { send(); }"),
    ]);

    setup_indexed_session(&services, repo.path(), "bool-test").await;

    // Test AND query
    let args = SearchArgs {
        query: "start AND server".to_string(),
        session: "bool-test".to_string(),
        limit: 10,
        files_only: false,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Boolean AND query should succeed");
}
