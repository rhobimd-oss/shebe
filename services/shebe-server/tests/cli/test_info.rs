//! Tests for get-server-info and show-config CLI commands
//!
//! These are simple commands that display static/config information.

use crate::cli::test_helpers::create_cli_test_services;
use shebe::cli::commands::config::{execute as execute_config, ConfigArgs};
use shebe::cli::commands::info::{execute as execute_info, InfoArgs};
use shebe::cli::OutputFormat;

// =============================================================================
// get-server-info tests
// =============================================================================

/// Test getting server info (human format)
#[tokio::test]
async fn test_server_info_human() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = InfoArgs { detailed: false };
    let result = execute_info(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Get server info should succeed");
}

/// Test getting server info (JSON format)
#[tokio::test]
async fn test_server_info_json() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = InfoArgs { detailed: false };
    let result = execute_info(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "Get server info (JSON) should succeed");
}

/// Test getting detailed server info
#[tokio::test]
async fn test_server_info_detailed() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = InfoArgs { detailed: true };
    let result = execute_info(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Get detailed server info should succeed");
}

// =============================================================================
// show-config tests
// =============================================================================

/// Test showing config (human format)
#[tokio::test]
async fn test_show_config_human() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ConfigArgs { all: false };
    let result = execute_config(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Show config should succeed");
}

/// Test showing config (JSON format)
#[tokio::test]
async fn test_show_config_json() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ConfigArgs { all: false };
    let result = execute_config(args, &services, OutputFormat::Json).await;
    assert!(result.is_ok(), "Show config (JSON) should succeed");
}

/// Test showing all config
#[tokio::test]
async fn test_show_config_all() {
    let (services, _storage_temp) = create_cli_test_services();

    let args = ConfigArgs { all: true };
    let result = execute_config(args, &services, OutputFormat::Human).await;
    assert!(result.is_ok(), "Show all config should succeed");
}
