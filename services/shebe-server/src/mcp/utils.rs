//! MCP utility functions for token limit management and formatting
//!
//! This module provides constants and helper functions for managing MCP protocol
//! token limits and building user-friendly warning messages.

/// MCP protocol token limit (25,000 tokens)
///
/// This is the maximum number of tokens that can be returned in a single MCP response.
/// Attempting to exceed this limit will result in a protocol error.
pub const MCP_TOKEN_LIMIT: usize = 25_000;

/// Default limit for list_dir when user doesn't specify limit parameter
///
/// This conservative default ensures reasonable performance and token usage
/// for initial repository exploration without overwhelming the user.
pub const LIST_DIR_DEFAULT_LIMIT: usize = 100;

/// Maximum limit for list_dir (hard cap enforced even if user requests more)
///
/// At ~25-30 chars per path, 500 files = ~12-15k tokens (well under 25k limit)
pub const LIST_DIR_MAX_LIMIT: usize = 500;

/// Maximum characters for read_file (20k chars = ~5k tokens with 80% safety margin)
///
/// This conservative limit ensures:
/// - Well under MCP 25k token limit (5k tokens + markdown overhead < 10k tokens)
/// - Room for warning messages, syntax highlighting, and metadata
/// - UTF-8 safety with character-based truncation
pub const READ_FILE_MAX_CHARS: usize = 20_000;

/// Build truncation warning message for list_dir
///
/// Creates a user-friendly warning that explains:
/// - What was truncated (number of files shown vs total)
/// - Why it was truncated (MCP token limit)
/// - What users can do (use limit parameter, find_file, bash tools)
///
/// # Arguments
/// * `shown_count` - Number of files actually displayed
/// * `total_count` - Total number of files in the session
/// * `session` - Session ID for example commands
///
/// # Returns
/// Formatted markdown warning message
pub fn build_list_dir_warning(shown_count: usize, total_count: usize, session: &str) -> String {
    let not_shown = total_count.saturating_sub(shown_count);
    format!(
        "⚠️ OUTPUT TRUNCATED - MAXIMUM {LIST_DIR_MAX_LIMIT} FILES DISPLAYED\n\n\
         Showing: {shown_count} of {total_count} files (first {shown_count}, alphabetically sorted)\n\
         Reason: Maximum display limit is {LIST_DIR_MAX_LIMIT} files (MCP 25k token limit)\n\
         Not shown: {not_shown} files\n\n\
         💡 SUGGESTIONS:\n\
         - Use `find_file` with patterns to filter: find_file(session=\"{session}\", pattern=\"*.yaml\")\n\
         - For pagination support, see: docs/work-plans/011-phase02-mcp-pagination-implementation.md\n\
         - For full file list, use bash: find /path/to/repo -type f | sort\n\n\
         ---\n\n\
         **Files 1-{shown_count} (of {total_count} total):**\n\n"
    )
}

/// Build truncation warning message for read_file
///
/// Creates a user-friendly warning that explains:
/// - What was truncated (characters/lines shown vs total)
/// - Why it was truncated (MCP token limit)
/// - What users can do (search_code, preview_chunk, bash tools)
///
/// # Arguments
/// * `shown_chars` - Number of characters actually displayed
/// * `total_chars` - Total number of characters in the file
/// * `estimated_lines` - Approximate number of lines shown (for user reference)
/// * `file_path` - Path to the file for example commands
///
/// # Returns
/// Formatted markdown warning message
pub fn build_read_file_warning(
    shown_chars: usize,
    total_chars: usize,
    estimated_lines: usize,
    file_path: &str,
) -> String {
    let not_shown = total_chars.saturating_sub(shown_chars);
    let percent = if total_chars > 0 {
        (shown_chars as f64 / total_chars as f64) * 100.0
    } else {
        0.0
    };

    format!(
        "⚠️ FILE TRUNCATED - SHOWING FIRST {READ_FILE_MAX_CHARS} CHARACTERS\n\n\
         Showing: Characters 1-{shown_chars} of {total_chars} total ({percent:.1}%)\n\
         Reason: Maximum display limit is {READ_FILE_MAX_CHARS} characters (MCP 25k token limit)\n\
         Not shown: {not_shown} characters\n\n\
         💡 SUGGESTIONS:\n\
         - Use `search_code` to find specific content in this file\n\
         - Use `preview_chunk` to view specific sections\n\
         - For full file, use bash: cat {file_path}\n\n\
         ---\n\n\
         **File:** `{file_path}`\n\
         **Showing:** First {shown_chars} characters (~{estimated_lines} lines)\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_dir_warning_formatting() {
        let warning = build_list_dir_warning(500, 5605, "istio");

        // Verify key information is present
        assert!(warning.contains("⚠️ OUTPUT TRUNCATED"));
        assert!(warning.contains("500 of 5605 files"));
        assert!(warning.contains("5105 files")); // not shown
        assert!(warning.contains("istio")); // session name in example
        assert!(warning.contains("MAXIMUM 500 FILES"));
        assert!(warning.contains("MCP 25k token limit"));
    }

    #[test]
    fn test_list_dir_warning_with_small_truncation() {
        let warning = build_list_dir_warning(100, 150, "small-repo");

        assert!(warning.contains("100 of 150 files"));
        assert!(warning.contains("50 files")); // not shown
    }

    #[test]
    fn test_read_file_warning_formatting() {
        let warning = build_read_file_warning(20000, 634000, 280, "/path/to/large.sql");

        // Verify key information is present
        assert!(warning.contains("⚠️ FILE TRUNCATED"));
        assert!(warning.contains("20000 of 634000 total"));
        assert!(warning.contains("614000 characters")); // not shown
        assert!(warning.contains("/path/to/large.sql"));
        assert!(warning.contains("~280 lines"));
        assert!(warning.contains("FIRST 20000 CHARACTERS"));
    }

    #[test]
    fn test_read_file_warning_percentage_calculation() {
        let warning = build_read_file_warning(20000, 100000, 200, "/test.txt");

        // 20000/100000 = 20%
        assert!(warning.contains("20.0%"));
    }

    #[test]
    fn test_read_file_warning_edge_case_zero_total() {
        // Should not panic on division by zero
        let warning = build_read_file_warning(0, 0, 0, "/empty.txt");

        assert!(warning.contains("0.0%"));
    }

    #[test]
    fn test_constants_are_reasonable() {
        // Verify constants meet design requirements
        assert_eq!(MCP_TOKEN_LIMIT, 25_000);
        assert_eq!(LIST_DIR_DEFAULT_LIMIT, 100);
        assert_eq!(LIST_DIR_MAX_LIMIT, 500);
        assert_eq!(READ_FILE_MAX_CHARS, 20_000);

        // Verify safety margins
        // LIST_DIR_MAX_LIMIT: 500 files * 30 chars/path avg = ~15k chars = ~3.75k tokens
        // This is well under 25k token limit
        assert!(LIST_DIR_MAX_LIMIT * 30 / 4 < MCP_TOKEN_LIMIT);

        // READ_FILE_MAX_CHARS: 20k chars = ~5k tokens (chars/4 rough estimate)
        // This leaves 20k tokens for markdown formatting, warning messages, etc.
        assert!(READ_FILE_MAX_CHARS / 4 < MCP_TOKEN_LIMIT / 2);
    }
}
