//! Core domain logic (protocol-agnostic)
//!
//! This module contains all business logic that is independent
//! of transport protocols (HTTP, MCP, etc).
//!
//! # Architecture
//!
//! - **config**: Configuration loading (TOML + environment)
//! - **error**: Error types and Result alias
//! - **types**: Domain data structures
//! - **xdg**: XDG directory handling
//! - **storage**: Session and Tantivy index management
//! - **search**: BM25 search implementation
//! - **indexer**: File walking and chunking pipeline
//! - **services**: Unified service container

pub mod config;
pub mod error;
pub mod indexer;
pub mod search;
pub mod services;
pub mod storage;
pub mod types;
pub mod xdg;

// Re-export key types for convenience
pub use config::Config;
pub use error::{Result, ShebeError};
pub use services::Services;
