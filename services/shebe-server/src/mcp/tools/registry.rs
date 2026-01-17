//! Tool registry for managing MCP tools

use super::handler::McpToolHandler;
use crate::mcp::protocol::ToolSchema;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for all available MCP tools
///
/// Maintains a collection of tool handlers and provides methods
/// for tool discovery and execution.
pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn McpToolHandler>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a tool handler
    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
        let name = handler.name().to_string();
        self.handlers.insert(name, handler);
    }

    /// Get a tool handler by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn McpToolHandler>> {
        self.handlers.get(name)
    }

    /// List all available tool schemas
    pub fn list(&self) -> Vec<ToolSchema> {
        self.handlers
            .values()
            .map(|handler| handler.schema())
            .collect()
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Get number of registered tools
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::error::McpError;
    use crate::mcp::protocol::{ContentBlock, ToolResult};
    use async_trait::async_trait;
    use serde_json::{json, Value};

    // Mock tool handler for testing
    struct MockToolHandler {
        name: String,
    }

    #[async_trait]
    impl McpToolHandler for MockToolHandler {
        fn name(&self) -> &str {
            &self.name
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: self.name.clone(),
                description: "Test tool".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            }
        }

        async fn execute(&self, _args: Value) -> Result<ToolResult, McpError> {
            Ok(ToolResult {
                content: vec![ContentBlock::Text {
                    text: "test result".to_string(),
                }],
            })
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(MockToolHandler {
            name: "test_tool".to_string(),
        });

        registry.register(handler);
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.contains("test_tool"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(MockToolHandler {
            name: "test_tool".to_string(),
        });

        registry.register(handler);
        let retrieved = registry.get("test_tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_tool");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = ToolRegistry::new();
        let retrieved = registry.get("nonexistent");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = ToolRegistry::new();
        let handler1 = Arc::new(MockToolHandler {
            name: "tool1".to_string(),
        });
        let handler2 = Arc::new(MockToolHandler {
            name: "tool2".to_string(),
        });

        registry.register(handler1);
        registry.register(handler2);

        let schemas = registry.list();
        assert_eq!(schemas.len(), 2);
    }

    #[test]
    fn test_registry_contains() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(MockToolHandler {
            name: "test_tool".to_string(),
        });

        registry.register(handler);
        assert!(registry.contains("test_tool"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.len(), 0);
    }
}
