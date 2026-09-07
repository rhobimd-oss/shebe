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
use std::io::{Read, Seek, SeekFrom};
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

    /// Validate file is within indexed paths
    fn validate_file_in_session(&self, session: &str, file_path: &Path) -> Result<(), McpError> {
        // Check if session exists first
        if !self.services.storage.session_exists(session) {
            return Err(McpError::InvalidRequest(format!(
                "Session '{session}' not found. \
                 Use list_sessions to see available sessions."
            )));
        }

        // Open the session's Tantivy index to verify file
        let index = self
            .services
            .storage
            .open_session_read(session)
            .map_err(McpError::from)?;

        let reader = index
            .reader()
            .map_err(|e| McpError::InternalError(format!("Failed to open index reader: {e}")))?;

        let searcher = reader.searcher();
        let schema = index.schema();

        let file_path_field = schema
            .get_field("file_path")
            .map_err(|e| McpError::InternalError(format!("Missing file_path field: {e}")))?;

        let session_field = schema
            .get_field("session")
            .map_err(|e| McpError::InternalError(format!("Missing session field: {e}")))?;

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

        let top_docs = searcher
            .search(&combined_query, &tantivy::collector::Count)
            .map_err(|e| McpError::InternalError(format!("Search failed: {e}")))?;

        if top_docs == 0 {
            return Err(McpError::InvalidRequest(format!(
                "File '{file_path_str}' not indexed in \
                 session '{session}'. Check file_path or \
                 re-index the session."
            )));
        }

        Ok(())
    }

    /// Read file with UTF-8 validation and auto-truncation
    ///
    /// Returns: (content, was_truncated, total_size_bytes)
    fn read_file_contents(&self, path: &Path) -> Result<(String, bool, usize), McpError> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| McpError::InternalError(format!("Failed to read file metadata: {e}")))?;
        let total_size = metadata.len() as usize;

        if total_size > READ_FILE_MAX_CHARS {
            let mut file = std::fs::File::open(path)
                .map_err(|e| McpError::InternalError(format!("Failed to open file: {e}")))?;

            let mut buffer = vec![0u8; READ_FILE_MAX_CHARS];
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| McpError::InternalError(format!("Failed to read file: {e}")))?;

            let content = ensure_utf8_boundary(&buffer[..bytes_read]);

            Ok((content, true, total_size))
        } else {
            let content = std::fs::read_to_string(path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::InvalidData {
                    McpError::InvalidRequest(
                        "File contains non-UTF-8 data \
                             (binary file). Cannot display \
                             in MCP response."
                            .to_string(),
                    )
                } else {
                    McpError::InternalError(format!("Failed to read file: {e}"))
                }
            })?;

            Ok((content, false, total_size))
        }
    }

    /// Read a chunk of file starting at byte offset
    ///
    /// Returns: (content, bytes_consumed, total_size_bytes)
    fn read_file_chunk(
        &self,
        path: &Path,
        offset: usize,
        length: usize,
    ) -> Result<(String, usize, usize), McpError> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| McpError::InternalError(format!("Failed to read file metadata: {e}")))?;
        let total_size = metadata.len() as usize;

        if offset >= total_size {
            return Ok((String::new(), 0, total_size));
        }

        let mut file = std::fs::File::open(path)
            .map_err(|e| McpError::InternalError(format!("Failed to open file: {e}")))?;

        // Seek to offset
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| McpError::InternalError(format!("Failed to seek in file: {e}")))?;

        // Read up to length bytes
        let remaining = total_size - offset;
        let read_size = length.min(remaining);
        let mut buffer = vec![0u8; read_size];
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| McpError::InternalError(format!("Failed to read file: {e}")))?;

        // Handle UTF-8 boundary at start: if we landed in the
        // middle of a multi-byte character, skip forward to the
        // next valid boundary.
        let start_skip = if offset > 0 {
            find_utf8_start(&buffer[..bytes_read])
        } else {
            0
        };

        // Handle UTF-8 boundary at end
        let content = ensure_utf8_boundary(&buffer[start_skip..bytes_read]);
        let bytes_consumed = bytes_read;

        Ok((content, bytes_consumed, total_size))
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

    /// Format response with byte range info for offset reads
    fn format_response_with_offset(
        &self,
        file_path: &str,
        contents: &str,
        total_size: usize,
        offset: usize,
        bytes_consumed: usize,
        session: &str,
    ) -> String {
        let lang = detect_language(file_path);
        let line_count = contents.lines().count();
        let end_byte = offset + bytes_consumed;

        format!(
            "**File:** `{}`\n\
             **Session:** `{}`\n\
             **Size:** {} (showing bytes {}-{} of {})\n\
             **Language:** {} ({} lines in chunk)\n\n\
             ```{}\n{}\n```",
            file_path,
            session,
            format_bytes(total_size as u64),
            offset,
            end_byte,
            total_size,
            if lang.is_empty() { "unknown" } else { lang },
            line_count,
            lang,
            contents
        )
    }
}

/// Ensure buffer ends on UTF-8 character boundary
///
/// If the buffer contains invalid UTF-8, this function will
/// truncate to the last valid UTF-8 character boundary.
fn ensure_utf8_boundary(buffer: &[u8]) -> String {
    match String::from_utf8(buffer.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            let valid_up_to = e.utf8_error().valid_up_to();
            String::from_utf8_lossy(&buffer[..valid_up_to]).to_string()
        }
    }
}

/// Find the start of the first valid UTF-8 character in buffer.
///
/// When seeking to an arbitrary byte offset, we may land in the
/// middle of a multi-byte UTF-8 sequence. Continuation bytes
/// have the pattern 10xxxxxx (0x80..0xBF). Skip them to find
/// the next character start.
fn find_utf8_start(buffer: &[u8]) -> usize {
    for (i, &byte) in buffer.iter().enumerate() {
        // A byte that is NOT a continuation byte (10xxxxxx)
        // is a valid character start
        if byte & 0b1100_0000 != 0b1000_0000 {
            return i;
        }
    }
    buffer.len() // entire buffer is continuation bytes
}

#[async_trait]
impl McpToolHandler for ReadFileHandler {
    fn name(&self) -> &str {
        "read_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "read_file".to_string(),
            description: "Retrieve full file contents from \
                indexed session. Use when search results or file \
                listings show a file you want to read. \
                Auto-truncates to 20,000 characters max to stay \
                under MCP 25k token limit (shows warning if \
                truncated). Binary files are rejected. Returns \
                Markdown-formatted code with syntax highlighting. \
                Supports offset-based pagination for reading large \
                files incrementally."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description":
                            "Session ID containing the file",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "file_path": {
                        "type": "string",
                        "description":
                            "Absolute path to file \
                             (from search results or list_dir)",
                        "minLength": 1
                    },
                    "max_size_kb": {
                        "type": "integer",
                        "description":
                            "Max file size in KB \
                             (default: 1024, max: 10240)",
                        "default": 1024,
                        "minimum": 1,
                        "maximum": 10240
                    },
                    "offset": {
                        "type": "integer",
                        "description":
                            "Byte offset to start reading from. \
                             Use the next-offset value from a \
                             previous response to continue \
                             reading. Default: 0 (start).",
                        "default": 0,
                        "minimum": 0
                    },
                    "length": {
                        "type": "integer",
                        "description":
                            "Max bytes to read from offset. \
                             Capped at 20000 characters. \
                             Default: 20000.",
                        "default": 20000,
                        "minimum": 1,
                        "maximum": 20000
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
            offset: Option<usize>,
            length: Option<usize>,
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
                "max_size_kb cannot exceed \
                 {ABSOLUTE_MAX_SIZE_KB} KB"
            )));
        }

        let path = PathBuf::from(&args.file_path);

        // Validate session exists and file is in session
        self.validate_file_in_session(&args.session, &path)?;

        // Check file exists
        if !path.exists() {
            return Err(McpError::InvalidRequest(format!(
                "File not found: {}. File may have been \
                 deleted since indexing. Try re-indexing \
                 the session.",
                args.file_path
            )));
        }

        // Determine if using offset-based pagination
        let using_offset = args.offset.is_some() || args.length.is_some();

        if using_offset {
            let offset = args.offset.unwrap_or(0);
            let length = args
                .length
                .unwrap_or(READ_FILE_MAX_CHARS)
                .min(READ_FILE_MAX_CHARS);

            let (contents, bytes_consumed, total_size) =
                self.read_file_chunk(&path, offset, length)?;

            let mut output = String::new();

            // Format with offset info
            let formatted = self.format_response_with_offset(
                &args.file_path,
                &contents,
                total_size,
                offset,
                bytes_consumed,
                &args.session,
            );
            output.push_str(&formatted);

            // Add next-offset hint if more content remains
            let next_offset = offset + bytes_consumed;
            if next_offset < total_size {
                output.push_str(&format!(
                    "\n\nNOTE: More content available. \
                     Use offset={next_offset} to read next chunk.\n"
                ));
            }

            Ok(text_content(output))
        } else {
            // Original behavior: read from start with
            // auto-truncation
            let (contents, was_truncated, total_size) = self.read_file_contents(&path)?;

            let mut output = String::new();

            if was_truncated {
                let shown_lines = contents.lines().count();
                let warning = build_read_file_warning(
                    contents.len(),
                    total_size,
                    shown_lines,
                    &args.file_path,
                );
                output.push_str(&warning);
            }

            let formatted =
                self.format_response(&args.file_path, &contents, total_size as u64, &args.session);
            output.push_str(&formatted);

            // Add next-offset hint if file was truncated
            if was_truncated {
                let next_offset = contents.len();
                output.push_str(&format!(
                    "\n\nNOTE: More content available. \
                     Use offset={next_offset} to read next chunk.\n"
                ));
            }

            Ok(text_content(output))
        }
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
        let full_path = PathBuf::from(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&full_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

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

    /// Helper to extract text from ToolResult
    fn extract_text(result: &ToolResult) -> &str {
        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        }
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
        let text = extract_text(&tool_result);

        assert!(text.contains("**File:**"));
        assert!(text.contains("**Session:** `test-session`"));
        assert!(text.contains("**Size:**"));
        assert!(text.contains("**Language:**"));
        assert!(text.contains(test_content));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let (handler, _temp) = setup_test_handler().await;

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
        }

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
        let (handler, _temp) = setup_test_handler().await;
        let large_content = "x".repeat(200_000);
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
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("FILE TRUNCATED"));

        // Should have next-offset hint
        assert!(text.contains("offset="));

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
        let text = extract_text(&tool_result);
        assert!(text.contains(special_content));

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

    // Truncation tests

    #[tokio::test]
    async fn test_read_file_truncation_large_file() {
        let (handler, _temp) = setup_test_handler().await;

        let content = "Line of test content that is quite long\n".repeat(1200);
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
        let text = extract_text(&tool_result);

        assert!(text.contains("FILE TRUNCATED"));
        assert!(text.contains("FIRST 20000 CHARACTERS"));
        assert!(text.contains("of"));
        assert!(text.len() < content.len());

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_no_truncation_small_file() {
        let (handler, _temp) = setup_test_handler().await;

        let content = "Small file content line\n".repeat(200);
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
        let text = extract_text(&tool_result);

        assert!(!text.contains("FILE TRUNCATED"));
        assert!(text.contains(&content));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_exactly_at_limit() {
        use crate::mcp::utils::READ_FILE_MAX_CHARS;

        let (handler, _temp) = setup_test_handler().await;

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
        let text = extract_text(&tool_result);

        assert!(!text.contains("FILE TRUNCATED"));
        assert!(text.contains(&content));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_utf8_boundary_safety() {
        let (handler, _temp) = setup_test_handler().await;

        // Each emoji is 4 bytes, 6000 emojis = ~24KB
        let content = "\u{1F600}".repeat(6000);
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
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("FILE TRUNCATED"));
        assert!(text.is_char_boundary(text.len()));
        assert!(text.contains("\u{1F600}"));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_utf8_mixed_content() {
        let (handler, _temp) = setup_test_handler().await;

        let mut content = String::new();
        content.push_str("ASCII text\n".repeat(500).as_str());
        content.push_str("\u{4E16}\u{754C}\n".repeat(500).as_str());
        content.push_str(
            "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}\n"
                .repeat(500)
                .as_str(),
        );
        content.push_str("\u{1F600}\u{1F601}\u{1F602}\n".repeat(500).as_str());

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
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.is_char_boundary(text.len()));

        let _ = fs::remove_file(file_path);
    }

    // Offset pagination tests

    #[tokio::test]
    async fn test_read_file_with_offset() {
        let (handler, _temp) = setup_test_handler().await;

        // Create file with known content
        let content = "AAAA\nBBBB\nCCCC\nDDDD\nEEEE\n";
        let file_path = create_test_session_with_file(
            &handler.services,
            "offset-test",
            "/tmp/shebe-read-offset.txt",
            content,
        )
        .await;

        // Read from offset 5 (start of "BBBB")
        let args = json!({
            "session": "offset-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 5,
            "length": 10
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should contain the offset content
        assert!(text.contains("BBBB"));
        // Should show byte range info
        assert!(text.contains("showing bytes"));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_offset_last_chunk() {
        let (handler, _temp) = setup_test_handler().await;

        let content = "short file";
        let file_path = create_test_session_with_file(
            &handler.services,
            "lastchunk-test",
            "/tmp/shebe-read-lastchunk.txt",
            content,
        )
        .await;

        // Read from offset 5 (last 5 bytes: "file\0")
        let args = json!({
            "session": "lastchunk-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 6,
            "length": 20000
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should contain remaining content
        assert!(text.contains("file"));
        // Should NOT have "more content available"
        assert!(!text.contains("offset="));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_offset_beyond_file_size() {
        let (handler, _temp) = setup_test_handler().await;

        let content = "tiny";
        let file_path = create_test_session_with_file(
            &handler.services,
            "beyond-test",
            "/tmp/shebe-read-beyond.txt",
            content,
        )
        .await;

        // Offset beyond file size
        let args = json!({
            "session": "beyond-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 10000
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show empty content (0 lines in chunk)
        assert!(text.contains("0 lines in chunk"));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_offset_utf8_boundary() {
        let (handler, _temp) = setup_test_handler().await;

        // "AAA" + multi-byte char (3 bytes for CJK)
        // CJK chars are 3 bytes each in UTF-8
        let content = "AAA\u{4E16}\u{754C}BBB";
        let file_path = create_test_session_with_file(
            &handler.services,
            "utf8off-test",
            "/tmp/shebe-read-utf8off.txt",
            content,
        )
        .await;

        // Offset 4 lands inside the first CJK char (byte 3 is
        // start of 3-byte sequence, byte 4 is continuation)
        let args = json!({
            "session": "utf8off-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 4,
            "length": 20000
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should be valid UTF-8 (no panic)
        assert!(text.is_char_boundary(text.len()));

        let _ = fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_file_offset_with_more_content_hint() {
        let (handler, _temp) = setup_test_handler().await;

        // 30KB file - larger than READ_FILE_MAX_CHARS
        let content = "A".repeat(30_000);
        let file_path = create_test_session_with_file(
            &handler.services,
            "morehint-test",
            "/tmp/shebe-read-morehint.txt",
            &content,
        )
        .await;

        // Read first chunk with offset=0
        let args = json!({
            "session": "morehint-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 0,
            "length": 10000
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should have "more content" hint with next offset
        assert!(text.contains("More content available"));
        assert!(text.contains("offset=10000"));

        let _ = fs::remove_file(file_path);
    }

    // Unit tests for find_utf8_start

    #[test]
    fn test_find_utf8_start_ascii() {
        // ASCII byte - already at start
        assert_eq!(find_utf8_start(b"hello"), 0);
    }

    #[test]
    fn test_find_utf8_start_continuation_bytes() {
        // Continuation bytes (10xxxxxx = 0x80..0xBF)
        // followed by ASCII
        assert_eq!(find_utf8_start(&[0x80, 0x80, b'A']), 2);
    }

    #[test]
    fn test_find_utf8_start_all_continuation() {
        // All continuation bytes - returns buffer length
        assert_eq!(find_utf8_start(&[0x80, 0x80, 0x80]), 3);
    }

    #[test]
    fn test_find_utf8_start_multibyte_lead() {
        // 2-byte lead byte (110xxxxx = 0xC0..0xDF)
        assert_eq!(find_utf8_start(&[0xC0, 0x80]), 0);
    }

    // -- Phase 3 tests: read_file offset pagination -----------------------

    /// P0 Center: No offset produces identical format to
    /// pre-pagination. No offset artifacts or metadata leaked.
    #[tokio::test]
    async fn test_read_file_no_offset_returns_same_format() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let file_path = create_test_session_with_file(
            &handler.services,
            "nooff-fmt-test",
            "/tmp/shebe-rf-nooff-fmt.rs",
            content,
        )
        .await;

        let args = json!({
            "session": "nooff-fmt-test",
            "file_path": file_path.to_str().unwrap()
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        // Standard format fields present
        assert!(text.contains("**File:**"));
        assert!(text.contains("**Session:** `nooff-fmt-test`"));
        assert!(text.contains("**Size:**"));
        assert!(text.contains("**Language:**"));
        assert!(text.contains(content));

        // No pagination artifacts
        assert!(!text.contains("showing bytes"));
        assert!(!text.contains("offset="));
        assert!(!text.contains("More content available"));
        assert!(!text.contains("lines in chunk"));

        let _ = fs::remove_file(file_path);
    }

    /// P1 Center: offset=0 returns same file content as omitting
    /// offset. Format differs but content is identical.
    #[tokio::test]
    async fn test_read_file_offset_zero_equals_no_offset() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "line one\nline two\nline three\n";
        let file_path = create_test_session_with_file(
            &handler.services,
            "off0-test",
            "/tmp/shebe-rf-off0.txt",
            content,
        )
        .await;

        // Without offset
        let args_no = json!({
            "session": "off0-test",
            "file_path": file_path.to_str().unwrap()
        });
        let result_no = handler.execute(args_no).await.unwrap();
        let text_no = extract_text(&result_no);

        // With offset=0
        let args_off = json!({
            "session": "off0-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 0
        });
        let result_off = handler.execute(args_off).await.unwrap();
        let text_off = extract_text(&result_off);

        // Both contain the original file content
        assert!(
            text_no.contains(content),
            "No-offset response missing file content"
        );
        assert!(
            text_off.contains(content),
            "Offset=0 response missing file content"
        );

        let _ = fs::remove_file(file_path);
    }

    /// P0 Center: Sequential chunks reassemble to original.
    /// Uses read_file_chunk directly for byte-level testing.
    #[tokio::test]
    async fn test_read_file_full_reassembly_matches_original() {
        let (handler, _temp) = setup_test_handler().await;

        // 50KB ASCII file with distinct lines
        let original: String = (0..2000)
            .map(|i| format!("Line {i:04}: test content\n"))
            .collect();

        let file_path = PathBuf::from("/tmp/shebe-rf-reassemble.txt");
        fs::write(&file_path, &original).unwrap();

        let chunk_len = 10000;
        let mut offset = 0;
        let mut reassembled = String::new();

        loop {
            let (content, consumed, total) = handler
                .read_file_chunk(&file_path, offset, chunk_len)
                .unwrap();
            if consumed == 0 {
                break;
            }
            reassembled.push_str(&content);
            offset += consumed;
            if offset >= total {
                break;
            }
        }

        assert_eq!(
            reassembled, original,
            "Reassembled chunks must match original"
        );

        let _ = fs::remove_file(file_path);
    }

    /// P1 Boundary: offset = file_size - 1 returns last byte.
    #[tokio::test]
    async fn test_read_file_offset_exact_file_end() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "0123456789";
        let file_path = create_test_session_with_file(
            &handler.services,
            "exactend-test",
            "/tmp/shebe-rf-exactend.txt",
            content,
        )
        .await;

        // offset=9 (file_size=10, last byte is '9')
        let args = json!({
            "session": "exactend-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 9,
            "length": 20000
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("showing bytes 9-10 of 10"));
        assert!(!text.contains("More content available"));

        let _ = fs::remove_file(file_path);
    }

    /// P1 Beyond: offset = file_size exactly returns empty content.
    #[tokio::test]
    async fn test_read_file_offset_at_file_size() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "0123456789";
        let file_path = create_test_session_with_file(
            &handler.services,
            "atsize-test",
            "/tmp/shebe-rf-atsize.txt",
            content,
        )
        .await;

        // offset=10 (exactly at file_size)
        let args = json!({
            "session": "atsize-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 10,
            "length": 20000
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("0 lines in chunk"));
        assert!(!text.contains("More content available"));

        let _ = fs::remove_file(file_path);
    }

    /// P1 Center: Small file needs no pagination. Entire file
    /// returned with no offset hints.
    #[tokio::test]
    async fn test_read_file_small_file_no_pagination() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "small file content\n";
        let file_path = create_test_session_with_file(
            &handler.services,
            "smallnp-test",
            "/tmp/shebe-rf-smallnp.txt",
            content,
        )
        .await;

        let args = json!({
            "session": "smallnp-test",
            "file_path": file_path.to_str().unwrap()
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains(content));
        assert!(!text.contains("FILE TRUNCATED"));
        assert!(!text.contains("More content available"));
        assert!(!text.contains("offset="));

        let _ = fs::remove_file(file_path);
    }

    /// P1 Boundary: Offset on byte 2 of a 4-byte UTF-8 char
    /// adjusts to the next valid boundary. Uses emoji (4 bytes:
    /// F0 9F 98 80) as test character.
    #[tokio::test]
    async fn test_read_file_offset_utf8_mid_character_specific() {
        let (handler, _temp) = setup_test_handler().await;

        // "AAA" (3 bytes) + emoji (4 bytes) + "BBB" (3 bytes)
        // Total: 10 bytes
        let content = "AAA\u{1F600}BBB";
        let file_path = create_test_session_with_file(
            &handler.services,
            "utf8mid-test",
            "/tmp/shebe-rf-utf8mid.txt",
            content,
        )
        .await;

        // Offset 4 lands on 0x9F (byte 2 of 4-byte emoji).
        // find_utf8_start skips continuation bytes 4,5,6 ->
        // content starts at byte 7 ("BBB").
        let args = json!({
            "session": "utf8mid-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 4,
            "length": 20000
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        // Valid UTF-8
        assert!(text.is_char_boundary(text.len()));

        // Content starts at next boundary after the mid-char
        assert!(
            text.contains("BBB"),
            "Should contain chars after the split emoji"
        );

        // endOffset reflects actual bytes consumed (4+6=10)
        assert!(text.contains("showing bytes 4-10 of 10"));

        let _ = fs::remove_file(file_path);
    }

    /// P1 Boundary: 10KB ASCII file in 1KB chunks. No bytes
    /// lost or duplicated at chunk boundaries.
    #[tokio::test]
    async fn test_read_file_chunk_boundaries_no_data_loss() {
        let (handler, _temp) = setup_test_handler().await;

        // 10000 bytes of distinct ASCII content
        let original: String = (0..10000_usize)
            .map(|i| char::from(b'A' + (i % 26) as u8))
            .collect();

        let file_path = PathBuf::from("/tmp/shebe-rf-chunkbd.txt");
        fs::write(&file_path, &original).unwrap();

        let chunk_len = 1000;
        let mut offset = 0;
        let mut reassembled = String::new();

        loop {
            let (content, consumed, total) = handler
                .read_file_chunk(&file_path, offset, chunk_len)
                .unwrap();
            if consumed == 0 {
                break;
            }
            reassembled.push_str(&content);
            offset += consumed;
            if offset >= total {
                break;
            }
        }

        assert_eq!(
            reassembled.len(),
            original.len(),
            "Reassembled length must match original"
        );
        assert_eq!(
            reassembled, original,
            "Reassembled content must match original"
        );

        let _ = fs::remove_file(file_path);
    }

    /// P2 Boundary: length larger than remaining bytes returns
    /// only what remains.
    #[tokio::test]
    async fn test_read_file_length_larger_than_remaining() {
        let (handler, _temp) = setup_test_handler().await;
        let content = "X".repeat(10000);
        let file_path = create_test_session_with_file(
            &handler.services,
            "largerlen-test",
            "/tmp/shebe-rf-largerlen.txt",
            &content,
        )
        .await;

        // offset=9000, length=5000: only 1000 bytes remain
        let args = json!({
            "session": "largerlen-test",
            "file_path": file_path.to_str().unwrap(),
            "offset": 9000,
            "length": 5000
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("showing bytes 9000-10000 of 10000"));
        assert!(!text.contains("More content available"));

        let _ = fs::remove_file(file_path);
    }
}
