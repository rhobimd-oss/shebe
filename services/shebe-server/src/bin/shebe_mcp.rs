//! Shebe MCP (Model Context Protocol) Server
//!
//! A stdio-based MCP server that exposes Shebe's search capabilities
//! as tools for Claude Code and other MCP clients.

use shebe::config::Config;
use shebe::mcp::{McpServer, ShebeServices};
use shebe::storage::MetadataValidator;
use shebe::xdg::{migrate_legacy_paths, XdgDirs};
use std::sync::Arc;

fn init_logging() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr) // Critical: stderr not stdout
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false) // No color codes
        .compact() // Concise format
        .init();
}

/// Validate all session metadata on startup
fn validate_sessions_on_startup(services: &ShebeServices) {
    tracing::info!("Validating session metadata...");

    let validator = MetadataValidator::new(&services.storage);

    match validator.validate_all_sessions() {
        Ok(reports) => {
            let mut inconsistent_count = 0;
            let mut repaired_count = 0;

            for report in &reports {
                if !report.is_consistent {
                    inconsistent_count += 1;
                    tracing::warn!(
                        "Session '{}' has inconsistent metadata: \
                         metadata_size={}B, actual_size={}B",
                        report.session_id,
                        report.metadata_size,
                        report.actual_size
                    );

                    // Attempt auto-repair
                    match validator.auto_repair(&report.session_id) {
                        Ok(true) => {
                            repaired_count += 1;
                            tracing::info!("Auto-repaired session '{}'", report.session_id);
                        }
                        Ok(false) => {
                            tracing::debug!("Session '{}' did not need repair", report.session_id);
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to repair session '{}': {}",
                                report.session_id,
                                e
                            );
                        }
                    }
                }
            }

            if inconsistent_count > 0 {
                tracing::warn!(
                    "Found {} inconsistent session(s), repaired {}",
                    inconsistent_count,
                    repaired_count
                );
            } else {
                tracing::info!("All {} session(s) have consistent metadata", reports.len());
            }
        }
        Err(e) => {
            tracing::error!("Failed to validate sessions: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    init_logging();

    // Initialize XDG directories
    let xdg = XdgDirs::new();
    tracing::debug!("XDG directories initialized");

    // Ensure XDG directories exist
    if let Err(e) = xdg.ensure_dirs_exist() {
        eprintln!("Failed to create XDG directories: {e}");
        std::process::exit(1);
    }

    // Run migration from legacy paths (if needed)
    if let Err(e) = migrate_legacy_paths(&xdg) {
        tracing::warn!("Migration warning: {}", e);
        tracing::info!("Continuing with current paths...");
    }

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {e}");
        std::process::exit(1);
    });

    // Create services
    let services = Arc::new(ShebeServices::new(config));

    // Validate session metadata on startup
    validate_sessions_on_startup(&services);

    // Create and run MCP server
    let mut server = McpServer::new(services);

    if let Err(e) = server.run().await {
        eprintln!("MCP server error: {e}");
        std::process::exit(1);
    }
}
