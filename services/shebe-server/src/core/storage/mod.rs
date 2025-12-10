//! Storage layer for Tantivy-based BM25 indexing.
//!
//! This module provides the storage layer for the Shebe RAG
//! service, wrapping Tantivy for BM25 full-text search and
//! managing session-based index isolation.
//!
//! # Architecture
//!
//! - **TantivyIndex**: Wraps Tantivy index operations
//! - **StorageManager**: Manages session-based storage
//! - **SessionMetadata**: Tracks session statistics
//!
//! # Session Storage Structure
//!
//! ```text
//! {storage_root}/sessions/
//! ├── {session-id-1}/
//! │   ├── meta.json           # Session metadata
//! │   └── tantivy/            # Tantivy index
//! │       ├── .managed.json
//! │       ├── meta.json
//! │       └── [segment files]
//! ```

mod session;
mod tantivy;
mod validator;

// Note: SessionConfig and SessionMetadata used in shebe-mcp binary and integration tests
#[allow(unused_imports)]
pub use session::{SessionConfig, SessionMetadata, StorageManager};
// Note: Used in shebe-mcp binary, not in lib tests
#[allow(unused_imports)]
pub use validator::{MetadataValidator, ValidationReport};
