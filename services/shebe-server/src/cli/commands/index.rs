//! Index command - index a repository for search

use crate::cli::output::{colors, format_duration};
use crate::cli::OutputFormat;
use crate::core::services::Services;
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

/// Arguments for the index command
#[derive(Args, Debug)]
pub struct IndexArgs {
    /// Path to the repository to index
    pub path: PathBuf,

    /// Session ID for the index
    #[arg(long, short = 's')]
    pub session: String,

    /// Characters per chunk (100-2000)
    #[arg(long, default_value = "512")]
    pub chunk_size: usize,

    /// Overlap between chunks (0-500)
    #[arg(long, default_value = "64")]
    pub overlap: usize,

    /// Glob patterns to include (can be specified multiple times)
    #[arg(long, short = 'i')]
    pub include: Vec<String>,

    /// Glob patterns to exclude (can be specified multiple times)
    #[arg(long, short = 'e')]
    pub exclude: Vec<String>,

    /// Force re-index if session exists
    #[arg(long, short = 'f')]
    pub force: bool,

    /// Suppress progress output
    #[arg(long, short = 'q')]
    pub quiet: bool,
}

/// Indexing result response
#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub session: String,
    pub path: String,
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub duration_secs: f64,
    pub throughput_files_per_sec: f64,
}

/// Execute the index command
pub async fn execute(
    args: IndexArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate path
    let path = args.path.canonicalize().map_err(|e| {
        format!(
            "Invalid path '{}': {}. Make sure the path exists and is accessible.",
            args.path.display(),
            e
        )
    })?;

    if !path.is_dir() {
        return Err(format!(
            "Path '{}' is not a directory. Shebe can only index directories, not individual files.",
            path.display()
        )
        .into());
    }

    // Validate session ID
    if args.session.is_empty() {
        return Err(
            "Session ID cannot be empty. Provide a name like 'myproject' or 'backend'.".into(),
        );
    }
    if args.session.len() > 64 {
        return Err(format!(
            "Session ID '{}' is too long ({} chars). Maximum length is 64 characters.",
            args.session,
            args.session.len()
        )
        .into());
    }

    // Validate session ID format
    if !args
        .session
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Session ID '{}' contains invalid characters. \
             Use only letters, numbers, hyphens and underscores.",
            args.session
        )
        .into());
    }

    // Check if session exists (returns bool, not Result)
    let session_exists = services.storage.session_exists(&args.session);
    if session_exists && !args.force {
        return Err(format!(
            "Session '{}' already exists. Use --force to re-index, \
             or choose a different session name.",
            args.session
        )
        .into());
    }

    // Validate chunk size
    if args.chunk_size < 100 || args.chunk_size > 2000 {
        return Err(format!(
            "Chunk size {} is out of range. Valid range is 100-2000 characters.",
            args.chunk_size
        )
        .into());
    }

    // Validate overlap
    if args.overlap > 500 {
        return Err(format!(
            "Overlap {} is too large. Maximum is 500 characters.",
            args.overlap
        )
        .into());
    }

    if args.overlap >= args.chunk_size {
        return Err(format!(
            "Overlap ({}) must be less than chunk size ({}).",
            args.overlap, args.chunk_size
        )
        .into());
    }

    // Build configuration
    let include_patterns = if args.include.is_empty() {
        services.config.indexing.include_patterns.clone()
    } else {
        args.include
    };

    let exclude_patterns = if args.exclude.is_empty() {
        services.config.indexing.exclude_patterns.clone()
    } else {
        args.exclude
    };

    // Index the repository
    if !args.quiet && format == OutputFormat::Human {
        eprintln!(
            "Indexing {} as '{}'...",
            colors::file_path(&path.display().to_string()),
            colors::session_id(&args.session)
        );
    }

    // Call index_repository with all individual arguments
    let stats = services.storage.index_repository(
        &args.session,
        &path,
        include_patterns,
        exclude_patterns,
        args.chunk_size,
        args.overlap,
        services.config.indexing.max_file_size_mb,
        args.force,
    )?;

    let duration_secs = stats.duration_ms as f64 / 1000.0;
    let throughput = if duration_secs > 0.0 {
        stats.files_indexed as f64 / duration_secs
    } else {
        0.0
    };

    let response = IndexResponse {
        session: args.session,
        path: path.to_string_lossy().into_owned(),
        files_indexed: stats.files_indexed,
        chunks_created: stats.chunks_created,
        duration_secs,
        throughput_files_per_sec: throughput,
    };

    match format {
        OutputFormat::Human => {
            println!(
                "{} {} files ({} chunks) in {}",
                colors::success("Indexed"),
                colors::number(&response.files_indexed.to_string()),
                colors::number(&response.chunks_created.to_string()),
                colors::number(&format_duration(response.duration_secs))
            );
            println!(
                "Throughput: {} files/sec",
                colors::number(&format!("{:.0}", response.throughput_files_per_sec))
            );
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}
