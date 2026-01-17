//! Error types and error handling for the Shebe RAG service.
//!
//! This module defines the error types used throughout the
//! application. Protocol-specific error handling (MCP error codes)
//! is handled in the respective adapter modules.

use thiserror::Error;

/// Result type alias for Shebe operations
pub type Result<T> = std::result::Result<T, ShebeError>;

/// Main error type for the Shebe service
#[derive(Error, Debug)]
pub enum ShebeError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("Invalid session: {0}")]
    InvalidSession(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Indexing failed: {0}")]
    IndexingFailed(String),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Invalid query field '{field}': {message}")]
    InvalidQueryField {
        field: String,
        message: String,
        valid_fields: Vec<String>,
        suggestion: Option<String>,
    },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),
}

impl ShebeError {
    /// Get user-friendly error message
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Check if this is a "not found" type error
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            ShebeError::SessionNotFound(_) | ShebeError::InvalidPath(_)
        )
    }

    /// Check if this is a conflict error (already exists)
    pub fn is_conflict(&self) -> bool {
        matches!(self, ShebeError::SessionAlreadyExists(_))
    }

    /// Check if this is a bad request error (invalid input)
    pub fn is_bad_request(&self) -> bool {
        matches!(
            self,
            ShebeError::InvalidSession(_)
                | ShebeError::InvalidQuery(_)
                | ShebeError::InvalidQueryField { .. }
                | ShebeError::ConfigError(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_is_not_found() {
        let err = ShebeError::SessionNotFound("test".to_string());
        assert!(err.is_not_found());
        assert!(!err.is_conflict());
        assert!(!err.is_bad_request());
    }

    #[test]
    fn test_session_exists_is_conflict() {
        let err = ShebeError::SessionAlreadyExists("test".to_string());
        assert!(err.is_conflict());
        assert!(!err.is_not_found());
        assert!(!err.is_bad_request());
    }

    #[test]
    fn test_invalid_query_is_bad_request() {
        let err = ShebeError::InvalidQuery("empty".to_string());
        assert!(err.is_bad_request());
        assert!(!err.is_not_found());
        assert!(!err.is_conflict());
    }

    #[test]
    fn test_indexing_failed_is_internal() {
        let err = ShebeError::IndexingFailed("disk full".to_string());
        assert!(!err.is_not_found());
        assert!(!err.is_conflict());
        assert!(!err.is_bad_request());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ShebeError::from(io_err);
        assert!(!err.is_not_found()); // IoError is internal, not "not found"
    }

    #[test]
    fn test_error_message() {
        let err = ShebeError::SessionNotFound("my-session".to_string());
        assert!(err.message().contains("my-session"));
        assert!(err.message().contains("not found"));
    }
}
