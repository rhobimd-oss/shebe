//! Find file by pattern tool handler

use super::handler::{text_content, McpToolHandler};
use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{ToolResult, ToolSchema};
use async_trait::async_trait;
use glob::Pattern as GlobPattern;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value as TantivyValue;
use tantivy::TantivyDocument;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 10000;

#[derive(Debug, Clone)]
pub enum PatternType {
    Glob,
    Regex,
}

impl PatternType {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "glob" => Ok(Self::Glob),
            "regex" => Ok(Self::Regex),
            _ => Err(format!(
                "Invalid pattern_type: '{s}'. Must be 'glob' or 'regex'."
            )),
        }
    }
}

pub struct FindFileHandler {
    services: Arc<Services>,
}

impl FindFileHandler {
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Get all file paths from session (helper)
    async fn get_all_file_paths(&self, session: &str) -> Result<Vec<String>, McpError> {
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
        let query = AllQuery;

        let file_path_field = index
            .schema()
            .get_field("file_path")
            .map_err(|e| McpError::InternalError(format!("file_path field missing: {e}")))?;

        let mut files = HashSet::new();

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(100000))
            .map_err(|e| McpError::InternalError(format!("Search failed: {e}")))?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| McpError::InternalError(format!("Doc retrieval failed: {e}")))?;

            if let Some(path) = retrieved_doc
                .get_first(file_path_field)
                .and_then(|v| v.as_str())
            {
                files.insert(path.to_string());
            }
        }

        Ok(files.into_iter().collect())
    }

    /// Match files using pattern
    async fn find_matching_files(
        &self,
        session: &str,
        pattern: &str,
        pattern_type: PatternType,
        limit: usize,
    ) -> Result<Vec<String>, McpError> {
        // Get all files from session
        let all_files = self.get_all_file_paths(session).await?;

        // Compile pattern and filter
        let matches: Vec<String> = match pattern_type {
            PatternType::Glob => {
                let glob = GlobPattern::new(pattern).map_err(|e| {
                    McpError::InvalidParams(format!("Invalid glob pattern '{pattern}': {e}"))
                })?;

                all_files
                    .into_iter()
                    .filter(|path| glob.matches(path))
                    .take(limit)
                    .collect()
            }
            PatternType::Regex => {
                let re = Regex::new(pattern).map_err(|e| {
                    McpError::InvalidParams(format!("Invalid regex pattern '{pattern}': {e}"))
                })?;

                all_files
                    .into_iter()
                    .filter(|path| re.is_match(path))
                    .take(limit)
                    .collect()
            }
        };

        Ok(matches)
    }

    /// Format results
    fn format_results(
        &self,
        session: &str,
        pattern: &str,
        matches: &[String],
        total_files: usize,
    ) -> String {
        let mut output = format!(
            "**Session:** `{}`\n\
             **Pattern:** `{}`\n\
             **Matches:** {} of {} total files\n\n",
            session,
            pattern,
            matches.len(),
            total_files
        );

        if matches.is_empty() {
            output
                .push_str("No files match the pattern. Try a different pattern or check session.");
            return output;
        }

        output.push_str("**Matched Files:**\n");
        for path in matches {
            output.push_str(&format!("- `{path}`\n"));
        }

        output
    }
}

#[async_trait]
impl McpToolHandler for FindFileHandler {
    fn name(&self) -> &str {
        "find_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "find_file".to_string(),
            description: "Find files by name/path pattern (like 'find' command). \
                         Supports glob patterns (*.rs, **/test/**/*.py) and regex. \
                         Use when you want to filter files by pattern. \
                         For listing all files without filtering, use list_dir. \
                         Examples: '*.rs' (all Rust), '**/test_*.py' (test files), \
                         '.*Controller\\.php$' (regex)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session ID to search",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Glob or regex pattern. Examples: '*.rs', '**/src/**/*.py', \
                                       '.*test.*' (regex)",
                        "minLength": 1
                    },
                    "pattern_type": {
                        "type": "string",
                        "description": "Pattern type: 'glob' (default) or 'regex'",
                        "default": "glob",
                        "enum": ["glob", "regex"]
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default: 100, max: 10000)",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 10000
                    }
                },
                "required": ["session", "pattern"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct FindFileArgs {
            session: String,
            pattern: String,
            #[serde(default = "default_pattern_type")]
            pattern_type: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_pattern_type() -> String {
            "glob".to_string()
        }
        fn default_limit() -> usize {
            DEFAULT_LIMIT
        }

        // Parse arguments
        let args: FindFileArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))?;

        // Validate pattern
        if args.pattern.trim().is_empty() {
            return Err(McpError::InvalidParams(
                "pattern cannot be empty".to_string(),
            ));
        }

        // Validate limit
        if args.limit > MAX_LIMIT {
            return Err(McpError::InvalidParams(format!(
                "limit cannot exceed {MAX_LIMIT}"
            )));
        }

        // Parse pattern type
        let pattern_type =
            PatternType::from_str(&args.pattern_type).map_err(McpError::InvalidParams)?;

        // Find matching files
        let total_files = self.get_all_file_paths(&args.session).await?.len();
        let matches = self
            .find_matching_files(&args.session, &args.pattern, pattern_type, args.limit)
            .await?;

        // Format response
        let formatted = self.format_results(&args.session, &args.pattern, &matches, total_files);

        Ok(text_content(formatted))
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

    async fn setup_test_handler() -> (FindFileHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();

        let services = Arc::new(Services::new(config));
        let handler = FindFileHandler::new(services);

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

    #[tokio::test]
    async fn test_find_glob_simple() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test.rs", "fn main() {}"),
                ("/tmp/shebe-main.rs", "fn test() {}"),
                ("/tmp/shebe-lib.py", "def test(): pass"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": "*.rs",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Matches:** 2 of 3"));
        assert!(text.contains("shebe-test.rs"));
        assert!(text.contains("shebe-main.rs"));
        assert!(!text.contains("lib.py"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test.rs");
        let _ = fs::remove_file("/tmp/shebe-main.rs");
        let _ = fs::remove_file("/tmp/shebe-lib.py");
    }

    #[tokio::test]
    async fn test_find_glob_recursive() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe/src/test.rs", "fn test() {}"),
                ("/tmp/shebe/tests/test.rs", "fn integration() {}"),
                ("/tmp/shebe/main.rs", "fn main() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": "**/test*.rs",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Matches:** 2 of 3"));
        assert!(text.contains("src/test.rs"));
        assert!(text.contains("tests/test.rs"));
        assert!(!text.contains("main.rs"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe/src/test.rs");
        let _ = fs::remove_file("/tmp/shebe/tests/test.rs");
        let _ = fs::remove_file("/tmp/shebe/main.rs");
        let _ = fs::remove_dir("/tmp/shebe/src");
        let _ = fs::remove_dir("/tmp/shebe/tests");
        let _ = fs::remove_dir("/tmp/shebe");
    }

    #[tokio::test]
    async fn test_find_regex() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe/TestController.php", "class TestController {}"),
                ("/tmp/shebe/UserController.php", "class UserController {}"),
                ("/tmp/shebe/config.php", "return [];"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": r".*Controller\.php$",
            "pattern_type": "regex",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Matches:** 2 of 3"));
        assert!(text.contains("TestController.php"));
        assert!(text.contains("UserController.php"));
        assert!(!text.contains("config.php"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe/TestController.php");
        let _ = fs::remove_file("/tmp/shebe/UserController.php");
        let _ = fs::remove_file("/tmp/shebe/config.php");
        let _ = fs::remove_dir("/tmp/shebe");
    }

    #[tokio::test]
    async fn test_find_no_matches() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![
                ("/tmp/shebe-test.rs", "fn main() {}"),
                ("/tmp/shebe-lib.rs", "fn test() {}"),
            ],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": "*.py",
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        assert!(text.contains("**Matches:** 0 of 2"));
        assert!(text.contains("No files match the pattern"));

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test.rs");
        let _ = fs::remove_file("/tmp/shebe-lib.rs");
    }

    #[tokio::test]
    async fn test_find_invalid_glob() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![("/tmp/shebe-test.rs", "fn main() {}")],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": "[unclosed",
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("Invalid glob pattern"));
        } else {
            panic!("Expected InvalidParams error for invalid glob");
        }

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test.rs");
    }

    #[tokio::test]
    async fn test_find_invalid_regex() {
        let (handler, _temp) = setup_test_handler().await;
        create_test_session_with_files(
            &handler.services,
            "test-session",
            vec![("/tmp/shebe-test.rs", "fn main() {}")],
        )
        .await;

        let args = json!({
            "session": "test-session",
            "pattern": "(unclosed",
            "pattern_type": "regex",
        });

        let result = handler.execute(args).await;
        assert!(result.is_err());

        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("Invalid regex pattern"));
        } else {
            panic!("Expected InvalidParams error for invalid regex");
        }

        // Cleanup
        let _ = fs::remove_file("/tmp/shebe-test.rs");
    }

    #[tokio::test]
    async fn test_find_with_limit() {
        let (handler, _temp) = setup_test_handler().await;

        let files: Vec<_> = (0..20)
            .map(|i| (format!("/tmp/shebe-test-{:02}.rs", i), "fn test() {}"))
            .collect();
        let file_refs: Vec<_> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();

        create_test_session_with_files(&handler.services, "test-session", file_refs).await;

        let args = json!({
            "session": "test-session",
            "pattern": "*.rs",
            "limit": 10,
        });

        let result = handler.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let text = match &tool_result.content[0] {
            crate::mcp::protocol::ContentBlock::Text { text } => text,
        };

        // Count matched files - should be 10 (limited)
        let match_count = text.matches("- `").count();
        assert_eq!(match_count, 10);

        // Cleanup
        for i in 0..20 {
            let _ = fs::remove_file(format!("/tmp/shebe-test-{:02}.rs", i));
        }
    }
}
