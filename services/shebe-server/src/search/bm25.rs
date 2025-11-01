//! BM25 search implementation using Tantivy.
//!
//! This module provides the SearchService for executing BM25-ranked
//! queries over indexed content.

use crate::error::{Result, ShebeError};
use crate::storage::StorageManager;
use crate::types::{SearchRequest, SearchResponse, SearchResult};
use std::sync::Arc;
use std::time::Instant;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, Value},
    TantivyDocument,
};

/// BM25 search service
pub struct SearchService {
    storage: Arc<StorageManager>,
    default_k: usize,
    max_k: usize,
}

impl SearchService {
    /// Create a new search service
    pub fn new(storage: Arc<StorageManager>, default_k: usize, max_k: usize) -> Self {
        Self {
            storage,
            default_k,
            max_k,
        }
    }

    /// Execute a search query
    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        self.search_session(&request.session, &request.query, request.k)
    }

    /// Execute search with explicit parameters
    pub fn search_session(
        &self,
        session_id: &str,
        query_str: &str,
        k: Option<usize>,
    ) -> Result<SearchResponse> {
        let start = Instant::now();

        // Validate query
        if query_str.trim().is_empty() {
            return Err(ShebeError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        // Check session exists
        if !self.storage.session_exists(session_id) {
            return Err(ShebeError::SessionNotFound(session_id.to_string()));
        }

        // Determine k (result limit)
        let k_limit = k.unwrap_or(self.default_k).min(self.max_k);

        // Open session index
        let index = self.storage.open_session(session_id)?;
        let reader = index
            .reader()
            .map_err(|e| ShebeError::SearchFailed(format!("Failed to create reader: {e}")))?;
        let searcher = reader.searcher();
        let schema = index.schema();

        // Get schema fields
        let text_field = schema
            .get_field("text")
            .map_err(|e| ShebeError::SearchFailed(format!("Missing text field: {e}")))?;
        let file_path_field = schema
            .get_field("file_path")
            .map_err(|e| ShebeError::SearchFailed(format!("Missing file_path field: {e}")))?;
        let offset_start_field = schema
            .get_field("offset_start")
            .map_err(|e| ShebeError::SearchFailed(format!("Missing offset_start field: {e}")))?;
        let offset_end_field = schema
            .get_field("offset_end")
            .map_err(|e| ShebeError::SearchFailed(format!("Missing offset_end field: {e}")))?;
        let chunk_index_field = schema
            .get_field("chunk_index")
            .map_err(|e| ShebeError::SearchFailed(format!("Missing chunk_index field: {e}")))?;

        // Parse query
        let query_parser = QueryParser::for_index(index.index(), vec![text_field]);

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| ShebeError::InvalidQuery(format!("Failed to parse query: {e}")))?;

        // Execute search with BM25 ranking
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(k_limit))
            .map_err(|e| ShebeError::SearchFailed(format!("Search failed: {e}")))?;

        // Extract results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address).map_err(|e| {
                ShebeError::SearchFailed(format!("Failed to retrieve document: {e}"))
            })?;

            results.push(SearchResult {
                score,
                text: Self::extract_text(&doc, text_field),
                file_path: Self::extract_text(&doc, file_path_field),
                chunk_index: Self::extract_i64(&doc, chunk_index_field) as usize,
                start_offset: Self::extract_i64(&doc, offset_start_field) as usize,
                end_offset: Self::extract_i64(&doc, offset_end_field) as usize,
            });
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let count = results.len();

        Ok(SearchResponse {
            query: query_str.to_string(),
            results,
            count,
            duration_ms,
        })
    }

    /// Extract text field from document
    fn extract_text(doc: &TantivyDocument, field: Field) -> String {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    /// Extract i64 field from document
    fn extract_i64(doc: &TantivyDocument, field: Field) -> i64 {
        doc.get_first(field).and_then(|v| v.as_i64()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SessionConfig;
    use crate::types::Chunk;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_service() -> (SearchService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageManager::new(temp_dir.path().to_path_buf()));

        let service = SearchService::new(storage, 10, 100);
        (service, temp_dir)
    }

    async fn create_test_session(storage: &Arc<StorageManager>, session_id: &str) {
        let mut index = storage
            .create_session(
                session_id,
                PathBuf::from("/test/repo"),
                SessionConfig::default(),
            )
            .unwrap();

        let chunks = vec![
            Chunk {
                text: "async function main() {}".to_string(),
                file_path: PathBuf::from("test.rs"),
                start_offset: 0,
                end_offset: 24,
                chunk_index: 0,
            },
            Chunk {
                text: "sync function helper() {}".to_string(),
                file_path: PathBuf::from("test.rs"),
                start_offset: 25,
                end_offset: 50,
                chunk_index: 1,
            },
            Chunk {
                text: "async fn process_data(x: i32) -> i32 { x * 2 }".to_string(),
                file_path: PathBuf::from("lib.rs"),
                start_offset: 0,
                end_offset: 47,
                chunk_index: 0,
            },
        ];

        index.add_chunks(&chunks, session_id).unwrap();
        index.commit().unwrap();
    }

    #[tokio::test]
    async fn test_search_basic_query() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        let response = service
            .search_session("test-session", "async function", Some(10))
            .unwrap();

        assert!(!response.results.is_empty());
        assert!(response.results[0].text.contains("async"));
        assert!(response.results[0].score > 0.0);
        assert_eq!(response.count, response.results.len());
    }

    #[tokio::test]
    async fn test_search_phrase_query() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        let response = service
            .search_session("test-session", "\"async function\"", Some(10))
            .unwrap();

        assert!(!response.results.is_empty());
        assert!(response.results[0].text.contains("async function"));
    }

    #[tokio::test]
    async fn test_search_empty_query_error() {
        let (service, _temp) = setup_test_service().await;

        let result = service.search_session("test-session", "", Some(10));

        assert!(result.is_err());
        match result {
            Err(ShebeError::InvalidQuery(_)) => {}
            _ => panic!("Expected InvalidQuery error"),
        }
    }

    #[tokio::test]
    async fn test_search_whitespace_query_error() {
        let (service, _temp) = setup_test_service().await;

        let result = service.search_session("test-session", "   ", Some(10));

        assert!(result.is_err());
        match result {
            Err(ShebeError::InvalidQuery(_)) => {}
            _ => panic!("Expected InvalidQuery error"),
        }
    }

    #[tokio::test]
    async fn test_search_session_not_found() {
        let (service, _temp) = setup_test_service().await;

        let result = service.search_session("nonexistent-session", "test query", Some(10));

        assert!(result.is_err());
        match result {
            Err(ShebeError::SessionNotFound(_)) => {}
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_search_k_parameter() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        // Request only 1 result
        let response = service
            .search_session("test-session", "function", Some(1))
            .unwrap();

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.count, 1);
    }

    #[tokio::test]
    async fn test_search_max_k_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageManager::new(temp_dir.path().to_path_buf()));

        // Create service with max_k = 2
        let service = SearchService::new(storage.clone(), 10, 2);
        create_test_session(&storage, "test-session").await;

        // Request 100 results (exceeds max_k)
        let response = service
            .search_session("test-session", "function", Some(100))
            .unwrap();

        // Should only return max_k results
        assert!(response.results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_default_k() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(StorageManager::new(temp_dir.path().to_path_buf()));

        // Create service with default_k = 5
        let service = SearchService::new(storage.clone(), 5, 100);
        create_test_session(&storage, "test-session").await;

        // Don't specify k (should use default)
        let response = service
            .search_session("test-session", "function", None)
            .unwrap();

        // Should return up to default_k results
        assert!(response.results.len() <= 5);
    }

    #[tokio::test]
    async fn test_search_result_metadata() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        let response = service
            .search_session("test-session", "async", Some(10))
            .unwrap();

        assert!(!response.results.is_empty());
        let result = &response.results[0];

        // Check metadata is populated
        assert!(!result.file_path.is_empty());
        assert!(result.end_offset > result.start_offset);
    }

    #[tokio::test]
    async fn test_search_request_wrapper() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        let request = SearchRequest {
            query: "async".to_string(),
            session: "test-session".to_string(),
            k: Some(10),
        };

        let response = service.search(request).unwrap();

        assert!(!response.results.is_empty());
        assert_eq!(response.query, "async");
    }

    #[tokio::test]
    async fn test_search_duration_tracking() {
        let (service, _temp) = setup_test_service().await;
        let storage = Arc::clone(&service.storage);
        create_test_session(&storage, "test-session").await;

        let response = service
            .search_session("test-session", "async", Some(10))
            .unwrap();

        // Duration should be tracked (non-negative by definition of u64)
        // Just verify it's returned
        let _ = response.duration_ms;
    }
}
