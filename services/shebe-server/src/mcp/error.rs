//! MCP-specific error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Tool error (code {0}): {1}")]
    ToolError(i32, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<crate::error::ShebeError> for McpError {
    fn from(err: crate::error::ShebeError) -> Self {
        use crate::error::ShebeError;
        match err {
            ShebeError::SessionNotFound(s) => McpError::ToolError(
                crate::mcp::protocol::SESSION_NOT_FOUND,
                format!("Session not found: {s}"),
            ),
            ShebeError::SessionAlreadyExists(s) => McpError::ToolError(
                crate::mcp::protocol::SESSION_ALREADY_EXISTS,
                format!("Session already exists: {s}"),
            ),
            ShebeError::InvalidSession(s) => {
                McpError::InvalidParams(format!("Invalid session: {s}"))
            }
            ShebeError::InvalidPath(p) => McpError::InvalidParams(format!("Invalid path: {p}")),
            ShebeError::InvalidQuery(s) => McpError::InvalidParams(format!("Invalid query: {s}")),
            ShebeError::ConfigError(s) => {
                McpError::InvalidParams(format!("Configuration error: {s}"))
            }
            ShebeError::IndexingFailed(s) => McpError::ToolError(
                crate::mcp::protocol::INDEXING_FAILED,
                format!("Indexing failed: {s}"),
            ),
            ShebeError::SearchFailed(s) => McpError::ToolError(
                crate::mcp::protocol::SEARCH_FAILED,
                format!("Search failed: {s}"),
            ),
            ShebeError::StorageError(s) => McpError::InternalError(format!("Storage error: {s}")),
            ShebeError::IoError(e) => McpError::InternalError(format!("I/O error: {e}")),
            ShebeError::SerdeError(e) => {
                McpError::InternalError(format!("Serialization error: {e}"))
            }
            ShebeError::TomlError(e) => {
                McpError::InternalError(format!("Configuration parse error: {e}"))
            }
        }
    }
}
