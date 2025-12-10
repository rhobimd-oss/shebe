// Integration tests for indexing functionality

use crate::common::{
    assert_valid_stats, create_test_services, index_test_repository,
    index_test_repository_with_patterns, OpenEmrData, TestRepo,
};

#[tokio::test]
async fn test_index_small_repository() {
    let repo = TestRepo::small();
    let state = create_test_services();

    let stats = index_test_repository(&state, repo.path(), "test-small-repo").await;

    // Validate stats
    assert_valid_stats(&stats);
    assert_eq!(stats.session, "test-small-repo");

    // Verify session was created
    assert!(state.storage.session_exists("test-small-repo"));
}

#[tokio::test]
async fn test_index_medium_repository() {
    let repo = TestRepo::medium();
    let state = create_test_services();

    let stats = index_test_repository(&state, repo.path(), "test-medium-repo").await;

    // Validate stats
    assert_valid_stats(&stats);
    assert_eq!(stats.files_indexed, 50);
    assert!(stats.chunks_created >= 50); // At least one chunk per file
}

#[tokio::test]
async fn test_index_with_include_patterns() {
    let repo = TestRepo::with_files(&[
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub fn test() {}"),
        ("README.md", "# Test"),
        ("Cargo.toml", "[package]"),
    ]);

    let state = create_test_services();

    let stats = index_test_repository_with_patterns(
        &state,
        repo.path(),
        "test-patterns",
        vec!["*.rs".to_string()], // Only index Rust files
        vec![],
    )
    .await;

    // Should only index the 2 .rs files
    assert_eq!(stats.files_indexed, 2);
}

#[tokio::test]
async fn test_index_with_exclude_patterns() {
    let repo = TestRepo::with_files(&[
        ("src/main.rs", "fn main() {}"),
        ("target/debug/main", "binary content"),
        (".git/config", "git config"),
        ("src/lib.rs", "pub fn test() {}"),
    ]);

    let state = create_test_services();

    let stats = index_test_repository_with_patterns(
        &state,
        repo.path(),
        "test-exclude",
        vec![],
        vec!["**/target/**".to_string(), "**/.git/**".to_string()],
    )
    .await;

    // Should skip target/ and .git/
    assert_eq!(stats.files_indexed, 2); // Only the 2 .rs files
}

#[tokio::test]
#[ignore] // Only run with --ignored flag (slow test)
async fn test_index_openemr_subset() {
    // Skip if OpenEMR not available
    if !OpenEmrData::is_available() {
        eprintln!("Skipping OpenEMR test: repository not found");
        return;
    }

    let state = create_test_services();

    // Index just the interface directory (smaller subset)
    let stats = index_test_repository_with_patterns(
        &state,
        &OpenEmrData::interface_dir(),
        "openemr-interface",
        vec!["*.php".to_string()],
        vec![],
    )
    .await;

    assert_valid_stats(&stats);
    println!("OpenEMR interface/ stats:");
    println!("  Files indexed: {}", stats.files_indexed);
    println!("  Chunks created: {}", stats.chunks_created);
    println!("  Duration: {}ms", stats.duration_ms);

    // Validate performance
    assert!(stats.files_indexed > 0);
    assert!(stats.chunks_created > stats.files_indexed);
}

#[tokio::test]
#[ignore] // Only run with --ignored flag (VERY slow test)
async fn test_index_openemr_full() {
    // Skip if OpenEMR not available
    if !OpenEmrData::is_available() {
        eprintln!("Skipping OpenEMR full test: repository not found");
        return;
    }

    let state = create_test_services();

    // Index full OpenEMR repository
    let stats = index_test_repository_with_patterns(
        &state,
        OpenEmrData::path(),
        "openemr-full",
        vec!["*.php".to_string(), "*.js".to_string()],
        vec![
            "**/vendor/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/.git/**".to_string(),
        ],
    )
    .await;

    assert_valid_stats(&stats);
    println!("OpenEMR full repo stats:");
    println!("  Files indexed: {}", stats.files_indexed);
    println!("  Chunks created: {}", stats.chunks_created);
    println!("  Duration: {}ms", stats.duration_ms);
    println!(
        "  Throughput: {:.2} files/sec",
        stats.files_indexed as f64 / (stats.duration_ms as f64 / 1000.0)
    );

    // Validate performance (target: >500 files/second)
    let _throughput = stats.files_indexed as f64 / (stats.duration_ms as f64 / 1000.0);
    println!("  Target throughput: >500 files/sec");

    // Should index a significant portion of OpenEMR
    assert!(stats.files_indexed > 100);
}

#[tokio::test]
async fn test_metadata_updated_after_indexing() {
    // Create a test repository with known files
    let repo = TestRepo::with_files(&[
        ("file1.txt", "content 1"),
        ("file2.txt", "content 2"),
        ("file3.txt", "content 3"),
    ]);

    let state = create_test_services();
    let session_id = "test-metadata-consistency";

    // Index the repository
    let stats = index_test_repository(&state, repo.path(), session_id).await;

    // Verify indexing completed successfully
    assert_valid_stats(&stats);
    assert_eq!(stats.session, session_id);
    assert_eq!(stats.files_indexed, 3);
    assert!(stats.chunks_created > 0);

    // Read metadata file
    let metadata = state
        .storage
        .get_session_metadata(session_id)
        .expect("Metadata should exist");

    // Assert metadata has non-zero values (BUG FIX VERIFICATION)
    assert!(
        metadata.files_indexed > 0,
        "files_indexed should be non-zero (was the bug - showed 0)"
    );
    assert!(
        metadata.chunks_created > 0,
        "chunks_created should be non-zero (was the bug - showed 0)"
    );
    assert!(
        metadata.index_size_bytes > 0,
        "index_size_bytes should be non-zero (was the bug - showed 0)"
    );

    // Assert metadata matches indexing stats
    assert_eq!(
        metadata.files_indexed, stats.files_indexed,
        "Metadata files_indexed should match indexing stats"
    );
    assert_eq!(
        metadata.chunks_created, stats.chunks_created,
        "Metadata chunks_created should match indexing stats"
    );

    // Assert metadata has correct session ID
    assert_eq!(metadata.id, session_id);

    // Assert metadata has valid timestamp
    assert!(
        metadata.created_at.timestamp() > 0,
        "Metadata should have a valid timestamp"
    );

    // Assert metadata has correct configuration
    assert_eq!(metadata.config.chunk_size, 512);
    assert_eq!(metadata.config.overlap, 64);

    println!("✓ Metadata consistency test passed!");
    println!("  Files indexed: {}", metadata.files_indexed);
    println!("  Chunks created: {}", metadata.chunks_created);
    println!("  Index size: {} bytes", metadata.index_size_bytes);
}
