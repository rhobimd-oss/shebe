//! Delete session tool handler

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_bytes;
use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct DeleteSessionHandler {
    services: Arc<Services>,
}

impl DeleteSessionHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Format deletion summary
    fn format_summary(
        &self,
        session: &str,
        files_indexed: usize,
        chunks_created: usize,
        index_size_bytes: u64,
    ) -> String {
        format!(
            "**Session Deleted:** `{}`\n\n\
             **Freed Resources:**\n\
             - Files indexed: {}\n\
             - Chunks removed: {}\n\
             - Disk space freed: {}\n\n\
             Session data and index permanently deleted.",
            session,
            files_indexed,
            chunks_created,
            format_bytes(index_size_bytes)
        )
    }
}

#[async_trait]
impl McpToolHandler for DeleteSessionHandler {
    fn name(&self) -> &str {
        "delete_session"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "delete_session".to_string(),
            description: "Delete a session and all associated data (index, metadata). \
                         This is a DESTRUCTIVE operation that cannot be undone. \
                         Requires confirm=true parameter to prevent accidental deletion. \
                         Frees disk space and removes session from list_sessions. \
                         To recreate session, re-run index_repository."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to delete",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "confirm": {
                        "type": "boolean",
                        "description": "Must be true to confirm deletion (safety check)",
                    }
                },
                "required": ["session", "confirm"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct DeleteArgs {
            session: String,
            confirm: bool,
        }

        // Parse arguments
        let args: DeleteArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Require explicit confirmation
        if !args.confirm {
            return Err(McpError::InvalidRequest(
                "Deletion requires confirm=true parameter. \
                 This prevents accidental session deletion."
                    .to_string(),
            ));
        }

        // Get session metadata before deletion (for summary)
        let metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .map_err(|e| {
                use crate::core::error::ShebeError;
                match e {
                    ShebeError::SessionNotFound(_) => McpError::InvalidRequest(format!(
                        "Session '{}' not found. Use list_sessions to see available sessions.",
                        args.session
                    )),
                    _ => McpError::from(e),
                }
            })?;

        // Extract stats for summary
        let files_indexed = metadata.files_indexed;
        let chunks_created = metadata.chunks_created;
        let index_size_bytes = metadata.index_size_bytes;

        // Delete session (atomic operation)
        self.services
            .storage
            .delete_session(&args.session)
            .map_err(|e| McpError::InternalError(format!("Failed to delete session: {e}")))?;

        // Format summary
        let summary = self.format_summary(
            &args.session,
            files_indexed,
            chunks_created,
            index_size_bytes,
        );

        Ok(text_content(summary))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::core::storage::SessionConfig;
    use crate::core::types::Chunk;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (DeleteSessionHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = DeleteSessionHandler::new(services);

        (handler, temp_dir)
    }

    async fn create_test_session(services: &Arc<Services>, session_id: &str) {
        let mut index = services
            .storage
            .create_session(
                session_id,
                PathBuf::from("/test/repo"),
                SessionConfig::default(),
            )
            .unwrap();

        let chunks = vec![Chunk {
            text: "test content".to_string(),
            file_path: PathBuf::from("test.rs"),
            start_offset: 0,
            end_offset: 12,
            chunk_index: 0,
        }];

        index.add_chunks(&chunks, session_id).unwrap();
        index.commit().unwrap();
    }

    #[tokio::test]
    async fn test_delete_session_valid() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session(&handler.services, "test-delete").await;

        // Verify session exists
        assert!(handler.services.storage.session_exists("test-delete"));

        let args = json!({
            "session": "test-delete",
            "confirm": true,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Session Deleted:**"));
        assert!(text.contains("test-delete"));
        assert!(text.contains("Files indexed:"));
        assert!(text.contains("Chunks removed:"));
        assert!(text.contains("Disk space freed:"));

        // Verify session is deleted
        assert!(!handler.services.storage.session_exists("test-delete"));
    }

    #[tokio::test]
    async fn test_delete_session_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "session": "nonexistent",
            "confirm": true,
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("Session"));
            assert!(msg.contains("not found"));
        } else {
            panic!("Expected InvalidRequest error");
        }
    }

    #[tokio::test]
    async fn test_delete_session_without_confirm() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session(&handler.services, "test-no-confirm").await;

        let args = json!({
            "session": "test-no-confirm",
            "confirm": false,
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("confirm=true"));
        } else {
            panic!("Expected InvalidRequest error for missing confirmation");
        }

        // Verify session still exists
        assert!(handler.services.storage.session_exists("test-no-confirm"));
    }

    #[tokio::test]
    async fn test_delete_metadata_summary() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session(&handler.services, "test-summary").await;

        // Get metadata before deletion to verify summary
        let metadata = handler
            .services
            .storage
            .get_session_metadata("test-summary")
            .unwrap();

        let args = json!({
            "session": "test-summary",
            "confirm": true,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Verify summary includes metadata
        assert!(text.contains(&metadata.files_indexed.to_string()));
        assert!(text.contains(&metadata.chunks_created.to_string()));
    }
}
