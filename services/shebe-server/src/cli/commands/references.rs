//! References command - find symbol references across indexed code
//!
//! This is the CLI equivalent of the `find_references` MCP tool.

use crate::cli::output::{colors, format_relative_time};
use crate::cli::OutputFormat;
use crate::core::services::Services;
use crate::core::storage::SessionMetadata;
use crate::core::types::SearchRequest;
use clap::Args;
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Arguments for the references command
#[derive(Args, Debug)]
pub struct ReferencesArgs {
    /// Symbol name to find references for (minimum 2 characters)
    pub symbol: String,

    /// Session ID to search
    #[arg(long, short = 's')]
    pub session: String,

    /// Symbol type hint for filtering by usage pattern
    #[arg(long, short = 't', default_value = "any")]
    pub symbol_type: SymbolTypeArg,

    /// File where symbol is defined (excluded from results)
    #[arg(long)]
    pub defined_in: Option<String>,

    /// Include definition site in results
    #[arg(long)]
    pub include_definition: bool,

    /// Lines of context around each reference (0-10)
    #[arg(long, short = 'c', default_value = "2")]
    pub context_lines: usize,

    /// Maximum number of references to return (1-500)
    #[arg(long, short = 'k', default_value = "50")]
    pub max_results: usize,
}

/// Symbol type for pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SymbolTypeArg {
    /// Match function/method calls (symbol(), .symbol())
    Function,
    /// Match type annotations (: symbol, <symbol>)
    Type,
    /// Match assignments and property access
    Variable,
    /// Same as variable
    Constant,
    /// Match all patterns (default)
    Any,
}

impl Default for SymbolTypeArg {
    fn default() -> Self {
        Self::Any
    }
}

/// A single reference to a symbol
#[derive(Debug, Serialize)]
pub struct Reference {
    pub file_path: String,
    pub line_number: usize,
    pub column: usize,
    pub context: String,
    pub pattern: String,
    pub confidence: f32,
}

/// References output response
#[derive(Debug, Serialize)]
pub struct ReferencesOutput {
    pub symbol: String,
    pub session: String,
    pub total_count: usize,
    pub high_confidence: usize,
    pub medium_confidence: usize,
    pub low_confidence: usize,
    pub unique_files: usize,
    pub references: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_indexed_at: Option<String>,
}

/// Build regex patterns for matching symbol usages based on symbol type.
fn build_patterns(symbol: &str, symbol_type: SymbolTypeArg) -> Vec<(Regex, &'static str, f32)> {
    let escaped = regex::escape(symbol);
    let mut patterns = Vec::new();

    // Function patterns
    match symbol_type {
        SymbolTypeArg::Function | SymbolTypeArg::Any => {
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
        SymbolTypeArg::Type | SymbolTypeArg::Any => {
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
        SymbolTypeArg::Variable | SymbolTypeArg::Constant | SymbolTypeArg::Any => {
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
    if file_path.ends_with(".md") || file_path.ends_with(".txt") || file_path.ends_with(".rst") {
        confidence -= 0.25;
    }

    confidence.clamp(0.0, 1.0)
}

/// Convert byte offset to line number (1-based).
fn byte_offset_to_line_number(content: &str, byte_offset: usize) -> usize {
    let bytes = content.as_bytes();
    let safe_offset = byte_offset.min(bytes.len());
    bytes[..safe_offset].iter().filter(|&&b| b == b'\n').count() + 1
}

/// Extract context lines around a line number.
fn extract_context_lines(content: &str, line_number: usize, context_count: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let line_idx = line_number.saturating_sub(1).min(lines.len() - 1);
    let start = line_idx.saturating_sub(context_count);
    let end = (line_idx + context_count + 1).min(lines.len());

    lines[start..end].join("\n")
}

/// Detect language from file extension.
fn detect_language(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "go" => "go",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "php" => "php",
        "rb" => "ruby",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "cs" => "csharp",
        "swift" => "swift",
        "sh" | "bash" | "zsh" => "bash",
        "sql" => "sql",
        "md" | "markdown" => "markdown",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "vue" => "vue",
        "svelte" => "svelte",
        _ => "",
    }
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

/// Format results for human-readable output.
fn format_human_output(output: &ReferencesOutput, session_metadata: Option<&SessionMetadata>) {
    if output.references.is_empty() {
        println!(
            "No references found for '{}' in session '{}'",
            colors::label(&output.symbol),
            colors::session_id(&output.session)
        );
        if let Some(meta) = session_metadata {
            println!(
                "Session last indexed: {} ({})",
                meta.last_indexed_at.format("%Y-%m-%d %H:%M:%S UTC"),
                format_relative_time(&meta.last_indexed_at)
            );
        }
        return;
    }

    // Group by confidence level
    let high: Vec<&Reference> = output
        .references
        .iter()
        .filter(|r| r.confidence >= 0.80)
        .collect();
    let medium: Vec<&Reference> = output
        .references
        .iter()
        .filter(|r| r.confidence >= 0.50 && r.confidence < 0.80)
        .collect();
    let low: Vec<&Reference> = output
        .references
        .iter()
        .filter(|r| r.confidence < 0.50)
        .collect();

    println!(
        "## References to '{}' ({} found)\n",
        colors::label(&output.symbol),
        colors::number(&output.total_count.to_string())
    );

    // High confidence
    if !high.is_empty() {
        println!(
            "### High Confidence ({})\n",
            colors::success(&high.len().to_string())
        );
        for r in &high {
            print_single_reference(r);
        }
    }

    // Medium confidence
    if !medium.is_empty() {
        println!(
            "### Medium Confidence ({})\n",
            colors::warning(&medium.len().to_string())
        );
        for r in &medium {
            print_single_reference(r);
        }
    }

    // Low confidence
    if !low.is_empty() {
        println!(
            "### Low Confidence ({})\n",
            colors::dim(&low.len().to_string())
        );
        for r in &low {
            print_single_reference(r);
        }
    }

    // Summary
    println!("---");
    println!("\nSummary:");
    println!(
        "  High confidence:   {} references",
        colors::success(&high.len().to_string())
    );
    println!(
        "  Medium confidence: {} references",
        colors::warning(&medium.len().to_string())
    );
    println!(
        "  Low confidence:    {} references",
        colors::dim(&low.len().to_string())
    );
    println!(
        "  Total files:       {}",
        colors::number(&output.unique_files.to_string())
    );

    // Session freshness
    if let Some(meta) = session_metadata {
        println!(
            "  Session indexed:   {} ({})",
            meta.last_indexed_at.format("%Y-%m-%d %H:%M:%S UTC"),
            format_relative_time(&meta.last_indexed_at)
        );
    }

    // Files to update (high confidence only)
    if !high.is_empty() {
        println!("\nFiles to update:");
        let high_files: HashSet<&str> = high.iter().map(|r| r.file_path.as_str()).collect();
        for file in high_files {
            println!("  {}", colors::file_path(file));
        }
    }
}

/// Print a single reference in human-readable format.
fn print_single_reference(r: &Reference) {
    let lang = detect_language(&r.file_path);
    println!(
        "#### {}:{}",
        colors::file_path(&r.file_path),
        colors::number(&r.line_number.to_string())
    );
    println!("```{lang}");
    println!("{}", r.context.trim());
    println!("```");
    println!("  Pattern: {}", colors::dim(&r.pattern));
    println!(
        "  Confidence: {}",
        colors::score(&format!("{:.2}", r.confidence))
    );
    println!();
}

/// Execute the references command
pub async fn execute(
    args: ReferencesArgs,
    services: &Arc<Services>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate session exists
    if !services.storage.session_exists(&args.session) {
        return Err(format!(
            "Session '{}' not found. Run 'shebe list-sessions' to see available sessions.",
            args.session
        )
        .into());
    }

    // Validate symbol
    let symbol = args.symbol.trim();
    if symbol.is_empty() {
        return Err("Symbol cannot be empty".into());
    }
    if symbol.len() < 2 {
        return Err("Symbol must be at least 2 characters".into());
    }

    // Clamp parameters
    let context_lines = args.context_lines.clamp(0, 10);
    let max_results = args.max_results.clamp(1, 500);

    // Search using SearchService
    let search_request = SearchRequest {
        query: symbol.to_string(),
        session: args.session.clone(),
        k: Some(max_results * 2), // Over-fetch to allow for filtering
    };
    let search_response = services.search.search(search_request)?;

    // Build patterns based on symbol_type
    let patterns = build_patterns(symbol, args.symbol_type);

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
        if let Some(symbol_pos) = result.text.find(symbol) {
            let absolute_offset = chunk_start + symbol_pos;
            let line_number = byte_offset_to_line_number(&file_content, absolute_offset);

            // Match against patterns for confidence scoring
            let (pattern_name, base_confidence) = patterns
                .iter()
                .find(|(regex, _, _)| regex.is_match(&result.text))
                .map(|(_, name, conf)| (*name, *conf))
                .unwrap_or(("word_match", 0.60));

            // Extract context lines
            let context = extract_context_lines(&file_content, line_number, context_lines);

            // Adjust confidence based on context
            let confidence = adjust_confidence(base_confidence, &result.file_path, &context);

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
    deduplicate_references(&mut references);

    // Sort by confidence (descending) and truncate
    references.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    references.truncate(max_results);

    // Get session metadata for timestamp
    let session_metadata = services.storage.get_session_metadata(&args.session).ok();

    // Count by confidence level
    let high_count = references.iter().filter(|r| r.confidence >= 0.80).count();
    let medium_count = references
        .iter()
        .filter(|r| r.confidence >= 0.50 && r.confidence < 0.80)
        .count();
    let low_count = references.iter().filter(|r| r.confidence < 0.50).count();
    let unique_files: HashSet<_> = references.iter().map(|r| &r.file_path).collect();

    let output = ReferencesOutput {
        symbol: symbol.to_string(),
        session: args.session.clone(),
        total_count: references.len(),
        high_confidence: high_count,
        medium_confidence: medium_count,
        low_confidence: low_count,
        unique_files: unique_files.len(),
        references,
        session_indexed_at: session_metadata
            .as_ref()
            .map(|m| m.last_indexed_at.to_rfc3339()),
    };

    match format {
        OutputFormat::Human => {
            format_human_output(&output, session_metadata.as_ref());
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
