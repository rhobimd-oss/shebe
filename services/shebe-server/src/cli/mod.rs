//! CLI adapter for Shebe
//!
//! Provides command-line interface for Shebe's search and indexing capabilities.
//! This module is parallel to `mcp/` - both depend on `core/` but not on each other.
//!
//! # Architecture
//!
//! ```text
//!              +------------------+
//!              |     core/        |
//!              |  (domain logic)  |
//!              +--------+---------+
//!                       |
//!          +------------+------------+
//!          |                         |
//!          v                         v
//! +------------------+      +------------------+
//! |      mcp/        |      |      cli/        |
//! | (stdio adapter)  |      | (clap adapter)   |
//! +------------------+      +------------------+
//! ```

pub mod commands;
pub mod output;

use clap::{Parser, Subcommand};

/// Shebe - BM25 Code Search Engine
///
/// A fast, full-text search engine for code repositories using BM25 ranking.
/// Index your codebase and search with keywords, phrases, or boolean queries.
#[derive(Parser, Debug)]
#[command(name = "shebe")]
#[command(author = "RHOBIMD HEALTH")]
#[command(version)]
#[command(about = "BM25 code search engine", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output (default)
    Human,
    /// JSON output for scripting
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Human
    }
}

/// Available CLI commands
///
/// Command names match MCP tool names (underscores become hyphens).
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index a repository for search
    #[command(name = "index-repository")]
    IndexRepository(commands::IndexArgs),

    /// Search indexed code with BM25 ranking
    #[command(name = "search-code")]
    SearchCode(commands::SearchArgs),

    /// Find all references to a symbol across the indexed codebase
    #[command(name = "find-references")]
    FindReferences(commands::ReferencesArgs),

    /// List all indexed sessions
    #[command(name = "list-sessions")]
    ListSessions(commands::session::ListArgs),

    /// Get detailed session information
    #[command(name = "get-session-info")]
    GetSessionInfo(commands::session::InfoArgs),

    /// Delete a session and all associated data
    #[command(name = "delete-session")]
    DeleteSession(commands::session::DeleteArgs),

    /// Re-index a session using stored repository path
    #[command(name = "reindex-session")]
    ReindexSession(commands::session::ReindexArgs),

    /// Show current configuration
    #[command(name = "show-config")]
    ShowConfig(commands::ConfigArgs),

    /// Show version and server information
    #[command(name = "get-server-info")]
    GetServerInfo(commands::InfoArgs),

    /// Generate shell completion scripts
    ///
    /// Output completion script to stdout. To install:
    ///
    ///   bash:  shebe completions bash > ~/.local/share/bash-completion/completions/shebe
    ///   zsh:   shebe completions zsh > ~/.zfunc/_shebe
    ///   fish:  shebe completions fish > ~/.config/fish/completions/shebe.fish
    Completions(commands::CompletionsArgs),
}

/// Run the CLI with the provided arguments
pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    use crate::core::config::Config;
    use crate::core::services::Services;
    use crate::core::xdg::{migrate_legacy_paths, XdgDirs};
    use std::sync::Arc;

    // Handle completions command early (doesn't need services)
    if let Commands::Completions(args) = cli.command {
        return commands::completions::execute(args);
    }

    // Initialize XDG directories
    let xdg = XdgDirs::new();
    xdg.ensure_dirs_exist()?;

    // Run migration from legacy paths (if needed)
    if let Err(e) = migrate_legacy_paths(&xdg) {
        output::print_warning(&format!("Migration issue: {e}"));
    }

    // Load configuration
    let config = Config::load()?;

    // Create services
    let services = Arc::new(Services::new(config));

    // Execute command
    match cli.command {
        Commands::IndexRepository(args) => {
            commands::index::execute(args, &services, cli.format).await
        }
        Commands::SearchCode(args) => commands::search::execute(args, &services, cli.format).await,
        Commands::FindReferences(args) => {
            commands::references::execute(args, &services, cli.format).await
        }
        Commands::ListSessions(args) => {
            commands::session::execute_list(args, &services, cli.format).await
        }
        Commands::GetSessionInfo(args) => {
            commands::session::execute_info(args, &services, cli.format).await
        }
        Commands::DeleteSession(args) => {
            commands::session::execute_delete(args, &services, cli.format).await
        }
        Commands::ReindexSession(args) => {
            commands::session::execute_reindex(args, &services, cli.format).await
        }
        Commands::ShowConfig(args) => commands::config::execute(args, &services, cli.format).await,
        Commands::GetServerInfo(args) => commands::info::execute(args, &services, cli.format).await,
        Commands::Completions(_) => unreachable!(), // Handled above
    }
}
