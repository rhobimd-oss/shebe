//! Core data types for the Shebe RAG service.
//!
//! This module defines all data structures used throughout the
//! application, including chunks, search results, requests, and
//! responses.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single text chunk from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The actual text content
    pub text: String,

    /// Source file path
    pub file_path: PathBuf,

    /// Byte offset where chunk starts in original file
    pub start_offset: usize,

    /// Byte offset where chunk ends in original file
    pub end_offset: usize,

    /// Sequential chunk number within the file
    pub chunk_index: usize,
}

/// Search result returned by query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// BM25 relevance score (higher = more relevant)
    pub score: f32,

    /// Chunk text content
    pub text: String,

    /// Source file path
    pub file_path: String,

    /// Chunk index within file
    pub chunk_index: usize,

    /// Byte offsets for highlighting
    pub start_offset: usize,
    pub end_offset: usize,
}

/// Statistics from an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of files successfully indexed
    pub files_indexed: usize,

    /// Total chunks created
    pub chunks_created: usize,

    /// Indexing duration in milliseconds
    pub duration_ms: u64,

    /// Session identifier
    pub session: String,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub id: String,

    /// Number of indexed files
    pub files: usize,

    /// Total chunks
    pub chunks: usize,

    /// Creation timestamp (ISO 8601)
    pub created_at: String,

    /// Index size in bytes
    pub size_bytes: u64,
}

/// Request to index a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRequest {
    /// Absolute path to repository
    pub path: String,

    /// Unique session identifier
    pub session: String,

    /// File patterns to include (glob syntax)
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// File patterns to exclude (glob syntax)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

/// Response from indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    /// Number of files indexed
    pub files_indexed: usize,

    /// Number of chunks created
    pub chunks_created: usize,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Session identifier
    pub session: String,
}

impl From<IndexStats> for IndexResponse {
    fn from(stats: IndexStats) -> Self {
        Self {
            files_indexed: stats.files_indexed,
            chunks_created: stats.chunks_created,
            duration_ms: stats.duration_ms,
            session: stats.session,
        }
    }
}

/// Request to search indexed content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query string
    pub query: String,

    /// Session identifier
    pub session: String,

    /// Number of results to return (optional)
    pub k: Option<usize>,
}

/// Response from search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Original query string
    pub query: String,

    /// Search results
    pub results: Vec<SearchResult>,

    /// Number of results returned
    pub count: usize,

    /// Query duration in milliseconds
    pub duration_ms: u64,
}

/// Response from listing sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionsResponse {
    /// List of session metadata
    pub sessions: Vec<SessionInfo>,
}

/// Response from deleting a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    /// Status message
    pub status: String,

    /// Session identifier that was deleted
    pub session: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,

    /// Service version
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk {
            text: "Hello, world!".to_string(),
            file_path: PathBuf::from("/test/file.rs"),
            start_offset: 0,
            end_offset: 13,
            chunk_index: 0,
        };

        assert_eq!(chunk.text, "Hello, world!");
        assert_eq!(chunk.chunk_index, 0);
    }

    #[test]
    fn test_index_stats_to_response() {
        let stats = IndexStats {
            files_indexed: 100,
            chunks_created: 500,
            duration_ms: 1000,
            session: "test-session".to_string(),
        };

        let response: IndexResponse = stats.into();
        assert_eq!(response.files_indexed, 100);
        assert_eq!(response.chunks_created, 500);
        assert_eq!(response.session, "test-session");
    }

    #[test]
    fn test_search_request_deserialization() {
        let json = r#"{
            "query": "test query",
            "session": "test-session",
            "k": 10
        }"#;

        let req: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "test query");
        assert_eq!(req.session, "test-session");
        assert_eq!(req.k, Some(10));
    }

    #[test]
    fn test_index_request_default_patterns() {
        let json = r#"{
            "path": "/test/path",
            "session": "test-session"
        }"#;

        let req: IndexRequest = serde_json::from_str(json).unwrap();
        assert!(req.include_patterns.is_empty());
        assert!(req.exclude_patterns.is_empty());
    }
}
