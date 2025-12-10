//! Error types and error handling for the Shebe RAG service.
//!
//! This module defines the error types used throughout the
//! application and provides conversion to HTTP status codes for
//! API responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
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
    /// Convert error to appropriate HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            ShebeError::SessionNotFound(_) | ShebeError::InvalidPath(_) => StatusCode::NOT_FOUND,
            ShebeError::SessionAlreadyExists(_) => StatusCode::CONFLICT,
            ShebeError::InvalidSession(_)
            | ShebeError::InvalidQuery(_)
            | ShebeError::ConfigError(_) => StatusCode::BAD_REQUEST,
            ShebeError::IndexingFailed(_)
            | ShebeError::SearchFailed(_)
            | ShebeError::StorageError(_)
            | ShebeError::IoError(_)
            | ShebeError::SerdeError(_)
            | ShebeError::TomlError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get user-friendly error message
    pub fn message(&self) -> String {
        self.to_string()
    }
}

/// Implement IntoResponse for automatic error conversion in Axum
impl IntoResponse for ShebeError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.message();

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_status() {
        let err = ShebeError::SessionNotFound("test".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_session_exists_status() {
        let err = ShebeError::SessionAlreadyExists("test".to_string());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_invalid_query_status() {
        let err = ShebeError::InvalidQuery("empty".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_indexing_failed_status() {
        let err = ShebeError::IndexingFailed("disk full".to_string());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ShebeError::from(io_err);
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_message() {
        let err = ShebeError::SessionNotFound("my-session".to_string());
        assert!(err.message().contains("my-session"));
        assert!(err.message().contains("not found"));
    }
}
