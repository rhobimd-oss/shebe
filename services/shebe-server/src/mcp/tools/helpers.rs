//! Helper functions for MCP tools

use chrono::{DateTime, Utc};

/// Format a timestamp as human-readable relative time.
///
/// # Examples
///
/// - "1 day ago"
/// - "3 hours ago"
/// - "45 minutes ago"
/// - "just now"
pub fn format_time_ago(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    let days = duration.num_days();
    if days > 0 {
        return if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{days} days ago")
        };
    }

    let hours = duration.num_hours();
    if hours > 0 {
        return if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{hours} hours ago")
        };
    }

    let minutes = duration.num_minutes();
    if minutes > 0 {
        return if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{minutes} minutes ago")
        };
    }

    "just now".to_string()
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Detect programming language from file extension
pub fn detect_language(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("jsx") => "javascript",
        Some("ts") => "typescript",
        Some("tsx") => "typescript",
        Some("java") => "java",
        Some("go") => "go",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") => "cpp",
        Some("c") | Some("h") => "c",
        Some("php") => "php",
        Some("rb") => "ruby",
        Some("sh") | Some("bash") => "bash",
        Some("sql") => "sql",
        Some("md") => "markdown",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("xml") => "xml",
        Some("html") | Some("htm") => "html",
        Some("css") => "css",
        Some("scss") | Some("sass") => "scss",
        Some("swift") => "swift",
        Some("kt") | Some("kts") => "kotlin",
        Some("cs") => "csharp",
        Some("ex") | Some("exs") => "elixir",
        Some("erl") | Some("hrl") => "erlang",
        Some("hs") => "haskell",
        Some("scala") | Some("sc") => "scala",
        Some("clj") | Some("cljs") | Some("cljc") => "clojure",
        Some("vim") => "vim",
        Some("lua") => "lua",
        Some("pl") | Some("pm") => "perl",
        Some("r") => "r",
        Some("jl") => "julia",
        _ => "",
    }
}

/// Truncate text if it exceeds max length
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    // Truncate at character boundary
    let truncated: String = text.chars().take(max_chars).collect();
    format!(
        "{}...\n\n[Truncated {} chars]",
        truncated,
        text.len() - max_chars
    )
}

/// Convert byte offset to 1-based line number.
///
/// Shebe uses character-based chunking (UTF-8 safe) but stores byte offsets
/// in Tantivy. The `start_offset` and `end_offset` fields in SearchResult
/// are byte positions, so we count newline bytes to determine line number.
///
/// # Arguments
///
/// * `file_content` - The full file content as a string
/// * `byte_offset` - Byte offset position in the file
///
/// # Returns
///
/// 1-based line number where the byte offset falls
pub fn byte_offset_to_line_number(file_content: &str, byte_offset: usize) -> usize {
    let safe_offset = byte_offset.min(file_content.len());
    file_content[..safe_offset]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

/// Extract context lines around a given line number.
///
/// Returns formatted lines with line numbers, suitable for displaying
/// code context in search results or references.
///
/// # Arguments
///
/// * `file_content` - The full file content as a string
/// * `line_number` - 1-based line number to center context around
/// * `context` - Number of lines to include before and after
///
/// # Returns
///
/// Formatted string with line numbers and content, e.g.:
/// ```text
///   10 | fn foo() {
///   11 |     bar()
///   12 | }
/// ```
pub fn extract_context_lines(file_content: &str, line_number: usize, context: usize) -> String {
    let lines: Vec<&str> = file_content.lines().collect();
    let total_lines = lines.len();

    if total_lines == 0 || line_number == 0 {
        return String::new();
    }

    // Calculate range (1-based line_number to 0-based index)
    let start_idx = line_number.saturating_sub(context + 1);
    let end_idx = (line_number + context).min(total_lines);

    lines[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let actual_line = start_idx + i + 1;
            let truncated = truncate_line(line, 120);
            format!("{actual_line:4} | {truncated}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Truncate a single line if it exceeds max length (for context display)
fn truncate_line(line: &str, max_len: usize) -> String {
    if line.len() <= max_len {
        line.to_string()
    } else {
        // Find a safe UTF-8 boundary
        let mut end = max_len.saturating_sub(3);
        while end > 0 && !line.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &line[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_b() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(2048), "2.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(5242880), "5.00 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(2147483648), "2.00 GB");
    }

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(detect_language("main.rs"), "rust");
        assert_eq!(detect_language("/path/to/lib.rs"), "rust");
    }

    #[test]
    fn test_detect_language_python() {
        assert_eq!(detect_language("script.py"), "python");
    }

    #[test]
    fn test_detect_language_javascript() {
        assert_eq!(detect_language("app.js"), "javascript");
        assert_eq!(detect_language("component.jsx"), "javascript");
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language("app.ts"), "typescript");
        assert_eq!(detect_language("component.tsx"), "typescript");
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(detect_language("unknown.xyz"), "");
        assert_eq!(detect_language("no_extension"), "");
    }

    #[test]
    fn test_detect_language_various() {
        assert_eq!(detect_language("code.cpp"), "cpp");
        assert_eq!(detect_language("code.php"), "php");
        assert_eq!(detect_language("script.sh"), "bash");
        assert_eq!(detect_language("data.json"), "json");
        assert_eq!(detect_language("config.toml"), "toml");
    }

    #[test]
    fn test_truncate_text_no_truncation() {
        let text = "Hello, world!";
        assert_eq!(truncate_text(text, 100), text);
    }

    #[test]
    fn test_truncate_text_exact_length() {
        let text = "Hello!";
        assert_eq!(truncate_text(text, 6), text);
    }

    #[test]
    fn test_truncate_text_with_truncation() {
        let text = "Hello, world!";
        let result = truncate_text(text, 5);
        assert!(result.starts_with("Hello"));
        assert!(result.contains("...\n\n[Truncated 8 chars]"));
    }

    #[test]
    fn test_truncate_text_utf8_safe() {
        let text = "Hello 世界"; // 9 chars (not bytes!)
        let result = truncate_text(text, 7);
        assert!(result.chars().count() > 7); // Includes truncation message
        assert!(result.contains("Hello 世"));
    }

    #[test]
    fn test_format_time_ago_days() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::days(3);
        assert_eq!(format_time_ago(timestamp), "3 days ago");
    }

    #[test]
    fn test_format_time_ago_one_day() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::days(1);
        assert_eq!(format_time_ago(timestamp), "1 day ago");
    }

    #[test]
    fn test_format_time_ago_hours() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::hours(5);
        assert_eq!(format_time_ago(timestamp), "5 hours ago");
    }

    #[test]
    fn test_format_time_ago_one_hour() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::hours(1);
        assert_eq!(format_time_ago(timestamp), "1 hour ago");
    }

    #[test]
    fn test_format_time_ago_minutes() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::minutes(45);
        assert_eq!(format_time_ago(timestamp), "45 minutes ago");
    }

    #[test]
    fn test_format_time_ago_one_minute() {
        use chrono::Duration;
        let timestamp = Utc::now() - Duration::minutes(1);
        assert_eq!(format_time_ago(timestamp), "1 minute ago");
    }

    #[test]
    fn test_format_time_ago_just_now() {
        let timestamp = Utc::now();
        assert_eq!(format_time_ago(timestamp), "just now");
    }

    // Tests for byte_offset_to_line_number

    #[test]
    fn test_byte_offset_to_line_number_first_line() {
        let content = "line1\nline2\nline3\n";
        assert_eq!(byte_offset_to_line_number(content, 0), 1); // Start of line1
        assert_eq!(byte_offset_to_line_number(content, 4), 1); // Middle of line1
        assert_eq!(byte_offset_to_line_number(content, 5), 1); // End of line1 (before \n)
    }

    #[test]
    fn test_byte_offset_to_line_number_second_line() {
        let content = "line1\nline2\nline3\n";
        assert_eq!(byte_offset_to_line_number(content, 6), 2); // Start of line2
        assert_eq!(byte_offset_to_line_number(content, 10), 2); // Middle of line2
        assert_eq!(byte_offset_to_line_number(content, 11), 2); // End of line2
    }

    #[test]
    fn test_byte_offset_to_line_number_third_line() {
        let content = "line1\nline2\nline3\n";
        assert_eq!(byte_offset_to_line_number(content, 12), 3); // Start of line3
        assert_eq!(byte_offset_to_line_number(content, 17), 3); // End of line3
    }

    #[test]
    fn test_byte_offset_to_line_number_beyond_end() {
        let content = "line1\nline2\nline3\n";
        // Content has 3 newlines, so offset beyond end should return line 4
        assert_eq!(byte_offset_to_line_number(content, 100), 4);
        assert_eq!(byte_offset_to_line_number(content, 18), 4); // After final \n
    }

    #[test]
    fn test_byte_offset_to_line_number_empty_content() {
        assert_eq!(byte_offset_to_line_number("", 0), 1);
        assert_eq!(byte_offset_to_line_number("", 10), 1);
    }

    #[test]
    fn test_byte_offset_to_line_number_no_newlines() {
        let content = "single line without newline";
        assert_eq!(byte_offset_to_line_number(content, 0), 1);
        assert_eq!(byte_offset_to_line_number(content, 10), 1);
        assert_eq!(byte_offset_to_line_number(content, 100), 1);
    }

    #[test]
    fn test_byte_offset_to_line_number_utf8() {
        // UTF-8 content where character count != byte count
        let content = "hello\n世界\nworld\n"; // "世界" is 6 bytes (2 chars * 3 bytes each)
        assert_eq!(byte_offset_to_line_number(content, 0), 1); // Start
        assert_eq!(byte_offset_to_line_number(content, 6), 2); // After "hello\n"
        assert_eq!(byte_offset_to_line_number(content, 12), 2); // Still on line 2 (within UTF-8)
        assert_eq!(byte_offset_to_line_number(content, 13), 3); // After "世界\n"
    }

    // Tests for extract_context_lines

    #[test]
    fn test_extract_context_lines_middle() {
        let content = "line1\nline2\nline3\nline4\nline5\n";
        let result = extract_context_lines(content, 3, 1);
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
        assert!(result.contains("line4"));
        assert!(!result.contains("line1"));
        assert!(!result.contains("line5"));
    }

    #[test]
    fn test_extract_context_lines_start_edge() {
        let content = "line1\nline2\nline3\nline4\nline5\n";
        let result = extract_context_lines(content, 1, 2);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
        assert!(!result.contains("line4"));
    }

    #[test]
    fn test_extract_context_lines_end_edge() {
        let content = "line1\nline2\nline3\nline4\nline5\n";
        let result = extract_context_lines(content, 5, 2);
        assert!(result.contains("line3"));
        assert!(result.contains("line4"));
        assert!(result.contains("line5"));
        assert!(!result.contains("line2"));
    }

    #[test]
    fn test_extract_context_lines_formatting() {
        let content = "first\nsecond\nthird\n";
        let result = extract_context_lines(content, 2, 0);
        // Should only contain line 2, formatted with line number
        assert!(result.contains("2 |"));
        assert!(result.contains("second"));
        assert!(!result.contains("first"));
        assert!(!result.contains("third"));
    }

    #[test]
    fn test_extract_context_lines_empty_content() {
        assert_eq!(extract_context_lines("", 1, 2), "");
    }

    #[test]
    fn test_extract_context_lines_zero_line_number() {
        let content = "line1\nline2\n";
        assert_eq!(extract_context_lines(content, 0, 2), "");
    }

    #[test]
    fn test_extract_context_lines_zero_context() {
        let content = "line1\nline2\nline3\n";
        let result = extract_context_lines(content, 2, 0);
        assert!(result.contains("line2"));
        assert!(!result.contains("line1"));
        assert!(!result.contains("line3"));
    }

    // Tests for truncate_line

    #[test]
    fn test_truncate_line_no_truncation() {
        assert_eq!(truncate_line("short line", 100), "short line");
    }

    #[test]
    fn test_truncate_line_exact_length() {
        assert_eq!(truncate_line("exact", 5), "exact");
    }

    #[test]
    fn test_truncate_line_with_truncation() {
        let result = truncate_line("this is a very long line that needs truncation", 20);
        assert!(result.len() <= 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_line_utf8_safe() {
        // Ensure we don't split a multi-byte character
        let content = "hello 世界 world"; // "世" and "界" are 3 bytes each
        let result = truncate_line(content, 10);
        // Should truncate safely without panic
        assert!(result.ends_with("...") || result.len() <= 10);
    }
}
