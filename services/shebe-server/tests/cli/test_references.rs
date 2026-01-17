//! Tests for find-references CLI command
//!
//! Tests the references command handler:
//! - Finding function references
//! - Finding type references
//! - Symbol validation (min 2 chars)
//! - Session not found errors
//! - Confidence filtering
//! - Context lines extraction

use crate::cli::test_helpers::{
    create_cli_test_services, create_test_repo, references_test_files, setup_indexed_session,
};
use shebe::cli::commands::references::{execute, ReferencesArgs, SymbolTypeArg};
use shebe::cli::OutputFormat;

/// Test finding function references
#[tokio::test]
async fn test_references_function_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&references_test_files());

    setup_indexed_session(&services, repo.path(), "refs-func").await;

    let args = ReferencesArgs {
        symbol: "helper_function".to_string(),
        session: "refs-func".to_string(),
        symbol_type: SymbolTypeArg::Function,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Finding function references should succeed: {:?}",
        result.err()
    );
}

/// Test finding function references (JSON format)
#[tokio::test]
async fn test_references_function_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&references_test_files());

    setup_indexed_session(&services, repo.path(), "refs-json").await;

    let args = ReferencesArgs {
        symbol: "helper_function".to_string(),
        session: "refs-json".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: true,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Json).await;
    assert!(
        result.is_ok(),
        "Finding references (JSON) should succeed: {:?}",
        result.err()
    );
}

/// Test finding type references
#[tokio::test]
async fn test_references_type() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        (
            "src/types.rs",
            r#"pub struct Config {
    pub port: u16,
    pub host: String,
}

pub struct Server {
    config: Config,
}"#,
        ),
        (
            "src/main.rs",
            r#"use crate::Config;

fn create_config() -> Config {
    Config { port: 8080, host: "localhost".to_string() }
}

fn use_config(config: Config) {
    println!("{}", config.port);
}"#,
        ),
    ]);

    setup_indexed_session(&services, repo.path(), "refs-type").await;

    let args = ReferencesArgs {
        symbol: "Config".to_string(),
        session: "refs-type".to_string(),
        symbol_type: SymbolTypeArg::Type,
        defined_in: None,
        include_definition: true,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Finding type references should succeed: {:?}",
        result.err()
    );
}

/// Test no references found
#[tokio::test]
async fn test_references_no_matches() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn main() {}")]);

    setup_indexed_session(&services, repo.path(), "refs-empty").await;

    let args = ReferencesArgs {
        symbol: "nonexistent_symbol_xyz".to_string(),
        session: "refs-empty".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    // Should succeed even with no results
    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "No matches should still succeed");
}

/// Test no references found (JSON format)
#[tokio::test]
async fn test_references_no_matches_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn main() {}")]);

    setup_indexed_session(&services, repo.path(), "refs-empty-json").await;

    let args = ReferencesArgs {
        symbol: "nonexistent_xyz".to_string(),
        session: "refs-empty-json".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "No matches (JSON) should still succeed");
}

/// Test session not found
#[tokio::test]
async fn test_references_session_not_found() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ReferencesArgs {
        symbol: "test_symbol".to_string(),
        session: "nonexistent-session".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Missing session should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error should mention 'not found': {}",
        err_msg
    );
}

/// Test symbol too short (must be at least 2 characters)
#[tokio::test]
async fn test_references_symbol_too_short() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn x() {}")]);

    setup_indexed_session(&services, repo.path(), "refs-short").await;

    let args = ReferencesArgs {
        symbol: "x".to_string(), // Only 1 character
        session: "refs-short".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Symbol too short should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("2 characters"),
        "Error should mention minimum length: {}",
        err_msg
    );
}

/// Test empty symbol
#[tokio::test]
async fn test_references_empty_symbol() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn test() {}")]);

    setup_indexed_session(&services, repo.path(), "refs-empty-sym").await;

    let args = ReferencesArgs {
        symbol: "".to_string(),
        session: "refs-empty-sym".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Empty symbol should fail");
}

/// Test with defined_in filter (exclude definition file)
#[tokio::test]
async fn test_references_with_defined_in() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/lib.rs", "pub fn my_func() {}"),
        (
            "src/main.rs",
            "use crate::my_func;\nfn main() { my_func(); }",
        ),
    ]);

    setup_indexed_session(&services, repo.path(), "refs-defined").await;

    let args = ReferencesArgs {
        symbol: "my_func".to_string(),
        session: "refs-defined".to_string(),
        symbol_type: SymbolTypeArg::Function,
        defined_in: Some("lib.rs".to_string()), // Exclude definition
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "References with defined_in should succeed");
}

/// Test max_results limit
#[tokio::test]
async fn test_references_max_results() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&references_test_files());

    setup_indexed_session(&services, repo.path(), "refs-limit").await;

    let args = ReferencesArgs {
        symbol: "helper_function".to_string(),
        session: "refs-limit".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: true,
        context_lines: 2,
        max_results: 2, // Limit to 2 results
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "References with max_results should succeed");
}

/// Test context_lines parameter
#[tokio::test]
async fn test_references_context_lines() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&references_test_files());

    setup_indexed_session(&services, repo.path(), "refs-context").await;

    // Test with 0 context lines
    let args = ReferencesArgs {
        symbol: "helper_function".to_string(),
        session: "refs-context".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: true,
        context_lines: 0,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "References with 0 context should succeed");

    // Test with max context lines (clamped to 10)
    let args_max = ReferencesArgs {
        symbol: "helper_function".to_string(),
        session: "refs-context".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: true,
        context_lines: 100, // Should be clamped to 10
        max_results: 50,
    };

    let result_max = execute(args_max, &services, OutputFormat::Human).await;
    assert!(
        result_max.is_ok(),
        "References with clamped context should succeed"
    );
}

/// Test variable symbol type
#[tokio::test]
async fn test_references_variable_type() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[(
        "src/main.rs",
        "let config = load();\nlet x = config.port;\nconfig.host = \"test\";",
    )]);

    setup_indexed_session(&services, repo.path(), "refs-var").await;

    let args = ReferencesArgs {
        symbol: "config".to_string(),
        session: "refs-var".to_string(),
        symbol_type: SymbolTypeArg::Variable,
        defined_in: None,
        include_definition: true,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Variable references should succeed: {:?}",
        result.err()
    );
}

/// Test whitespace-only symbol (should be rejected)
#[tokio::test]
async fn test_references_whitespace_symbol() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("src/main.rs", "fn test() {}")]);

    setup_indexed_session(&services, repo.path(), "refs-ws").await;

    let args = ReferencesArgs {
        symbol: "   ".to_string(), // Whitespace only
        session: "refs-ws".to_string(),
        symbol_type: SymbolTypeArg::Any,
        defined_in: None,
        include_definition: false,
        context_lines: 2,
        max_results: 50,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Whitespace-only symbol should fail");
}
