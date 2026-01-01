//! Info command - show version and server information

use crate::cli::OutputFormat;
use crate::core::services::Services;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

/// Arguments for the info command
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Show detailed information
    #[arg(long, short = 'd')]
    pub detailed: bool,
}

/// Server information response
#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub protocol: String,
    pub tools: u32,
    pub data_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<u32>,
}

/// Execute the info command
pub async fn execute(
    args: InfoArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get data directory from XDG
    let xdg = crate::core::xdg::XdgDirs::new();
    let data_dir = xdg.state_dir.to_string_lossy().into_owned();

    let sessions = if args.detailed {
        Some(services.storage.list_sessions()?.len() as u32)
    } else {
        None
    };

    let info = InfoResponse {
        name: "shebe".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol: "MCP 2024-11-05".to_string(),
        tools: 14,
        data_dir,
        sessions,
    };

    match format {
        OutputFormat::Human => {
            println!("shebe {}", info.version);
            println!("Protocol: {}", info.protocol);
            println!("Tools: {}", info.tools);
            println!("Data: {}", info.data_dir);
            if let Some(count) = info.sessions {
                println!("Sessions: {count}");
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
    }

    Ok(())
}
