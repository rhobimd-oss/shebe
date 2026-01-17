//! Upgrade session tool handler
//!
//! Convenience tool that deletes and re-indexes a session in one step.
//! Useful for schema migrations when a session uses an old schema version.

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_bytes;
use crate::core::services::Services;
use crate::core::storage::SCHEMA_VERSION;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

/// Upgrade session handler
pub struct UpgradeSessionHandler {
    services: Arc<Services>,
}

impl UpgradeSessionHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Format upgrade result
    fn format_result(
        &self,
        session: &str,
        old_schema: u32,
        new_schema: u32,
        stats: &crate::core::types::IndexStats,
        index_size_bytes: u64,
        duration_secs: f64,
    ) -> String {
        format!(
            "# Session Upgraded: `{}`\n\n\
             **Schema Migration:**\n\
             - Previous version: v{}\n\
             - Current version: v{}\n\n\
             **Indexing Statistics:**\n\
             - Files indexed: {}\n\
             - Chunks created: {}\n\
             - Index size: {}\n\
             - Duration: {:.2}s\n\
             - Throughput: {:.0} files/sec\n\n\
             Session is now compatible with the current schema.",
            session,
            old_schema,
            new_schema,
            stats.files_indexed,
            stats.chunks_created,
            format_bytes(index_size_bytes),
            duration_secs,
            if duration_secs > 0.0 {
                stats.files_indexed as f64 / duration_secs
            } else {
                0.0
            }
        )
    }
}

#[async_trait]
impl McpToolHandler for UpgradeSessionHandler {
    fn name(&self) -> &str {
        "upgrade_session"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "upgrade_session".to_string(),
            description: "Upgrade a session to the current schema version. \
                         Deletes the existing session and re-indexes using the stored \
                         repository path and configuration. Fast (~1-3 seconds). \
                         Use when a session fails with 'old schema version' error."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to upgrade",
                        "pattern": "^[a-zA-Z0-9_-]{1,64}$"
                    }
                },
                "required": ["session"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        use crate::core::error::ShebeError;

        // Parse arguments
        let args: UpgradeArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // 1. Get session metadata (includes repository_path, config and schema_version)
        let metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .map_err(|e| match e {
                ShebeError::SessionNotFound(_) => McpError::InvalidRequest(format!(
                    "Session '{}' not found. Use list_sessions to see available sessions.",
                    args.session
                )),
                _ => McpError::from(e),
            })?;

        let old_schema = metadata.schema_version;

        // 2. Check if upgrade is needed
        if old_schema >= SCHEMA_VERSION {
            return Ok(text_content(format!(
                "Session '{}' is already at schema v{} (current version). No upgrade needed.",
                args.session, old_schema
            )));
        }

        // 3. Validate repository path still exists
        if !metadata.repository_path.exists() {
            return Err(McpError::InvalidRequest(format!(
                "Repository path no longer exists: {}\n\
                 Session '{}' cannot be upgraded.\n\
                 Possible solutions:\n\
                 - Move repository back to original location\n\
                 - Delete session and create new one at current location",
                metadata.repository_path.display(),
                args.session
            )));
        }

        if !metadata.repository_path.is_dir() {
            return Err(McpError::InvalidRequest(format!(
                "Repository path is not a directory: {}",
                metadata.repository_path.display()
            )));
        }

        // 4. Store config before deleting session
        let config = metadata.config.clone();
        let repo_path = metadata.repository_path.clone();

        // 5. Delete old session
        self.services
            .storage
            .delete_session(&args.session)
            .map_err(|e| McpError::InternalError(format!("Failed to delete session: {e}")))?;

        // 6. Re-index repository with same configuration
        let start = Instant::now();
        let stats = self
            .services
            .storage
            .index_repository(
                &args.session,
                &repo_path,
                config.include_patterns.clone(),
                config.exclude_patterns.clone(),
                config.chunk_size,
                config.overlap,
                100,   // max_file_size_mb default
                false, // force (already deleted above)
            )
            .map_err(|e| McpError::InternalError(format!("Re-indexing failed: {e}")))?;
        let duration_secs = start.elapsed().as_secs_f64();

        // 7. Get updated metadata to retrieve index size
        let updated_metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .map_err(|e| McpError::InternalError(format!("Failed to get updated metadata: {e}")))?;

        // 8. Format result
        let result = self.format_result(
            &args.session,
            old_schema,
            SCHEMA_VERSION,
            &stats,
            updated_metadata.index_size_bytes,
            duration_secs,
        );

        Ok(text_content(result))
    }
}

#[derive(Debug, Deserialize)]
struct UpgradeArgs {
    session: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;

    use tempfile::TempDir;

    async fn setup_test_handler() -> (UpgradeSessionHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = UpgradeSessionHandler::new(services);

        (handler, temp_dir)
    }

    async fn create_test_session(
        services: &Arc<Services>,
        repo_path: &std::path::Path,
        session_id: &str,
    ) {
        // Create test repository
        std::fs::create_dir_all(repo_path).unwrap();
        std::fs::write(repo_path.join("test.rs"), "fn main() {}").unwrap();

        // Index repository
        services
            .storage
            .index_repository(
                session_id,
                repo_path,
                vec!["**/*".to_string()],
                vec!["**/target/**".to_string()],
                512,
                64,
                100,
                false,
            )
            .unwrap();
    }

    #[tokio::test]
    async fn test_upgrade_session_handler_name() {
        let (handler, _temp) = setup_test_handler().await;
        assert_eq!(handler.name(), "upgrade_session");
    }

    #[tokio::test]
    async fn test_upgrade_session_handler_schema() {
        let (handler, _temp) = setup_test_handler().await;
        let schema = handler.schema();
        assert_eq!(schema.name, "upgrade_session");
        assert!(schema.description.contains("Upgrade"));
    }

    #[tokio::test]
    async fn test_upgrade_session_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "session": "nonexistent"
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("not found"));
            assert!(msg.contains("list_sessions"));
        } else {
            panic!("Expected InvalidRequest error");
        }
    }

    #[tokio::test]
    async fn test_upgrade_session_already_current() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-current").await;

        // Session is already at current schema version (just created)
        let args = json!({
            "session": "test-current"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("already at schema"));
        assert!(text.contains("No upgrade needed"));
    }

    #[tokio::test]
    async fn test_upgrade_session_old_schema() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-old").await;

        // Manually downgrade schema version to simulate old session
        let mut metadata = handler
            .services
            .storage
            .get_session_metadata("test-old")
            .unwrap();
        metadata.schema_version = 1; // Old schema
        handler
            .services
            .storage
            .update_session_metadata("test-old", &metadata)
            .unwrap();

        let args = json!({
            "session": "test-old"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("Session Upgraded:"));
        assert!(text.contains("Previous version: v1"));
        assert!(text.contains(&format!("Current version: v{}", SCHEMA_VERSION)));
        assert!(text.contains("Files indexed:"));
    }

    #[tokio::test]
    async fn test_upgrade_session_path_not_exists() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-missing").await;

        // Manually downgrade schema version
        let mut metadata = handler
            .services
            .storage
            .get_session_metadata("test-missing")
            .unwrap();
        metadata.schema_version = 1;
        handler
            .services
            .storage
            .update_session_metadata("test-missing", &metadata)
            .unwrap();

        // Delete repository directory
        std::fs::remove_dir_all(&repo_path).unwrap();

        let args = json!({
            "session": "test-missing"
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("no longer exists"));
            assert!(msg.contains("Possible solutions:"));
        } else {
            panic!("Expected InvalidRequest error for missing path");
        }
    }

    #[tokio::test]
    async fn test_upgrade_preserves_config() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");

        // Create test repository
        std::fs::create_dir_all(&repo_path).unwrap();
        std::fs::write(repo_path.join("test.rs"), "fn main() {}").unwrap();

        // Index with custom config
        handler
            .services
            .storage
            .index_repository(
                "test-config",
                &repo_path,
                vec!["**/*.rs".to_string()],
                vec!["**/target/**".to_string()],
                1024, // Custom chunk_size
                128,  // Custom overlap
                100,
                false,
            )
            .unwrap();

        // Manually downgrade schema version
        let mut metadata = handler
            .services
            .storage
            .get_session_metadata("test-config")
            .unwrap();
        let old_config = metadata.config.clone();
        metadata.schema_version = 1;
        handler
            .services
            .storage
            .update_session_metadata("test-config", &metadata)
            .unwrap();

        let args = json!({
            "session": "test-config"
        });

        handler.execute(args).await.unwrap();

        // Verify config was preserved
        let new_metadata = handler
            .services
            .storage
            .get_session_metadata("test-config")
            .unwrap();
        assert_eq!(new_metadata.config.chunk_size, old_config.chunk_size);
        assert_eq!(new_metadata.config.overlap, old_config.overlap);
        assert_eq!(
            new_metadata.config.include_patterns,
            old_config.include_patterns
        );
    }
}
