//! Get server info tool handler
//!
//! Returns version and build information about the running shebe-mcp server.

use super::handler::{text_content, McpToolHandler};
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct GetServerInfoHandler;

impl Default for GetServerInfoHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GetServerInfoHandler {
    pub fn new() -> Self {
        Self
    }

    fn format_info(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");
        let rust_version = env!("CARGO_PKG_RUST_VERSION");

        let mut output = String::from("# Shebe MCP Server Information\n\n");

        output.push_str("## Version\n");
        output.push_str(&format!("- **Version:** {version}\n"));
        output.push_str(&format!("- **Rust Version:** {rust_version}\n\n"));

        output.push_str("## Server Details\n");
        output.push_str("- **Name:** shebe-mcp\n");
        output.push_str("- **Description:** BM25 full-text search MCP server\n");
        output.push_str("- **Protocol:** MCP 2024-11-05\n\n");

        output.push_str("## Available Tools\n");
        output.push_str("- search_code: Search indexed code\n");
        output.push_str("- list_sessions: List all sessions\n");
        output.push_str("- get_session_info: Get session details\n");
        output.push_str("- index_repository: Index a repository (synchronous)\n");
        output.push_str("- get_server_info: Show server version (this tool)\n");
        output.push_str("- show_shebe_config: Show current configuration\n");
        output.push_str("- read_file: Read full file contents from session\n");
        output.push_str("- delete_session: Delete session and all data\n");
        output.push_str("- list_dir: List all files in session\n");
        output.push_str("- find_file: Find files by pattern (glob/regex)\n");
        output.push_str("- preview_chunk: Show N lines before/after search result chunk\n");
        output.push_str("- reindex_session: Re-index session using stored repository path\n");

        output
    }
}

#[async_trait]
impl McpToolHandler for GetServerInfoHandler {
    fn name(&self) -> &str {
        "get_server_info"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "get_server_info".to_string(),
            description: "Get version and build information about the running shebe-mcp server. \
                         Returns server version, protocol version, and available tools. \
                         Use this to check which version of shebe-mcp is running. \
                         Fast operation (<1ms)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult, McpError> {
        let text = self.format_info();
        Ok(text_content(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_server_info_handler_name() {
        let handler = GetServerInfoHandler::new();
        assert_eq!(handler.name(), "get_server_info");
    }

    #[tokio::test]
    async fn test_get_server_info_handler_schema() {
        let handler = GetServerInfoHandler::new();
        let schema = handler.schema();

        assert_eq!(schema.name, "get_server_info");
        assert!(!schema.description.is_empty());
        assert!(schema.input_schema.is_object());
    }

    #[tokio::test]
    async fn test_get_server_info_execute() {
        let handler = GetServerInfoHandler::new();

        let result = handler.execute(json!({})).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => {
                assert!(text.contains("# Shebe MCP Server Information"));
                assert!(text.contains("## Version"));
                assert!(text.contains("## Server Details"));
                assert!(text.contains("## Available Tools"));
                assert!(text.contains(env!("CARGO_PKG_VERSION")));
            }
        }
    }

    #[tokio::test]
    async fn test_format_info_contains_version() {
        let handler = GetServerInfoHandler::new();
        let output = handler.format_info();

        assert!(output.contains(env!("CARGO_PKG_VERSION")));
        assert!(output.contains("shebe-mcp"));
        assert!(output.contains("MCP 2024-11-05"));
    }

    #[tokio::test]
    async fn test_format_info_lists_tools() {
        let handler = GetServerInfoHandler::new();
        let output = handler.format_info();

        assert!(output.contains("search_code"));
        assert!(output.contains("list_sessions"));
        assert!(output.contains("get_session_info"));
        assert!(output.contains("index_repository"));
        // Note: index_status was removed in v0.3.0 (synchronous indexing)
        assert!(output.contains("get_server_info"));
        assert!(output.contains("show_shebe_config"));
        assert!(output.contains("preview_chunk"));
    }
}
