//! HTTP REST adapter
//!
//! Depends only on core/. Never imports from mcp/.
//!
//! Provides HTTP endpoints for indexing, searching and session
//! management via Axum web framework.

pub mod handlers;
pub mod middleware;

pub use handlers::*;
