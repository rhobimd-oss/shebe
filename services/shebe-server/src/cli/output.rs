//! Output formatting for CLI commands
//!
//! Provides utilities for formatting command output in human-readable
//! or JSON formats. Supports colored output (respects NO_COLOR env var).

use crate::cli::OutputFormat;

/// Color scheme for CLI output
pub mod colors {
    use colored::{ColoredString, Colorize};

    /// Style for labels/headers
    pub fn label(s: &str) -> ColoredString {
        s.bold()
    }

    /// Style for session IDs
    pub fn session_id(s: &str) -> ColoredString {
        s.cyan()
    }

    /// Style for file paths
    pub fn file_path(s: &str) -> ColoredString {
        s.blue()
    }

    /// Style for numbers/counts
    pub fn number(s: &str) -> ColoredString {
        s.yellow()
    }

    /// Style for success messages
    pub fn success(s: &str) -> ColoredString {
        s.green()
    }

    /// Style for warning messages
    pub fn warning(s: &str) -> ColoredString {
        s.yellow()
    }

    /// Style for error messages
    pub fn error(s: &str) -> ColoredString {
        s.red().bold()
    }

    /// Style for dim/secondary text
    pub fn dim(s: &str) -> ColoredString {
        s.dimmed()
    }

    /// Style for search scores
    pub fn score(s: &str) -> ColoredString {
        s.magenta()
    }

    /// Style for rank numbers
    pub fn rank(s: &str) -> ColoredString {
        s.green().bold()
    }
}

/// Format bytes into human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    let gb_val = bytes as f64 / GB as f64;
    let mb_val = bytes as f64 / MB as f64;
    let kb_val = bytes as f64 / KB as f64;

    if bytes >= GB {
        format!("{gb_val:.1} GB")
    } else if bytes >= MB {
        format!("{mb_val:.1} MB")
    } else if bytes >= KB {
        format!("{kb_val:.1} KB")
    } else {
        format!("{bytes} B")
    }
}

/// Format bytes with color
pub fn format_bytes_colored(bytes: u64) -> String {
    format!("{}", colors::number(&format_bytes(bytes)))
}

/// Format duration into human-readable string
pub fn format_duration(secs: f64) -> String {
    if secs >= 60.0 {
        let mins = (secs / 60.0).floor();
        let remaining_secs = secs - (mins * 60.0);
        format!("{mins:.0}m {remaining_secs:.1}s")
    } else if secs >= 1.0 {
        format!("{secs:.2}s")
    } else {
        let ms = secs * 1000.0;
        format!("{ms:.0}ms")
    }
}

/// Format duration with color
pub fn format_duration_colored(secs: f64) -> String {
    format!("{}", colors::number(&format_duration(secs)))
}

/// Format relative time (e.g., "2h ago", "3d ago")
pub fn format_relative_time(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    let secs = duration.num_seconds();
    if secs < 0 {
        return "in the future".to_string();
    }

    let mins = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if days > 0 {
        format!("{days}d ago")
    } else if hours > 0 {
        format!("{hours}h ago")
    } else if mins > 0 {
        format!("{mins}m ago")
    } else {
        "just now".to_string()
    }
}

/// Format relative time with color (dim for older items)
pub fn format_relative_time_colored(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    format!("{}", colors::dim(&format_relative_time(timestamp)))
}

/// Print output based on format
pub fn print_output<T: serde::Serialize>(data: &T, format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            // Human format should be handled by the caller
            // This is a fallback that just prints JSON
            if let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{json}");
            }
        }
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{json}");
            }
        }
    }
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{}", colors::success(message));
}

/// Print a warning message
pub fn print_warning(message: &str) {
    eprintln!("{}: {}", colors::warning("Warning"), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{}: {}", colors::error("Error"), message);
}

/// Print a header/title
pub fn print_header(title: &str) {
    println!("{}", colors::label(title));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.5), "500ms");
        assert_eq!(format_duration(1.5), "1.50s");
        assert_eq!(format_duration(65.5), "1m 5.5s");
    }
}
