//! Search command - search indexed code

use crate::cli::output::colors;
use crate::cli::OutputFormat;
use crate::core::services::Services;
use crate::core::types::SearchRequest;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

/// Arguments for the search command
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query (supports boolean operators: AND, OR, NOT)
    pub query: String,

    /// Session ID to search
    #[arg(long, short = 's')]
    pub session: String,

    /// Maximum number of results (1-100)
    #[arg(long, short = 'k', default_value = "10")]
    pub limit: usize,

    /// Only show file paths (no content)
    #[arg(long)]
    pub files_only: bool,
}

/// Search result item
#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub rank: usize,
    pub file: String,
    pub score: f32,
    pub chunk_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Search response
#[derive(Debug, Serialize)]
pub struct SearchResponseOutput {
    pub query: String,
    pub session: String,
    pub total_results: usize,
    pub results: Vec<SearchResultItem>,
}

/// Execute the search command
pub async fn execute(
    args: SearchArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate session exists (returns bool, not Result)
    if !services.storage.session_exists(&args.session) {
        return Err(format!(
            "Session '{}' not found. Run 'shebe list-sessions' to see available sessions.",
            args.session
        )
        .into());
    }

    // Validate limit
    let limit = args.limit.clamp(1, 100);

    // Create search request
    let request = SearchRequest {
        query: args.query.clone(),
        session: args.session.clone(),
        k: Some(limit),
    };

    // Perform search
    let response = services.search.search(request)?;

    let output = SearchResponseOutput {
        query: args.query.clone(),
        session: args.session.clone(),
        total_results: response.count,
        results: response
            .results
            .iter()
            .enumerate()
            .map(|(i, r)| SearchResultItem {
                rank: i + 1,
                file: r.file_path.clone(),
                score: r.score,
                chunk_index: r.chunk_index,
                text: if args.files_only {
                    None
                } else {
                    Some(r.text.clone())
                },
            })
            .collect(),
    };

    match format {
        OutputFormat::Human => {
            if output.results.is_empty() {
                println!(
                    "No results found for '{}' in session '{}'",
                    colors::label(&args.query),
                    colors::session_id(&output.session)
                );
            } else {
                println!(
                    "Found {} result(s) in '{}':\n",
                    colors::number(&output.total_results.to_string()),
                    colors::session_id(&output.session)
                );

                for result in &output.results {
                    if args.files_only {
                        println!("{}", colors::file_path(&result.file));
                    } else {
                        println!(
                            "[{}] {} {}",
                            colors::rank(&result.rank.to_string()),
                            colors::file_path(&result.file),
                            colors::dim(&format!("(score: {:.2})", result.score))
                        );
                        if let Some(text) = &result.text {
                            // Indent and truncate text for display
                            let lines: Vec<&str> = text.lines().take(5).collect();
                            for line in lines {
                                let truncated = if line.len() > 100 {
                                    format!("{}...", &line[..97])
                                } else {
                                    line.to_string()
                                };
                                println!("    {}", colors::dim(&truncated));
                            }
                        }
                        println!();
                    }
                }
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
