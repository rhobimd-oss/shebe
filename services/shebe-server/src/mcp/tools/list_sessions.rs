//! List sessions tool handler

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_bytes;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use crate::mcp::services::ShebeServices;
use crate::storage::SessionMetadata;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ListSessionsHandler {
    services: Arc<ShebeServices>,
}

impl ListSessionsHandler {
    pub fn new(services: Arc<ShebeServices>) -> Self {
        Self { services }
    }

    fn format_sessions(&self, sessions: &[SessionMetadata]) -> String {
        if sessions.is_empty() {
            return "No sessions available. Use the REST API or index a repository first."
                .to_string();
        }

        let mut output = format!("Available sessions ({}):\n\n", sessions.len());

        for session in sessions {
            output.push_str(&format!("## {}\n", session.id));
            output.push_str(&format!("- **Files:** {}\n", session.files_indexed));
            output.push_str(&format!("- **Chunks:** {}\n", session.chunks_created));
            output.push_str(&format!(
                "- **Size:** {}\n",
                format_bytes(session.index_size_bytes)
            ));
            output.push_str(&format!("- **Created:** {}\n\n", session.created_at));
        }

        output
    }
}

#[async_trait]
impl McpToolHandler for ListSessionsHandler {
    fn name(&self) -> &str {
        "list_sessions"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "list_sessions".to_string(),
            description: "List all indexed code sessions available for search_code queries. \
                         Shows: session ID, file count, chunk count, index size, creation timestamp. \
                         \
                         USE THIS FIRST: Run before search_code to discover which sessions exist. \
                         Each session represents a specific indexed repository/codebase. \
                         \
                         PERFORMANCE: <10ms (very fast, low overhead). \
                         \
                         WORKFLOW: list_sessions -> search_code (with discovered session ID) -> get_session_info (optional details)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult, McpError> {
        // Get sessions from storage
        let sessions = self
            .services
            .storage
            .list_sessions()
            .map_err(McpError::from)?;

        // Format output
        let text = self.format_sessions(&sessions);

        Ok(text_content(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::SessionConfig;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (ListSessionsHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(ShebeServices::new(config));
        let handler = ListSessionsHandler::new(services);

        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_list_sessions_handler_name() {
        let (handler, _temp) = setup_test_handler().await;
        assert_eq!(handler.name(), "list_sessions");
    }

    #[tokio::test]
    async fn test_list_sessions_handler_schema() {
        let (handler, _temp) = setup_test_handler().await;
        let schema = handler.schema();

        assert_eq!(schema.name, "list_sessions");
        assert!(!schema.description.is_empty());
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let (handler, _temp) = setup_test_handler().await;

        let result = handler.execute(json!({})).await.unwrap();

        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("No sessions available"));
            }
        }
    }

    #[tokio::test]
    async fn test_list_sessions_with_data() {
        let (handler, _temp) = setup_test_handler().await;

        // Create test sessions
        let config = SessionConfig::default();
        handler
            .services
            .storage
            .create_session("session1", PathBuf::from("/test/repo"), config.clone())
            .unwrap();
        handler
            .services
            .storage
            .create_session("session2", PathBuf::from("/test/repo"), config.clone())
            .unwrap();

        let result = handler.execute(json!({})).await.unwrap();

        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("Available sessions (2)"));
                assert!(text.contains("## session1"));
                assert!(text.contains("## session2"));
                assert!(text.contains("**Files:**"));
                assert!(text.contains("**Chunks:**"));
                assert!(text.contains("**Size:**"));
                assert!(text.contains("**Created:**"));
            }
        }
    }

    #[tokio::test]
    async fn test_format_sessions_empty() {
        let (handler, _temp) = setup_test_handler().await;
        let sessions = vec![];

        let output = handler.format_sessions(&sessions);
        assert!(output.contains("No sessions available"));
    }

    #[tokio::test]
    async fn test_format_sessions_markdown() {
        let (handler, _temp) = setup_test_handler().await;

        use chrono::{TimeZone, Utc};
        let sessions = vec![SessionMetadata {
            id: "test-session".to_string(),
            repository_path: PathBuf::from("/test/repo"),
            created_at: Utc.with_ymd_and_hms(2025, 10, 21, 10, 0, 0).unwrap(),
            last_indexed_at: Utc.with_ymd_and_hms(2025, 10, 21, 10, 0, 0).unwrap(),
            files_indexed: 100,
            chunks_created: 500,
            index_size_bytes: 1048576, // 1 MB
            config: SessionConfig::default(),
            schema_version: 3,
        }];

        let output = handler.format_sessions(&sessions);

        assert!(output.contains("Available sessions (1)"));
        assert!(output.contains("## test-session"));
        assert!(output.contains("**Files:** 100"));
        assert!(output.contains("**Chunks:** 500"));
        assert!(output.contains("**Size:** 1.00 MB"));
        assert!(output.contains("**Created:** 2025-10-21")); // Check for date only, not full timestamp
    }

    #[tokio::test]
    async fn test_list_sessions_multiple() {
        let (handler, _temp) = setup_test_handler().await;

        let config = SessionConfig::default();
        handler
            .services
            .storage
            .create_session("s1", PathBuf::from("/test/repo"), config.clone())
            .unwrap();
        handler
            .services
            .storage
            .create_session("s2", PathBuf::from("/test/repo"), config.clone())
            .unwrap();
        handler
            .services
            .storage
            .create_session("s3", PathBuf::from("/test/repo"), config.clone())
            .unwrap();

        let result = handler.execute(json!({})).await.unwrap();

        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("Available sessions (3)"));
            }
        }
    }
}
