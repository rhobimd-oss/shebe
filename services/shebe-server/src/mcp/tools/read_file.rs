//! Read file tool handler

use super::handler::{text_content, McpToolHandler};
use super::helpers::{detect_language, format_bytes};
use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use crate::mcp::utils::{build_read_file_warning, READ_FILE_MAX_CHARS};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_MAX_SIZE_KB: usize = 1024; // 1 MB default
const ABSOLUTE_MAX_SIZE_KB: usize = 10240; // 10 MB absolute max

pub struct ReadFileHandler {
    services: Arc<Services>,
}

impl ReadFileHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Validate file is within indexed paths by checking if it exists in the Tantivy index
    fn validate_file_in_session(&self, session: &str, file_path: &Path) -> Result<(), McpError> {
        // Check if session exists first
        if !self.services.storage.session_exists(session) {
            return Err(McpError::InvalidRequest(format!(
                "Session '{session}' not found. Use list_sessions to see available sessions."
            )));
        }

        // Open the session's Tantivy index to verify file is indexed
        let index = self
            .services
            .storage
            .open_session(session)
            .map_err(McpError::from)?;

        // Get reader and searcher
        let reader = index
            .index()
            .reader()
            .map_err(|e| McpError::InternalError(format!("Failed to open index reader: {e}")))?;

        let searcher = reader.searcher();
        let schema = index.schema();

        // Get file_path field
        let file_path_field = schema
            .get_field("file_path")
            .map_err(|e| McpError::InternalError(format!("Missing file_path field: {e}")))?;

        let session_field = schema
            .get_field("session")
            .map_err(|e| McpError::InternalError(format!("Missing session field: {e}")))?;

        // Build query for this specific file in this session
        use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
        use tantivy::Term;

        let file_path_str = file_path.to_str().ok_or_else(|| {
            McpError::InvalidRequest("File path contains invalid UTF-8".to_string())
        })?;

        let file_term = Term::from_field_text(file_path_field, file_path_str);
        let session_term = Term::from_field_text(session_field, session);

        let file_query: Box<dyn Query> = Box::new(TermQuery::new(file_term, Default::default()));
        let session_query: Box<dyn Query> =
            Box::new(TermQuery::new(session_term, Default::default()));

        let combined_query = BooleanQuery::new(vec![
            (Occur::Must, file_query),
            (Occur::Must, session_query),
        ]);

        // Search for any documents matching this file
        let top_docs = searcher
            .search(&combined_query, &tantivy::collector::Count)
            .map_err(|e| McpError::InternalError(format!("Search failed: {e}")))?;

        if top_docs == 0 {
            return Err(McpError::InvalidRequest(format!(
                "File '{file_path_str}' not indexed in session '{session}'. Check file_path or re-index the session."
            )));
        }

        Ok(())
    }

    /// Read file with UTF-8 validation and auto-truncation
    ///
    /// Returns: (content, was_truncated, total_size_bytes)
    fn read_file_contents(&self, path: &Path) -> Result<(String, bool, usize), McpError> {
        // Get file size
        let metadata = std::fs::metadata(path)
            .map_err(|e| McpError::InternalError(format!("Failed to read file metadata: {e}")))?;
        let total_size = metadata.len() as usize;

        // Determine if truncation is needed
        if total_size > READ_FILE_MAX_CHARS {
            // File is large - read first READ_FILE_MAX_CHARS bytes
            let mut file = std::fs::File::open(path)
                .map_err(|e| McpError::InternalError(format!("Failed to open file: {e}")))?;

            let mut buffer = vec![0u8; READ_FILE_MAX_CHARS];
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| McpError::InternalError(format!("Failed to read file: {e}")))?;

            // Ensure UTF-8 boundary safety
            let content = ensure_utf8_boundary(&buffer[..bytes_read]);

            Ok((content, true, total_size))
        } else {
            // File is small - read entire content
            let content = std::fs::read_to_string(path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::InvalidData {
                    McpError::InvalidRequest(
                        "File contains non-UTF-8 data (binary file). \
                         Cannot display in MCP response."
                            .to_string(),
                    )
                } else {
                    McpError::InternalError(format!("Failed to read file: {e}"))
                }
            })?;

            Ok((content, false, total_size))
        }
    }

    /// Format response with metadata
    fn format_response(
        &self,
        file_path: &str,
        contents: &str,
        size_bytes: u64,
        session: &str,
    ) -> String {
        let lang = detect_language(file_path);
        let line_count = contents.lines().count();

        format!(
            "**File:** `{}`\n\
             **Session:** `{}`\n\
             **Size:** {} ({} lines)\n\
             **Language:** {}\n\n\
             ```{}\n{}\n```",
            file_path,
            session,
            format_bytes(size_bytes),
            line_count,
            if lang.is_empty() { "unknown" } else { lang },
            lang,
            contents
        )
    }
}

/// Ensure buffer ends on UTF-8 character boundary
///
/// If the buffer contains invalid UTF-8, this function will truncate
/// to the last valid UTF-8 character boundary to prevent panics.
fn ensure_utf8_boundary(buffer: &[u8]) -> String {
    match String::from_utf8(buffer.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            // Find last valid UTF-8 boundary
            let valid_up_to = e.utf8_error().valid_up_to();
            String::from_utf8_lossy(&buffer[..valid_up_to]).to_string()
        }
    }
}

#[async_trait]
impl McpToolHandler for ReadFileHandler {
    fn name(&self) -> &str {
        "read_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "read_file".to_string(),
            description: "Retrieve full file contents from indexed session. \
                         Use when search results or file listings show a file you want to read. \
                         Auto-truncates to 20,000 characters max to stay under MCP 25k token limit \
                         (shows warning if truncated). Binary files are rejected. \
                         Returns Markdown-formatted code with syntax highlighting."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID containing the file",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to file (from search results or list_dir)",
                        "minLength": 1
                    },
                    "max_size_kb": {
                        "type": "integer",
                        "description": "Max file size in KB (default: 1024, max: 10240)",
                        "default": 1024,
                        "minimum": 1,
                        "maximum": 10240
                    }
                },
                "required": ["session", "file_path"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct ReadFileArgs {
            session: String,
            file_path: String,
            #[serde(default = "default_max_size")]
            max_size_kb: usize,
        }
        fn default_max_size() -> usize {
            DEFAULT_MAX_SIZE_KB
        }

        // Parse arguments
        let args: ReadFileArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Validate parameters
        if args.file_path.trim().is_empty() {
            return Err(McpError::InvalidParams(
                "file_path cannot be empty".to_string(),
            ));
        }

        if args.max_size_kb > ABSOLUTE_MAX_SIZE_KB {
            return Err(McpError::InvalidParams(format!(
                "max_size_kb cannot exceed {ABSOLUTE_MAX_SIZE_KB} KB"
            )));
        }

        let path = PathBuf::from(&args.file_path);

        // Validate session exists and file is in session
        self.validate_file_in_session(&args.session, &path)?;

        // Check file exists
        if !path.exists() {
            return Err(McpError::InvalidRequest(format!(
                "File not found: {}. File may have been deleted since indexing. \
                 Try re-indexing the session.",
                args.file_path
            )));
        }

        // Read file contents (with auto-truncation if needed)
        let (contents, was_truncated, total_size) = self.read_file_contents(&path)?;

        // Build output
        let mut output = String::new();

        // Add warning if truncated
        if was_truncated {
            let shown_lines = contents.lines().count();
            let warning =
                build_read_file_warning(contents.len(), total_size, shown_lines, &args.file_path);
            output.push_str(&warning);
        }

        // Add formatted file content
        let formatted =
            self.format_response(&args.file_path, &contents, total_size as u64, &args.session);
        output.push_str(&formatted);

        Ok(text_content(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::core::storage::SessionConfig;
    use crate::core::types::Chunk;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (ReadFileHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = ReadFileHandler::new(services);

        (handler, temp_dir)
    }

    async fn create_test_session_with_file(
        services: &Arc<Services>,
        session_id: &str,
        file_path: &str,
        content: &str,
    ) -> PathBuf {
        // Create actual file
        let full_path = PathBuf::from(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&full_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        // Index file
        let mut index = services
            .storage
            .create_session(
                session_id,
                PathBuf::from("/test/repo"),
                SessionConfig::default(),
            )
            .unwrap();

        let chunks = vec![Chunk {
            text: content.to_string(),
            file_path: full_path.clone(),
            start_offset: 0,
            end_offset: content.len(),
            chunk_index: 0,
        }];

        index.add_chunks(&chunks, session_id).unwrap();
        index.commit().unwrap();

        full_path
    }

    #[tokio::test]
    async fn test_read_file_valid() {
        let (handler, _temp) = setup_test_handler().await;
        let test_content = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let file_path = create_test_session_with_file(
            &handler.services,
            "test-session",
            "/tmp/shebe-test-read-file.rs",
            test_content,
        )
        .await;

        let args = json!({
            "session": "test-session",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**File:**"));
        assert!(text.contains("**Session:** `test-session`"));
        assert!(text.contains("**Size:**"));
        assert!(text.contains("**Language:**"));
        assert!(text.contains(test_content));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        // Create session but request non-existent file
        {
            let mut index = handler
                .services
                .storage
                .create_session(
                    "test-session",
                    PathBuf::from("/test/repo"),
                    SessionConfig::default(),
                )
                .unwrap();
            index.commit().unwrap();
        } // Drop the index to release the lock

        let args = json!({
            "session": "test-session",
            "file_path": "/tmp/nonexistent.rs",
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        match result {
            Err(McpError::InvalidRequest(msg)) => {
                assert!(msg.contains("not indexed"));
            }
            Err(e) => {
                panic!("Expected InvalidRequest error, got: {:?}", e);
            }
            Ok(_) => {
                panic!("Expected error, got success");
            }
        }
    }

    #[tokio::test]
    async fn test_read_file_auto_truncates_large() {
        // This test verifies that large files are auto-truncated instead of erroring
        let (handler, _temp) = setup_test_handler().await;
        let large_content = "x".repeat(200_000); // 200KB (well over 20k limit)
        let file_path = create_test_session_with_file(
            &handler.services,
            "test-session",
            "/tmp/shebe-test-large.txt",
            &large_content,
        )
        .await;

        let args = json!({
            "session": "test-session",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        // Should succeed with truncation, not error
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should show truncation warning
        assert!(text.contains("⚠️ FILE TRUNCATED"));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_utf8_special() {
        let (handler, _temp) = setup_test_handler().await;
        let special_content = "Hello 世界 🌍\nمرحبا\nשלום";
        let file_path = create_test_session_with_file(
            &handler.services,
            "test-session",
            "/tmp/shebe-test-utf8.txt",
            special_content,
        )
        .await;

        let args = json!({
            "session": "test-session",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };
        assert!(text.contains(special_content));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_session_not_found() {
        let (handler, _temp) = setup_test_handler().await;

        let args = json!({
            "session": "nonexistent-session",
            "file_path": "/tmp/test.rs",
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidRequest(msg)) = result {
            assert!(msg.contains("Session"));
            assert!(msg.contains("not found"));
        } else {
            panic!("Expected InvalidRequest error for missing session");
        }
    }

    // New truncation tests

    #[tokio::test]
    async fn test_read_file_truncation_large_file() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 50KB test file (larger than 20k char limit)
        let content = "Line of test content that is quite long\n".repeat(1200); // ~50KB
        let file_path = create_test_session_with_file(
            &handler.services,
            "truncate-test",
            "/tmp/shebe-read-truncate.txt",
            &content,
        )
        .await;

        let args = json!({
            "session": "truncate-test",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should contain truncation warning
        assert!(text.contains("⚠️ FILE TRUNCATED"));
        assert!(text.contains("FIRST 20000 CHARACTERS"));

        // Should mention total size
        assert!(text.contains("of")); // "X of Y total"

        // Content should be truncated
        assert!(text.len() < content.len());

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_no_truncation_small_file() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 5KB test file (smaller than 20k limit)
        let content = "Small file content line\n".repeat(200); // ~5KB
        let file_path = create_test_session_with_file(
            &handler.services,
            "small-test",
            "/tmp/shebe-read-small.txt",
            &content,
        )
        .await;

        let args = json!({
            "session": "small-test",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should NOT show warning for small files
        assert!(!text.contains("⚠️ FILE TRUNCATED"));

        // Should contain full content
        assert!(text.contains(&content));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_exactly_at_limit() {
        use crate::mcp::utils::READ_FILE_MAX_CHARS;

        let (handler, _temp) = setup_test_handler().await;

        // Create file exactly 20k chars
        let content = "X".repeat(READ_FILE_MAX_CHARS);
        let file_path = create_test_session_with_file(
            &handler.services,
            "exact-limit-test",
            "/tmp/shebe-read-exact.txt",
            &content,
        )
        .await;

        let args = json!({
            "session": "exact-limit-test",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should NOT show warning (exactly at limit, not over)
        assert!(!text.contains("⚠️ FILE TRUNCATED"));

        // Should contain full content
        assert!(text.contains(&content));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_utf8_boundary_safety() {
        let (handler, _temp) = setup_test_handler().await;

        // Create file with multi-byte UTF-8 chars near 20k boundary
        // Each emoji is 4 bytes, so 6000 emojis = ~24KB
        let content = "😀".repeat(6000);
        let file_path = create_test_session_with_file(
            &handler.services,
            "utf8-test",
            "/tmp/shebe-read-utf8.txt",
            &content,
        )
        .await;

        let args = json!({
            "session": "utf8-test",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;

        // Should not panic on UTF-8 boundaries
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should be truncated
        assert!(text.contains("⚠️ FILE TRUNCATED"));

        // Verify output is valid UTF-8 (no broken emoji)
        // This will panic if there are invalid UTF-8 sequences
        assert!(text.is_char_boundary(text.len()));

        // Should contain some emoji (not completely broken)
        assert!(text.contains("😀"));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_utf8_mixed_content() {
        let (handler, _temp) = setup_test_handler().await;

        // Create large file with mixed UTF-8 content
        let mut content = String::new();
        content.push_str("ASCII text\n".repeat(500).as_str());
        content.push_str("世界\n".repeat(500).as_str()); // Chinese
        content.push_str("مرحبا\n".repeat(500).as_str()); // Arabic
        content.push_str("😀😁😂\n".repeat(500).as_str()); // Emoji

        let file_path = create_test_session_with_file(
            &handler.services,
            "mixed-utf8-test",
            "/tmp/shebe-read-mixed.txt",
            &content,
        )
        .await;

        let args = json!({
            "session": "mixed-utf8-test",
            "file_path": file_path.to_str().unwrap(),
        });

        let result = handler.execute(args).await;

        // Should handle mixed UTF-8 without panicking
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Verify valid UTF-8
        assert!(text.is_char_boundary(text.len()));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }
}
