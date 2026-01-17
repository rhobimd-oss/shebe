//! Tests for session CLI commands (list, info, delete, reindex)
//!
//! Tests the session command handlers:
//! - list-sessions: List all indexed sessions
//! - get-session-info: Get detailed session metadata
//! - delete-session: Delete a session (with --force)
//! - reindex-session: Re-index a session

use crate::cli::test_helpers::{create_cli_test_services, create_test_repo, setup_indexed_session};
use shebe::cli::commands::session::{
    execute_delete, execute_info, execute_list, execute_reindex, DeleteArgs, InfoArgs, ListArgs,
    ReindexArgs,
};
use shebe::cli::OutputFormat;

// =============================================================================
// list-sessions tests
// =============================================================================

/// Test listing sessions when none exist
#[tokio::test]
async fn test_list_sessions_empty_human() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ListArgs {};
    let result = execute_list(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "List empty sessions should succeed");
}

/// Test listing sessions when none exist (JSON format)
#[tokio::test]
async fn test_list_sessions_empty_json() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ListArgs {};
    let result = execute_list(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "List empty sessions (JSON) should succeed");
}

/// Test listing a single session
#[tokio::test]
async fn test_list_sessions_single() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn test() {}")]);

    setup_indexed_session(&services, repo.path(), "single-session").await;

    let args = ListArgs {};
    let result = execute_list(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "List single session should succeed");
}

/// Test listing multiple sessions
#[tokio::test]
async fn test_list_sessions_multiple() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo1 = create_test_repo(&[("file1.rs", "fn one() {}")]);
    let repo2 = create_test_repo(&[("file2.rs", "fn two() {}")]);

    setup_indexed_session(&services, repo1.path(), "session-one").await;
    setup_indexed_session(&services, repo2.path(), "session-two").await;

    let args = ListArgs {};
    let result = execute_list(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "List multiple sessions should succeed");
}

// =============================================================================
// get-session-info tests
// =============================================================================

/// Test getting info for a valid session
#[tokio::test]
async fn test_info_valid_session_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub fn lib() {}"),
    ]);

    setup_indexed_session(&services, repo.path(), "info-test").await;

    let args = InfoArgs {
        session: "info-test".to_string(),
    };
    let result = execute_info(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Get session info should succeed");
}

/// Test getting info for a valid session (JSON format)
#[tokio::test]
async fn test_info_valid_session_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn test() {}")]);

    setup_indexed_session(&services, repo.path(), "info-json").await;

    let args = InfoArgs {
        session: "info-json".to_string(),
    };
    let result = execute_info(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "Get session info (JSON) should succeed");
}

/// Test getting info for non-existent session
#[tokio::test]
async fn test_info_session_not_found() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = InfoArgs {
        session: "nonexistent".to_string(),
    };
    let result = execute_info(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Get info for missing session should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error should mention 'not found': {}",
        err_msg
    );
}

// =============================================================================
// delete-session tests
// =============================================================================

/// Test deleting a session with --force flag
#[tokio::test]
async fn test_delete_force_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn delete_me() {}")]);

    setup_indexed_session(&services, repo.path(), "delete-test").await;
    assert!(services.storage.session_exists("delete-test"));

    let args = DeleteArgs {
        session: "delete-test".to_string(),
        force: true,
    };
    let result = execute_delete(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Delete with --force should succeed");
    assert!(
        !services.storage.session_exists("delete-test"),
        "Session should be deleted"
    );
}

/// Test deleting a session with --force flag (JSON format)
#[tokio::test]
async fn test_delete_force_json() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn delete_me() {}")]);

    setup_indexed_session(&services, repo.path(), "delete-json").await;

    let args = DeleteArgs {
        session: "delete-json".to_string(),
        force: true,
    };
    let result = execute_delete(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "Delete with --force (JSON) should succeed");
}

/// Test deleting non-existent session
#[tokio::test]
async fn test_delete_session_not_found() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = DeleteArgs {
        session: "nonexistent".to_string(),
        force: true,
    };
    let result = execute_delete(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Delete missing session should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error should mention 'not found': {}",
        err_msg
    );
}

// =============================================================================
// reindex-session tests
// =============================================================================

/// Test reindexing a session with --force flag
#[tokio::test]
async fn test_reindex_with_force_human() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn reindex_me() {}")]);

    setup_indexed_session(&services, repo.path(), "reindex-test").await;

    let args = ReindexArgs {
        session: "reindex-test".to_string(),
        chunk_size: None,
        overlap: None,
        force: true,
    };
    let result = execute_reindex(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Reindex with --force should succeed");
}

/// Test reindexing with config override
#[tokio::test]
async fn test_reindex_config_change() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn config_change() {}")]);

    setup_indexed_session(&services, repo.path(), "reindex-config").await;

    let args = ReindexArgs {
        session: "reindex-config".to_string(),
        chunk_size: Some(256),
        overlap: None,
        force: false, // Config change should allow reindex without --force
    };
    let result = execute_reindex(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Reindex with config change should succeed");
}

/// Test reindex without config change or --force (should fail)
#[tokio::test]
async fn test_reindex_no_change_error() {
    let (services, _storage_temp) = create_cli_test_services();
    let repo = create_test_repo(&[("file.rs", "fn no_change() {}")]);

    setup_indexed_session(&services, repo.path(), "reindex-nochange").await;

    let args = ReindexArgs {
        session: "reindex-nochange".to_string(),
        chunk_size: None,
        overlap: None,
        force: false,
    };
    let result = execute_reindex(args, &services, OutputFormat::Human).await;
    assert!(
        result.is_err(),
        "Reindex without change or --force should fail"
    );
}

/// Test reindex when repository path no longer exists
#[tokio::test]
async fn test_reindex_path_not_exists() {
    let (services, _storage_temp) = create_cli_test_services();

    // Create and index a repo, then delete the repo directory
    let repo = create_test_repo(&[("file.rs", "fn temp() {}")]);
    let repo_path = repo.path().to_path_buf();
    setup_indexed_session(&services, &repo_path, "reindex-deleted").await;

    // Drop the repo to delete the temp directory
    drop(repo);

    let args = ReindexArgs {
        session: "reindex-deleted".to_string(),
        chunk_size: None,
        overlap: None,
        force: true,
    };
    let result = execute_reindex(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Reindex with missing path should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("no longer exists"),
        "Error should mention path no longer exists: {}",
        err_msg
    );
}

/// Test reindex on non-existent session
#[tokio::test]
async fn test_reindex_session_not_found() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ReindexArgs {
        session: "nonexistent".to_string(),
        chunk_size: None,
        overlap: None,
        force: true,
    };
    let result = execute_reindex(args, &services, OutputFormat::Human).await;
    assert!(result.is_err(), "Reindex missing session should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error should mention 'not found': {}",
        err_msg
    );
}
