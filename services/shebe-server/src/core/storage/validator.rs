//! Metadata validation and consistency checking.
//!
//! This module provides tools to validate that session metadata
//! matches the actual state of the Tantivy index on disk.

use crate::core::error::Result;
use crate::core::storage::StorageManager;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Metadata validation report
#[allow(dead_code)] // Used in MCP binary, not in HTTP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Session ID validated
    pub session_id: String,

    /// Metadata values
    pub metadata_files: usize,
    pub metadata_chunks: usize,
    pub metadata_size: u64,

    /// Actual measured values
    pub actual_size: u64,

    /// Validation results
    pub size_matches: bool,
    pub is_consistent: bool,

    /// Validation timestamp
    pub validated_at: String,
}

/// Metadata validator for session consistency checks
#[allow(dead_code)] // Used in MCP binary, not in HTTP server
pub struct MetadataValidator<'a> {
    storage_manager: &'a StorageManager,
}

#[allow(dead_code)] // Methods used in MCP binary, not in HTTP server
impl<'a> MetadataValidator<'a> {
    /// Create a new metadata validator
    pub fn new(storage_manager: &'a StorageManager) -> Self {
        Self { storage_manager }
    }

    /// Validate a session's metadata against actual index state
    ///
    /// Checks:
    /// - Index size on disk matches metadata
    /// - Files indexed count is non-zero (if index exists)
    /// - Chunks created count is non-zero (if index exists)
    pub fn validate_session(&self, session_id: &str) -> Result<ValidationReport> {
        // Read metadata file
        let metadata = self.storage_manager.get_session_metadata(session_id)?;

        // Measure actual index size on disk
        let actual_size = self.measure_index_size(session_id)?;

        // Check if size matches (within 1MB tolerance for small variations)
        let size_tolerance = 1024 * 1024; // 1MB
        let size_diff = actual_size.abs_diff(metadata.index_size_bytes);
        let size_matches = size_diff < size_tolerance;

        // Tantivy creates small metadata files (~10KB) even for empty indexes
        // Only require non-zero file/chunk counts if index has substantial data
        let empty_index_threshold = 100 * 1024; // 100KB
        let has_indexed_data = actual_size > empty_index_threshold;

        // Overall consistency check
        let is_consistent = size_matches
            && (!has_indexed_data || metadata.files_indexed > 0)
            && (!has_indexed_data || metadata.chunks_created > 0);

        Ok(ValidationReport {
            session_id: session_id.to_string(),
            metadata_files: metadata.files_indexed,
            metadata_chunks: metadata.chunks_created,
            metadata_size: metadata.index_size_bytes,
            actual_size,
            size_matches,
            is_consistent,
            validated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Measure actual index size on disk
    fn measure_index_size(&self, session_id: &str) -> Result<u64> {
        let session_path = self.storage_manager.get_session_path(session_id);
        Ok(calculate_directory_size(&session_path.join("tantivy")))
    }

    /// Auto-repair metadata inconsistencies
    ///
    /// Attempts to repair metadata by recalculating actual values.
    /// Only repairs if the index has substantial data and metadata is inconsistent.
    pub fn auto_repair(&self, session_id: &str) -> Result<bool> {
        let report = self.validate_session(session_id)?;

        // Only repair if index has substantial data but metadata is wrong
        let empty_index_threshold = 100 * 1024; // 100KB (same as validation)
        let has_indexed_data = report.actual_size > empty_index_threshold;

        if has_indexed_data && !report.is_consistent {
            tracing::info!(
                "Auto-repairing metadata for session '{}': actual_size={}",
                session_id,
                report.actual_size
            );

            // We can't reliably recalculate files/chunks without re-indexing,
            // but we can at least update the size
            let mut metadata = self.storage_manager.get_session_metadata(session_id)?;
            metadata.index_size_bytes = report.actual_size;

            // If metadata shows 0 files/chunks but index has substantial data, warn
            if metadata.files_indexed == 0 {
                tracing::warn!(
                    "Session '{}' has index data but metadata shows 0 files. \
                     Cannot auto-repair file/chunk counts without re-indexing.",
                    session_id
                );
            }

            self.storage_manager
                .update_session_metadata(session_id, &metadata)?;

            Ok(true) // Repaired
        } else {
            Ok(false) // No repair needed
        }
    }

    /// Validate all sessions
    pub fn validate_all_sessions(&self) -> Result<Vec<ValidationReport>> {
        let sessions = self.storage_manager.list_sessions()?;
        let mut reports = Vec::new();

        for session in sessions {
            match self.validate_session(&session.id) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    tracing::error!("Failed to validate session '{}': {}", session.id, e);
                }
            }
        }

        Ok(reports)
    }
}

/// Calculate total size of a directory recursively
#[allow(dead_code)] // Used in validator tests, not in HTTP server
fn calculate_directory_size(dir_path: &Path) -> u64 {
    let mut total_size = 0u64;

    if !dir_path.exists() {
        return 0;
    }

    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }

    total_size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::ShebeError;
    use crate::core::storage::SessionConfig;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_validate_empty_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        // Create empty session
        let config = SessionConfig::default();
        manager
            .create_session("test-session", PathBuf::from("/test/repo"), config)
            .unwrap();

        // Validate
        let validator = MetadataValidator::new(&manager);
        let report = validator.validate_session("test-session").unwrap();

        // Empty session should be consistent
        assert!(report.is_consistent);
        assert_eq!(report.metadata_files, 0);
        assert_eq!(report.metadata_chunks, 0);
        // Tantivy creates small metadata files (~1-10KB) even for empty indexes
        assert!(
            report.actual_size < 100 * 1024,
            "Empty index should be < 100KB"
        );
    }

    #[test]
    fn test_validate_nonexistent_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let validator = MetadataValidator::new(&manager);
        let result = validator.validate_session("nonexistent");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ShebeError::SessionNotFound(_)
        ));
    }

    #[test]
    fn test_validate_all_sessions() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();

        // Create multiple sessions
        manager
            .create_session("session1", PathBuf::from("/test/repo"), config.clone())
            .unwrap();
        manager
            .create_session("session2", PathBuf::from("/test/repo"), config.clone())
            .unwrap();
        manager
            .create_session("session3", PathBuf::from("/test/repo"), config.clone())
            .unwrap();

        // Validate all
        let validator = MetadataValidator::new(&manager);
        let reports = validator.validate_all_sessions().unwrap();

        assert_eq!(reports.len(), 3);
        for report in reports {
            assert!(report.is_consistent);
        }
    }

    #[test]
    fn test_auto_repair_consistent_session() {
        let temp_dir = tempdir().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf());

        let config = SessionConfig::default();
        manager
            .create_session("test-session", PathBuf::from("/test/repo"), config)
            .unwrap();

        let validator = MetadataValidator::new(&manager);

        // Consistent session should not need repair
        let repaired = validator.auto_repair("test-session").unwrap();
        assert!(!repaired);
    }

    #[test]
    fn test_calculate_directory_size() {
        let temp_dir = tempdir().unwrap();

        // Create some test files
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content22").unwrap();

        let size = calculate_directory_size(temp_dir.path());
        assert_eq!(size, 17); // 8 + 9 bytes
    }

    #[test]
    fn test_calculate_directory_size_nonexistent() {
        let size = calculate_directory_size(Path::new("/nonexistent/path"));
        assert_eq!(size, 0);
    }
}
