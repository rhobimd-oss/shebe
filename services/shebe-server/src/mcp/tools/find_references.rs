//! Find all references to a symbol across the indexed codebase.
//!
//! # Core Objective
//!
//! Answer the question: **"What are all the references I'm going to have to update?"**
//!
//! This tool is designed for the **discovery phase** of refactoring - quickly enumerating
//! all locations that need attention before making changes. It is **complementary** to
//! AST-aware tools like Serena, not a replacement.
//!
//! # Why This Tool Exists
//!
//! When refactoring, Claude needs to know what will break before making changes:
//!
//! | Approach | Tokens per Reference | Problem |
//! |----------|---------------------|---------|
//! | Serena `find_symbol` | ~500+ | Returns full code bodies |
//! | Grep/ripgrep | ~100+ | No confidence scoring, high noise |
//! | **find_references** | ~50-70 | Locations only, confidence-ranked |
//!
//! # Workflow
//!
//! ```text
//! Discovery Phase              Modification Phase
//! ----------------             ------------------
//! find_references              Serena/AST tools
//! "What needs to change?"  ->  "Make the change"
//! ```
//!
//! # Output Design
//!
//! The tool returns:
//! - **Locations** (file:line), not full code bodies
//! - **Confidence scores** (high/medium/low) to prioritize work
//! - **"Files to update"** list for systematic refactoring
//! - **Session freshness** to warn about stale indexes
//!
//! # When NOT to Use
//!
//! - When semantic precision is critical (use Serena for actual renames)
//! - For understanding code structure (use `search_code` or `get_symbols_overview`)
//! - For single-file searches (use grep or read the file directly)

use super::handler::{text_content, McpToolHandler};
use super::helpers::{
    byte_offset_to_line_number, detect_language, extract_context_lines, format_time_ago,
};
use crate::core::services::Services;
use crate::core::storage::SessionMetadata;
use crate::core::types::SearchRequest;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Handler for the find_references MCP tool.
pub struct FindReferencesHandler {
    services: Arc<Services>,
}

/// Type of symbol being searched for, used to select appropriate patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Function,
    Type,
    Variable,
    Constant,
    Any,
}

/// A single reference to a symbol found in the codebase.
///
/// Designed for minimal token usage while providing actionable information:
/// - Location (file:line) for navigation
/// - Minimal context (configurable lines) for verification
/// - Confidence score for prioritization
/// - Pattern name for understanding match type
///
/// Typical token cost: ~50-70 tokens per reference (vs ~500+ for full code bodies).
#[derive(Debug)]
pub struct Reference {
    /// Absolute path to the file containing the reference.
    /// Used in "Files to update" list for systematic refactoring.
    pub file_path: String,
    /// 1-based line number for IDE navigation (file:line format).
    pub line_number: usize,
    /// Column offset within the line (0-based). For precise cursor positioning.
    pub column: usize,
    /// Context lines around the reference (configurable via `context_lines` param).
    /// Kept minimal to reduce token usage while allowing verification.
    pub context: String,
    /// Pattern that matched (e.g., "function_call", "type_annotation").
    /// Helps understand why this location was flagged.
    pub pattern: String,
    /// Confidence score (0.0 to 1.0). Used for grouping:
    /// - High (>=0.80): Definitely update
    /// - Medium (0.50-0.79): Review before updating
    /// - Low (<0.50): Possible false positive
    pub confidence: f32,
}

impl FindReferencesHandler {
    /// Create a new FindReferencesHandler with access to services.
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Parse symbol type string into enum.
    fn parse_symbol_type(s: &Option<String>) -> SymbolType {
        match s.as_deref() {
            Some("function") => SymbolType::Function,
            Some("type") => SymbolType::Type,
            Some("variable") => SymbolType::Variable,
            Some("constant") => SymbolType::Constant,
            _ => SymbolType::Any,
        }
    }

    /// Build regex patterns for matching symbol usages based on symbol type.
    fn build_patterns(symbol: &str, symbol_type: SymbolType) -> Vec<(Regex, &'static str, f32)> {
        let escaped = regex::escape(symbol);
        let mut patterns = Vec::new();

        // Function patterns
        match symbol_type {
            SymbolType::Function | SymbolType::Any => {
                // Function call: symbol(
                if let Ok(r) = Regex::new(&format!(r"{escaped}\s*\(")) {
                    patterns.push((r, "function_call", 0.95));
                }
                // Method call: .symbol(
                if let Ok(r) = Regex::new(&format!(r"\.{escaped}\s*\(")) {
                    patterns.push((r, "method_call", 0.92));
                }
            }
            _ => {}
        }

        // Type patterns
        match symbol_type {
            SymbolType::Type | SymbolType::Any => {
                // Type annotation: : symbol
                if let Ok(r) = Regex::new(&format!(r":\s*{escaped}")) {
                    patterns.push((r, "type_annotation", 0.85));
                }
                // Return type: -> symbol
                if let Ok(r) = Regex::new(&format!(r"->\s*{escaped}")) {
                    patterns.push((r, "return_type", 0.85));
                }
                // Generic type: <symbol
                if let Ok(r) = Regex::new(&format!(r"<{escaped}")) {
                    patterns.push((r, "generic_type", 0.85));
                }
                // Type instantiation: symbol{
                if let Ok(r) = Regex::new(&format!(r"{escaped}\s*\{{")) {
                    patterns.push((r, "type_instantiation", 0.85));
                }
            }
            _ => {}
        }

        // Variable/constant patterns
        match symbol_type {
            SymbolType::Variable | SymbolType::Constant | SymbolType::Any => {
                // Assignment target: symbol =
                if let Ok(r) = Regex::new(&format!(r"{escaped}\s*=")) {
                    patterns.push((r, "assignment_target", 0.80));
                }
                // Assignment value: = symbol
                if let Ok(r) = Regex::new(&format!(r"=\s*{escaped}")) {
                    patterns.push((r, "assignment_value", 0.80));
                }
                // Property access: symbol.
                if let Ok(r) = Regex::new(&format!(r"{escaped}\.")) {
                    patterns.push((r, "property_access", 0.85));
                }
            }
            _ => {}
        }

        // Import patterns (apply to all types)
        if let Ok(r) = Regex::new(&format!(r"import.*{escaped}")) {
            patterns.push((r, "import", 0.90));
        }
        if let Ok(r) = Regex::new(&format!(r"use\s+.*{escaped}")) {
            patterns.push((r, "use_statement", 0.90));
        }
        if let Ok(r) = Regex::new(&format!(r"from\s+.*import.*{escaped}")) {
            patterns.push((r, "python_import", 0.90));
        }

        // Fallback: word boundary match
        if let Ok(r) = Regex::new(&format!(r"\b{escaped}\b")) {
            patterns.push((r, "word_match", 0.60));
        }

        patterns
    }

    /// Adjust confidence score based on context.
    fn adjust_confidence(base_confidence: f32, file_path: &str, line_content: &str) -> f32 {
        let mut confidence = base_confidence;

        // Test files likely need updates
        if file_path.contains("test") || file_path.contains("spec") {
            confidence += 0.05;
        }

        // Comments reduce confidence
        let trimmed = line_content.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
            || trimmed.starts_with("/*")
        {
            confidence -= 0.30;
        }

        // String literals reduce confidence (rough heuristic)
        let quote_count = line_content.matches('"').count() + line_content.matches('\'').count();
        if quote_count >= 2 {
            confidence -= 0.20;
        }

        // Documentation files have lower confidence
        if file_path.ends_with(".md") || file_path.ends_with(".txt") || file_path.ends_with(".rst")
        {
            confidence -= 0.25;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Deduplicate references, keeping highest confidence per location.
    fn deduplicate_references(references: &mut Vec<Reference>) {
        // Sort by confidence descending first
        references.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Then sort by location to group duplicates
        references.sort_by(|a, b| {
            (&a.file_path, a.line_number)
                .cmp(&(&b.file_path, b.line_number))
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        // Deduplicate by location (keeps first = highest confidence)
        references.dedup_by(|a, b| a.file_path == b.file_path && a.line_number == b.line_number);
    }

    /// Format results as markdown output.
    fn format_results(
        &self,
        symbol: &str,
        references: &[Reference],
        session_metadata: Option<&SessionMetadata>,
    ) -> String {
        if references.is_empty() {
            let mut output = format!("No references found for `{symbol}`\n");
            if let Some(meta) = session_metadata {
                output.push_str(&format!(
                    "\nSession last indexed: {} ({})\n",
                    meta.last_indexed_at.format("%Y-%m-%d %H:%M:%S UTC"),
                    format_time_ago(meta.last_indexed_at)
                ));
            }
            return output;
        }

        // Group by confidence level
        let mut high: Vec<&Reference> = Vec::new();
        let mut medium: Vec<&Reference> = Vec::new();
        let mut low: Vec<&Reference> = Vec::new();

        for r in references {
            if r.confidence >= 0.80 {
                high.push(r);
            } else if r.confidence >= 0.50 {
                medium.push(r);
            } else {
                low.push(r);
            }
        }

        let mut output = format!(
            "## References to `{symbol}` ({} found)\n\n",
            references.len()
        );

        // High confidence
        if !high.is_empty() {
            output.push_str(&format!("### High Confidence ({})\n\n", high.len()));
            for r in &high {
                output.push_str(&self.format_single_reference(r));
            }
        }

        // Medium confidence
        if !medium.is_empty() {
            output.push_str(&format!("### Medium Confidence ({})\n\n", medium.len()));
            for r in &medium {
                output.push_str(&self.format_single_reference(r));
            }
        }

        // Low confidence
        if !low.is_empty() {
            output.push_str(&format!("### Low Confidence ({})\n\n", low.len()));
            for r in &low {
                output.push_str(&self.format_single_reference(r));
            }
        }

        // Summary
        let unique_files: HashSet<_> = references.iter().map(|r| &r.file_path).collect();

        output.push_str("---\n\n**Summary:**\n");
        output.push_str(&format!("- High confidence: {} references\n", high.len()));
        output.push_str(&format!(
            "- Medium confidence: {} references\n",
            medium.len()
        ));
        output.push_str(&format!("- Low confidence: {} references\n", low.len()));
        output.push_str(&format!("- Total files: {}\n", unique_files.len()));

        // Session freshness
        if let Some(meta) = session_metadata {
            output.push_str(&format!(
                "- Session indexed: {} ({})\n",
                meta.last_indexed_at.format("%Y-%m-%d %H:%M:%S UTC"),
                format_time_ago(meta.last_indexed_at)
            ));
        }

        // Files to update (high confidence only)
        if !high.is_empty() {
            output.push_str("\n**Files to update:**\n");
            let high_files: HashSet<_> = high.iter().map(|r| r.file_path.as_str()).collect();
            for file in high_files {
                output.push_str(&format!("- `{file}`\n"));
            }
        }

        output
    }

    /// Format a single reference for output.
    fn format_single_reference(&self, r: &Reference) -> String {
        let lang = detect_language(&r.file_path);
        format!(
            "#### {}:{}\n```{}\n{}\n```\n- **Pattern:** {}\n- **Confidence:** {:.2}\n\n",
            r.file_path,
            r.line_number,
            lang,
            r.context.trim(),
            r.pattern,
            r.confidence
        )
    }
}

#[async_trait]
impl McpToolHandler for FindReferencesHandler {
    fn name(&self) -> &str {
        "find_references"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "find_references".to_string(),
            description: r#"Find all references to a symbol across the indexed codebase.

## Core Objective

Answer: "What are all the references I'm going to have to update?"

This is a **discovery** tool for the pre-refactoring phase. It enumerates locations
efficiently (~50-70 tokens per reference) so you know what needs to change before
making modifications.

## Discovery vs Modification

| Phase | Tool | Purpose |
|-------|------|---------|
| Discovery | find_references | Enumerate what needs to change |
| Modification | Serena/AST tools | Make changes with semantic precision |

Use find_references first to get the list, then use appropriate tools to make changes.

## When to Use

- Before renaming any symbol (enumerate all usages)
- Before deleting a function/type (check for callers)
- To estimate refactoring scope (how many files affected?)

## When NOT to Use

- For the actual rename (use Serena for semantic precision)
- For single-file searches (just read the file)
- When you need AST-level accuracy (use LSP tools)

## Example

```json
{
  "symbol": "handleLogin",
  "session": "myapp",
  "symbol_type": "function",
  "defined_in": "src/auth/handlers.go"
}
```

## Symbol Types

- `function`: Matches function/method calls (symbol(), .symbol())
- `type`: Matches type annotations (: symbol, <symbol>)
- `variable`: Matches assignments and property access
- `constant`: Same as variable
- `any`: Matches all patterns (default)

## Confidence Levels

- **High (0.80+):** Very likely a real reference, should be updated
- **Medium (0.50-0.79):** Probable reference, review before updating
- **Low (<0.50):** Possible false positive (comments, strings, docs)

## Output

Returns a "Files to update" list with high-confidence references grouped first.
Use this list to systematically update each file."#
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Symbol name to find references for",
                        "minLength": 2,
                        "maxLength": 200
                    },
                    "session": {
                        "type": "string",
                        "description": "Session ID to search",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "symbol_type": {
                        "type": "string",
                        "enum": ["function", "type", "variable", "constant", "any"],
                        "description": "Hint for filtering by usage pattern",
                        "default": "any"
                    },
                    "defined_in": {
                        "type": "string",
                        "description": "File where symbol is defined (excluded from results)"
                    },
                    "include_definition": {
                        "type": "boolean",
                        "description": "Include definition site in results",
                        "default": false
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Lines of context around each reference",
                        "default": 2,
                        "minimum": 0,
                        "maximum": 10
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum references to return. Limit configurable via max_k setting (default: 100).",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 500
                    }
                },
                "required": ["symbol", "session"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct FindReferencesArgs {
            symbol: String,
            session: String,
            #[serde(default)]
            symbol_type: Option<String>,
            #[serde(default)]
            defined_in: Option<String>,
            #[serde(default)]
            include_definition: bool,
            #[serde(default = "default_context_lines")]
            context_lines: usize,
            #[serde(default = "default_max_results")]
            max_results: usize,
        }
        fn default_context_lines() -> usize {
            2
        }
        fn default_max_results() -> usize {
            50
        }

        // Parse arguments
        let args: FindReferencesArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Validate symbol
        if args.symbol.trim().is_empty() {
            return Err(McpError::InvalidParams(
                "Symbol cannot be empty".to_string(),
            ));
        }
        if args.symbol.len() < 2 {
            return Err(McpError::InvalidParams(
                "Symbol must be at least 2 characters".to_string(),
            ));
        }

        // Search using SearchService
        let search_request = SearchRequest {
            query: args.symbol.clone(),
            session: args.session.clone(),
            k: Some(args.max_results * 2), // Over-fetch to allow for filtering
        };
        let search_response = self
            .services
            .search
            .search(search_request)
            .map_err(McpError::from)?;

        // Build patterns based on symbol_type
        let symbol_type = Self::parse_symbol_type(&args.symbol_type);
        let patterns = Self::build_patterns(&args.symbol, symbol_type);

        // Process search results
        let mut references: Vec<Reference> = Vec::new();
        let mut files_cache: HashMap<String, String> = HashMap::new();

        for result in search_response.results {
            // Skip definition file if requested
            if !args.include_definition {
                if let Some(ref defined_in) = args.defined_in {
                    if result.file_path.ends_with(defined_in) {
                        continue;
                    }
                }
            }

            // Read file content (cached to avoid re-reading)
            let file_content = if let Some(content) = files_cache.get(&result.file_path) {
                content.clone()
            } else {
                match std::fs::read_to_string(&result.file_path) {
                    Ok(content) => {
                        files_cache.insert(result.file_path.clone(), content.clone());
                        content
                    }
                    Err(_) => continue, // Skip unreadable files
                }
            };

            // Find symbol position and calculate line number
            let chunk_start = result.start_offset;
            if let Some(symbol_pos) = result.text.find(&args.symbol) {
                let absolute_offset = chunk_start + symbol_pos;
                let line_number = byte_offset_to_line_number(&file_content, absolute_offset);

                // Match against patterns for confidence scoring
                let (pattern_name, base_confidence) = patterns
                    .iter()
                    .find(|(regex, _, _)| regex.is_match(&result.text))
                    .map(|(_, name, conf)| (*name, *conf))
                    .unwrap_or(("word_match", 0.60));

                // Extract context lines
                let context = extract_context_lines(&file_content, line_number, args.context_lines);

                // Adjust confidence based on context
                let confidence =
                    Self::adjust_confidence(base_confidence, &result.file_path, &context);

                references.push(Reference {
                    file_path: result.file_path,
                    line_number,
                    column: symbol_pos,
                    context,
                    pattern: pattern_name.to_string(),
                    confidence,
                });
            }
        }

        // Deduplicate (keep highest confidence per location)
        Self::deduplicate_references(&mut references);

        // Sort by confidence (descending) and truncate
        references.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        references.truncate(args.max_results);

        // Get session metadata for timestamp
        let session_metadata = self
            .services
            .storage
            .get_session_metadata(&args.session)
            .ok();

        // Format and return results
        let output = self.format_results(&args.symbol, &references, session_metadata.as_ref());
        Ok(text_content(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol_type() {
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("function".to_string())),
            SymbolType::Function
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("type".to_string())),
            SymbolType::Type
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("variable".to_string())),
            SymbolType::Variable
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("constant".to_string())),
            SymbolType::Constant
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("any".to_string())),
            SymbolType::Any
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&None),
            SymbolType::Any
        );
        assert_eq!(
            FindReferencesHandler::parse_symbol_type(&Some("unknown".to_string())),
            SymbolType::Any
        );
    }

    #[test]
    fn test_build_patterns_function() {
        let patterns = FindReferencesHandler::build_patterns("handleLogin", SymbolType::Function);

        // Should have function_call and method_call patterns
        let pattern_names: Vec<_> = patterns.iter().map(|(_, name, _)| *name).collect();
        assert!(pattern_names.contains(&"function_call"));
        assert!(pattern_names.contains(&"method_call"));
        assert!(pattern_names.contains(&"word_match")); // Fallback always included
    }

    #[test]
    fn test_build_patterns_type() {
        let patterns = FindReferencesHandler::build_patterns("MyType", SymbolType::Type);

        let pattern_names: Vec<_> = patterns.iter().map(|(_, name, _)| *name).collect();
        assert!(pattern_names.contains(&"type_annotation"));
        assert!(pattern_names.contains(&"return_type"));
        assert!(pattern_names.contains(&"generic_type"));
    }

    #[test]
    fn test_build_patterns_any() {
        let patterns = FindReferencesHandler::build_patterns("symbol", SymbolType::Any);

        // Should have patterns from all categories
        let pattern_names: Vec<_> = patterns.iter().map(|(_, name, _)| *name).collect();
        assert!(pattern_names.contains(&"function_call"));
        assert!(pattern_names.contains(&"type_annotation"));
        assert!(pattern_names.contains(&"assignment_target"));
        assert!(pattern_names.contains(&"import"));
    }

    #[test]
    fn test_function_call_pattern_matches() {
        let patterns = FindReferencesHandler::build_patterns("handleLogin", SymbolType::Function);
        let call_pattern = &patterns[0].0;

        assert!(call_pattern.is_match("handleLogin()"));
        assert!(call_pattern.is_match("handleLogin(ctx)"));
        assert!(call_pattern.is_match("result := handleLogin(req)"));
        assert!(call_pattern.is_match("handleLogin  ()")); // Whitespace ok
        assert!(!call_pattern.is_match("handleLoginError")); // No false positive
    }

    #[test]
    fn test_adjust_confidence_comment() {
        let base = 0.95;
        let adjusted =
            FindReferencesHandler::adjust_confidence(base, "src/auth.go", "// handleLogin comment");
        assert!(adjusted < base);
        assert!(adjusted < 0.70);
    }

    #[test]
    fn test_adjust_confidence_test_file() {
        let base = 0.90;
        let adjusted = FindReferencesHandler::adjust_confidence(
            base,
            "src/auth_test.go",
            "result := handleLogin(ctx)",
        );
        assert!(adjusted > base);
    }

    #[test]
    fn test_adjust_confidence_doc_file() {
        let base = 0.80;
        let adjusted =
            FindReferencesHandler::adjust_confidence(base, "docs/api.md", "The handleLogin func");
        assert!(adjusted < base);
    }

    #[test]
    fn test_adjust_confidence_string_literal() {
        let base = 0.80;
        let adjusted = FindReferencesHandler::adjust_confidence(
            base,
            "src/config.go",
            r#"name := "handleLogin""#,
        );
        assert!(adjusted < base);
    }

    #[test]
    fn test_adjust_confidence_clamp() {
        // Very negative adjustments should clamp to 0
        let adjusted = FindReferencesHandler::adjust_confidence(
            0.30,
            "docs/readme.md",
            "// handleLogin in string \"test\"",
        );
        assert!(adjusted >= 0.0);
        assert!(adjusted <= 1.0);
    }

    #[test]
    fn test_deduplicate_keeps_highest_confidence() {
        let mut refs = vec![
            Reference {
                file_path: "a.rs".to_string(),
                line_number: 10,
                column: 0,
                context: "".to_string(),
                pattern: "word_match".to_string(),
                confidence: 0.60,
            },
            Reference {
                file_path: "a.rs".to_string(),
                line_number: 10,
                column: 0,
                context: "".to_string(),
                pattern: "function_call".to_string(),
                confidence: 0.95,
            },
        ];

        FindReferencesHandler::deduplicate_references(&mut refs);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].confidence, 0.95);
        assert_eq!(refs[0].pattern, "function_call");
    }

    #[test]
    fn test_deduplicate_different_locations() {
        let mut refs = vec![
            Reference {
                file_path: "a.rs".to_string(),
                line_number: 10,
                column: 0,
                context: "".to_string(),
                pattern: "test".to_string(),
                confidence: 0.80,
            },
            Reference {
                file_path: "a.rs".to_string(),
                line_number: 20,
                column: 0,
                context: "".to_string(),
                pattern: "test".to_string(),
                confidence: 0.80,
            },
            Reference {
                file_path: "b.rs".to_string(),
                line_number: 10,
                column: 0,
                context: "".to_string(),
                pattern: "test".to_string(),
                confidence: 0.80,
            },
        ];

        FindReferencesHandler::deduplicate_references(&mut refs);

        // All three should remain (different locations)
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_symbol_with_regex_chars() {
        // Symbols containing regex metacharacters should be escaped
        let patterns = FindReferencesHandler::build_patterns("foo.bar", SymbolType::Any);
        let word_pattern = patterns.last().unwrap();

        assert!(word_pattern.0.is_match("foo.bar"));
        assert!(!word_pattern.0.is_match("fooXbar")); // . should not match any char
    }
}
