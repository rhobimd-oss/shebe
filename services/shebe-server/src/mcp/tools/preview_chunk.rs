//! Preview chunk with context tool handler
//!
//! Provides context expansion for search results by showing N lines before and after a chunk.

use super::handler::{text_content, McpToolHandler};
use super::helpers::detect_language;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use crate::mcp::services::ShebeServices;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{Term, Value as TantivyValue};
use tantivy::{IndexReader, TantivyDocument};

const DEFAULT_CONTEXT_LINES: usize = 10;
const MAX_CONTEXT_LINES: usize = 100;

pub struct PreviewChunkHandler {
    services: Arc<ShebeServices>,
}

impl PreviewChunkHandler {
    pub fn new(services: Arc<ShebeServices>) -> Self {
        Self { services }
    }

    /// Get chunk metadata from Tantivy
    async fn get_chunk_metadata(
        &self,
        session: &str,
        file_path: &str,
        chunk_index: usize,
    ) -> Result<ChunkMetadata, McpError> {
        // Open session index
        let index = self
            .services
            .storage
            .open_session(session)
            .map_err(McpError::from)?;

        let reader: IndexReader = index
            .index()
            .reader()
            .map_err(|e| McpError::InternalError(format!("Failed to open reader: {e}")))?;

        let searcher = reader.searcher();
        let schema = index.schema();

        // Get required fields
        let file_path_field = schema
            .get_field("file_path")
            .map_err(|e| McpError::InternalError(format!("file_path field missing: {e}")))?;
        let chunk_index_field = schema
            .get_field("chunk_index")
            .map_err(|e| McpError::InternalError(format!("chunk_index field missing: {e}")))?;
        let offset_start_field = schema
            .get_field("offset_start")
            .map_err(|e| McpError::InternalError(format!("offset_start field missing: {e}")))?;
        let offset_end_field = schema
            .get_field("offset_end")
            .map_err(|e| McpError::InternalError(format!("offset_end field missing: {e}")))?;

        // Query for specific chunk
        let file_term = Term::from_field_text(file_path_field, file_path);
        let chunk_term = Term::from_field_i64(chunk_index_field, chunk_index as i64);

        let file_query: Box<dyn Query> = Box::new(TermQuery::new(file_term, Default::default()));
        let chunk_query: Box<dyn Query> = Box::new(TermQuery::new(chunk_term, Default::default()));

        let query = BooleanQuery::new(vec![(Occur::Must, file_query), (Occur::Must, chunk_query)]);

        let top_docs = searcher
            .search(&query, &tantivy::collector::TopDocs::with_limit(1))
            .map_err(|e| McpError::InternalError(format!("Search failed: {e}")))?;

        if top_docs.is_empty() {
            return Err(McpError::InvalidRequest(format!(
                "Chunk not found: file '{file_path}', chunk index {chunk_index}. \
                 File may not be indexed or chunk index invalid."
            )));
        }

        // Extract chunk metadata
        let (_score, doc_address) = &top_docs[0];
        let retrieved_doc: TantivyDocument = searcher
            .doc(*doc_address)
            .map_err(|e| McpError::InternalError(format!("Doc retrieval failed: {e}")))?;

        let offset_start = retrieved_doc
            .get_first(offset_start_field)
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::InternalError("Missing offset_start".to_string()))?
            as usize;

        let offset_end = retrieved_doc
            .get_first(offset_end_field)
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::InternalError("Missing offset_end".to_string()))?
            as usize;

        Ok(ChunkMetadata {
            file_path: file_path.to_string(),
            chunk_index,
            offset_start,
            offset_end,
        })
    }

    /// Read file and extract lines with context
    fn extract_context_lines(
        &self,
        file_path: &Path,
        chunk_metadata: &ChunkMetadata,
        context_lines: usize,
    ) -> Result<ContextExtraction, McpError> {
        // Read file
        let contents = std::fs::read_to_string(file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                McpError::InvalidRequest(format!(
                    "File not found: {}. May have been deleted or moved since indexing. \
                     Try re-indexing the session.",
                    file_path.display()
                ))
            } else if e.kind() == std::io::ErrorKind::InvalidData {
                McpError::InvalidRequest("File contains non-UTF-8 data (binary file).".to_string())
            } else {
                McpError::InternalError(format!("Failed to read file: {e}"))
            }
        })?;

        // Convert byte offsets to line numbers
        let line_info = self.offset_to_lines(
            &contents,
            chunk_metadata.offset_start,
            chunk_metadata.offset_end,
        )?;

        // Calculate context boundaries
        let start_line = line_info.start_line.saturating_sub(context_lines);
        let end_line = (line_info.end_line + context_lines).min(line_info.total_lines - 1);

        // Extract lines
        let all_lines: Vec<&str> = contents.lines().collect();
        let context_lines_vec: Vec<String> = all_lines[start_line..=end_line]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = start_line + i + 1; // 1-indexed
                format!("{line_num:4} | {line}")
            })
            .collect();

        Ok(ContextExtraction {
            lines: context_lines_vec,
            chunk_start_line: line_info.start_line + 1, // 1-indexed
            chunk_end_line: line_info.end_line + 1,     // 1-indexed
            context_start_line: start_line + 1,         // 1-indexed
            context_end_line: end_line + 1,             // 1-indexed
            total_lines: line_info.total_lines,
        })
    }

    /// Convert byte offsets to line numbers
    fn offset_to_lines(
        &self,
        text: &str,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<LineInfo, McpError> {
        let mut line_num = 0;
        let mut current_offset = 0;
        let mut start_line = None;
        let mut end_line = None;

        for line in text.lines() {
            let line_len = line.len() + 1; // +1 for newline

            // Check if chunk starts in this line
            if start_line.is_none()
                && current_offset <= start_offset
                && start_offset < current_offset + line_len
            {
                start_line = Some(line_num);
            }

            // Check if chunk ends in this line
            if end_line.is_none()
                && current_offset <= end_offset
                && end_offset <= current_offset + line_len
            {
                end_line = Some(line_num);
            }

            current_offset += line_len;
            line_num += 1;
        }

        Ok(LineInfo {
            start_line: start_line.ok_or_else(|| {
                McpError::InternalError("Could not determine chunk start line".to_string())
            })?,
            end_line: end_line.ok_or_else(|| {
                McpError::InternalError("Could not determine chunk end line".to_string())
            })?,
            total_lines: line_num,
        })
    }

    /// Format preview with chunk boundaries
    fn format_preview(
        &self,
        extraction: &ContextExtraction,
        file_path: &str,
        session: &str,
    ) -> String {
        let lang = detect_language(file_path);

        let mut output = format!(
            "**File:** `{}`\n\
             **Session:** `{}`\n\
             **Chunk Lines:** {}-{} (of {} total)\n\
             **Context:** {} lines before + {} lines after\n\n",
            file_path,
            session,
            extraction.chunk_start_line,
            extraction.chunk_end_line,
            extraction.total_lines,
            extraction.chunk_start_line - extraction.context_start_line,
            extraction.context_end_line - extraction.chunk_end_line
        );

        // Add visual chunk boundaries
        output.push_str(&format!("```{lang}\n"));

        for (i, line) in extraction.lines.iter().enumerate() {
            let line_num = extraction.context_start_line + i;

            // Mark chunk boundaries
            if line_num == extraction.chunk_start_line {
                output.push_str("┌─ CHUNK START ─────────────────────\n");
            }

            output.push_str(line);
            output.push('\n');

            if line_num == extraction.chunk_end_line {
                output.push_str("└─ CHUNK END ───────────────────────\n");
            }
        }

        output.push_str("```\n");
        output
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct ChunkMetadata {
    file_path: String,
    chunk_index: usize,
    offset_start: usize,
    offset_end: usize,
}

#[derive(Debug)]
struct LineInfo {
    start_line: usize,
    end_line: usize,
    total_lines: usize,
}

#[derive(Debug)]
struct ContextExtraction {
    lines: Vec<String>,
    chunk_start_line: usize,
    chunk_end_line: usize,
    context_start_line: usize,
    context_end_line: usize,
    total_lines: usize,
}

#[async_trait]
impl McpToolHandler for PreviewChunkHandler {
    fn name(&self) -> &str {
        "preview_chunk"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "preview_chunk".to_string(),
            description: "Show N lines before and after a search result chunk. \
                         Provides context expansion without retrieving the entire file. \
                         Use when search results need more surrounding code for understanding. \
                         Shows chunk boundaries with visual markers and line numbers. \
                         Default: 10 lines context (configurable, max 100)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID containing the chunk",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Absolute file path (from search result)",
                        "minLength": 1
                    },
                    "chunk_index": {
                        "type": "integer",
                        "description": "Chunk index (from search result)",
                        "minimum": 0
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Lines before/after chunk (default: 10, max: 100)",
                        "default": 10,
                        "minimum": 0,
                        "maximum": 100
                    }
                },
                "required": ["session", "file_path", "chunk_index"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct PreviewArgs {
            session: String,
            file_path: String,
            chunk_index: usize,
            #[serde(default = "default_context_lines")]
            context_lines: usize,
        }
        fn default_context_lines() -> usize {
            DEFAULT_CONTEXT_LINES
        }

        // Parse arguments
        let args: PreviewArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Validate context_lines
        if args.context_lines > MAX_CONTEXT_LINES {
            return Err(McpError::InvalidParams(format!(
                "context_lines cannot exceed {MAX_CONTEXT_LINES}"
            )));
        }

        // Get chunk metadata from Tantivy
        let chunk_metadata = self
            .get_chunk_metadata(&args.session, &args.file_path, args.chunk_index)
            .await?;

        // Extract context from file
        let path = Path::new(&args.file_path);
        let extraction = self.extract_context_lines(path, &chunk_metadata, args.context_lines)?;

        // Format response
        let formatted = self.format_preview(&extraction, &args.file_path, &args.session);

        Ok(text_content(formatted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_lines_single_line() {
        let text = "line1\nline2\nline3\n";
        let handler = create_test_handler();

        let result = handler.offset_to_lines(text, 6, 11).unwrap();

        assert_eq!(result.start_line, 1); // line2
        assert_eq!(result.end_line, 1);
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn test_offset_to_lines_multiline() {
        let text = "line1\nline2\nline3\n";
        let handler = create_test_handler();

        let result = handler.offset_to_lines(text, 0, 17).unwrap();

        assert_eq!(result.start_line, 0); // line1
        assert_eq!(result.end_line, 2); // line3
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn test_offset_to_lines_at_start() {
        let text = "line1\nline2\nline3\n";
        let handler = create_test_handler();

        let result = handler.offset_to_lines(text, 0, 5).unwrap();

        assert_eq!(result.start_line, 0);
        assert_eq!(result.end_line, 0);
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn test_offset_to_lines_at_end() {
        let text = "line1\nline2\nline3\n";
        let handler = create_test_handler();

        let result = handler.offset_to_lines(text, 12, 17).unwrap();

        assert_eq!(result.start_line, 2);
        assert_eq!(result.end_line, 2);
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn test_offset_to_lines_utf8() {
        let text = "hello 世界\nemoji 🚀\ntest\n";
        let handler = create_test_handler();

        // Line 0: "hello 世界" = 12 bytes (0-11), newline at 12
        // Line 1: "emoji 🚀" = 10 bytes (13-22), newline at 23
        // Line 2: "test" = 4 bytes (24-27), newline at 28
        // Byte 15 is on line 1, byte 27 is on line 2
        let result = handler.offset_to_lines(text, 15, 27).unwrap();

        assert_eq!(result.start_line, 1);
        assert_eq!(result.end_line, 2);
    }

    #[test]
    fn test_default_context_lines() {
        assert_eq!(DEFAULT_CONTEXT_LINES, 10);
    }

    #[test]
    fn test_max_context_lines() {
        assert_eq!(MAX_CONTEXT_LINES, 100);
    }

    #[test]
    fn test_preview_chunk_handler_name() {
        let handler = create_test_handler();
        assert_eq!(handler.name(), "preview_chunk");
    }

    // Helper function to create test handler
    fn create_test_handler() -> PreviewChunkHandler {
        let config = crate::config::Config::default();
        let services = Arc::new(crate::mcp::services::ShebeServices::new(config));

        PreviewChunkHandler::new(services)
    }
}
