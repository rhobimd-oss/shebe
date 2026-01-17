//! Query preprocessing for improved search UX.
//!
//! This module provides query preprocessing to handle common syntax patterns
//! that would otherwise cause Tantivy parse errors:
//! - Curly braces in URL templates: `{id}` -> `\{id\}`
//! - URL-like paths: `/users/{id}` -> `"/users/\{id\}"`
//! - Multi-colon identifiers: `pkg:scope:name` -> `"pkg:scope:name"`
//!
//! It also provides field validation to give helpful error messages when
//! users try to use invalid field prefixes.
//!
//! Additionally, this module supports a "literal" search mode that escapes
//! ALL special characters, allowing exact string searches without any
//! query syntax interpretation.

use crate::core::error::ShebeError;
use once_cell::sync::Lazy;
use regex::Regex;

// Regex patterns compiled once at startup
static URL_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^/[a-zA-Z0-9_/{}\-]+").unwrap());

static MULTI_COLON_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\w+:\w+:\w+").unwrap());

/// Preprocess a query string for Tantivy compatibility.
///
/// Applies the following transformations (in normal mode):
/// 1. Auto-quotes URL-like patterns starting with `/`
/// 2. Auto-quotes multi-colon patterns (e.g., `pkg:scope:name`)
/// 3. Escapes curly braces `{` and `}` as `\{` and `\}`
///
/// In literal mode (`literal=true`), ALL special characters are escaped,
/// allowing exact string searches without any query syntax interpretation.
///
/// Already-quoted strings are returned unchanged (except for brace escaping
/// inside the quotes if needed).
///
/// # Examples
///
/// ```
/// use shebe_server::core::search::preprocess_query;
///
/// // Normal mode: Curly braces are escaped
/// assert_eq!(preprocess_query("{id}", false), "\\{id\\}");
///
/// // Normal mode: URL patterns are quoted and braces escaped
/// assert_eq!(preprocess_query("/users/{id}", false), "\"/users/\\{id\\}\"");
///
/// // Normal mode: Multi-colon patterns are quoted
/// assert_eq!(preprocess_query("negusa:calendar:read", false), "\"negusa:calendar:read\"");
///
/// // Literal mode: ALL special characters are escaped
/// assert_eq!(preprocess_query("file:test", true), "file\\:test");
/// ```
pub fn preprocess_query(query: &str, literal: bool) -> String {
    let trimmed = query.trim();

    // Skip processing if empty
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    // Literal mode: escape ALL special characters for exact string search
    if literal {
        return escape_all_special(trimmed);
    }

    // If already fully quoted, only escape braces inside
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 1 {
        // Extract content inside quotes, escape braces, re-quote
        let inner = &trimmed[1..trimmed.len() - 1];
        let escaped = escape_braces(inner);
        return format!("\"{escaped}\"");
    }

    let mut result = trimmed.to_string();

    // Check if this looks like a URL pattern (starts with /)
    if URL_PATTERN.is_match(&result) {
        // Escape braces first, then quote the whole thing
        let escaped = escape_braces(&result);
        return format!("\"{escaped}\"");
    }

    // Check if this is a multi-colon pattern (word:word:word+)
    if MULTI_COLON_PATTERN.is_match(&result) {
        // Quote to prevent field prefix interpretation
        let escaped = escape_braces(&result);
        return format!("\"{escaped}\"");
    }

    // For other queries, just escape braces
    result = escape_braces(&result);

    result
}

/// Escape curly braces for Tantivy query syntax.
fn escape_braces(s: &str) -> String {
    s.replace('{', "\\{").replace('}', "\\}")
}

/// Escape ALL special characters for literal search mode.
///
/// This allows searching for exact strings without any query syntax interpretation.
/// Special characters that are escaped include: : { } [ ] ( ) @ " \ + - ! ^ ~ *
fn escape_all_special(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            ':' | '{' | '}' | '[' | ']' | '(' | ')' | '@' | '"' | '\\' | '+' | '-' | '!' | '^'
            | '~' | '*' => {
                result.push('\\');
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

// Valid field names for Tantivy schema
const VALID_FIELDS: [&str; 2] = ["content", "file_path"];

// Pattern to detect potential field prefixes (word:nonspace)
// We'll do additional validation in code to avoid look-behind
static FIELD_PREFIX_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\w+):([^\s:])").unwrap());

/// Validate that all field prefixes in a query are valid.
///
/// Returns an error if an invalid field prefix is detected, with helpful
/// suggestions for common mistakes.
///
/// # Examples
///
/// ```
/// use shebe_server::core::search::validate_query_fields;
///
/// // Valid fields pass
/// assert!(validate_query_fields("content:test").is_ok());
/// assert!(validate_query_fields("file_path:main.rs").is_ok());
///
/// // Invalid fields return helpful errors
/// assert!(validate_query_fields("file:test.rs").is_err());
/// ```
pub fn validate_query_fields(query: &str) -> Result<(), ShebeError> {
    // Skip validation if query is quoted (phrase query)
    let trimmed = query.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return Ok(());
    }

    for cap in FIELD_PREFIX_PATTERN.captures_iter(query) {
        let match_start = cap.get(0).unwrap().start();
        let field = &cap[1];

        // Only consider it a field prefix if it's at the start or preceded by whitespace
        if match_start > 0 {
            let prev_char = query.chars().nth(match_start - 1).unwrap_or(' ');
            if !prev_char.is_whitespace() {
                // Not a field prefix, it's part of a larger token (e.g., multi-colon pattern)
                continue;
            }
        }

        // Skip if this is a valid field
        if VALID_FIELDS.contains(&field) {
            continue;
        }

        // Skip common words that might look like field prefixes but aren't
        // (e.g., "http:" in URLs, "https:", etc.)
        if matches!(field, "http" | "https" | "ftp" | "mailto") {
            continue;
        }

        let rest_of_query = &query[cap.get(0).unwrap().end() - 1..];
        let value_hint = rest_of_query.split_whitespace().next().unwrap_or("");

        return Err(ShebeError::InvalidQueryField {
            field: field.to_string(),
            message: format!(
                "Use 'file_path:{value_hint}' for path matching, or search content directly."
            ),
            valid_fields: VALID_FIELDS.iter().map(|s| s.to_string()).collect(),
            suggestion: suggest_field_alias(field),
        });
    }

    Ok(())
}

/// Suggest a valid field name for common aliases.
fn suggest_field_alias(field: &str) -> Option<String> {
    match field.to_lowercase().as_str() {
        "file" | "filename" | "path" | "filepath" | "name" => Some("file_path".to_string()),
        "code" | "text" | "body" | "source" | "src" => Some("content".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Task 1.2.1: Curly brace escaping tests (normal mode)

    #[test]
    fn test_escape_single_brace_pair() {
        assert_eq!(preprocess_query("{id}", false), "\\{id\\}");
    }

    #[test]
    fn test_escape_multiple_braces() {
        assert_eq!(
            preprocess_query("{a}{b}{c}", false),
            "\\{a\\}\\{b\\}\\{c\\}"
        );
    }

    #[test]
    fn test_escape_braces_in_text() {
        assert_eq!(
            preprocess_query("function({arg})", false),
            "function(\\{arg\\})"
        );
    }

    #[test]
    fn test_no_braces_unchanged() {
        assert_eq!(preprocess_query("simple query", false), "simple query");
    }

    // Task 1.2.2: URL pattern auto-quoting tests (normal mode)

    #[test]
    fn test_quote_simple_url_path() {
        assert_eq!(preprocess_query("/api/users", false), "\"/api/users\"");
    }

    #[test]
    fn test_quote_url_with_braces() {
        assert_eq!(
            preprocess_query("/users/{id}", false),
            "\"/users/\\{id\\}\""
        );
    }

    #[test]
    fn test_quote_complex_url() {
        assert_eq!(
            preprocess_query("/users/{id}/roles/{role}", false),
            "\"/users/\\{id\\}/roles/\\{role\\}\""
        );
    }

    #[test]
    fn test_url_with_dashes() {
        assert_eq!(
            preprocess_query("/api/user-profiles", false),
            "\"/api/user-profiles\""
        );
    }

    // Task 1.2.3: Multi-colon pattern tests (normal mode)

    #[test]
    fn test_quote_three_part_colon() {
        assert_eq!(
            preprocess_query("negusa:calendar:read", false),
            "\"negusa:calendar:read\""
        );
    }

    #[test]
    fn test_quote_four_part_colon() {
        assert_eq!(
            preprocess_query("pkg:scope:name:v1", false),
            "\"pkg:scope:name:v1\""
        );
    }

    #[test]
    fn test_single_colon_not_quoted() {
        // Single colon is a valid field prefix, should not be quoted
        assert_eq!(
            preprocess_query("file_path:test.rs", false),
            "file_path:test.rs"
        );
    }

    #[test]
    fn test_two_colons_not_quoted() {
        // Two colons (word:word) is still a field prefix pattern
        assert_eq!(preprocess_query("content:hello", false), "content:hello");
    }

    // Already quoted tests (normal mode)

    #[test]
    fn test_already_quoted_unchanged() {
        assert_eq!(
            preprocess_query("\"already quoted\"", false),
            "\"already quoted\""
        );
    }

    #[test]
    fn test_already_quoted_escapes_inner_braces() {
        assert_eq!(
            preprocess_query("\"/users/{id}\"", false),
            "\"/users/\\{id\\}\""
        );
    }

    #[test]
    fn test_partial_quote_not_special() {
        // Only one quote - not considered quoted
        assert_eq!(preprocess_query("\"partial", false), "\"partial");
    }

    // Edge cases (normal mode)

    #[test]
    fn test_empty_query() {
        assert_eq!(preprocess_query("", false), "");
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(preprocess_query("   ", false), "");
    }

    #[test]
    fn test_whitespace_trimmed() {
        assert_eq!(preprocess_query("  hello  ", false), "hello");
    }

    #[test]
    fn test_boolean_operators_preserved() {
        // Boolean operators should work normally
        assert_eq!(
            preprocess_query("auth AND session", false),
            "auth AND session"
        );
    }

    #[test]
    fn test_phrase_query_preserved() {
        assert_eq!(
            preprocess_query("\"exact phrase\"", false),
            "\"exact phrase\""
        );
    }

    // Task 1.1.1: Field validation tests

    #[test]
    fn test_validate_valid_content_field() {
        assert!(validate_query_fields("content:test").is_ok());
    }

    #[test]
    fn test_validate_valid_file_path_field() {
        assert!(validate_query_fields("file_path:main.rs").is_ok());
    }

    #[test]
    fn test_validate_no_field_prefix() {
        assert!(validate_query_fields("simple search query").is_ok());
    }

    #[test]
    fn test_validate_invalid_file_field() {
        let result = validate_query_fields("file:test.rs");
        assert!(result.is_err());
        if let Err(ShebeError::InvalidQueryField {
            field,
            suggestion,
            valid_fields,
            ..
        }) = result
        {
            assert_eq!(field, "file");
            assert_eq!(suggestion, Some("file_path".to_string()));
            assert!(valid_fields.contains(&"content".to_string()));
            assert!(valid_fields.contains(&"file_path".to_string()));
        } else {
            panic!("Expected InvalidQueryField error");
        }
    }

    #[test]
    fn test_validate_invalid_code_field() {
        let result = validate_query_fields("code:function");
        assert!(result.is_err());
        if let Err(ShebeError::InvalidQueryField { suggestion, .. }) = result {
            assert_eq!(suggestion, Some("content".to_string()));
        }
    }

    #[test]
    fn test_validate_unknown_field_no_suggestion() {
        let result = validate_query_fields("xyz:something");
        assert!(result.is_err());
        if let Err(ShebeError::InvalidQueryField { suggestion, .. }) = result {
            assert_eq!(suggestion, None);
        }
    }

    #[test]
    fn test_validate_quoted_query_skipped() {
        // Quoted queries should not trigger field validation
        assert!(validate_query_fields("\"file:test.rs\"").is_ok());
    }

    #[test]
    fn test_validate_http_url_skipped() {
        // http: and https: should not be flagged as invalid fields
        assert!(validate_query_fields("http://example.com").is_ok());
        assert!(validate_query_fields("https://example.com").is_ok());
    }

    #[test]
    fn test_suggest_file_aliases() {
        assert_eq!(suggest_field_alias("file"), Some("file_path".to_string()));
        assert_eq!(
            suggest_field_alias("filename"),
            Some("file_path".to_string())
        );
        assert_eq!(suggest_field_alias("path"), Some("file_path".to_string()));
        assert_eq!(
            suggest_field_alias("filepath"),
            Some("file_path".to_string())
        );
    }

    #[test]
    fn test_suggest_content_aliases() {
        assert_eq!(suggest_field_alias("code"), Some("content".to_string()));
        assert_eq!(suggest_field_alias("text"), Some("content".to_string()));
        assert_eq!(suggest_field_alias("body"), Some("content".to_string()));
        assert_eq!(suggest_field_alias("source"), Some("content".to_string()));
    }

    #[test]
    fn test_suggest_unknown_no_suggestion() {
        assert_eq!(suggest_field_alias("unknown"), None);
        assert_eq!(suggest_field_alias("random"), None);
    }

    // Task 2.2/2.3: Literal search mode tests

    #[test]
    fn test_literal_escapes_colon() {
        // In literal mode, colons are escaped (no field prefixes)
        assert_eq!(preprocess_query("file:test.rs", true), "file\\:test.rs");
    }

    #[test]
    fn test_literal_escapes_braces() {
        assert_eq!(preprocess_query("{id}", true), "\\{id\\}");
    }

    #[test]
    fn test_literal_escapes_brackets() {
        assert_eq!(preprocess_query("array[0]", true), "array\\[0\\]");
    }

    #[test]
    fn test_literal_escapes_parens() {
        assert_eq!(preprocess_query("func(arg)", true), "func\\(arg\\)");
    }

    #[test]
    fn test_literal_escapes_quotes() {
        assert_eq!(preprocess_query("say \"hello\"", true), "say \\\"hello\\\"");
    }

    #[test]
    fn test_literal_escapes_plus_minus() {
        assert_eq!(preprocess_query("a + b - c", true), "a \\+ b \\- c");
    }

    #[test]
    fn test_literal_escapes_boolean_operators() {
        // In literal mode, AND/OR are just words, but special chars still escaped
        assert_eq!(preprocess_query("a AND b", true), "a AND b");
    }

    #[test]
    fn test_literal_escapes_at_symbol() {
        assert_eq!(preprocess_query("@annotation", true), "\\@annotation");
    }

    #[test]
    fn test_literal_escapes_backslash() {
        assert_eq!(
            preprocess_query("path\\to\\file", true),
            "path\\\\to\\\\file"
        );
    }

    #[test]
    fn test_literal_escapes_exclamation() {
        assert_eq!(preprocess_query("!important", true), "\\!important");
    }

    #[test]
    fn test_literal_escapes_caret_tilde() {
        assert_eq!(preprocess_query("~test^2", true), "\\~test\\^2");
    }

    #[test]
    fn test_literal_escapes_asterisk() {
        assert_eq!(preprocess_query("*.rs", true), "\\*.rs");
    }

    #[test]
    fn test_literal_complex_code_pattern() {
        // A realistic Go code pattern - % is not a special char, only parens and quotes
        assert_eq!(
            preprocess_query("fmt.Printf(\"%s\")", true),
            "fmt.Printf\\(\\\"%s\\\"\\)"
        );
    }

    #[test]
    fn test_literal_url_pattern_no_quoting() {
        // In literal mode, URL patterns are NOT auto-quoted, just escaped
        assert_eq!(preprocess_query("/users/{id}", true), "/users/\\{id\\}");
    }

    #[test]
    fn test_literal_multi_colon_no_quoting() {
        // In literal mode, multi-colon patterns are NOT auto-quoted, just escaped
        assert_eq!(
            preprocess_query("negusa:calendar:read", true),
            "negusa\\:calendar\\:read"
        );
    }

    #[test]
    fn test_literal_empty_query() {
        assert_eq!(preprocess_query("", true), "");
    }

    #[test]
    fn test_literal_whitespace_trimmed() {
        assert_eq!(preprocess_query("  hello  ", true), "hello");
    }

    #[test]
    fn test_literal_plain_text_unchanged() {
        // Plain text without special chars should pass through
        assert_eq!(preprocess_query("simple query", true), "simple query");
    }
}
