// Test helper functions

use shebe::core::config::Config;
use shebe::core::indexer::IndexingPipeline;
use shebe::core::services::Services;
use shebe::core::storage::SessionConfig;
use shebe::core::types::IndexStats;
use std::path::Path;

/// Create test services with temporary storage
#[allow(dead_code)] // Used in integration tests
pub fn create_test_services() -> Services {
    let mut config = Config::default();

    // Use temporary directory for tests
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    config.storage.index_dir = temp_dir.path().to_path_buf();
    // Keep temp dir alive for duration of test
    std::mem::forget(temp_dir);

    Services::new(config)
}

/// Assert that index stats are valid
#[allow(dead_code)] // Used in integration tests
pub fn assert_valid_stats(stats: &IndexStats) {
    assert!(
        stats.files_indexed > 0,
        "Expected files_indexed > 0, got {}",
        stats.files_indexed
    );
    assert!(
        stats.chunks_created > 0,
        "Expected chunks_created > 0, got {}",
        stats.chunks_created
    );
    assert!(
        stats.chunks_created >= stats.files_indexed,
        "Expected chunks_created ({}) >= files_indexed ({})",
        stats.chunks_created,
        stats.files_indexed
    );
    assert!(
        stats.duration_ms > 0,
        "Expected duration_ms > 0, got {}",
        stats.duration_ms
    );
}

/// Index a test repository and return the session ID
#[allow(dead_code)] // Used in integration tests
pub async fn index_test_repository(
    services: &Services,
    repo_path: &Path,
    session_id: &str,
) -> IndexStats {
    index_test_repository_with_patterns(services, repo_path, session_id, vec![], vec![]).await
}

/// Index a test repository with custom patterns
#[allow(dead_code)] // Used in integration tests
pub async fn index_test_repository_with_patterns(
    services: &Services,
    repo_path: &Path,
    session_id: &str,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> IndexStats {
    let config = &services.config;

    // Prepare patterns for both pipeline and SessionConfig
    let include_for_config = if include_patterns.is_empty() {
        vec!["**/*".to_string()]
    } else {
        include_patterns.clone()
    };

    let exclude_for_config = if exclude_patterns.is_empty() {
        vec![
            "**/target/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/.git/**".to_string(),
        ]
    } else {
        exclude_patterns.clone()
    };

    // Create indexing pipeline
    let pipeline = IndexingPipeline::new(
        config.indexing.chunk_size,
        config.indexing.overlap,
        include_patterns,
        exclude_patterns,
        config.indexing.max_file_size_mb,
    )
    .expect("Failed to create indexing pipeline");

    // Index directory
    let start = std::time::Instant::now();
    let (chunks, stats) = pipeline
        .index_directory(repo_path)
        .expect("Failed to index directory");

    // Create session
    let mut index = services
        .storage
        .create_session(
            session_id,
            repo_path.to_path_buf(),
            SessionConfig {
                chunk_size: config.indexing.chunk_size,
                overlap: config.indexing.overlap,
                include_patterns: include_for_config.clone(),
                exclude_patterns: exclude_for_config.clone(),
            },
        )
        .expect("Failed to create session");

    // Add chunks to index
    index
        .add_chunks(&chunks, session_id)
        .expect("Failed to add chunks");

    // Commit index
    index.commit().expect("Failed to commit index");

    let duration_ms = start.elapsed().as_millis() as u64;

    // Update session metadata after indexing (critical fix for bug)
    let session_path = services.storage.get_session_path(session_id);
    let index_size_bytes = calculate_index_size(&session_path);

    use chrono::Utc;
    use shebe::core::storage::SessionMetadata;
    let now = Utc::now();
    let metadata = SessionMetadata {
        id: session_id.to_string(),
        repository_path: repo_path.to_path_buf(),
        created_at: now,
        last_indexed_at: now,
        files_indexed: stats.files_indexed,
        chunks_created: stats.chunks_created,
        index_size_bytes,
        config: SessionConfig {
            chunk_size: config.indexing.chunk_size,
            overlap: config.indexing.overlap,
            include_patterns: include_for_config,
            exclude_patterns: exclude_for_config,
        },
        schema_version: 3,
    };

    services
        .storage
        .update_session_metadata(session_id, &metadata)
        .expect("Failed to update session metadata");

    IndexStats {
        files_indexed: stats.files_indexed,
        chunks_created: stats.chunks_created,
        duration_ms,
        session: session_id.to_string(),
    }
}

/// Calculate total size of index directory on disk
#[allow(dead_code)] // Used by index_test_repository_with_patterns
fn calculate_index_size(session_path: &Path) -> u64 {
    use walkdir::WalkDir;

    let mut total_size = 0u64;

    // Walk the tantivy directory
    let tantivy_path = session_path.join("tantivy");
    if tantivy_path.exists() {
        for entry in WalkDir::new(&tantivy_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
    }

    total_size
}

/// Wait for async operation with timeout
#[allow(dead_code)] // Reserved for future async tests
pub async fn wait_with_timeout<F, T>(future: F, timeout_ms: u64) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    let timeout = tokio::time::Duration::from_millis(timeout_ms);
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => Ok(result),
        Err(_) => Err(format!("Operation timed out after {}ms", timeout_ms)),
    }
}
