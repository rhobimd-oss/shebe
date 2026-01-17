//! Session commands - list, info, delete, reindex sessions
//!
//! These commands are exposed as top-level CLI commands matching MCP tool names:
//! - `list-sessions` (MCP: list_sessions)
//! - `get-session-info` (MCP: get_session_info)
//! - `delete-session` (MCP: delete_session)
//! - `reindex-session` (MCP: reindex_session)

use crate::cli::output::{colors, format_bytes, format_relative_time};
use crate::cli::OutputFormat;
use crate::core::services::Services;
use clap::Args;
use serde::Serialize;
use std::io::{self, Write};
use std::sync::Arc;

/// Arguments for session list
#[derive(Args, Debug)]
pub struct ListArgs {}

/// Arguments for session info
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Session ID
    pub session: String,
}

/// Arguments for session delete
#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Session ID
    pub session: String,

    /// Skip confirmation prompt
    #[arg(long, short = 'f')]
    pub force: bool,
}

/// Arguments for session reindex
#[derive(Args, Debug)]
pub struct ReindexArgs {
    /// Session ID
    pub session: String,

    /// Override chunk size
    #[arg(long)]
    pub chunk_size: Option<usize>,

    /// Override overlap
    #[arg(long)]
    pub overlap: Option<usize>,

    /// Force re-index even if config unchanged
    #[arg(long, short = 'f')]
    pub force: bool,
}

/// Session list item
#[derive(Debug, Serialize)]
pub struct SessionListItem {
    pub id: String,
    pub files: usize,
    pub chunks: usize,
    pub size_bytes: u64,
    pub indexed_at: String,
}

/// Session list response
#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub count: usize,
    pub sessions: Vec<SessionListItem>,
}

/// Detailed session info
#[derive(Debug, Serialize)]
pub struct SessionInfoResponse {
    pub id: String,
    pub repository_path: String,
    pub files: usize,
    pub chunks: usize,
    pub size_bytes: u64,
    pub indexed_at: String,
    pub config: SessionConfigInfo,
}

#[derive(Debug, Serialize)]
pub struct SessionConfigInfo {
    pub chunk_size: usize,
    pub overlap: usize,
}

/// Execute list-sessions command
pub async fn execute_list(
    _args: ListArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let sessions = services.storage.list_sessions()?;

    let response = SessionListResponse {
        count: sessions.len(),
        sessions: sessions
            .iter()
            .map(|s| SessionListItem {
                id: s.id.clone(),
                files: s.files_indexed,
                chunks: s.chunks_created,
                size_bytes: s.index_size_bytes,
                indexed_at: s.last_indexed_at.to_rfc3339(),
            })
            .collect(),
    };

    match format {
        OutputFormat::Human => {
            if response.sessions.is_empty() {
                println!(
                    "No sessions found. Run '{}' to index a repository.",
                    colors::label("shebe index-repository <path> -s <session>")
                );
            } else {
                println!(
                    "{} ({}):",
                    colors::label("Sessions"),
                    colors::number(&response.count.to_string())
                );
                for session in &response.sessions {
                    // Parse the timestamp for relative time
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&session.indexed_at) {
                        let utc = dt.with_timezone(&chrono::Utc);
                        println!(
                            "  {:<20} {:>6} files  {:>8} chunks  {:>10}  {}",
                            colors::session_id(&session.id),
                            colors::number(&session.files.to_string()),
                            colors::number(&session.chunks.to_string()),
                            colors::number(&format_bytes(session.size_bytes)),
                            colors::dim(&format_relative_time(&utc))
                        );
                    } else {
                        println!(
                            "  {:<20} {:>6} files  {:>8} chunks  {:>10}",
                            colors::session_id(&session.id),
                            colors::number(&session.files.to_string()),
                            colors::number(&session.chunks.to_string()),
                            colors::number(&format_bytes(session.size_bytes))
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

/// Execute get-session-info command
pub async fn execute_info(
    args: InfoArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // get_session_metadata returns Result<SessionMetadata>, throws SessionNotFound if not found
    let metadata = services
        .storage
        .get_session_metadata(&args.session)
        .map_err(|_| {
            format!(
                "Session '{}' not found. Run 'shebe list-sessions' to see available sessions.",
                args.session
            )
        })?;

    let response = SessionInfoResponse {
        id: metadata.id.clone(),
        repository_path: metadata.repository_path.to_string_lossy().into_owned(),
        files: metadata.files_indexed,
        chunks: metadata.chunks_created,
        size_bytes: metadata.index_size_bytes,
        indexed_at: metadata.last_indexed_at.to_rfc3339(),
        config: SessionConfigInfo {
            chunk_size: metadata.config.chunk_size,
            overlap: metadata.config.overlap,
        },
    };

    match format {
        OutputFormat::Human => {
            println!(
                "{}: {}",
                colors::label("Session"),
                colors::session_id(&response.id)
            );
            println!(
                "  {}: {}",
                colors::label("Repository"),
                colors::file_path(&response.repository_path)
            );
            println!(
                "  {}: {}",
                colors::label("Files"),
                colors::number(&response.files.to_string())
            );
            println!(
                "  {}: {}",
                colors::label("Chunks"),
                colors::number(&response.chunks.to_string())
            );
            println!(
                "  {}: {}",
                colors::label("Size"),
                colors::number(&format_bytes(response.size_bytes))
            );
            println!(
                "  {}: {}",
                colors::label("Indexed"),
                colors::dim(&response.indexed_at)
            );
            println!("  {}:", colors::label("Config"));
            println!(
                "    chunk_size: {}",
                colors::number(&response.config.chunk_size.to_string())
            );
            println!(
                "    overlap: {}",
                colors::number(&response.config.overlap.to_string())
            );
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

/// Execute delete-session command
pub async fn execute_delete(
    args: DeleteArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if session exists (returns bool, not Result)
    if !services.storage.session_exists(&args.session) {
        return Err(format!(
            "Session '{}' not found. Run 'shebe list-sessions' to see available sessions.",
            args.session
        )
        .into());
    }

    // Confirmation prompt unless --force
    if !args.force {
        print!(
            "Delete session '{}'? [y/N] ",
            colors::session_id(&args.session)
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", colors::dim("Cancelled."));
            return Ok(());
        }
    }

    services.storage.delete_session(&args.session)?;

    match format {
        OutputFormat::Human => {
            println!(
                "{} session '{}'",
                colors::success("Deleted"),
                colors::session_id(&args.session)
            );
        }
        OutputFormat::Json => {
            let response = serde_json::json!({
                "deleted": true,
                "session": args.session
            });
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

/// Execute reindex-session command
pub async fn execute_reindex(
    args: ReindexArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get existing session metadata
    let metadata = services
        .storage
        .get_session_metadata(&args.session)
        .map_err(|_| {
            format!(
                "Session '{}' not found. Run 'shebe list-sessions' to see available sessions.",
                args.session
            )
        })?;

    // Get repository path (it's a PathBuf, not Option)
    let path = metadata.repository_path.clone();
    if !path.exists() {
        return Err(format!(
            "Repository path '{}' no longer exists. \
             Delete the session with 'shebe delete-session {}' and re-index from the new location.",
            path.display(),
            args.session
        )
        .into());
    }

    // Build config with overrides
    let chunk_size = args.chunk_size.unwrap_or(metadata.config.chunk_size);
    let overlap = args.overlap.unwrap_or(metadata.config.overlap);
    let include_patterns = metadata.config.include_patterns.clone();
    let exclude_patterns = metadata.config.exclude_patterns.clone();

    // Check if config changed
    let config_changed = args.chunk_size.is_some() || args.overlap.is_some();
    if !args.force && !config_changed {
        return Err("No configuration changes. Use --force to re-index anyway, \
             or specify --chunk-size or --overlap to change settings."
            .into());
    }

    // Delete existing session
    services.storage.delete_session(&args.session)?;

    // Re-index
    if format == OutputFormat::Human {
        eprintln!(
            "Re-indexing '{}' from {}...",
            colors::session_id(&args.session),
            colors::file_path(&path.display().to_string())
        );
    }

    // Call index_repository with all individual arguments
    let stats = services.storage.index_repository(
        &args.session,
        &path,
        include_patterns,
        exclude_patterns,
        chunk_size,
        overlap,
        services.config.indexing.max_file_size_mb,
        true, // force=true since we already deleted the session
    )?;

    let duration_secs = stats.duration_ms as f64 / 1000.0;

    match format {
        OutputFormat::Human => {
            println!(
                "{} {} files ({} chunks) in {:.2}s",
                colors::success("Indexed"),
                colors::number(&stats.files_indexed.to_string()),
                colors::number(&stats.chunks_created.to_string()),
                duration_secs
            );
        }
        OutputFormat::Json => {
            let response = serde_json::json!({
                "session": args.session,
                "repository_path": path.display().to_string(),
                "files_indexed": stats.files_indexed,
                "chunks_created": stats.chunks_created,
                "duration_secs": duration_secs
            });
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}
