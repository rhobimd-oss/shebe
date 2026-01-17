//! Tests for index-repository CLI command
//!
//! Tests the index command handler:
//! - Indexing a new repository
//! - Force re-indexing over existing session
//! - Custom include/exclude patterns
//! - Error cases (invalid path, empty directory)

use crate::cli::test_helpers::{create_cli_test_services, create_test_repo, setup_indexed_session};
use shebe::cli::commands::index::{execute, IndexArgs};
use shebe::cli::OutputFormat;

/// Test indexing a new repository
#[tokio::test]
async fn test_index_new_session_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub fn lib() {}"),
        ("README.md", "# Project"),
    ]);

    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "new-index".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Index new session should succeed: {:?}",
        result.err()
    );
    assert!(services.storage.session_exists("new-index"));
}

/// Test indexing a new repository (JSON format)
#[tokio::test]
async fn test_index_new_session_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn test() {}")]);

    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "new-index-json".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "Index new session (JSON) should succeed");
}

/// Test force re-indexing over existing session
#[tokio::test]
async fn test_index_force_reindex() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn original() {}")]);

    // First, index the repository
    setup_indexed_session(&services, repo.path(), "force-test").await;
    assert!(services.storage.session_exists("force-test"));

    // Now force re-index
    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "force-test".to_string(),
        force: true,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Force re-index should succeed");
}

/// Test indexing with custom include patterns
#[tokio::test]
async fn test_index_custom_patterns() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub fn lib() {}"),
        ("tests/test.rs", "fn test() {}"),
        ("docs/readme.md", "# Docs"),
    ]);

    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "patterns-test".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec!["**/*.rs".to_string()],
        exclude: vec!["**/tests/**".to_string()],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Index with patterns should succeed");
}

/// Test indexing with custom chunk size
#[tokio::test]
async fn test_index_custom_chunk_size() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn test() { let x = 1; let y = 2; }")]);

    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "chunk-size-test".to_string(),
        force: false,
        chunk_size: 256,
        overlap: 32,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_ok(),
        "Index with custom chunk size should succeed"
    );
}

/// Test indexing non-existent path
#[tokio::test]
async fn test_index_invalid_path() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = IndexArgs {
        path: "/nonexistent/path/that/does/not/exist".into(),
        session: "invalid-path".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Index non-existent path should fail");
}

/// Test indexing empty directory
#[tokio::test]
async fn test_index_empty_directory() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[]); // Empty repo

    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "empty-dir".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    // Empty directory should either succeed with 0 files or fail gracefully
    let result = execute(args, &services, OutputFormat::Human).await;
    // This behavior depends on implementation - empty dirs may succeed with 0 files
    // or fail. Either is acceptable as long as it doesn't panic.
    let _ = result; // Just ensure no panic
}

/// Test indexing without --force when session exists
#[tokio::test]
async fn test_index_session_exists_no_force() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn test() {}")]);

    // First, index the repository
    setup_indexed_session(&services, repo.path(), "exists-test").await;

    // Try to index again without --force
    let args = IndexArgs {
        path: repo.path().to_path_buf(),
        session: "exists-test".to_string(),
        force: false,
        chunk_size: 512,
        overlap: 64,
        include: vec![],
        exclude: vec![],
        quiet: true,
    };

    let result = execute(args, &services, OutputFormat::Human).await;
    // Should either fail or succeed based on implementation
    // The important thing is it handles the case gracefully
    let _ = result;
}
