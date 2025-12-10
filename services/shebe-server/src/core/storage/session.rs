//! Session-based storage management.
//!
//! This module manages session-based indexes, including
//! creation, deletion, and metadata tracking.

use crate::core::error::{Result, ShebeError};
use crate::core::storage::tantivy::{TantivyIndex, SCHEMA_VERSION};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub chunk_size: usize,
    pub overlap: usize,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            overlap: 64,
            include_patterns: vec!["**/*".to_string()],
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
        }
    }
}

/// Session metadata (Schema v3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub repository_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_indexed_at: DateTime<Utc>,
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub index_size_bytes: u64,
    pub config: SessionConfig,
    pub schema_version: u32,
}

/// Session-based storage manager
pub struct StorageManager {
    /// Root directory for all sessions
    storage_root: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(storage_root: PathBuf) -> Self {
        Self { storage_root }
    }

    /// Get session directory path
    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.storage_root.join("sessions").join(session_id)
    }

    /// Get Tantivy index directory path
    fn tantivy_dir(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("tantivy")
    }

    /// Get metadata file path
    fn metadata_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("meta.json")
    }

    /// Create a new session
    pub fn create_session(
        &self,
        session_id: &str,
        repository_path: PathBuf,
        config: SessionConfig,
    ) -> Result<TantivyIndex> {
        let session_dir = self.session_dir(session_id);

        // Check if session already exists
        if session_dir.exists() {
            return Err(ShebeError::SessionAlreadyExists(session_id.to_string()));
        }

        // Create session directory
        fs::create_dir_all(&session_dir)?;

        // Create Tantivy index
        let tantivy_dir = self.tantivy_dir(session_id);
        let index = TantivyIndex::create(&tantivy_dir)?;

        // Write initial metadata
        let now = Utc::now();
        let metadata = SessionMetadata {
            id: session_id.to_string(),
            repository_path,
            created_at: now,
            last_indexed_at: now,
            files_indexed: 0,
            chunks_created: 0,
            index_size_bytes: 0,
            config,
            schema_version: SCHEMA_VERSION,
        };
        self.update_session_metadata(session_id, &metadata)?;

        Ok(index)
    }

    /// Open an existing session
    pub fn open_session(&self, session_id: &str) -> Result<TantivyIndex> {
        let tantivy_dir = self.tantivy_dir(session_id);

        if !tantivy_dir.exists() {
            return Err(ShebeError::SessionNotFound(session_id.to_string()));
        }

        // Check schema version compatibility
        let metadata = self.get_session_metadata(session_id)?;
        if metadata.schema_version < SCHEMA_VERSION {
            return Err(ShebeError::InvalidSession(format!(
                "Session '{}' uses old schema version {} (current: v{}). \
                 Missing fields: repository_path, last_indexed_at, patterns. \
                 Please re-index this session: \
                 mcp__shebe__index_repository(path=\"/path/to/repo\", session=\"{}\")",
                session_id, metadata.schema_version, SCHEMA_VERSION, session_id
            )));
        }

        TantivyIndex::open(&tantivy_dir)
    }

    /// Check if a session exists
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.session_dir(session_id).exists()
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let session_dir = self.session_dir(session_id);

        if !session_dir.exists() {
            return Err(ShebeError::SessionNotFound(session_id.to_string()));
        }

        fs::remove_dir_all(session_dir)?;
        Ok(())
    }

    /// Get session metadata
    pub fn get_session_metadata(&self, session_id: &str) -> Result<SessionMetadata> {
        let meta_path = self.metadata_path(session_id);

        if !meta_path.exists() {
            return Err(ShebeError::SessionNotFound(session_id.to_string()));
        }

        let contents = fs::read_to_string(&meta_path)?;
        let metadata: SessionMetadata = serde_json::from_str(&contents)?;

        Ok(metadata)
    }

    /// Update session metadata
    pub fn update_session_metadata(
        &self,
        session_id: &str,
        metadata: &SessionMetadata,
    ) -> Result<()> {
        let meta_path = self.metadata_path(session_id);

        let json = serde_json::to_string_pretty(metadata)?;
        fs::write(meta_path, json)?;

        Ok(())
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let sessions_dir = self.storage_root.join("sessions");

        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        for entry in fs::read_dir(sessions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(session_id) = entry.file_name().to_str() {
                    if let Ok(metadata) = self.get_session_metadata(session_id) {
                        sessions.push(metadata);
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get the full path to a session directory
    pub fn get_session_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id)
    }

    /// Index a repository synchronously (v0.3.0 - simplified)
    ///
    /// Indexes the specified directory, creates a session, and returns statistics.
    /// This method blocks until indexing is complete.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique session identifier
    /// * `path` - Absolute path to repository to index
    /// * `include_patterns` - Glob patterns for files to include
    /// * `exclude_patterns` - Glob patterns for files to exclude
    /// * `chunk_size` - Characters per chunk
    /// * `overlap` - Overlapping characters between chunks
    /// * `max_file_size_mb` - Maximum file size in MB to process
    /// * `force` - If true, delete existing session and re-index
    ///
    /// # Returns
    ///
    /// IndexStats with files_indexed, chunks_created, and duration_secs
    ///
    /// # Errors
    ///
    /// Returns error if session already exists (unless force=true) or indexing fails
    #[allow(dead_code)] // Used in MCP binary and tests, not in HTTP server
    #[allow(clippy::too_many_arguments)] // All parameters are necessary
    pub fn index_repository(
        &self,
        session_id: &str,
        path: &std::path::Path,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        chunk_size: usize,
        overlap: usize,
        max_file_size_mb: usize,
        force: bool,
    ) -> Result<crate::core::types::IndexStats> {
        use std::time::Instant;

        let start = Instant::now();

        // Handle force re-indexing
        if self.session_exists(session_id) {
            if force {
                self.delete_session(session_id)?;
            } else {
                return Err(ShebeError::SessionAlreadyExists(session_id.to_string()));
            }
        }

        // Create session config with patterns first (before moving into pipeline)
        let session_config = SessionConfig {
            chunk_size,
            overlap,
            include_patterns: include_patterns.clone(),
            exclude_patterns: exclude_patterns.clone(),
        };

        // Create indexing pipeline
        let pipeline = crate::core::indexer::IndexingPipeline::new(
            chunk_size,
            overlap,
            include_patterns,
            exclude_patterns,
            max_file_size_mb,
        )?;

        // Index directory
        let (chunks, mut stats) = pipeline.index_directory(path)?;

        // Create session and get index
        let mut index =
            self.create_session(session_id, path.to_path_buf(), session_config.clone())?;

        // Add chunks to index
        index.add_chunks(&chunks, session_id)?;

        // Commit index
        index.commit()?;

        // Calculate index size
        let session_path = self.get_session_path(session_id);
        let index_size_bytes = calculate_directory_size(&session_path);

        // Update metadata with correct counts and last_indexed_at
        let mut metadata = self.get_session_metadata(session_id)?;
        metadata.last_indexed_at = Utc::now();
        metadata.files_indexed = stats.files_indexed;
        metadata.chunks_created = stats.chunks_created;
        metadata.index_size_bytes = index_size_bytes;

        self.update_session_metadata(session_id, &metadata)?;

        // Calculate duration in seconds
        let duration_secs = start.elapsed().as_secs_f64();

        // Return stats
        stats.session = session_id.to_string();
        stats.duration_ms = (duration_secs * 1000.0) as u64;

        Ok(stats)
    }
}

/// Calculate directory size recursively
#[allow(dead_code)] // Used by index_repository method
fn calculate_directory_size(path: &std::path::Path) -> u64 {
    let mut total = 0;

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        total += calculate_directory_size(&entry.path());
                    } else {
                        total += metadata.len();
                    }
                }
            }
        }
    } else if path.is_file() {
        if let Ok(metadata) = fs::metadata(path) {
            total = metadata.len();
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        let result = manager.create_session("test-session", repo_path.clone(), config);
        assert!(result.is_ok());

        // Verify session exists
        assert!(manager.session_exists("test-session"));
    }

    #[test]
    fn test_create_duplicate_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        manager
            .create_session("test-session", repo_path.clone(), config.clone())
            .unwrap();

        // Try to create duplicate
        let result = manager.create_session("test-session", repo_path.clone(), config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShebeError::SessionAlreadyExists(_)
        ));
    }

    #[test]
    fn test_open_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Open existing session
        let result = manager.open_session("test-session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_nonexistent_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let result = manager.open_session("nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShebeError::SessionNotFound(_)
        ));
    }

    #[test]
    fn test_delete_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Verify it exists
        assert!(manager.session_exists("test-session"));

        // Delete it
        let result = manager.delete_session("test-session");
        assert!(result.is_ok());

        // Verify it's gone
        assert!(!manager.session_exists("test-session"));
    }

    #[test]
    fn test_session_isolation() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        // Create two sessions
        manager
            .create_session("session1", repo_path.clone(), config.clone())
            .unwrap();
        manager
            .create_session("session2", repo_path.clone(), config.clone())
            .unwrap();

        // Verify both exist
        assert!(manager.session_exists("session1"));
        assert!(manager.session_exists("session2"));

        // Delete one
        manager.delete_session("session1").unwrap();

        // Verify only one remains
        assert!(!manager.session_exists("session1"));
        assert!(manager.session_exists("session2"));
    }

    #[test]
    fn test_session_metadata() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Get metadata
        let metadata = manager.get_session_metadata("test-session").unwrap();

        assert_eq!(metadata.id, "test-session");
        assert_eq!(metadata.files_indexed, 0);
        assert_eq!(metadata.chunks_created, 0);
        assert_eq!(metadata.config.chunk_size, 512);
        assert_eq!(metadata.config.overlap, 64);
    }

    #[test]
    fn test_update_metadata() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Update metadata
        let mut metadata = manager.get_session_metadata("test-session").unwrap();
        metadata.files_indexed = 100;
        metadata.chunks_created = 500;

        manager
            .update_session_metadata("test-session", &metadata)
            .unwrap();

        // Verify update
        let updated = manager.get_session_metadata("test-session").unwrap();
        assert_eq!(updated.files_indexed, 100);
        assert_eq!(updated.chunks_created, 500);
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");

        // Create multiple sessions
        manager
            .create_session("session1", repo_path.clone(), config.clone())
            .unwrap();
        manager
            .create_session("session2", repo_path.clone(), config.clone())
            .unwrap();
        manager
            .create_session("session3", repo_path.clone(), config.clone())
            .unwrap();

        // List all sessions
        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 3);

        // Verify session IDs
        let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
        assert!(ids.contains(&"session1".to_string()));
        assert!(ids.contains(&"session2".to_string()));
        assert!(ids.contains(&"session3".to_string()));
    }

    #[test]
    fn test_list_sessions_empty() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 0);
    }

    // Helper to create test fixture with files
    fn create_test_fixture(base_dir: &std::path::Path) -> std::path::PathBuf {
        let fixture_dir = base_dir.join("test-repo");
        fs::create_dir_all(&fixture_dir).unwrap();

        // Create test files with content
        fs::write(fixture_dir.join("file1.txt"), "Hello world from file 1").unwrap();
        fs::write(fixture_dir.join("file2.txt"), "Hello world from file 2").unwrap();
        fs::write(fixture_dir.join("file3.txt"), "Hello world from file 3").unwrap();

        // Create subdirectory with files
        let subdir = fixture_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("file4.txt"), "Hello from subdirectory").unwrap();

        fixture_dir
    }

    #[test]
    fn test_index_repository_returns_stats() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create test fixture
        let repo_path = create_test_fixture(temp_dir.path());

        // Index repository
        let stats = manager
            .index_repository(
                "test-session",
                &repo_path,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        // Verify stats
        assert_eq!(stats.files_indexed, 4, "Should index 4 .txt files");
        assert!(stats.chunks_created > 0, "Should create chunks");
        assert!(
            stats.duration_ms > 0,
            "Duration should be tracked and positive"
        );
        assert_eq!(stats.session, "test-session");
    }

    #[test]
    fn test_metadata_correct_after_indexing() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create test fixture
        let repo_path = create_test_fixture(temp_dir.path());

        // Index repository
        let stats = manager
            .index_repository(
                "test-session",
                &repo_path,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        // Read metadata file
        let metadata = manager.get_session_metadata("test-session").unwrap();

        // Verify metadata matches stats
        assert_eq!(
            metadata.files_indexed, stats.files_indexed,
            "Metadata files_indexed should match stats"
        );
        assert_eq!(
            metadata.chunks_created, stats.chunks_created,
            "Metadata chunks_created should match stats"
        );
        assert!(
            metadata.index_size_bytes > 0,
            "Index size should be calculated"
        );
        assert_eq!(metadata.id, "test-session");
        assert_eq!(metadata.config.chunk_size, 512);
        assert_eq!(metadata.config.overlap, 64);
    }

    #[test]
    fn test_duration_tracking() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create test fixture
        let repo_path = create_test_fixture(temp_dir.path());

        // Index repository
        let stats = manager
            .index_repository(
                "test-session",
                &repo_path,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        // Verify duration is reasonable
        let duration_secs = stats.duration_ms as f64 / 1000.0;
        assert!(
            duration_secs > 0.0,
            "Duration should be positive: {} seconds",
            duration_secs
        );
        assert!(
            duration_secs < 10.0,
            "Duration should be under 10 seconds for small repo: {} seconds",
            duration_secs
        );
    }

    #[test]
    fn test_index_repository_force_reindex() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create test fixture
        let repo_path = create_test_fixture(temp_dir.path());

        // Index once
        let stats1 = manager
            .index_repository(
                "test-session",
                &repo_path,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        assert_eq!(stats1.files_indexed, 4);

        // Try to index again without force (should fail)
        let result = manager.index_repository(
            "test-session",
            &repo_path,
            vec!["**/*.txt".to_string()],
            vec![],
            512,
            64,
            10,
            false,
        );

        assert!(
            result.is_err(),
            "Should fail when session exists without force=true"
        );
        assert!(matches!(
            result.unwrap_err(),
            ShebeError::SessionAlreadyExists(_)
        ));

        // Re-index with force=true (should succeed)
        let stats2 = manager
            .index_repository(
                "test-session",
                &repo_path,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                true, // force=true
            )
            .unwrap();

        assert_eq!(stats2.files_indexed, 4);
        assert_eq!(stats2.session, "test-session");
    }

    #[test]
    fn test_index_repository_with_filters() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create test fixture
        let fixture_dir = temp_dir.path().join("test-repo");
        fs::create_dir_all(&fixture_dir).unwrap();

        // Create files with different extensions
        fs::write(fixture_dir.join("file1.txt"), "Text file 1").unwrap();
        fs::write(fixture_dir.join("file2.md"), "Markdown file").unwrap();
        fs::write(fixture_dir.join("file3.txt"), "Text file 2").unwrap();

        // Index only .txt files
        let stats = manager
            .index_repository(
                "test-session",
                &fixture_dir,
                vec!["**/*.txt".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        assert_eq!(stats.files_indexed, 2, "Should only index .txt files");
    }

    #[test]
    fn test_index_repository_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create empty directory
        let empty_dir = temp_dir.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        // Index empty directory
        let stats = manager
            .index_repository(
                "test-session",
                &empty_dir,
                vec!["**/*".to_string()],
                vec![],
                512,
                64,
                10,
                false,
            )
            .unwrap();

        assert_eq!(stats.files_indexed, 0, "Should index 0 files");
        assert_eq!(stats.chunks_created, 0, "Should create 0 chunks");

        // Verify session was still created with correct metadata
        let metadata = manager.get_session_metadata("test-session").unwrap();
        assert_eq!(metadata.files_indexed, 0);
        assert_eq!(metadata.chunks_created, 0);
    }

    #[test]
    fn test_new_session_has_current_schema_version() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create session
        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");
        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Verify metadata has current schema version
        let metadata = manager.get_session_metadata("test-session").unwrap();
        assert_eq!(
            metadata.schema_version, SCHEMA_VERSION,
            "New sessions should have current schema version"
        );
    }

    #[test]
    fn test_open_old_schema_version_fails() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create session
        let config = SessionConfig::default();
        let repo_path = PathBuf::from("/test/repo");
        manager
            .create_session("test-session", repo_path.clone(), config)
            .unwrap();

        // Manually update metadata to simulate old schema version
        let mut metadata = manager.get_session_metadata("test-session").unwrap();
        metadata.schema_version = 1; // Old schema version
        manager
            .update_session_metadata("test-session", &metadata)
            .unwrap();

        // Attempt to open session should fail with clear error
        let result = manager.open_session("test-session");
        assert!(result.is_err(), "Opening old schema should fail");

        let err = result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("old schema version"),
            "Error should mention schema version: {}",
            err_msg
        );
        assert!(
            err_msg.contains("Please re-index") || err_msg.contains("Please reindex"),
            "Error should suggest reindexing: {}",
            err_msg
        );
    }

    // NOTE: Backward compatibility test removed - project policy is NO backward compatibility
    // Old sessions (v1, v2) must be re-indexed to v3
}
