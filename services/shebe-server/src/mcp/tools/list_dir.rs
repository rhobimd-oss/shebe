//! List directory (all files) tool handler

use super::handler::{text_content, McpToolHandler};
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use crate::mcp::services::ShebeServices;
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
                "Invalid sort order: '{s}'. Must be 'alpha', 'size', or 'indexed'."
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
    services: Arc<ShebeServices>,
}

impl ListDirHandler {
    pub fn new(services: Arc<ShebeServices>) -> Self {
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
            .open_session(session)
            .map_err(McpError::from)?;

        let reader = index
            .index()
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
            .search(&query, &TopDocs::with_limit(100000))
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
                files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
            }
            SortOrder::Indexed => {
                // Keep insertion order (effectively indexed order)
            }
        }

        Ok(files)
    }

    /// Format file list as Markdown
    fn format_file_list(&self, session: &str, files: &[FileEntry], total: usize) -> String {
        let mut output = format!(
            "**Session:** `{}`\n\
             **Files:** {} (showing {})\n\n",
            session,
            total,
            files.len()
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
            description: "List all files indexed in a session (like 'ls' command). \
                         Simple directory listing with no filtering. Use when you want to see all \
                         files in a session. For pattern-based search, use find_file instead. \
                         Returns list sorted alphabetically by default. Auto-truncates to 500 files \
                         max to stay under MCP 25k token limit (shows warning if truncated)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to list files from",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max files to return (default: 100, max: 500)",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 500
                    },
                    "sort": {
                        "type": "string",
                        "description": "Sort order: 'alpha' (default), 'size', 'indexed'",
                        "default": "alpha",
                        "enum": ["alpha", "size", "indexed"]
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
                // Enforce maximum limit
                if requested > LIST_DIR_MAX_LIMIT {
                    LIST_DIR_MAX_LIMIT
                } else {
                    requested
                }
            }
            None => LIST_DIR_DEFAULT_LIMIT, // Use default if not specified
        };

        // Get all files from index
        let mut all_files = self.get_file_list(&args.session, sort).await?;
        let total_count = all_files.len();

        // Truncate to effective limit
        all_files.truncate(effective_limit);
        let shown_count = all_files.len();

        // Determine if truncation warning needed
        let was_truncated = total_count > shown_count;

        // Build output
        let mut output = String::new();

        // Add warning if truncated
        if was_truncated {
            let warning = build_list_dir_warning(shown_count, total_count, &args.session);
            output.push_str(&warning);
        }

        // Add file list
        let formatted = self.format_file_list(&args.session, &all_files, total_count);
        output.push_str(&formatted);

        Ok(text_content(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::SessionConfig;
    use crate::types::Chunk;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_handler() -> (ListDirHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(ShebeServices::new(config));
        let handler = ListDirHandler::new(services);

        (handler, temp_dir)
    }

    async fn create_test_session_with_files(
        services: &Arc<ShebeServices>,
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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Session:** `test-session`"));
        assert!(text.contains("**Files:** 3 (showing 3)"));
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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("(showing 2)"));

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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

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
                ("/tmp/shebe-test-small.rs", "fn test() {}"), // ~13 bytes
                (
                    "/tmp/shebe-test-large.rs",
                    "fn test() {}\n".repeat(100).as_str(),
                ), // ~1300 bytes
                (
                    "/tmp/shebe-test-medium.rs",
                    "fn test() {}\n".repeat(10).as_str(),
                ), // ~130 bytes
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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("(showing 100)"));

        // Cleanup
        for i in 0..150 {
            let _ = fs::remove_file(format!("/tmp/shebe-test-{:03}.rs", i));
        }
    }

    // New truncation tests

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

        // Call without limit parameter - should use default (100)
        let args = json!({
            "session": "truncate-session",
            "sort": "alpha"
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should show warning (200 files > 100 default)
        assert!(text.contains("⚠️ OUTPUT TRUNCATED"));
        assert!(text.contains("200")); // total files

        // Count actual file entries (should be exactly 100)
        let file_count = text.matches("| `/tmp/shebe-truncate-").count();
        assert_eq!(file_count, LIST_DIR_DEFAULT_LIMIT);

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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should enforce max limit of 500
        let file_count = text.matches("| `/tmp/shebe-maxlimit-").count();
        assert_eq!(file_count, LIST_DIR_MAX_LIMIT);

        // Should show warning
        assert!(text.contains("⚠️ OUTPUT TRUNCATED"));
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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should NOT show warning (50 files < 100 default)
        assert!(!text.contains("⚠️ OUTPUT TRUNCATED"));

        // Should show all 50 files
        let file_count = text.matches("| `/tmp/shebe-small-").count();
        assert_eq!(file_count, 50);

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
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Should show exactly 250 files
        let file_count = text.matches("| `/tmp/shebe-userlimit-").count();
        assert_eq!(file_count, 250);

        // Should show warning (400 total > 250 shown)
        assert!(text.contains("⚠️ OUTPUT TRUNCATED"));
        assert!(text.contains("250 of 400"));

        // Cleanup
        for i in 0..400 {
            let _ = fs::remove_file(format!("/tmp/shebe-userlimit-{:03}.rs", i));
        }
    }
}
