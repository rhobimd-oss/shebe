//! Tantivy integration for BM25 full-text search.
//!
//! This module wraps Tantivy operations for creating,
//! managing and searching indexes.

use crate::core::error::{Result, ShebeError};
use crate::core::types::Chunk;
use chrono::Utc;
use std::path::Path;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter};

/// Current schema version
/// Version 1: Initial schema (chunk_index STORED only)
/// Version 2: Added INDEXED flag to chunk_index for preview_chunk queries
/// Version 3: Added repository_path, last_indexed_at and patterns to SessionMetadata
pub const SCHEMA_VERSION: u32 = 3;

/// Create the Tantivy schema for chunk indexing
///
/// Fields:
/// - text: Full-text searchable content (TEXT | STORED)
/// - file_path: Source file path (STRING | STORED)
/// - session: Session identifier (STRING | STORED)
/// - offset_start: Byte offset start (i64 | STORED)
/// - offset_end: Byte offset end (i64 | STORED)
/// - chunk_index: Sequential chunk number (i64 | STORED)
/// - indexed_at: Timestamp (Date | STORED)
pub fn create_schema() -> Schema {
    let mut builder = Schema::builder();

    // Searchable text content
    builder.add_text_field("text", TEXT | STORED);

    // Metadata (stored for retrieval)
    builder.add_text_field("file_path", STRING | STORED);
    builder.add_text_field("session", STRING | STORED);

    // Offset fields for highlighting
    builder.add_i64_field("offset_start", STORED);
    builder.add_i64_field("offset_end", STORED);
    builder.add_i64_field("chunk_index", INDEXED | STORED);

    // Timestamp
    builder.add_date_field("indexed_at", STORED);

    builder.build()
}

/// Tantivy index wrapper
pub struct TantivyIndex {
    /// Tantivy index instance
    index: Index,

    /// Schema definition
    schema: Schema,

    /// Index writer (for adding documents)
    writer: IndexWriter,
}

impl std::fmt::Debug for TantivyIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyIndex")
            .field("schema", &"<schema>")
            .finish()
    }
}

impl TantivyIndex {
    /// Create a new Tantivy index at the given path
    pub fn create(index_dir: &Path) -> Result<Self> {
        // Create schema
        let schema = create_schema();

        // Create index directory
        std::fs::create_dir_all(index_dir)?;

        // Create Tantivy index
        let index = Index::create_in_dir(index_dir, schema.clone())
            .map_err(|e| ShebeError::StorageError(format!("Failed to create index: {e}")))?;

        // Create index writer (50MB heap)
        let writer = index
            .writer(50_000_000)
            .map_err(|e| ShebeError::StorageError(format!("Failed to create writer: {e}")))?;

        Ok(Self {
            index,
            schema,
            writer,
        })
    }

    /// Open an existing Tantivy index
    pub fn open(index_dir: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_dir)
            .map_err(|e| ShebeError::StorageError(format!("Failed to open index: {e}")))?;

        let schema = index.schema();

        let writer = index
            .writer(50_000_000)
            .map_err(|e| ShebeError::StorageError(format!("Failed to create writer: {e}")))?;

        Ok(Self {
            index,
            schema,
            writer,
        })
    }

    /// Add chunks to the index (batch operation)
    pub fn add_chunks(&mut self, chunks: &[Chunk], session_id: &str) -> Result<()> {
        // Get schema fields
        let text_field = self
            .schema
            .get_field("text")
            .map_err(|e| ShebeError::StorageError(format!("Missing text field: {e}")))?;
        let file_path_field = self
            .schema
            .get_field("file_path")
            .map_err(|e| ShebeError::StorageError(format!("Missing file_path field: {e}")))?;
        let session_field = self
            .schema
            .get_field("session")
            .map_err(|e| ShebeError::StorageError(format!("Missing session field: {e}")))?;
        let offset_start_field = self
            .schema
            .get_field("offset_start")
            .map_err(|e| ShebeError::StorageError(format!("Missing offset_start field: {e}")))?;
        let offset_end_field = self
            .schema
            .get_field("offset_end")
            .map_err(|e| ShebeError::StorageError(format!("Missing offset_end field: {e}")))?;
        let chunk_index_field = self
            .schema
            .get_field("chunk_index")
            .map_err(|e| ShebeError::StorageError(format!("Missing chunk_index field: {e}")))?;
        let indexed_at_field = self
            .schema
            .get_field("indexed_at")
            .map_err(|e| ShebeError::StorageError(format!("Missing indexed_at field: {e}")))?;

        let now = Utc::now();

        // Add each chunk as a document
        for chunk in chunks {
            let doc = doc!(
                text_field => chunk.text.as_str(),
                file_path_field =>
                    chunk.file_path.to_str().unwrap_or(""),
                session_field => session_id,
                offset_start_field => chunk.start_offset as i64,
                offset_end_field => chunk.end_offset as i64,
                chunk_index_field => chunk.chunk_index as i64,
                indexed_at_field => tantivy::DateTime::from_timestamp_secs(
                    now.timestamp()
                ),
            );

            self.writer
                .add_document(doc)
                .map_err(|e| ShebeError::StorageError(format!("Failed to add document: {e}")))?;
        }

        Ok(())
    }

    /// Commit changes to disk
    pub fn commit(&mut self) -> Result<()> {
        self.writer
            .commit()
            .map_err(|e| ShebeError::StorageError(format!("Failed to commit: {e}")))?;
        Ok(())
    }

    /// Get an index reader for searching
    pub fn reader(&self) -> Result<IndexReader> {
        self.index
            .reader()
            .map_err(|e| ShebeError::StorageError(format!("Failed to create reader: {e}")))
    }

    /// Get the schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Get a reference to the underlying Tantivy index
    pub fn index(&self) -> &Index {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_schema_has_all_fields() {
        let schema = create_schema();

        // Verify all 7 fields exist
        assert!(schema.get_field("text").is_ok());
        assert!(schema.get_field("file_path").is_ok());
        assert!(schema.get_field("session").is_ok());
        assert!(schema.get_field("offset_start").is_ok());
        assert!(schema.get_field("offset_end").is_ok());
        assert!(schema.get_field("chunk_index").is_ok());
        assert!(schema.get_field("indexed_at").is_ok());
    }

    #[test]
    fn test_create_new_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");

        let index = TantivyIndex::create(&index_dir);
        assert!(index.is_ok());

        // Verify directory was created
        assert!(index_dir.exists());
    }

    #[test]
    fn test_create_and_open_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");

        // Create index
        let mut index = TantivyIndex::create(&index_dir).unwrap();

        // Add test chunk
        let chunk = Chunk {
            text: "test content".to_string(),
            file_path: PathBuf::from("/test/file.rs"),
            start_offset: 0,
            end_offset: 12,
            chunk_index: 0,
        };

        index.add_chunks(&[chunk], "test-session").unwrap();
        index.commit().unwrap();

        // Drop the index to release file locks
        drop(index);

        // Reopen index
        let reopened = TantivyIndex::open(&index_dir).unwrap();
        assert!(reopened.schema().get_field("text").is_ok());
    }

    #[test]
    fn test_add_multiple_chunks() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");
        let mut index = TantivyIndex::create(&index_dir).unwrap();

        let chunks = vec![
            Chunk {
                text: "chunk 1".to_string(),
                file_path: PathBuf::from("/test/file1.rs"),
                start_offset: 0,
                end_offset: 7,
                chunk_index: 0,
            },
            Chunk {
                text: "chunk 2".to_string(),
                file_path: PathBuf::from("/test/file1.rs"),
                start_offset: 7,
                end_offset: 14,
                chunk_index: 1,
            },
            Chunk {
                text: "chunk 3".to_string(),
                file_path: PathBuf::from("/test/file2.rs"),
                start_offset: 0,
                end_offset: 7,
                chunk_index: 0,
            },
        ];

        let result = index.add_chunks(&chunks, "test-session");
        assert!(result.is_ok());

        let commit_result = index.commit();
        assert!(commit_result.is_ok());
    }

    #[test]
    fn test_empty_chunks_vector() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");
        let mut index = TantivyIndex::create(&index_dir).unwrap();

        // Empty chunks should succeed (no-op)
        let result = index.add_chunks(&[], "test-session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_nonexistent_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("nonexistent");

        let result = TantivyIndex::open(&index_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_index_is_indexed() {
        let schema = create_schema();
        let chunk_index_field = schema.get_field("chunk_index").unwrap();
        let field_entry = schema.get_field_entry(chunk_index_field);

        // Verify chunk_index field is indexed (required for preview_chunk queries)
        assert!(
            field_entry.is_indexed(),
            "chunk_index field must be INDEXED to support preview_chunk tool queries"
        );
    }

    #[test]
    fn test_schema_version_constant() {
        // Verify schema version is set to 2 after adding INDEXED flag to chunk_index
        assert_eq!(
            SCHEMA_VERSION, 3,
            "SCHEMA_VERSION should be 3 after adding repository_path and patterns"
        );
    }
}
