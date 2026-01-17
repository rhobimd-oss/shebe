//! Config command - show current configuration

use crate::cli::OutputFormat;
use crate::core::services::Services;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

/// Arguments for the config command
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Show all configuration including defaults
    #[arg(long, short = 'a')]
    pub all: bool,
}

/// Configuration response
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub data_dir: String,
    pub indexing: IndexingConfig,
    pub search: SearchConfig,
}

#[derive(Debug, Serialize)]
pub struct IndexingConfig {
    pub chunk_size: usize,
    pub overlap: usize,
    pub default_include: Vec<String>,
    pub default_exclude: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchConfig {
    pub default_k: usize,
    pub max_k: usize,
}

/// Execute the config command
pub async fn execute(
    _args: ConfigArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = &services.config;

    // Get data directory from XDG
    let xdg = crate::core::xdg::XdgDirs::new();
    let data_dir = xdg.state_dir.to_string_lossy().into_owned();

    let response = ConfigResponse {
        data_dir,
        indexing: IndexingConfig {
            chunk_size: config.indexing.chunk_size,
            overlap: config.indexing.overlap,
            default_include: config.indexing.include_patterns.clone(),
            default_exclude: config.indexing.exclude_patterns.clone(),
        },
        search: SearchConfig {
            default_k: config.search.default_k,
            max_k: config.search.max_k,
        },
    };

    match format {
        OutputFormat::Human => {
            println!("Configuration:");
            println!("  data_dir: {}", response.data_dir);
            println!("  indexing:");
            println!("    chunk_size: {}", response.indexing.chunk_size);
            println!("    overlap: {}", response.indexing.overlap);
            println!(
                "    default_include: {:?}",
                response.indexing.default_include
            );
            println!(
                "    default_exclude: {:?}",
                response.indexing.default_exclude
            );
            println!("  search:");
            println!("    default_k: {}", response.search.default_k);
            println!("    max_k: {}", response.search.max_k);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}
