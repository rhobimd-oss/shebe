//! Get session info tool handler

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_bytes;
use crate::core::services::Services;
use crate::core::storage::SessionMetadata;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct GetSessionInfoHandler {
    services: Arc<Services>,
}

impl GetSessionInfoHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    fn format_info(&self, metadata: &SessionMetadata) -> String {
        let mut output = format!("# Session: {}\n\n", metadata.id);

        output.push_str("## Overview\n");
        output.push_str("- **Status:** Ready\n");
        output.push_str(&format!(
            "- **Repository Path:** {}\n",
            metadata.repository_path.display()
        ));
        output.push_str(&format!("- **Files:** {}\n", metadata.files_indexed));
        output.push_str(&format!("- **Chunks:** {}\n", metadata.chunks_created));
        output.push_str(&format!(
            "- **Size:** {}\n",
            format_bytes(metadata.index_size_bytes)
        ));
        output.push_str(&format!(
            "- **Created:** {}\n",
            metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!(
            "- **Last Indexed:** {}\n\n",
            metadata.last_indexed_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        output.push_str("## Configuration\n");
        output.push_str(&format!(
            "- **Chunk size:** {} chars\n",
            metadata.config.chunk_size
        ));
        output.push_str(&format!(
            "- **Overlap:** {} chars\n",
            metadata.config.overlap
        ));
        output.push_str(&format!(
            "- **Include patterns:** {}\n",
            metadata.config.include_patterns.join(", ")
        ));
        output.push_str(&format!(
            "- **Exclude patterns:** {}\n\n",
            metadata.config.exclude_patterns.join(", ")
        ));

        output.push_str("## Statistics\n");
        let avg_chunks = metadata.chunks_created as f64 / metadata.files_indexed.max(1) as f64;
        output.push_str(&format!("- **Avg chunks/file:** {avg_chunks:.2}\n"));

        if metadata.chunks_created > 0 {
            let avg_chunk_size =
                (metadata.index_size_bytes as f64 / metadata.chunks_created.max(1) as f64) as u64;
            output.push_str(&format!(
                "- **Avg chunk size:** {}\n",
                format_bytes(avg_chunk_size)
            ));
        }

        output
    }
}

#[async_trait]
impl McpToolHandler for GetSessionInfoHandler {
    fn name(&self) -> &str {
        "get_session_info"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "get_session_info".to_string(),
            description: "Get detailed metadata and statistics for a specific indexed session. \
                         Shows: status, file count, chunk count, index size, creation date, \
                         chunk configuration (size/overlap), computed statistics (avg chunks/file, avg chunk size). \
                         \
                         USE THIS TO: \
                         (1) Verify indexing results after index_repository completes, \
                         (2) Understand session scope and size before large search operations, \
                         (3) Debug search issues (check if session has expected file count). \
                         \
                         PERFORMANCE: <5ms (very fast, single metadata file read). \
                         \
                         OPTIONAL: Not required for search_code, but helpful for context."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to inspect",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    }
                },
                "required": ["session"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct InfoArgs {
            session: String,
        }

        let args: InfoArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Get metadata from storage
        let metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .map_err(McpError::from)?;

        // Format output
        let text = self.format_info(&metadata);

        Ok(text_content(text))
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

    async fn setup_test_handler() -> (GetSessionInfoHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = GetSessionInfoHandler::new(services);

        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_get_session_info_handler_name() {
        let (handler, _temp) = setup_test_handler().await;
        assert_eq!(handler.name(), "get_session_info");
    }

    #[tokio::test]
    async fn test_get_session_info_handler_schema() {
        let (handler, _temp) = setup_test_handler().await;
        let schema = handler.schema();

        assert_eq!(schema.name, "get_session_info");
        assert!(!schema.description.is_empty());
        assert!(schema.input_schema.is_object());
    }

    #[tokio::test]
    async fn test_get_session_info_valid() {
        let (handler, _temp) = setup_test_handler().await;

        let config = SessionConfig::default();
        handler
            .services
            .storage
            .create_session("test-session", PathBuf::from("/test/repo"), config)
            .unwrap();

        let args = json!({
            "session": "test-session"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_session_info_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "session": "nonexistent"
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_session_info_invalid_args() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "invalid": "field"
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::InvalidParams(_)));
    }

    #[tokio::test]
    async fn test_format_info_markdown() {
        let (handler, _temp) = setup_test_handler().await;

        use chrono::{TimeZone, Utc};
        let metadata = SessionMetadata {
            id: "test-session".to_string(),
            repository_path: PathBuf::from("/test/repo"),
            created_at: Utc.with_ymd_and_hms(2025, 10, 21, 10, 0, 0).unwrap(),
            last_indexed_at: Utc.with_ymd_and_hms(2025, 10, 21, 10, 0, 0).unwrap(),
            files_indexed: 100,
            chunks_created: 500,
            index_size_bytes: 52428800, // 50 MB
            config: SessionConfig::default(),
            schema_version: 3,
        };

        let output = handler.format_info(&metadata);

        assert!(output.contains("# Session: test-session"));
        assert!(output.contains("## Overview"));
        assert!(output.contains("**Status:** Ready"));
        assert!(output.contains("**Repository Path:**"));
        assert!(output.contains("**Files:** 100"));
        assert!(output.contains("**Chunks:** 500"));
        assert!(output.contains("**Size:** 50.00 MB"));
        assert!(output.contains("**Created:** 2025-10-21"));
        assert!(output.contains("**Last Indexed:** 2025-10-21"));
        assert!(output.contains("## Configuration"));
        assert!(output.contains("**Chunk size:** 512 chars"));
        assert!(output.contains("**Overlap:** 64 chars"));
        assert!(output.contains("**Include patterns:**"));
        assert!(output.contains("**Exclude patterns:**"));
        assert!(output.contains("## Statistics"));
        assert!(output.contains("**Avg chunks/file:** 5.00"));
    }

    #[tokio::test]
    async fn test_get_session_info_with_data() {
        let (handler, _temp) = setup_test_handler().await;

        let config = SessionConfig::default();
        let mut index = handler
            .services
            .storage
            .create_session("test-session", PathBuf::from("/test/repo"), config)
            .unwrap();

        // Add some data
        let chunks = vec![Chunk {
            text: "test content".to_string(),
            file_path: PathBuf::from("test.rs"),
            start_offset: 0,
            end_offset: 12,
            chunk_index: 0,
        }];
        index.add_chunks(&chunks, "test-session").unwrap();
        index.commit().unwrap();

        let args = json!({
            "session": "test-session"
        });

        let result = handler.execute(args).await.unwrap();

        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("# Session: test-session"));
                assert!(text.contains("## Overview"));
                assert!(text.contains("## Configuration"));
                assert!(text.contains("## Statistics"));
            }
        }
    }
}
