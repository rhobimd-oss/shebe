//! MCP (Model Context Protocol) adapter
//!
//! Depends only on core/.
//!
//! This module implements a JSON-RPC 2.0 compliant MCP server that
//! exposes Shebe's search capabilities as MCP tools for Claude Code.

pub mod error;
pub mod handlers;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;
pub mod utils;

// Re-export main types
pub use error::McpError;
pub use server::McpServer;
pub use tools::{McpToolHandler, ToolRegistry};
