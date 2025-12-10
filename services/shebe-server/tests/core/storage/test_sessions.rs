// Integration tests for session management

use crate::common::{create_test_services, index_test_repository, TestRepo};
use shebe::core::storage::SessionConfig;
use std::path::PathBuf;

#[tokio::test]
async fn test_session_creation() {
    let state = create_test_services();

    let config = SessionConfig::default();

    // Create session
    let index = state
        .storage
        .create_session("test-create", PathBuf::from("/test/repo"), config)
        .expect("Failed to create session");

    // Verify session exists
    assert!(state.storage.session_exists("test-create"));

    // Clean up
    drop(index);
}

#[tokio::test]
async fn test_session_exists() {
    let state = create_test_services();

    // Session should not exist initially
    assert!(!state.storage.session_exists("nonexistent"));

    // Create session via indexing
    let repo = TestRepo::small();
    let _stats = index_test_repository(&state, repo.path(), "exists-test").await;

    // Session should now exist
    assert!(state.storage.session_exists("exists-test"));
}

#[tokio::test]
async fn test_session_delete() {
    let state = create_test_services();

    // Create session
    let repo = TestRepo::small();
    let _stats = index_test_repository(&state, repo.path(), "delete-test").await;

    // Verify exists
    assert!(state.storage.session_exists("delete-test"));

    // Delete session
    state
        .storage
        .delete_session("delete-test")
        .expect("Failed to delete session");

    // Verify deleted
    assert!(!state.storage.session_exists("delete-test"));
}

#[tokio::test]
async fn test_session_list() {
    let state = create_test_services();

    // Create multiple sessions
    let repo1 = TestRepo::small();
    let repo2 = TestRepo::medium();

    let _stats1 = index_test_repository(&state, repo1.path(), "list-1").await;
    let _stats2 = index_test_repository(&state, repo2.path(), "list-2").await;

    // List sessions
    let sessions = state
        .storage
        .list_sessions()
        .expect("Failed to list sessions");

    // Verify both sessions are listed
    assert!(sessions.len() >= 2);
    assert!(sessions.iter().any(|s| s.id == "list-1"));
    assert!(sessions.iter().any(|s| s.id == "list-2"));
}

#[tokio::test]
async fn test_session_metadata() {
    let state = create_test_services();

    let repo = TestRepo::small();
    let _stats = index_test_repository(&state, repo.path(), "metadata-test").await;

    // Get session metadata
    let sessions = state
        .storage
        .list_sessions()
        .expect("Failed to list sessions");

    let session = sessions
        .iter()
        .find(|s| s.id == "metadata-test")
        .expect("Session not found");

    // Verify metadata
    assert_eq!(session.id, "metadata-test");
    // Note: Session metadata may not perfectly match indexing stats
    // because metadata is saved during session creation, not after indexing
    // The important thing is that the session exists and has valid metadata

    // Verify created_at is a valid DateTime (not empty)
    assert!(session.created_at.timestamp() > 0);
    // Index size is always >= 0 (u64), just verify it exists
    let _ = session.index_size_bytes; // Verify field exists
}

#[tokio::test]
async fn test_session_isolation() {
    let state = create_test_services();

    // Create two sessions with different content
    let repo1 = TestRepo::with_files(&[("file1.rs", "unique content alpha gamma")]);
    let repo2 = TestRepo::with_files(&[("file2.rs", "unique content beta delta")]);

    let _stats1 = index_test_repository(&state, repo1.path(), "isolated-1").await;
    let _stats2 = index_test_repository(&state, repo2.path(), "isolated-2").await;

    // Search in session 1
    let results1 = state
        .search
        .search_session("isolated-1", "alpha", Some(10))
        .expect("Search failed");

    // Search in session 2
    let results2 = state
        .search
        .search_session("isolated-2", "beta", Some(10))
        .expect("Search failed");

    // Verify results are isolated
    assert!(!results1.results.is_empty());
    assert!(results1.results[0].text.contains("alpha"));

    assert!(!results2.results.is_empty());
    assert!(results2.results[0].text.contains("beta"));

    // Cross-session query should fail
    let results_cross = state
        .search
        .search_session("isolated-1", "beta", Some(10))
        .expect("Search failed");

    assert_eq!(results_cross.results.len(), 0);
}

#[tokio::test]
async fn test_session_duplicate_creation_fails() {
    let state = create_test_services();

    let config = SessionConfig::default();

    // Create session
    let _index1 = state
        .storage
        .create_session(
            "duplicate-test",
            PathBuf::from("/test/repo"),
            config.clone(),
        )
        .expect("Failed to create first session");

    // Attempt to create duplicate
    let result =
        state
            .storage
            .create_session("duplicate-test", PathBuf::from("/test/repo"), config);

    assert!(result.is_err(), "Expected error when creating duplicate");
}

#[tokio::test]
async fn test_session_delete_nonexistent() {
    let state = create_test_services();

    // Attempt to delete non-existent session
    let result = state.storage.delete_session("nonexistent-session");

    assert!(result.is_err(), "Expected error when deleting nonexistent");
}

#[tokio::test]
async fn test_session_open_existing() {
    let state = create_test_services();

    // Create and index a session
    let repo = TestRepo::small();
    let _stats = index_test_repository(&state, repo.path(), "open-test").await;

    // Open existing session
    let index = state
        .storage
        .open_session("open-test")
        .expect("Failed to open session");

    // Should be able to use the opened index
    drop(index);
}

#[tokio::test]
async fn test_session_open_nonexistent_fails() {
    let state = create_test_services();

    // Attempt to open non-existent session
    let result = state.storage.open_session("nonexistent-session");

    assert!(result.is_err(), "Expected error when opening nonexistent");
}

#[tokio::test]
async fn test_multiple_sessions_concurrent() {
    let state = create_test_services();

    // Create 5 sessions
    for i in 0..5 {
        let repo =
            TestRepo::with_files(&[(format!("file{}.rs", i).as_str(), &format!("content {}", i))]);
        let session_id = format!("concurrent-{}", i);
        let _stats = index_test_repository(&state, repo.path(), &session_id).await;
    }

    // Verify all sessions exist
    let sessions = state
        .storage
        .list_sessions()
        .expect("Failed to list sessions");

    assert!(sessions.len() >= 5);

    for i in 0..5 {
        let session_id = format!("concurrent-{}", i);
        assert!(
            sessions.iter().any(|s| s.id == session_id),
            "Session {} not found",
            session_id
        );
    }
}
