//! Tests for CLI output formatting helpers
//!
//! Tests the output formatting utilities:
//! - Byte formatting (KB, MB, GB)
//! - Duration formatting (ms, s, m)
//! - Relative time formatting (just now, minutes ago, hours ago, days ago)
//! - Color helpers (respects NO_COLOR)
//! - Print helpers (print_success, print_warning, print_error)

use chrono::{Duration, Utc};
use shebe::cli::output::{format_bytes, format_duration, format_relative_time};

// =============================================================================
// format_bytes tests
// =============================================================================

/// Test byte formatting with various sizes
#[test]
fn test_format_bytes_various_sizes() {
    // Bytes (under 1 KB)
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1), "1 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1023), "1023 B");

    // Kilobytes
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(10240), "10.0 KB");
    assert_eq!(format_bytes(102400), "100.0 KB");

    // Megabytes
    assert_eq!(format_bytes(1048576), "1.0 MB");
    assert_eq!(format_bytes(1572864), "1.5 MB");
    assert_eq!(format_bytes(10485760), "10.0 MB");
    assert_eq!(format_bytes(104857600), "100.0 MB");

    // Gigabytes
    assert_eq!(format_bytes(1073741824), "1.0 GB");
    assert_eq!(format_bytes(1610612736), "1.5 GB");
    assert_eq!(format_bytes(10737418240), "10.0 GB");
}

/// Test byte formatting edge cases
#[test]
fn test_format_bytes_edge_cases() {
    // Boundary values
    assert_eq!(format_bytes(1024 - 1), "1023 B"); // Just under 1 KB
    assert_eq!(format_bytes(1024), "1.0 KB"); // Exactly 1 KB
    assert_eq!(format_bytes(1048576 - 1), "1024.0 KB"); // Just under 1 MB
    assert_eq!(format_bytes(1048576), "1.0 MB"); // Exactly 1 MB
    assert_eq!(format_bytes(1073741824 - 1), "1024.0 MB"); // Just under 1 GB
    assert_eq!(format_bytes(1073741824), "1.0 GB"); // Exactly 1 GB
}

// =============================================================================
// format_duration tests
// =============================================================================

/// Test duration formatting with various times
#[test]
fn test_format_duration_various_times() {
    // Milliseconds (under 1 second)
    assert_eq!(format_duration(0.001), "1ms");
    assert_eq!(format_duration(0.1), "100ms");
    assert_eq!(format_duration(0.5), "500ms");
    assert_eq!(format_duration(0.999), "999ms");

    // Seconds
    assert_eq!(format_duration(1.0), "1.00s");
    assert_eq!(format_duration(1.5), "1.50s");
    assert_eq!(format_duration(30.0), "30.00s");
    assert_eq!(format_duration(59.99), "59.99s");

    // Minutes
    assert_eq!(format_duration(60.0), "1m 0.0s");
    assert_eq!(format_duration(90.0), "1m 30.0s");
    assert_eq!(format_duration(125.5), "2m 5.5s");
}

/// Test duration formatting edge cases
#[test]
fn test_format_duration_edge_cases() {
    // Zero
    assert_eq!(format_duration(0.0), "0ms");

    // Very small values
    assert_eq!(format_duration(0.0001), "0ms");
    assert_eq!(format_duration(0.0005), "0ms"); // Rounds to 0ms

    // Boundary at 1 second
    assert!(format_duration(0.999).ends_with("ms"));
    assert!(format_duration(1.0).ends_with("s"));

    // Boundary at 60 seconds
    assert!(!format_duration(59.9).contains("m"));
    assert!(format_duration(60.0).contains("m"));
}

// =============================================================================
// format_relative_time tests
// =============================================================================

/// Test relative time formatting - just now
#[test]
fn test_format_relative_time_just_now() {
    let now = Utc::now();
    assert_eq!(format_relative_time(&now), "just now");

    let ten_seconds_ago = now - Duration::seconds(10);
    assert_eq!(format_relative_time(&ten_seconds_ago), "just now");

    let fifty_nine_seconds_ago = now - Duration::seconds(59);
    assert_eq!(format_relative_time(&fifty_nine_seconds_ago), "just now");
}

/// Test relative time formatting - minutes ago
#[test]
fn test_format_relative_time_minutes() {
    let now = Utc::now();

    let one_minute_ago = now - Duration::minutes(1);
    assert_eq!(format_relative_time(&one_minute_ago), "1m ago");

    let five_minutes_ago = now - Duration::minutes(5);
    assert_eq!(format_relative_time(&five_minutes_ago), "5m ago");

    let fifty_nine_minutes_ago = now - Duration::minutes(59);
    assert_eq!(format_relative_time(&fifty_nine_minutes_ago), "59m ago");
}

/// Test relative time formatting - hours ago
#[test]
fn test_format_relative_time_hours() {
    let now = Utc::now();

    let one_hour_ago = now - Duration::hours(1);
    assert_eq!(format_relative_time(&one_hour_ago), "1h ago");

    let five_hours_ago = now - Duration::hours(5);
    assert_eq!(format_relative_time(&five_hours_ago), "5h ago");

    let twenty_three_hours_ago = now - Duration::hours(23);
    assert_eq!(format_relative_time(&twenty_three_hours_ago), "23h ago");
}

/// Test relative time formatting - days ago
#[test]
fn test_format_relative_time_days() {
    let now = Utc::now();

    let one_day_ago = now - Duration::days(1);
    assert_eq!(format_relative_time(&one_day_ago), "1d ago");

    let seven_days_ago = now - Duration::days(7);
    assert_eq!(format_relative_time(&seven_days_ago), "7d ago");

    let thirty_days_ago = now - Duration::days(30);
    assert_eq!(format_relative_time(&thirty_days_ago), "30d ago");
}

/// Test relative time formatting - future time
#[test]
fn test_format_relative_time_future() {
    let now = Utc::now();
    let future = now + Duration::hours(1);
    assert_eq!(format_relative_time(&future), "in the future");
}

// =============================================================================
// Color helper tests
// Note: These test that colors don't break output, not visual appearance.
// The `colored` crate respects NO_COLOR env var automatically.
// =============================================================================

/// Test that color functions return valid strings
#[test]
fn test_colors_return_valid_strings() {
    use shebe::cli::output::colors;

    // All color functions should return non-empty strings
    let label = colors::label("test");
    assert!(!label.to_string().is_empty());

    let session = colors::session_id("my-session");
    assert!(!session.to_string().is_empty());

    let path = colors::file_path("/path/to/file");
    assert!(!path.to_string().is_empty());

    let num = colors::number("42");
    assert!(!num.to_string().is_empty());

    let success = colors::success("done");
    assert!(!success.to_string().is_empty());

    let warn = colors::warning("caution");
    assert!(!warn.to_string().is_empty());

    let err = colors::error("failed");
    assert!(!err.to_string().is_empty());

    let dim = colors::dim("secondary");
    assert!(!dim.to_string().is_empty());

    let sc = colors::score("0.85");
    assert!(!sc.to_string().is_empty());

    let rank = colors::rank("#1");
    assert!(!rank.to_string().is_empty());
}

/// Test that colors preserve the original text
#[test]
fn test_colors_preserve_text() {
    use shebe::cli::output::colors;

    // The original text should be present in the output
    let label = colors::label("important");
    assert!(label.to_string().contains("important"));

    let session = colors::session_id("test-session-123");
    assert!(session.to_string().contains("test-session-123"));

    let path = colors::file_path("src/main.rs");
    assert!(path.to_string().contains("src/main.rs"));
}

// =============================================================================
// format_bytes_colored and format_duration_colored tests
// =============================================================================

/// Test colored formatting functions
#[test]
fn test_colored_formatting_functions() {
    use shebe::cli::output::{format_bytes_colored, format_duration_colored};

    // These should return strings containing the formatted value
    let bytes = format_bytes_colored(1024);
    assert!(bytes.contains("1.0 KB"));

    let duration = format_duration_colored(1.5);
    assert!(duration.contains("1.50s"));
}

/// Test relative time colored formatting
#[test]
fn test_format_relative_time_colored() {
    use shebe::cli::output::format_relative_time_colored;

    let now = Utc::now();
    let one_hour_ago = now - Duration::hours(1);

    let colored = format_relative_time_colored(&one_hour_ago);
    assert!(colored.contains("1h ago"));
}
