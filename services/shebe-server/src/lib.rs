//! Shebe - Simple RAG Service for Code Search
//!
//! A production-ready RAG service using BM25 full-text search
//! via Tantivy. Designed for fast, reliable code search with
//! UTF-8 safety and session isolation.
//!
//! # Architecture
//!
//! The codebase is organized into three main modules:
//!
//! - **core**: Domain logic (protocol-agnostic)
//!   - config, error, types, xdg
//!   - storage (session management, Tantivy)
//!   - search (BM25 queries)
//!   - indexer (file walking, chunking)
//!   - services (unified service container)
//!
//! - **http**: REST API adapter (depends on core)
//!   - handlers, middleware
//!
//! - **mcp**: MCP adapter (depends on core)
//!   - server, tools, protocol
//!
//! # Key Features
//!
//! - UTF-8 safe chunking (character-based, never panics)
//! - BM25 search via Tantivy (no vector embeddings)
//! - Session-based indexing (isolated indexes)
//! - REST API (5 endpoints)
//! - MCP server (12 tools)
//! - Production ready (Docker, logging, metrics)

// Core domain logic (protocol-agnostic)
pub mod core;

// HTTP REST adapter
pub mod http;

// MCP (Model Context Protocol) adapter
pub mod mcp;

// Re-export commonly used types for convenience
pub use core::config::Config;
pub use core::error::{Result, ShebeError};
pub use core::services::Services;
pub use core::storage::{SessionConfig, SessionMetadata, StorageManager};
pub use core::types::*;

#[cfg(test)]
mod tests {
    // Module-level integration tests are in tests/ directory
}
