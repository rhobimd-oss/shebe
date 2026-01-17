//! Unified service container for Shebe
//!
//! Provides shared access to all core services.

use crate::core::config::Config;
use crate::core::error::Result;
use crate::core::indexer::IndexingPipeline;
use crate::core::search::SearchService;
use crate::core::storage::StorageManager;
use std::sync::Arc;

/// Unified services container
///
/// All adapters use this same struct for service access.
#[derive(Clone)]
pub struct Services {
    /// Storage manager for session CRUD operations
    pub storage: Arc<StorageManager>,

    /// Search service for BM25 queries
    pub search: Arc<SearchService>,

    /// Application configuration
    pub config: Arc<Config>,
}

impl Services {
    /// Create services from configuration
    pub fn new(config: Config) -> Self {
        let storage = Arc::new(StorageManager::new(config.storage.index_dir.clone()));

        let search = Arc::new(SearchService::new(
            Arc::clone(&storage),
            config.search.default_k,
            config.search.max_k,
        ));

        Self {
            storage,
            search,
            config: Arc::new(config),
        }
    }

    /// Create an IndexingPipeline with request-specific patterns
    ///
    /// Pipelines are created per-request since include/exclude patterns vary.
    pub fn create_pipeline(
        &self,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Result<IndexingPipeline> {
        IndexingPipeline::new(
            self.config.indexing.chunk_size,
            self.config.indexing.overlap,
            include_patterns,
            exclude_patterns,
            self.config.indexing.max_file_size_mb,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_services_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Services::new(config);

        assert_eq!(services.config.search.default_k, 10);
        assert_eq!(services.config.search.max_k, 100);
    }

    #[test]
    fn test_services_clone() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Services::new(config);
        let cloned = services.clone();

        // Both should point to same Arc instances
        assert!(Arc::ptr_eq(&services.storage, &cloned.storage));
        assert!(Arc::ptr_eq(&services.search, &cloned.search));
        assert!(Arc::ptr_eq(&services.config, &cloned.config));
    }

    #[test]
    fn test_create_pipeline() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Services::new(config);

        // Pipeline should be created successfully with config values
        let _pipeline = services
            .create_pipeline(
                vec!["**/*.rs".to_string()],
                vec!["**/target/**".to_string()],
            )
            .expect("Pipeline creation should succeed");
    }
}
