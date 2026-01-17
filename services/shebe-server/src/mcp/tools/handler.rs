//! Tool handler trait and common types

use crate::mcp::error::McpError;
use crate::mcp::protocol::{ContentBlock, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde_json::Value;

/// Trait for MCP tool implementations
///
/// Each tool (search_code, list_sessions, etc.) implements this trait
/// to provide schema and execution logic.
#[async_trait]
pub trait McpToolHandler: Send + Sync {
    /// Tool name (e.g., "search_code")
    fn name(&self) -> &str;

    /// Tool schema for tools/list
    fn schema(&self) -> ToolSchema;

    /// Execute tool with arguments
    async fn execute(&self, args: Value) -> Result<ToolResult, McpError>;
}

/// Helper function to create a text content block
pub fn text_content(text: String) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text { text }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content() {
        let result = text_content("test message".to_string());
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "test message"),
        }
    }
}
