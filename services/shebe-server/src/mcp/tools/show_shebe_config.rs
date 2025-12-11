//! Show Shebe config tool handler
//!
//! Returns the current configuration of the running shebe-mcp server.

use super::handler::{text_content, McpToolHandler};
use crate::core::config::Config;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ShowShebeConfigHandler {
    config: Arc<Config>,
}

impl ShowShebeConfigHandler {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    fn format_config(&self) -> String {
        let mut output = String::from("# Shebe MCP Configuration\n\n");

        output.push_str("## Server\n");
        output.push_str(&format!("- **Host:** {}\n", self.config.server.host));
        output.push_str(&format!("- **Port:** {}\n", self.config.server.port));
        output.push_str(&format!(
            "- **Log Level:** {}\n\n",
            self.config.server.log_level
        ));

        output.push_str("## Indexing\n");
        output.push_str(&format!(
            "- **Chunk Size:** {} chars\n",
            self.config.indexing.chunk_size
        ));
        output.push_str(&format!(
            "- **Overlap:** {} chars\n",
            self.config.indexing.overlap
        ));
        output.push_str(&format!(
            "- **Max File Size:** {} MB\n",
            self.config.indexing.max_file_size_mb
        ));
        output.push_str(&format!(
            "- **Include Patterns:** {} patterns\n",
            self.config.indexing.include_patterns.len()
        ));
        output.push_str(&format!(
            "- **Exclude Patterns:** {} patterns\n\n",
            self.config.indexing.exclude_patterns.len()
        ));

        output.push_str("## Storage\n");
        output.push_str(&format!(
            "- **Index Directory:** {}\n\n",
            self.config.storage.index_dir.display()
        ));

        output.push_str("## Search\n");
        output.push_str(&format!(
            "- **Default K:** {}\n",
            self.config.search.default_k
        ));
        output.push_str(&format!("- **Max K:** {}\n", self.config.search.max_k));
        output.push_str(&format!(
            "- **Max Query Length:** {}\n\n",
            self.config.search.max_query_length
        ));

        output.push_str("## Limits\n");
        output.push_str(&format!(
            "- **Max Concurrent Indexes:** {}\n",
            self.config.limits.max_concurrent_indexes
        ));
        output.push_str(&format!(
            "- **Request Timeout:** {}s\n",
            self.config.limits.request_timeout_sec
        ));

        output
    }

    fn format_config_detailed(&self) -> String {
        let mut output = self.format_config();

        output.push_str("\n## Include Patterns\n");
        for pattern in &self.config.indexing.include_patterns {
            output.push_str(&format!("- `{pattern}`\n"));
        }

        output.push_str("\n## Exclude Patterns\n");
        for pattern in &self.config.indexing.exclude_patterns {
            output.push_str(&format!("- `{pattern}`\n"));
        }

        output
    }
}

#[async_trait]
impl McpToolHandler for ShowShebeConfigHandler {
    fn name(&self) -> &str {
        "show_shebe_config"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "show_shebe_config".to_string(),
            description: "Show the current configuration of the running shebe-mcp server. \
                         Shows all settings: indexing, search, storage and limits. \
                         Use this to understand how the server is configured. \
                         Fast operation (<1ms)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "detailed": {
                        "type": "boolean",
                        "description": "Show detailed configuration including all patterns",
                        "default": false
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        // Parse optional detailed parameter
        let detailed = args
            .get("detailed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let text = if detailed {
            self.format_config_detailed()
        } else {
            self.format_config()
        };

        Ok(text_content(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;

    fn setup_test_handler() -> ShowShebeConfigHandler {
        let config = Arc::new(Config::default());
        ShowShebeConfigHandler::new(config)
    }

    #[tokio::test]
    async fn test_show_shebe_config_handler_name() {
        let handler = setup_test_handler();
        assert_eq!(handler.name(), "show_shebe_config");
    }

    #[tokio::test]
    async fn test_show_shebe_config_handler_schema() {
        let handler = setup_test_handler();
        let schema = handler.schema();

        assert_eq!(schema.name, "show_shebe_config");
        assert!(!schema.description.is_empty());
        assert!(schema.input_schema.is_object());
    }

    #[tokio::test]
    async fn test_show_shebe_config_execute_basic() {
        let handler = setup_test_handler();

        let result = handler.execute(json!({})).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("# Shebe MCP Configuration"));
                assert!(text.contains("## Server"));
                assert!(text.contains("## Indexing"));
                assert!(text.contains("## Storage"));
                assert!(text.contains("## Search"));
                assert!(text.contains("## Limits"));
            }
        }
    }

    #[tokio::test]
    async fn test_show_shebe_config_execute_detailed() {
        let handler = setup_test_handler();

        let result = handler.execute(json!({"detailed": true})).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("# Shebe MCP Configuration"));
                assert!(text.contains("## Include Patterns"));
                assert!(text.contains("## Exclude Patterns"));
                assert!(text.contains("*.rs"));
                assert!(text.contains("**/node_modules/**"));
            }
        }
    }

    #[tokio::test]
    async fn test_show_shebe_config_format_contains_defaults() {
        let handler = setup_test_handler();
        let output = handler.format_config();

        assert!(output.contains("127.0.0.1"));
        assert!(output.contains("3000"));
        assert!(output.contains("512 chars")); // chunk size
        assert!(output.contains("64 chars")); // overlap
        assert!(output.contains("10 MB")); // max file size
    }

    #[tokio::test]
    async fn test_show_shebe_config_format_detailed_lists_patterns() {
        let handler = setup_test_handler();
        let output = handler.format_config_detailed();

        assert!(output.contains("## Include Patterns"));
        assert!(output.contains("## Exclude Patterns"));
        assert!(output.contains("`*.rs`"));
        assert!(output.contains("`**/target/**`"));
    }
}
