//! CLI test helpers
//!
//! Provides utilities for testing CLI commands including:
//! - Test repo creation with specific file content
//! - Session setup for search/reference tests
//! - Arc<Services> wrappers matching CLI execute() signatures

use shebe::core::config::Config;
use shebe::core::services::Services;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Create test services wrapped in Arc (matching CLI execute() signatures)
pub fn create_cli_test_services() -> (Arc<Services>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut config = Config::default();
    config.storage.index_dir = temp_dir.path().to_path_buf();

    let services = Arc::new(Services::new(config));
    (services, temp_dir)
}

/// Create a test repository with specified files
///
/// # Arguments
/// * `files` - Slice of (relative_path, content) tuples
///
/// # Returns
/// TempDir containing the test repository (keep alive during test)
pub fn create_test_repo(files: &[(&str, &str)]) -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp dir");
    for (path, content) in files {
        let full_path = temp.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create directories");
        }
        std::fs::write(&full_path, content).expect("Failed to write file");
    }
    temp
}

/// Index a test repository and return session ID
///
/// # Arguments
/// * `services` - Arc<Services> for indexing
/// * `repo_path` - Path to the repository
/// * `session_id` - Session ID to use
pub async fn setup_indexed_session(
    services: &Arc<Services>,
    repo_path: &Path,
    session_id: &str,
) -> String {
    // Use the storage manager's index_repository method directly
    services
        .storage
        .index_repository(
            session_id,
            repo_path,
            vec!["**/*".to_string()],
            vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
            ],
            512,  // chunk_size
            64,   // overlap
            10,   // max_file_size_mb
            true, // force
        )
        .expect("Failed to index repository");

    session_id.to_string()
}

/// Standard test files for search tests
pub fn search_test_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "src/main.rs",
            r#"fn main() {
    println!("Hello, world!");
    let config = load_config();
    run_server(config);
}

fn load_config() -> Config {
    Config::default()
}
"#,
        ),
        (
            "src/lib.rs",
            r#"pub mod server;
pub mod config;

pub use config::Config;
pub use server::run_server;
"#,
        ),
        (
            "src/server.rs",
            r#"use crate::Config;

pub fn run_server(config: Config) {
    println!("Starting server on port {}", config.port);
}

pub fn stop_server() {
    println!("Server stopped");
}
"#,
        ),
        (
            "src/config.rs",
            r#"#[derive(Default)]
pub struct Config {
    pub port: u16,
    pub host: String,
}

impl Config {
    pub fn new(port: u16, host: String) -> Self {
        Self { port, host }
    }
}
"#,
        ),
        (
            "README.md",
            "# Test Project\n\nA sample project for testing.",
        ),
    ]
}

/// Test files with symbol references for find-references tests
pub fn references_test_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "src/lib.rs",
            r#"pub mod utils;
pub mod handlers;

pub use utils::helper_function;
pub use handlers::process_request;
"#,
        ),
        (
            "src/utils.rs",
            r#"/// A helper function used across the codebase
pub fn helper_function(input: &str) -> String {
    format!("Processed: {}", input)
}

pub fn another_helper() -> i32 {
    42
}
"#,
        ),
        (
            "src/handlers.rs",
            r#"use crate::utils::helper_function;

pub fn process_request(data: &str) -> String {
    let result = helper_function(data);
    format!("Response: {}", result)
}

pub fn handle_error(msg: &str) -> String {
    helper_function(msg)
}
"#,
        ),
        (
            "src/main.rs",
            r#"mod utils;
mod handlers;

use utils::helper_function;
use handlers::process_request;

fn main() {
    let input = "test";
    let processed = helper_function(input);
    println!("{}", processed);

    let response = process_request("request");
    println!("{}", response);
}
"#,
        ),
        (
            "tests/test_utils.rs",
            r#"use my_crate::helper_function;

#[test]
fn test_helper() {
    let result = helper_function("test");
    assert!(result.contains("Processed"));
}
"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_repo() {
        let files = [("test.txt", "hello"), ("dir/nested.txt", "world")];
        let repo = create_test_repo(&files);

        assert!(repo.path().join("test.txt").exists());
        assert!(repo.path().join("dir/nested.txt").exists());

        let content = std::fs::read_to_string(repo.path().join("test.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_create_cli_test_services() {
        let (services, _temp) = create_cli_test_services();
        // Verify services are created and config is accessible
        assert!(services.config.indexing.chunk_size > 0);
    }

    #[tokio::test]
    async fn test_setup_indexed_session() {
        let (services, _storage_temp) = create_cli_test_services();
        let repo = create_test_repo(&[("file.rs", "fn test() {}")]);

        let session_id = setup_indexed_session(&services, repo.path(), "test-session").await;

        assert_eq!(session_id, "test-session");
        assert!(services.storage.session_exists("test-session"));
    }

    #[test]
    fn test_search_test_files_not_empty() {
        let files = search_test_files();
        assert!(!files.is_empty());
        assert!(files.len() >= 4);
    }

    #[test]
    fn test_references_test_files_not_empty() {
        let files = references_test_files();
        assert!(!files.is_empty());
        assert!(files.len() >= 4);
    }
}
