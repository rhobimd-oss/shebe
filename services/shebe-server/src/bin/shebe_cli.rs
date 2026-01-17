//! Shebe CLI - Command-line interface for Shebe code search
//!
//! A direct command-line interface for Shebe's search and indexing capabilities.
//! Use this for scripting, automation, or manual operations without an MCP client.
//!
//! # Examples
//!
//! ```bash
//! # Index a repository
//! shebe index /path/to/repo --session myproject
//!
//! # Search for code
//! shebe search "authentication" --session myproject
//!
//! # List sessions
//! shebe session list
//!
//! # Show configuration
//! shebe config
//! ```

use clap::Parser;
use shebe::cli::{run, Cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
