//! List directory (all files) tool handler

use super::handler::{text_content, McpToolHandler};
use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::pagination::{session_fingerprint, ListDirCursor};
use crate::mcp::protocol::{ToolResult, ToolSchema};
use crate::mcp::utils::{build_list_dir_warning, LIST_DIR_DEFAULT_LIMIT, LIST_DIR_MAX_LIMIT};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value as TantivyValue;
use tantivy::TantivyDocument;

#[derive(Debug, Clone)]
pub enum SortOrder {
    Alpha,   // Alphabetical by path
    Size,    // By file size (largest first)
    Indexed, // By indexed order (insertion order)
}

impl SortOrder {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "alpha" => Ok(Self::Alpha),
            "size" => Ok(Self::Size),
            "indexed" => Ok(Self::Indexed),
            _ => Err(format!(
                "Invalid sort order: '{s}'. \
                 Must be 'alpha', 'size' or 'indexed'."
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    chunk_count: usize,
    size_bytes: u64,
}

pub struct ListDirHandler {
    services: Arc<Services>,
}

impl ListDirHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Get unique file paths from Tantivy index
    async fn get_file_list(
        &self,
        session: &str,
        sort: SortOrder,
    ) -> Result<Vec<FileEntry>, McpError> {
        // Open session index
        let index = self
            .services
            .storage
            .open_session_read(session)
            .map_err(McpError::from)?;

        let reader = index
            .reader()
            .map_err(|e| McpError::InternalError(format!("Failed to open reader: {e}")))?;

        let searcher = reader.searcher();

        // Query all documents
        let query = AllQuery;

        // Collect unique file_path values
        let file_path_field = index
            .schema()
            .get_field("file_path")
            .map_err(|e| McpError::InternalError(format!("file_path field missing: {e}")))?;

        let mut file_map: HashMap<String, FileEntry> = HashMap::new();

        // Collect documents (we need to aggregate by file_path)
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(100000).order_by_score())
            .map_err(|e| McpError::InternalError(format!("Search failed: {e}")))?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| McpError::InternalError(format!("Doc retrieval failed: {e}")))?;

            // Extract file_path
            let file_path = retrieved_doc
                .get_first(file_path_field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InternalError("Missing file_path".to_string()))?
                .to_string();

            // Track unique files
            file_map
                .entry(file_path.clone())
                .or_insert_with(|| FileEntry {
                    path: file_path,
                    chunk_count: 0,
                    size_bytes: 0, // Will populate if sort=size
                })
                .chunk_count += 1;
        }

        // Convert to vec
        let mut files: Vec<FileEntry> = file_map.into_values().collect();

        // Sort by requested order
        match sort {
            SortOrder::Alpha => files.sort_by(|a, b| a.path.cmp(&b.path)),
            SortOrder::Size => {
                // Need to stat files for size
                for entry in &mut files {
                    if let Ok(metadata) = std::fs::metadata(&entry.path) {
                        entry.size_bytes = metadata.len();
                    }
                }
                files.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
            }
            SortOrder::Indexed => {
                // Keep insertion order (effectively indexed order)
            }
        }

        Ok(files)
    }

    /// Format file list as Markdown with pagination info
    fn format_file_list(
        &self,
        session: &str,
        files: &[FileEntry],
        total: usize,
        range_start: usize,
        range_end: usize,
    ) -> String {
        let mut output = format!(
            "**Session:** `{}`\n\
             **Files:** {} (showing {}-{})\n\n",
            session,
            total,
            range_start + 1,
            range_end,
        );

        if files.is_empty() {
            output.push_str("No files found in this session.");
            return output;
        }

        output.push_str("| File Path | Chunks |\n");
        output.push_str("|-----------|--------|\n");

        for entry in files {
            output.push_str(&format!("| `{}` | {} |\n", entry.path, entry.chunk_count));
        }

        output
    }
}

#[async_trait]
impl McpToolHandler for ListDirHandler {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "list_dir".to_string(),
            description: "List all files indexed in a session \
                (like 'ls' command). Simple directory listing with \
                no filtering. Use when you want to see all files in \
                a session. For pattern-based search, use find_file \
                instead. Returns list sorted alphabetically by \
                default. Auto-truncates to 500 files max to stay \
                under MCP 25k token limit (shows warning if \
                truncated). Supports cursor-based pagination for \
                navigating large file lists."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description":
                            "Session ID to list files from",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "limit": {
                        "type": "integer",
                        "description":
                            "Max files to return \
                             (default: 100, max: 500)",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 500
                    },
                    "sort": {
                        "type": "string",
                        "description":
                            "Sort order: 'alpha' (default), \
                             'size', 'indexed'",
                        "default": "alpha",
                        "enum": ["alpha", "size", "indexed"]
                    },
                    "cursor": {
                        "type": "string",
                        "description":
                            "Pagination cursor from previous \
                             response. Omit for first page."
                    }
                },
                "required": ["session"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct ListDirArgs {
            session: String,
            #[serde(default = "default_limit")]
            limit: Option<usize>,
            #[serde(default = "default_sort")]
            sort: String,
            cursor: Option<String>,
        }
        fn default_limit() -> Option<usize> {
            None
        }
        fn default_sort() -> String {
            "alpha".to_string()
        }

        // Parse arguments
        let args: ListDirArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Parse sort order
        let sort = SortOrder::from_str(&args.sort).map_err(McpError::InvalidParams)?;

        // Determine effective limit
        let effective_limit = match args.limit {
            Some(requested) => {
                if requested > LIST_DIR_MAX_LIMIT {
                    LIST_DIR_MAX_LIMIT
                } else {
                    requested
                }
            }
            None => LIST_DIR_DEFAULT_LIMIT,
        };

        // Determine start index from cursor
        let start_index = if let Some(ref cursor_str) = args.cursor {
            let cursor = ListDirCursor::decode(cursor_str).map_err(McpError::InvalidParams)?;

            // Verify sort mode matches
            if cursor.sort != args.sort {
                return Err(McpError::InvalidParams(format!(
                    "Cursor sort mode '{}' does not match \
                     requested sort mode '{}'. Use the same \
                     sort mode or omit the cursor.",
                    cursor.sort, args.sort
                )));
            }

            // Verify fingerprint against current session
            let metadata = self
                .services
                .storage
                .get_session_metadata(&args.session)
                .map_err(McpError::from)?;

            cursor.verify(&metadata).map_err(McpError::InvalidParams)?;

            cursor.last_index + 1
        } else {
            0
        };

        // Get all files from index
        let all_files = self.get_file_list(&args.session, sort).await?;
        let total_count = all_files.len();

        // Compute page slice
        let page_end = (start_index + effective_limit).min(total_count);
        let page_files = if start_index < total_count {
            &all_files[start_index..page_end]
        } else {
            &[]
        };
        let shown_count = page_files.len();

        // Check if there are more results after this page
        let has_more = page_end < total_count;

        // Build output
        let mut output = String::new();

        // Add truncation warning only on first page without cursor
        if args.cursor.is_none() && total_count > effective_limit {
            let warning = build_list_dir_warning(
                effective_limit.min(total_count),
                total_count,
                &args.session,
            );
            output.push_str(&warning);
        }

        // Add file list with range info
        let formatted = self.format_file_list(
            &args.session,
            page_files,
            total_count,
            start_index,
            start_index + shown_count,
        );
        output.push_str(&formatted);

        // Add next-page cursor if more results exist
        if has_more {
            let metadata = self
                .services
                .storage
                .get_session_metadata(&args.session)
                .map_err(McpError::from)?;

            let next_cursor = ListDirCursor {
                last_index: page_end - 1,
                sort: args.sort.clone(),
                fingerprint: session_fingerprint(&metadata),
            };

            output.push_str(&format!(
                "\nNOTE: More results available. \
                 Use cursor=\"{}\" to fetch next page.\n",
                next_cursor.encode()
            ));
        }

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
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (ListDirHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = ListDirHandler::new(services);

        (handler, temp_dir)
    }

    async fn create_test_session_with_files(
        services: &Arc<Services>,
        session_id: &str,
        files: Vec<(&str, &str)>,
    ) {
        let mut index = services
            .storage
            .create_session(
                session_id,
                PathBuf::from("/test/repo"),
                SessionConfig::default(),
            )
            .unwrap();

        for (file_path, content) in files {
            let full_path = PathBuf::from(file_path);
            if let Some(parent) = full_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut file = fs::File::create(&full_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();

            let chunks = vec![Chunk {
                text: content.to_string(),
                file_path: full_path.clone(),
                start_offset: 0,
                end_offset: content.len(),
                chunk_index: 0,
            }];

            index.add_chunks(&chunks, session_id).unwrap();
        }

        index.commit().unwrap();
    }

    /// Helper to extract text from ToolResult
    fn extract_text(result: &ToolResult) -> &str {
        match &result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        }
    }

    #[tokio::test]
    async fn test_list_dir_basic() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test-a.rs", "fn main() {}"),
                ("/tmp/shebe-test-b.rs", "fn test() {}"),
                ("/tmp/shebe-test-c.rs", "fn run() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("**Session:** `test-session`"));
        assert!(text.contains("**Files:** 3 (showing 1-3)"));
        assert!(text.contains("/tmp/shebe-test-a.rs"));
        assert!(text.contains("/tmp/shebe-test-b.rs"));
        assert!(text.contains("/tmp/shebe-test-c.rs"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test-a.rs");
        let _ = fs::remove_file("/tmp/shebe-test-b.rs");
        let _ = fs::remove_file("/tmp/shebe-test-c.rs");
    }

    #[tokio::test]
    async fn test_list_dir_with_limit() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test-1.rs", "fn test1() {}"),
                ("/tmp/shebe-test-2.rs", "fn test2() {}"),
                ("/tmp/shebe-test-3.rs", "fn test3() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "limit": 2,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("(showing 1-2)"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test-1.rs");
        let _ = fs::remove_file("/tmp/shebe-test-2.rs");
        let _ = fs::remove_file("/tmp/shebe-test-3.rs");
    }

    #[tokio::test]
    async fn test_list_dir_sort_alpha() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test-z.rs", "fn z() {}"),
                ("/tmp/shebe-test-a.rs", "fn a() {}"),
                ("/tmp/shebe-test-m.rs", "fn m() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "sort": "alpha",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Verify alphabetical order (a before m before z)
        let a_pos = text.find("shebe-test-a.rs").unwrap();
        let m_pos = text.find("shebe-test-m.rs").unwrap();
        let z_pos = text.find("shebe-test-z.rs").unwrap();
        assert!(a_pos < m_pos && m_pos < z_pos);

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test-z.rs");
        let _ = fs::remove_file("/tmp/shebe-test-a.rs");
        let _ = fs::remove_file("/tmp/shebe-test-m.rs");
    }

    #[tokio::test]
    async fn test_list_dir_sort_size() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test-small.rs", "fn test() {}"),
                (
                    "/tmp/shebe-test-large.rs",
                    "fn test() {}\n".repeat(100).as_str(),
                ),
                (
                    "/tmp/shebe-test-medium.rs",
                    "fn test() {}\n".repeat(10).as_str(),
                ),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "sort": "size",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Verify size order (large before medium before small)
        let large_pos = text.find("shebe-test-large.rs").unwrap();
        let medium_pos = text.find("shebe-test-medium.rs").unwrap();
        let small_pos = text.find("shebe-test-small.rs").unwrap();
        assert!(large_pos < medium_pos && medium_pos < small_pos);

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test-small.rs");
        let _ = fs::remove_file("/tmp/shebe-test-large.rs");
        let _ = fs::remove_file("/tmp/shebe-test-medium.rs");
    }

    #[tokio::test]
    async fn test_list_dir_empty_session() {
        let (handler, _temp) = setup_test_handler().await;
        {
            let mut index = handler
                .services
                .storage
                .create_session(
                    "empty-session",
                    PathBuf::from("/test/repo"),
                    SessionConfig::default(),
                )
                .unwrap();
            index.commit().unwrap();
        } // Drop the index to release the lock

        let args = json!({
            "session": "empty-session",
        });

        let result = handler.execute(args).await;
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("No files found in this session"));
    }

    #[tokio::test]
    async fn test_list_dir_large_session() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 150 files
        let files: Vec<_> = (0..150)
            .map(|i| (format!("/tmp/shebe-test-{:03}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "large-session", file_refs).await;

        let args = json!({
            "session": "large-session",
            "limit": 100,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        assert!(text.contains("(showing 1-100)"));

        // Cleanup
        for i in 0..150 {
            let _ = fs::remove_file(format!("/tmp/shebe-test-{:03}.rs", i));
        }
    }

    // Truncation tests

    #[tokio::test]
    async fn test_list_dir_default_limit_with_truncation() {
        use crate::mcp::utils::LIST_DIR_DEFAULT_LIMIT;

        let (handler, _temp) = setup_test_handler().await;

        // Create 200 files (more than default limit of 100)
        let files: Vec<_> = (0..200)
            .map(|i| (format!("/tmp/shebe-truncate-{:03}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "truncate-session", file_refs).await;

        // Call without limit - should use default (100)
        let args = json!({
            "session": "truncate-session",
            "sort": "alpha"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show warning (200 files > 100 default)
        assert!(text.contains("OUTPUT TRUNCATED"));
        assert!(text.contains("200")); // total files

        // Count actual file entries (should be exactly 100)
        let file_count = text.matches("| `/tmp/shebe-truncate-").count();
        assert_eq!(file_count, LIST_DIR_DEFAULT_LIMIT);

        // Should have a next-page cursor
        assert!(text.contains("cursor="));

        // Cleanup
        for i in 0..200 {
            let _ = fs::remove_file(format!("/tmp/shebe-truncate-{:03}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_max_limit_enforced() {
        use crate::mcp::utils::LIST_DIR_MAX_LIMIT;

        let (handler, _temp) = setup_test_handler().await;

        // Create 600 files (more than max limit of 500)
        let files: Vec<_> = (0..600)
            .map(|i| (format!("/tmp/shebe-maxlimit-{:03}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "maxlimit-session", file_refs).await;

        // User requests 1000 files, but max is 500
        let args = json!({
            "session": "maxlimit-session",
            "limit": 1000,
            "sort": "alpha"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should enforce max limit of 500
        let file_count = text.matches("| `/tmp/shebe-maxlimit-").count();
        assert_eq!(file_count, LIST_DIR_MAX_LIMIT);

        // Should show warning
        assert!(text.contains("OUTPUT TRUNCATED"));
        assert!(text.contains("MAXIMUM 500 FILES"));

        // Cleanup
        for i in 0..600 {
            let _ = fs::remove_file(format!("/tmp/shebe-maxlimit-{:03}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_no_truncation_small_repo() {
        let (handler, _temp) = setup_test_handler().await;

        // Repository with fewer files than default limit
        let files: Vec<_> = (0..50)
            .map(|i| (format!("/tmp/shebe-small-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "small-session", file_refs).await;

        let args = json!({"session": "small-session"});
        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should NOT show warning (50 files < 100 default)
        assert!(!text.contains("OUTPUT TRUNCATED"));

        // Should show all 50 files
        let file_count = text.matches("| `/tmp/shebe-small-").count();
        assert_eq!(file_count, 50);

        // Should NOT have a cursor (all files shown)
        assert!(!text.contains("cursor="));

        // Cleanup
        for i in 0..50 {
            let _ = fs::remove_file(format!("/tmp/shebe-small-{:02}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_user_limit_within_max() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 400 files
        let files: Vec<_> = (0..400)
            .map(|i| (format!("/tmp/shebe-userlimit-{:03}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "userlimit-session", file_refs).await;

        // User requests 250 (within max 500)
        let args = json!({
            "session": "userlimit-session",
            "limit": 250
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show exactly 250 files
        let file_count = text.matches("| `/tmp/shebe-userlimit-").count();
        assert_eq!(file_count, 250);

        // Should show warning (400 total > 250 shown)
        assert!(text.contains("OUTPUT TRUNCATED"));
        assert!(text.contains("250 of 400"));

        // Cleanup
        for i in 0..400 {
            let _ = fs::remove_file(format!("/tmp/shebe-userlimit-{:03}.rs", i));
        }
    }

    // Pagination cursor tests

    #[tokio::test]
    async fn test_list_dir_pagination_first_page() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 5 files
        let files: Vec<_> = (0..5)
            .map(|i| (format!("/tmp/shebe-page-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "page-session", file_refs).await;

        // First page with limit 2
        let args = json!({
            "session": "page-session",
            "limit": 2,
            "sort": "alpha"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show first 2 of 5
        assert!(text.contains("(showing 1-2)"));
        assert!(text.contains("**Files:** 5"));

        // Should have next cursor
        assert!(text.contains("cursor="));
        assert!(text.contains("More results available"));

        // Cleanup
        for i in 0..5 {
            let _ = fs::remove_file(format!("/tmp/shebe-page-{:02}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_pagination_second_page() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 5 files
        let files: Vec<_> = (0..5)
            .map(|i| (format!("/tmp/shebe-page2-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "page2-session", file_refs).await;

        // Build a cursor for "after index 1" (first page was 0..2)
        let metadata = handler
            .services
            .storage
            .get_session_metadata("page2-session")
            .unwrap();
        let cursor = ListDirCursor {
            last_index: 1,
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&metadata),
        };

        let args = json!({
            "session": "page2-session",
            "limit": 2,
            "sort": "alpha",
            "cursor": cursor.encode()
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show items 3-4 of 5
        assert!(text.contains("(showing 3-4)"));

        // Should still have cursor (item 5 remaining)
        assert!(text.contains("cursor="));

        // Should NOT have truncation warning (cursor page)
        assert!(!text.contains("OUTPUT TRUNCATED"));

        // Cleanup
        for i in 0..5 {
            let _ = fs::remove_file(format!("/tmp/shebe-page2-{:02}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_pagination_last_page() {
        let (handler, _temp) = setup_test_handler().await;

        // Create 5 files
        let files: Vec<_> = (0..5)
            .map(|i| (format!("/tmp/shebe-lastpg-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "lastpg-session", file_refs).await;

        // Build cursor for "after index 3" (previous pages: 0-1, 2-3)
        let metadata = handler
            .services
            .storage
            .get_session_metadata("lastpg-session")
            .unwrap();
        let cursor = ListDirCursor {
            last_index: 3,
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&metadata),
        };

        let args = json!({
            "session": "lastpg-session",
            "limit": 2,
            "sort": "alpha",
            "cursor": cursor.encode()
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = extract_text(&tool_result);

        // Should show item 5 of 5 (only 1 remaining)
        assert!(text.contains("(showing 5-5)"));

        // Should NOT have cursor (last page)
        assert!(!text.contains("cursor="));

        // Cleanup
        for i in 0..5 {
            let _ = fs::remove_file(format!("/tmp/shebe-lastpg-{:02}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_pagination_invalid_cursor() {
        let (handler, _temp) = setup_test_handler().await;
        {
            let mut index = handler
                .services
                .storage
                .create_session(
                    "inv-cursor-session",
                    PathBuf::from("/test/repo"),
                    SessionConfig::default(),
                )
                .unwrap();
            index.commit().unwrap();
        }

        let args = json!({
            "session": "inv-cursor-session",
            "cursor": "not-a-valid-cursor"
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        match result {
            Err(McpError::InvalidParams(msg)) => {
                assert!(
                    msg.contains("Invalid cursor"),
                    "Error should mention invalid cursor: {msg}"
                );
            }
            other => {
                panic!("Expected InvalidParams, got: {:?}", other);
            }
        }
    }

    #[tokio::test]
    async fn test_list_dir_pagination_stale_cursor() {
        let (handler, _temp) = setup_test_handler().await;

        // Create session with files
        let files: Vec<_> = (0..3)
            .map(|i| (format!("/tmp/shebe-stale-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "stale-session", file_refs).await;

        // Build cursor with wrong fingerprint (simulates stale)
        let cursor = ListDirCursor {
            last_index: 0,
            sort: "alpha".to_string(),
            fingerprint: "0-0-0".to_string(),
        };

        let args = json!({
            "session": "stale-session",
            "sort": "alpha",
            "cursor": cursor.encode()
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        match result {
            Err(McpError::InvalidParams(msg)) => {
                assert!(
                    msg.contains("stale"),
                    "Error should mention stale cursor: {msg}"
                );
            }
            other => {
                panic!("Expected InvalidParams, got: {:?}", other);
            }
        }

        // Cleanup
        for i in 0..3 {
            let _ = fs::remove_file(format!("/tmp/shebe-stale-{:02}.rs", i));
        }
    }

    #[tokio::test]
    async fn test_list_dir_pagination_sort_mismatch() {
        let (handler, _temp) = setup_test_handler().await;

        // Create session
        let files: Vec<_> = (0..3)
            .map(|i| (format!("/tmp/shebe-sortmm-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "sortmm-session", file_refs).await;

        // Build cursor with sort=alpha
        let metadata = handler
            .services
            .storage
            .get_session_metadata("sortmm-session")
            .unwrap();
        let cursor = ListDirCursor {
            last_index: 0,
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&metadata),
        };

        // Request with sort=size but cursor has sort=alpha
        let args = json!({
            "session": "sortmm-session",
            "sort": "size",
            "cursor": cursor.encode()
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        match result {
            Err(McpError::InvalidParams(msg)) => {
                assert!(
                    msg.contains("sort mode"),
                    "Error should mention sort mismatch: {msg}"
                );
            }
            other => {
                panic!("Expected InvalidParams, got: {:?}", other);
            }
        }

        // Cleanup
        for i in 0..3 {
            let _ = fs::remove_file(format!("/tmp/shebe-sortmm-{:02}.rs", i));
        }
    }

    // -- Phase 2 helpers --------------------------------------------------

    /// Extract cursor value from list_dir output text.
    /// Returns None when no cursor is present in the output.
    fn extract_cursor(text: &str) -> Option<String> {
        let prefix = "cursor=\"";
        let start = text.find(prefix)? + prefix.len();
        let end = start + text[start..].find('"')?;
        Some(text[start..end].to_string())
    }

    /// Extract file paths from list_dir markdown table rows.
    fn extract_file_paths(text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| line.starts_with("| `"))
            .filter_map(|line| {
                let start = line.find('`')? + 1;
                let end = start + line[start..].find('`')?;
                Some(line[start..end].to_string())
            })
            .collect()
    }

    // -- Phase 2 tests: list_dir pagination --------------------------------

    /// P0 Center: No cursor produces identical format to pre-pagination.
    /// No pagination metadata leak, no format change.
    #[tokio::test]
    async fn test_list_dir_no_cursor_returns_same_format() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "compat-session",
            vec![
                ("/tmp/shebe-compat-a.rs", "fn a() {}"),
                ("/tmp/shebe-compat-b.rs", "fn b() {}"),
                ("/tmp/shebe-compat-c.rs", "fn c() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "compat-session",
            "sort": "alpha"
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        // Standard format fields present
        assert!(text.contains("**Session:** `compat-session`"));
        assert!(text.contains("**Files:** 3 (showing 1-3)"));
        assert!(text.contains("| File Path | Chunks |"));

        // No pagination artifacts leaked
        assert!(!text.contains("cursor="));
        assert!(!text.contains("More results available"));
        assert!(!text.contains("nextCursor"));

        // Cleanup
        for s in &["a", "b", "c"] {
            let _ = fs::remove_file(format!("/tmp/shebe-compat-{s}.rs"));
        }
    }

    /// P1 Boundary: Single-file session needs no pagination.
    #[tokio::test]
    async fn test_list_dir_pagination_single_file_session() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "single-session",
            vec![("/tmp/shebe-single-a.rs", "fn main() {}")],
        )
        .await;

        let args = json!({
            "session": "single-session",
            "limit": 100,
            "sort": "alpha"
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("(showing 1-1)"));
        assert!(text.contains("**Files:** 1"));
        assert!(!text.contains("cursor="));
        assert!(!text.contains("More results available"));

        let _ = fs::remove_file("/tmp/shebe-single-a.rs");
    }

    /// P1 Boundary: limit=1 pages through all files one at a time.
    /// 5 files => 5 sequential pages, each with exactly 1 file.
    #[tokio::test]
    async fn test_list_dir_pagination_limit_one() {
        let (handler, _temp) = setup_test_handler().await;

        let files: Vec<_> = (0..5)
            .map(|i| (format!("/tmp/shebe-lim1-{i:02}.rs"), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "lim1-session", file_refs).await;

        let mut collected = Vec::new();
        let mut cursor_str: Option<String> = None;
        let mut pages = 0;

        loop {
            let mut args = json!({
                "session": "lim1-session",
                "limit": 1,
                "sort": "alpha"
            });
            if let Some(ref c) = cursor_str {
                args["cursor"] = json!(c);
            }

            let result = handler.execute(args).await.unwrap();
            let text = extract_text(&result);
            pages += 1;

            let paths = extract_file_paths(text);
            assert_eq!(paths.len(), 1, "Each page should contain exactly 1 file");
            collected.extend(paths);

            cursor_str = extract_cursor(text);
            if cursor_str.is_none() {
                break;
            }
        }

        assert_eq!(pages, 5, "5 files at limit=1 needs 5 pages");
        assert_eq!(collected.len(), 5);

        for i in 0..5 {
            let _ = fs::remove_file(format!("/tmp/shebe-lim1-{i:02}.rs"));
        }
    }

    /// P1 Boundary: Exact divisibility -- last page is full, no
    /// extra empty page. 500 files / limit=100 = exactly 5 pages.
    #[tokio::test]
    async fn test_list_dir_pagination_exact_divisible() {
        let (handler, _temp) = setup_test_handler().await;

        let total = 500_usize;
        let limit = 100_usize;
        let files: Vec<_> = (0..total)
            .map(|i| (format!("/tmp/shebe-exact-{i:03}.rs"), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "exact-session", file_refs).await;

        // Jump directly to the last page via cursor
        let metadata = handler
            .services
            .storage
            .get_session_metadata("exact-session")
            .unwrap();
        let cursor = ListDirCursor {
            last_index: total - limit - 1, // 399
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&metadata),
        };

        let args = json!({
            "session": "exact-session",
            "limit": limit,
            "sort": "alpha",
            "cursor": cursor.encode()
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        // Last page: items 401-500
        assert!(text.contains("(showing 401-500)"));

        // Exactly 100 files on last page
        let paths = extract_file_paths(text);
        assert_eq!(paths.len(), limit);

        // No cursor -- last page, exact divisibility
        assert!(
            !text.contains("cursor="),
            "Last page of exact-divisible set must not have cursor"
        );

        for i in 0..total {
            let _ = fs::remove_file(format!("/tmp/shebe-exact-{i:03}.rs"));
        }
    }

    /// P1 Boundary: Limit larger than total files returns all
    /// files in one page with no cursor.
    #[tokio::test]
    async fn test_list_dir_pagination_limit_exceeds_total() {
        let (handler, _temp) = setup_test_handler().await;

        let total = 50_usize;
        let files: Vec<_> = (0..total)
            .map(|i| (format!("/tmp/shebe-exceed-{i:02}.rs"), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "exceed-session", file_refs).await;

        // limit=10000 gets capped to MAX_LIMIT (500), still > 50
        let args = json!({
            "session": "exceed-session",
            "limit": 10000,
            "sort": "alpha"
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("(showing 1-50)"));
        assert!(text.contains("**Files:** 50"));
        assert_eq!(extract_file_paths(text).len(), total);
        assert!(!text.contains("cursor="));

        for i in 0..total {
            let _ = fs::remove_file(format!("/tmp/shebe-exceed-{i:02}.rs"));
        }
    }

    /// P2 Beyond: Empty session (0 files) with no cursor returns
    /// empty list and no pagination cursor.
    #[tokio::test]
    async fn test_list_dir_pagination_empty_session_with_cursor() {
        let (handler, _temp) = setup_test_handler().await;
        {
            let mut index = handler
                .services
                .storage
                .create_session(
                    "empty-pg-session",
                    PathBuf::from("/test/repo"),
                    SessionConfig::default(),
                )
                .unwrap();
            index.commit().unwrap();
        }

        let args = json!({
            "session": "empty-pg-session",
            "sort": "alpha"
        });

        let result = handler.execute(args).await.unwrap();
        let text = extract_text(&result);

        assert!(text.contains("No files found in this session"));
        assert!(!text.contains("cursor="));
        assert!(!text.contains("More results available"));
    }

    /// P1 Center: Page through all files; concatenated results
    /// contain no duplicates and no gaps.
    #[tokio::test]
    async fn test_list_dir_pagination_content_no_overlap() {
        let (handler, _temp) = setup_test_handler().await;

        let total = 250_usize;
        let files: Vec<_> = (0..total)
            .map(|i| (format!("/tmp/shebe-noovlp-{i:03}.rs"), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "noovlp-session", file_refs).await;

        let mut all_files = Vec::new();
        let mut cursor_str: Option<String> = None;

        loop {
            let mut args = json!({
                "session": "noovlp-session",
                "limit": 100,
                "sort": "alpha"
            });
            if let Some(ref c) = cursor_str {
                args["cursor"] = json!(c);
            }

            let result = handler.execute(args).await.unwrap();
            let text = extract_text(&result);
            all_files.extend(extract_file_paths(text));

            cursor_str = extract_cursor(text);
            if cursor_str.is_none() {
                break;
            }
        }

        // No duplicates
        let unique: std::collections::HashSet<_> = all_files.iter().collect();
        assert_eq!(
            unique.len(),
            all_files.len(),
            "No duplicate files across pages"
        );

        // No gaps: total matches session file count
        assert_eq!(all_files.len(), total);

        for i in 0..total {
            let _ = fs::remove_file(format!("/tmp/shebe-noovlp-{i:03}.rs"));
        }
    }

    /// P1 Center: Sort order maintained at page boundaries.
    /// Last file on page N alphabetically precedes first on N+1.
    #[tokio::test]
    async fn test_list_dir_pagination_sort_maintained_across_pages() {
        let (handler, _temp) = setup_test_handler().await;

        let files: Vec<_> = (0..10)
            .map(|i| (format!("/tmp/shebe-sortpg-{i:02}.rs"), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "sortpg-session", file_refs).await;

        let mut pages: Vec<Vec<String>> = Vec::new();
        let mut cursor_str: Option<String> = None;

        loop {
            let mut args = json!({
                "session": "sortpg-session",
                "limit": 3,
                "sort": "alpha"
            });
            if let Some(ref c) = cursor_str {
                args["cursor"] = json!(c);
            }

            let result = handler.execute(args).await.unwrap();
            let text = extract_text(&result);
            pages.push(extract_file_paths(text));

            cursor_str = extract_cursor(text);
            if cursor_str.is_none() {
                break;
            }
        }

        // Verify sort at page boundaries:
        // last file on page N < first file on page N+1
        for i in 0..pages.len() - 1 {
            let last = pages[i].last().unwrap();
            let first = pages[i + 1].first().unwrap();
            assert!(
                last < first,
                "Page {i} last '{last}' should precede \
                 page {} first '{first}'",
                i + 1
            );
        }

        for i in 0..10 {
            let _ = fs::remove_file(format!("/tmp/shebe-sortpg-{i:02}.rs"));
        }
    }
}
