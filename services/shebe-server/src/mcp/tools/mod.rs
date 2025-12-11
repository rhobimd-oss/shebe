//! MCP tool implementations
//!
//! This module contains all MCP tool handlers that expose Shebe's
//! functionality to Claude Code.

pub mod delete_session;
pub mod find_file;
pub mod find_references;
pub mod get_server_info;
pub mod get_session_info;
pub mod handler;
pub mod helpers;
pub mod index_repository;
pub mod list_dir;
pub mod list_sessions;
pub mod preview_chunk;
pub mod read_file;
pub mod registry;
pub mod reindex_session;
pub mod search_code;
pub mod show_shebe_config;
pub mod upgrade_session;

pub use delete_session::DeleteSessionHandler;
pub use find_file::FindFileHandler;
pub use find_references::FindReferencesHandler;
pub use get_server_info::GetServerInfoHandler;
pub use get_session_info::GetSessionInfoHandler;
pub use handler::{text_content, McpToolHandler};
pub use helpers::{detect_language, format_bytes, truncate_text};
pub use index_repository::IndexRepositoryHandler;
pub use list_dir::ListDirHandler;
pub use list_sessions::ListSessionsHandler;
pub use preview_chunk::PreviewChunkHandler;
pub use read_file::ReadFileHandler;
pub use registry::ToolRegistry;
pub use reindex_session::ReindexSessionHandler;
pub use search_code::SearchCodeHandler;
pub use show_shebe_config::ShowShebeConfigHandler;
pub use upgrade_session::UpgradeSessionHandler;
