//! MCP adapter integration tests
//!
//! Tests for MCP protocol handling and tool implementations.

mod common;

// MCP submodules - tests/mcp/ directory
mod mcp {
    pub mod find_references_tests;
    pub mod handler_tests;
    pub mod protocol_tests;
}
