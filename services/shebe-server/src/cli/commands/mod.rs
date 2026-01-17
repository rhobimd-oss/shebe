//! CLI command implementations
//!
//! Each command module handles argument parsing and execution for a specific CLI command.
//! Command names match MCP tool names (underscores become hyphens in CLI).

pub mod completions;
pub mod config;
pub mod index;
pub mod info;
pub mod references;
pub mod search;
pub mod session;

// Re-export argument types for use in mod.rs
pub use completions::CompletionsArgs;
pub use config::ConfigArgs;
pub use index::IndexArgs;
pub use info::InfoArgs;
pub use references::ReferencesArgs;
pub use search::SearchArgs;
