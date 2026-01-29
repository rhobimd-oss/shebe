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

impl From<crate::core::error::ShebeError> for McpError {
    fn from(err: crate::core::error::ShebeError) -> Self {
        use crate::core::error::ShebeError;
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
            ShebeError::InvalidQueryField {
                field,
                message,
                valid_fields,
                suggestion,
            } => {
                let mut msg = format!(
                    "Invalid query: Unknown field '{}'.\nValid fields: {}",
                    field,
                    valid_fields.join(", ")
                );
                if let Some(hint) = suggestion {
                    msg.push_str(&format!("\nHint: Did you mean '{hint}'?"));
                }
                msg.push_str(&format!("\nDetails: {message}"));
                McpError::InvalidParams(msg)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::ShebeError;
    use crate::mcp::protocol;

    #[test]
    fn test_session_not_found_to_mcp_error() {
        let err = ShebeError::SessionNotFound("test-sess".to_string());
        let mcp: McpError = err.into();
        match mcp {
            McpError::ToolError(code, msg) => {
                assert_eq!(code, protocol::SESSION_NOT_FOUND);
                assert!(msg.contains("test-sess"));
            }
            other => panic!("Expected ToolError, got: {other:?}"),
        }
    }

    #[test]
    fn test_session_already_exists_to_mcp_error() {
        let err = ShebeError::SessionAlreadyExists("dup".to_string());
        let mcp: McpError = err.into();
        match mcp {
            McpError::ToolError(code, msg) => {
                assert_eq!(code, protocol::SESSION_ALREADY_EXISTS);
                assert!(msg.contains("dup"));
            }
            other => panic!("Expected ToolError, got: {other:?}"),
        }
    }

    #[test]
    fn test_invalid_session_to_mcp_error() {
        let err = ShebeError::InvalidSession("bad".to_string());
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InvalidParams(_)));
    }

    #[test]
    fn test_invalid_path_to_mcp_error() {
        let err = ShebeError::InvalidPath("/bad/path".to_string());
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InvalidParams(_)));
    }

    #[test]
    fn test_invalid_query_to_mcp_error() {
        let err = ShebeError::InvalidQuery("empty".to_string());
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InvalidParams(_)));
    }

    #[test]
    fn test_invalid_query_field_to_mcp_error() {
        let err = ShebeError::InvalidQueryField {
            field: "bad_field".to_string(),
            message: "unknown".to_string(),
            valid_fields: vec!["content".to_string(), "file_path".to_string()],
            suggestion: Some("content".to_string()),
        };
        let mcp: McpError = err.into();
        match mcp {
            McpError::InvalidParams(msg) => {
                assert!(msg.contains("bad_field"));
                assert!(msg.contains("content"));
                assert!(msg.contains("Did you mean"));
            }
            other => panic!("Expected InvalidParams, got: {other:?}"),
        }
    }

    #[test]
    fn test_config_error_to_mcp_error() {
        let err = ShebeError::ConfigError("bad config".to_string());
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InvalidParams(_)));
    }

    #[test]
    fn test_indexing_failed_to_mcp_error() {
        let err = ShebeError::IndexingFailed("disk full".to_string());
        let mcp: McpError = err.into();
        match mcp {
            McpError::ToolError(code, _) => {
                assert_eq!(code, protocol::INDEXING_FAILED);
            }
            other => panic!("Expected ToolError, got: {other:?}"),
        }
    }

    #[test]
    fn test_search_failed_to_mcp_error() {
        let err = ShebeError::SearchFailed("parse error".to_string());
        let mcp: McpError = err.into();
        match mcp {
            McpError::ToolError(code, _) => {
                assert_eq!(code, protocol::SEARCH_FAILED);
            }
            other => panic!("Expected ToolError, got: {other:?}"),
        }
    }

    #[test]
    fn test_storage_error_to_mcp_error() {
        let err = ShebeError::StorageError("corrupt".to_string());
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InternalError(_)));
    }

    #[test]
    fn test_io_error_to_mcp_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ShebeError::IoError(io_err);
        let mcp: McpError = err.into();
        assert!(matches!(mcp, McpError::InternalError(_)));
    }

    #[test]
    fn test_mcp_error_display() {
        let err = McpError::ParseError("bad json".to_string());
        assert_eq!(format!("{err}"), "Parse error: bad json");

        let err = McpError::InvalidParams("missing field".to_string());
        assert_eq!(format!("{err}"), "Invalid params: missing field");

        let err = McpError::InternalError("oops".to_string());
        assert_eq!(format!("{err}"), "Internal error: oops");

        let err = McpError::ToolError(-32001, "not found".to_string());
        assert_eq!(format!("{err}"), "Tool error (code -32001): not found");

        let err = McpError::InvalidRequest("bad".to_string());
        assert_eq!(format!("{err}"), "Invalid request: bad");
    }

    #[test]
    fn test_io_error_from_impl() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let mcp: McpError = McpError::from(io_err);
        assert!(matches!(mcp, McpError::Io(_)));
    }

    #[test]
    fn test_json_error_from_impl() {
        let json_result: Result<serde_json::Value, _> = serde_json::from_str("not json");
        let json_err = json_result.unwrap_err();
        let mcp: McpError = McpError::from(json_err);
        assert!(matches!(mcp, McpError::Json(_)));
    }
}
