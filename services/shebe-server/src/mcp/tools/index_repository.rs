//! Repository indexing tool handler
//!
//! Implements the index_repository MCP tool for indexing code repositories
//! directly from Claude Code.

use super::handler::{text_content, McpToolHandler};
use super::helpers::format_time_ago;
use crate::core::services::Services;
use crate::core::storage::SCHEMA_VERSION;
use crate::mcp::error::McpError;
use crate::mcp::protocol::ToolResult;
use crate::mcp::protocol::ToolSchema;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Default include pattern (all files)
const DEFAULT_INCLUDE: &[&str] = &["**/*"];

/// Default exclude patterns (common build/dependency directories)
const DEFAULT_EXCLUDE: &[&str] = &[
    "**/target/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/dist/**",
    "**/build/**",
    "**/*.pyc",
    "**/__pycache__/**",
    "**/.venv/**",
    "**/venv/**",
];

/// Request parameters for index_repository tool
#[derive(Debug, Deserialize)]
struct IndexRequest {
    /// Absolute path to repository
    path: String,
    /// Session identifier
    session: String,
    /// Glob patterns to include (optional)
    #[serde(default)]
    include_patterns: Option<Vec<String>>,
    /// Glob patterns to exclude (optional)
    #[serde(default)]
    exclude_patterns: Option<Vec<String>>,
    /// Characters per chunk (optional, default: 512)
    #[serde(default = "default_chunk_size")]
    chunk_size: usize,
    /// Overlap between chunks (optional, default: 64)
    #[serde(default = "default_overlap")]
    overlap: usize,
    /// Force re-indexing if session exists (optional, default: true)
    #[serde(default = "default_force")]
    force: bool,
}

fn default_chunk_size() -> usize {
    512
}

fn default_overlap() -> usize {
    64
}

fn default_force() -> bool {
    true
}

/// Handler for index_repository MCP tool
pub struct IndexRepositoryHandler {
    services: Arc<Services>,
}

impl IndexRepositoryHandler {
    /// Create new index_repository handler
    pub fn new(services: Arc<Services>) -> Self {
        Self { services }
    }

    /// Validate and canonicalize repository path
    fn validate_path(path: &str) -> Result<PathBuf, McpError> {
        let path = PathBuf::from(path);

        // Must be absolute
        if !path.is_absolute() {
            return Err(McpError::InvalidParams("Path must be absolute".to_string()));
        }

        // Must exist
        if !path.exists() {
            return Err(McpError::InvalidParams(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        // Must be a directory
        if !path.is_dir() {
            return Err(McpError::InvalidParams(
                "Path must be a directory".to_string(),
            ));
        }

        // Canonicalize to prevent path traversal attacks
        let canonical = path
            .canonicalize()
            .map_err(|e| McpError::InvalidParams(format!("Cannot resolve path: {e}")))?;

        Ok(canonical)
    }

    /// Validate session identifier
    fn validate_session(session: &str) -> Result<(), McpError> {
        // Length check
        if session.is_empty() || session.len() > 64 {
            return Err(McpError::InvalidParams(
                "Session must be 1-64 characters".to_string(),
            ));
        }

        // Character check (alphanumeric, hyphen, underscore)
        if !session
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(McpError::InvalidParams(
                "Session must contain only alphanumeric, hyphen, underscore".to_string(),
            ));
        }

        // Must start with alphanumeric
        if !session.chars().next().unwrap().is_alphanumeric() {
            return Err(McpError::InvalidParams(
                "Session must start with alphanumeric character".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate chunk size parameter
    fn validate_chunk_size(size: usize) -> Result<(), McpError> {
        if !(100..=2000).contains(&size) {
            return Err(McpError::InvalidParams(
                "Chunk size must be between 100 and 2000 characters".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate overlap parameter
    fn validate_overlap(overlap: usize) -> Result<(), McpError> {
        if overlap > 500 {
            return Err(McpError::InvalidParams(
                "Overlap must not exceed 500 characters".to_string(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl McpToolHandler for IndexRepositoryHandler {
    fn name(&self) -> &str {
        "index_repository"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "index_repository".to_string(),
            description: "Index a code repository for BM25 full-text search (REQUIRED before search_code works). \
                         Runs SYNCHRONOUSLY (blocks until complete) and returns actual statistics. \
                         \
                         PERFORMANCE (tested on 6,364 files): \
                         - Small repos (<100 files): 1-4 seconds, \
                         - Medium repos (~1,000 files): 2-4 seconds, \
                         - Large repos (~6,000 files): 10-15 seconds, \
                         - Very large repos (~10,000 files): 20-30 seconds. \
                         Throughput: 1,500-2,000 files/sec (varies with system load). \
                         \
                         CREATES A SESSION for future search_code queries. Session persists until deleted. \
                         Supports polyglot codebases (PHP+SQL+JS+HTML+CSS+Rust+Python+etc). \
                         \
                         FILE FILTERING: Use glob patterns. Defaults exclude build artifacts (target/, node_modules/, \
                         .git/, dist/, __pycache__/). Customize with include_patterns and exclude_patterns. \
                         \
                         CHUNKING: Default 512 chars/chunk with 64 char overlap. Increase chunk_size (max 2000) \
                         for verbose languages (Java, C++), decrease (min 100) for dense code (Python, Ruby)."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the repository to index"
                    },
                    "session": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9][a-zA-Z0-9-_]{0,63}$",
                        "description": "Unique session identifier (alphanumeric, hyphens, underscores, max 64 chars)"
                    },
                    "include_patterns": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns for files to include (default: all)",
                        "default": DEFAULT_INCLUDE
                    },
                    "exclude_patterns": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns for files to exclude",
                        "default": DEFAULT_EXCLUDE
                    },
                    "chunk_size": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 2000,
                        "default": 512,
                        "description": "Number of characters per chunk"
                    },
                    "overlap": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 500,
                        "default": 64,
                        "description": "Number of overlapping characters between chunks"
                    },
                    "force": {
                        "type": "boolean",
                        "default": true,
                        "description": "Re-index even if session exists. Default is true (always re-indexes). \
                                       Set to false to skip if session exists."
                    }
                },
                "required": ["path", "session"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, McpError> {
        // Parse request
        let req: IndexRequest = serde_json::from_value(args)
            .map_err(|e| McpError::InvalidParams(format!("Invalid parameters: {e}")))?;

        // Validate parameters
        let path = Self::validate_path(&req.path)?;
        Self::validate_session(&req.session)?;
        Self::validate_chunk_size(req.chunk_size)?;
        Self::validate_overlap(req.overlap)?;

        // Check if session already exists (unless force)
        let session_exists = self.services.storage.session_exists(&req.session);

        if session_exists && !req.force {
            // Get metadata for enhanced error message
            let metadata = self
                .services
                .storage
                .get_session_metadata(&req.session)
                .map_err(McpError::from)?;

            let schema_status = if metadata.schema_version == SCHEMA_VERSION {
                "current"
            } else {
                "outdated"
            };

            return Err(McpError::InvalidParams(format!(
                "Session '{}' already exists.\n\
                 - Last indexed: {} ({})\n\
                 - Files indexed: {}\n\
                 - Schema version: v{} ({})\n\
                 Use force=true to re-index, or use existing session for search.",
                req.session,
                metadata.last_indexed_at.format("%Y-%m-%d %H:%M UTC"),
                format_time_ago(metadata.last_indexed_at),
                metadata.files_indexed,
                metadata.schema_version,
                schema_status
            )));
        }

        // Prepare indexing configuration
        let include_patterns = req
            .include_patterns
            .unwrap_or_else(|| DEFAULT_INCLUDE.iter().map(|s| s.to_string()).collect());
        let exclude_patterns = req
            .exclude_patterns
            .unwrap_or_else(|| DEFAULT_EXCLUDE.iter().map(|s| s.to_string()).collect());

        // Get max file size from config
        let max_file_size_mb = self.services.config.indexing.max_file_size_mb;

        // Index repository synchronously
        let stats = self.services.storage.index_repository(
            &req.session,
            &path,
            include_patterns,
            exclude_patterns,
            req.chunk_size,
            req.overlap,
            max_file_size_mb,
            req.force,
        )?;

        // Format completion message
        let message = format!(
            "Indexing complete!\n\
             Files indexed: {}\n\
             Chunks created: {}\n\
             Duration: {:.1}s",
            stats.files_indexed,
            stats.chunks_created,
            stats.duration_ms as f64 / 1000.0
        );

        Ok(text_content(message))
    }
}
