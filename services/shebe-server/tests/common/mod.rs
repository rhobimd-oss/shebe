// Common test utilities and fixtures

pub mod fixtures;
pub mod helpers;

// Re-export commonly used items
// Note: These may appear unused in unit tests but are used in integration tests
#[allow(unused_imports)]
pub use fixtures::{OpenEmrData, TestRepo};
#[allow(unused_imports)]
pub use helpers::{
    assert_valid_stats, create_test_services, index_test_repository,
    index_test_repository_with_patterns,
};
