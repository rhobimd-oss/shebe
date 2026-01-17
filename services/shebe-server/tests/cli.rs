//! CLI adapter integration tests
//!
//! Tests for CLI command handlers. These tests call the execute() functions
//! directly with test services, avoiding the complexity of E2E binary spawning.
//!
//! Test organization mirrors the CLI commands:
//! - search: search-code command
//! - session: list/info/delete/reindex commands
//! - index: index-repository command
//! - references: find-references command
//! - config: show-config command
//! - info: get-server-info command
//! - output: output formatting helpers

mod common;

// CLI submodules - tests/cli/ directory
mod cli {
    pub mod test_helpers;
    pub mod test_index;
    pub mod test_info;
    pub mod test_output;
    pub mod test_references;
    pub mod test_search;
    pub mod test_session;
}
