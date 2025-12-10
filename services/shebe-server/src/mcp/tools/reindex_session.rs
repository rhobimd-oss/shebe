//! Re-index session tool handler

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_bytes;
use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

/// Re-index session handler
pub struct ReindexSessionHandler {
    services: Arc<Services>,
}

impl ReindexSessionHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Validate configuration bounds
    fn validate_config(&self, chunk_size: usize, overlap: usize) -> Result<(), McpError> {
        if !(100..=2000).contains(&chunk_size) {
            return Err(McpError::InvalidParams(format!(
                "chunk_size must be between 100 and 2000 (got: {chunk_size})"
            )));
        }

        if overlap > 500 {
            return Err(McpError::InvalidParams(format!(
                "overlap must be between 0 and 500 (got: {overlap})"
            )));
        }

        if overlap >= chunk_size {
            return Err(McpError::InvalidParams(format!(
                "overlap ({overlap}) must be less than chunk_size ({chunk_size})"
            )));
        }

        Ok(())
    }

    /// Compare configurations
    fn compare_configs(
        &self,
        old: &crate::core::storage::SessionConfig,
        new: &crate::core::storage::SessionConfig,
    ) -> ConfigComparison {
        ConfigComparison {
            chunk_size_changed: old.chunk_size != new.chunk_size,
            overlap_changed: old.overlap != new.overlap,
            any_changed: old.chunk_size != new.chunk_size || old.overlap != new.overlap,
        }
    }

    /// Format re-indexing result
    fn format_result(
        &self,
        session: &str,
        stats: &crate::core::types::IndexStats,
        index_size_bytes: u64,
        old_config: &crate::core::storage::SessionConfig,
        new_config: &crate::core::storage::SessionConfig,
        duration_secs: f64,
    ) -> String {
        let mut output = format!(
            "# Session Re-Indexed: `{}`\n\n\
             **Indexing Statistics:**\n\
             - Files indexed: {}\n\
             - Chunks created: {}\n\
             - Index size: {}\n\
             - Duration: {:.2}s\n\
             - Throughput: {:.0} files/sec\n\n",
            session,
            stats.files_indexed,
            stats.chunks_created,
            format_bytes(index_size_bytes),
            duration_secs,
            stats.files_indexed as f64 / duration_secs
        );

        // Show config changes if any
        let comparison = self.compare_configs(old_config, new_config);
        if comparison.any_changed {
            output.push_str("**Configuration Changes:**\n");

            if comparison.chunk_size_changed {
                output.push_str(&format!(
                    "- Chunk size: {} -> {}\n",
                    old_config.chunk_size, new_config.chunk_size
                ));
            }

            if comparison.overlap_changed {
                output.push_str(&format!(
                    "- Overlap: {} -> {}\n",
                    old_config.overlap, new_config.overlap
                ));
            }

            output.push('\n');
        }

        output.push_str(
            "**Note:** Session metadata (repository_path, last_indexed_at) updated automatically.",
        );

        output
    }
}

#[async_trait]
impl McpToolHandler for ReindexSessionHandler {
    fn name(&self) -> &str {
        "reindex_session"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "reindex_session".to_string(),
            description: "Re-index a session using stored repository path. \
                         Convenient for schema migrations or config changes. \
                         Automatically retrieves original path and config from metadata. \
                         Supports config overrides (chunk_size, overlap). \
                         Use force=true to re-index even if config unchanged."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to re-index",
                        "pattern": "^[a-zA-Z0-9_-]{1,64}$"
                    },
                    "chunk_size": {
                        "type": "integer",
                        "description": "Override chunk size (optional, default: use stored config)",
                        "minimum": 100,
                        "maximum": 2000
                    },
                    "overlap": {
                        "type": "integer",
                        "description": "Override overlap (optional, default: use stored config)",
                        "minimum": 0,
                        "maximum": 500
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force re-index even if config unchanged (default: false)",
                        "default": false
                    }
                },
                "required": ["session"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        use crate::core::error::ShebeError;

        // Parse arguments
        let args: ReindexArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // 1. Get session metadata (includes repository_path and config)
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

        // 2. Validate repository path still exists
        if !metadata.repository_path.exists() {
            return Err(McpError::InvalidRequest(format!(
                "Repository path no longer exists: {}\n\
                 Session '{}' cannot be re-indexed.\n\
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

        // 3. Merge configuration (stored + overrides)
        let old_config = metadata.config.clone();
        let new_config = crate::core::storage::SessionConfig {
            chunk_size: args.chunk_size.unwrap_or(old_config.chunk_size),
            overlap: args.overlap.unwrap_or(old_config.overlap),
            include_patterns: old_config.include_patterns.clone(), // Preserve patterns
            exclude_patterns: old_config.exclude_patterns.clone(),
        };

        // 4. Validate new configuration
        self.validate_config(new_config.chunk_size, new_config.overlap)?;

        // 5. Check if force is needed
        let comparison = self.compare_configs(&old_config, &new_config);
        if !comparison.any_changed && !args.force {
            return Err(McpError::InvalidRequest(format!(
                "Configuration unchanged for session '{}'. \
                 Use force=true to re-index anyway.\n\
                 Current config: chunk_size={}, overlap={}",
                args.session, old_config.chunk_size, old_config.overlap
            )));
        }

        // 6. Delete existing session
        self.services
            .storage
            .delete_session(&args.session)
            .map_err(|e| McpError::InternalError(format!("Failed to delete session: {e}")))?;

        // 7. Re-index repository
        let start = Instant::now();
        let stats = self
            .services
            .storage
            .index_repository(
                &args.session,
                &metadata.repository_path,
                new_config.include_patterns.clone(),
                new_config.exclude_patterns.clone(),
                new_config.chunk_size,
                new_config.overlap,
                100,   // max_file_size_mb default
                false, // force (already deleted above)
            )
            .map_err(|e| McpError::InternalError(format!("Re-indexing failed: {e}")))?;
        let duration_secs = start.elapsed().as_secs_f64();

        // Get updated metadata to retrieve index size
        let updated_metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .map_err(|e| McpError::InternalError(format!("Failed to get updated metadata: {e}")))?;

        // 8. Format result
        let result = self.format_result(
            &args.session,
            &stats,
            updated_metadata.index_size_bytes,
            &old_config,
            &new_config,
            duration_secs,
        );

        Ok(text_content(result))
    }
}

#[derive(Debug, Deserialize)]
struct ReindexArgs {
    session: String,
    #[serde(default)]
    chunk_size: Option<usize>,
    #[serde(default)]
    overlap: Option<usize>,
    #[serde(default)]
    force: bool,
}

struct ConfigComparison {
    chunk_size_changed: bool,
    overlap_changed: bool,
    any_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::core::storage::SessionConfig;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (ReindexSessionHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = ReindexSessionHandler::new(services);

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
    async fn test_reindex_session_basic() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-session").await;

        let args = json!({
            "session": "test-session",
            "force": true,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("Session Re-Indexed:"));
        assert!(text.contains("test-session"));
        assert!(text.contains("Files indexed:"));
        assert!(text.contains("Chunks created:"));
    }

    #[tokio::test]
    async fn test_reindex_session_with_config_override() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-override").await;

        let args = json!({
            "session": "test-override",
            "chunk_size": 1024,
            "overlap": 128,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("Configuration Changes:"));
        assert!(text.contains("512 -> 1024")); // chunk_size changed
        assert!(text.contains("64 -> 128")); // overlap changed

        // Verify new config stored
        let metadata = handler
            .services
            .storage
            .get_session_metadata("test-override")
            .unwrap();
        assert_eq!(metadata.config.chunk_size, 1024);
        assert_eq!(metadata.config.overlap, 128);
    }

    #[tokio::test]
    async fn test_reindex_session_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "session": "nonexistent",
            "force": true,
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
    async fn test_reindex_session_path_not_exists() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-missing").await;

        // Delete repository directory
        std::fs::remove_dir_all(&repo_path).unwrap();

        let args = json!({
            "session": "test-missing",
            "force": true,
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
    async fn test_reindex_session_unchanged_without_force() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-noforce").await;

        let args = json!({
            "session": "test-noforce",
            // No config changes, no force
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("Configuration unchanged"));
            assert!(msg.contains("force=true"));
        } else {
            panic!("Expected InvalidRequest error for unchanged config");
        }
    }

    #[tokio::test]
    async fn test_reindex_session_invalid_chunk_size() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-invalid").await;

        let args = json!({
            "session": "test-invalid",
            "chunk_size": 50, // Too small (< 100)
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("chunk_size"));
            assert!(msg.contains("100 and 2000"));
        } else {
            panic!("Expected InvalidParams error");
        }
    }

    #[tokio::test]
    async fn test_reindex_session_overlap_too_large() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-overlap").await;

        let args = json!({
            "session": "test-overlap",
            "chunk_size": 512,
            "overlap": 600, // Too large (> 500)
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("overlap"));
            assert!(msg.contains("0 and 500"));
        } else {
            panic!("Expected InvalidParams error");
        }
    }

    #[tokio::test]
    async fn test_reindex_session_updates_last_indexed_at() {
        let (handler, temp_dir) = setup_test_handler().await;
        let repo_path = temp_dir.path().join("test_repo");
        create_test_session(&handler.services, &repo_path, "test-timestamp").await;

        // Get original timestamp
        let old_metadata = handler
            .services
            .storage
            .get_session_metadata("test-timestamp")
            .unwrap();
        let old_timestamp = old_metadata.last_indexed_at;

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(100));

        let args = json!({
            "session": "test-timestamp",
            "force": true,
        });

        handler.execute(args).await.unwrap();

        // Verify timestamp updated
        let new_metadata = handler
            .services
            .storage
            .get_session_metadata("test-timestamp")
            .unwrap();
        assert!(new_metadata.last_indexed_at > old_timestamp);
    }
}
