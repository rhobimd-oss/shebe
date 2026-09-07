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
/// Version 4: Tantivy 0.26 (index format 7, nanosecond dates), field set unchanged
pub const SCHEMA_VERSION: u32 = 4;

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

/// Write-path Tantivy index wrapper (create, add, commit)
pub struct TantivyIndex {
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

/// Writer-lock retry budget. The lock is an OS advisory lock on
/// `.tantivy-writer.lock`, held for the full duration of an index run
/// and released by the OS when the holder exits. Keep the budget short
/// so a busy session reports quickly instead of stalling the caller.
const WRITER_LOCK_ATTEMPTS: u32 = 5;
const WRITER_LOCK_BASE_DELAY_MS: u64 = 100;

/// Acquire the index writer, with a bounded retry on the cross-process
/// writer lock. Only `TantivyError::LockFailure` retries: every other
/// writer error does not heal with time and fails at once.
fn acquire_writer(index: &Index, index_dir: &Path, session_id: &str) -> Result<IndexWriter> {
    let mut waited_ms = 0u64;
    for attempt in 1..=WRITER_LOCK_ATTEMPTS {
        match index.writer(50_000_000) {
            Ok(writer) => return Ok(writer),
            Err(tantivy::TantivyError::LockFailure(..)) if attempt < WRITER_LOCK_ATTEMPTS => {
                let delay = backoff_delay_ms(attempt);
                waited_ms += delay;
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            Err(tantivy::TantivyError::LockFailure(..)) => {
                return Err(ShebeError::IndexLocked {
                    session: session_id.to_string(),
                    lock_path: index_dir.join(".tantivy-writer.lock").display().to_string(),
                    attempts: WRITER_LOCK_ATTEMPTS,
                    waited_ms,
                });
            }
            Err(e) => {
                return Err(ShebeError::StorageError(format!(
                    "Failed to create writer: {e}"
                )));
            }
        }
    }
    unreachable!("the final attempt returns above")
}

/// Exponential backoff with additive jitter: 100, 200, 400, 800 ms plus
/// 0-25 percent. Jitter comes from the clock subsecond nanoseconds, so no
/// rand crate is needed for one desynchronization hint.
fn backoff_delay_ms(attempt: u32) -> u64 {
    let base = WRITER_LOCK_BASE_DELAY_MS << (attempt - 1);
    let jitter_cap = base / 4;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    base + nanos % (jitter_cap + 1)
}

impl TantivyIndex {
    /// Create a new Tantivy index at the given path
    pub fn create(index_dir: &Path, session_id: &str) -> Result<Self> {
        // Create schema
        let schema = create_schema();

        // Create index directory
        std::fs::create_dir_all(index_dir)?;

        // Create Tantivy index
        let index = Index::create_in_dir(index_dir, schema.clone())
            .map_err(|e| ShebeError::StorageError(format!("Failed to create index: {e}")))?;

        // Create index writer (50MB heap) behind the lock retry. The writer
        // keeps the index alive internally, so the handle itself is not stored.
        let writer = acquire_writer(&index, index_dir, session_id)?;

        Ok(Self { schema, writer })
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
}

/// Read-only Tantivy index handle. Takes no writer lock and no writer heap.
pub struct TantivyReader {
    /// Tantivy index instance
    index: Index,

    /// Schema definition
    schema: Schema,

    /// Shared index reader (cheap Arc handle)
    reader: IndexReader,
}

impl std::fmt::Debug for TantivyReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivyReader")
            .field("schema", &"<schema>")
            .finish()
    }
}

impl TantivyReader {
    /// Open an existing Tantivy index for reading only
    pub fn open(index_dir: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_dir)
            .map_err(|e| ShebeError::StorageError(format!("Failed to open index: {e}")))?;

        let schema = index.schema();

        let reader = index
            .reader()
            .map_err(|e| ShebeError::StorageError(format!("Failed to open reader: {e}")))?;

        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Get an index reader for searching
    pub fn reader(&self) -> Result<IndexReader> {
        Ok(self.reader.clone())
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

        let index = TantivyIndex::create(&index_dir, "test-session");
        assert!(index.is_ok());

        // Verify directory was created
        assert!(index_dir.exists());
    }

    #[test]
    fn test_reader_open_existing_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");

        // Create index
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

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

        // Drop the index to release the writer lock
        drop(index);

        // Reopen for reading
        let reopened = TantivyReader::open(&index_dir).unwrap();
        assert!(reopened.schema().get_field("text").is_ok());
    }

    #[test]
    fn test_add_multiple_chunks() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("test_index");
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

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
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

        // Empty chunks should succeed (no-op)
        let result = index.add_chunks(&[], "test-session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_reader_open_nonexistent_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("nonexistent");

        let result = TantivyReader::open(&index_dir);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Failed to open index"),
            "Error should keep the existing open-failure text: {}",
            err_msg
        );
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
        // Verify schema version is set to 4 for the tantivy 0.26 migration (index format 7)
        assert_eq!(
            SCHEMA_VERSION, 4,
            "SCHEMA_VERSION should be 4 after the tantivy 0.26 migration"
        );
    }

    // --- Phase 1C: Boundary tests ---

    #[test]
    fn test_search_empty_index() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("empty_index");
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

        // Commit empty index so reader can access it
        index.commit().unwrap();

        let read_handle = TantivyReader::open(&index_dir).unwrap();
        let reader = read_handle.reader().unwrap();
        let searcher = reader.searcher();
        let schema = read_handle.schema();

        let text_field = schema.get_field("text").unwrap();
        let query_parser =
            tantivy::query::QueryParser::for_index(read_handle.index(), vec![text_field]);
        let query = query_parser.parse_query("anything").unwrap();

        let top_docs = searcher
            .search(
                &query,
                &tantivy::collector::TopDocs::with_limit(10).order_by_score(),
            )
            .unwrap();

        assert!(top_docs.is_empty(), "Empty index should return no results");
    }

    #[test]
    fn test_reader_after_adding_chunks() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("search_index");
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

        let chunks = vec![
            Chunk {
                text: "fn hello_world() { println!(\"hello\"); }".to_string(),
                file_path: PathBuf::from("/src/main.rs"),
                start_offset: 0,
                end_offset: 40,
                chunk_index: 0,
            },
            Chunk {
                text: "fn goodbye() { println!(\"bye\"); }".to_string(),
                file_path: PathBuf::from("/src/lib.rs"),
                start_offset: 0,
                end_offset: 34,
                chunk_index: 0,
            },
        ];

        index.add_chunks(&chunks, "test-session").unwrap();
        index.commit().unwrap();

        let read_handle = TantivyReader::open(&index_dir).unwrap();
        let reader = read_handle.reader().unwrap();
        let searcher = reader.searcher();
        let schema = read_handle.schema();

        let text_field = schema.get_field("text").unwrap();
        let query_parser =
            tantivy::query::QueryParser::for_index(read_handle.index(), vec![text_field]);
        let query = query_parser.parse_query("hello_world").unwrap();

        let top_docs = searcher
            .search(
                &query,
                &tantivy::collector::TopDocs::with_limit(10).order_by_score(),
            )
            .unwrap();

        assert_eq!(top_docs.len(), 1, "Should find exactly one match");
    }

    #[test]
    fn test_debug_impl() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("debug_index");
        let index = TantivyIndex::create(&index_dir, "test-session").unwrap();

        let debug_str = format!("{:?}", index);
        assert!(
            debug_str.contains("TantivyIndex"),
            "Debug output should contain struct name"
        );
    }

    #[test]
    fn test_reader_open_while_writer_lock_held() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("locked_index");

        // The live TantivyIndex holds the file-based writer lock, so one
        // process reproduces the two-process contention.
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();
        index.commit().unwrap();

        let read_handle = TantivyReader::open(&index_dir);
        assert!(
            read_handle.is_ok(),
            "Reader must open while a writer holds the lock"
        );

        // The writer stays alive past the reader open
        drop(index);
    }

    #[test]
    fn test_reader_sees_last_commit_only() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("snapshot_index");
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();

        let committed_chunk = Chunk {
            text: "committed content".to_string(),
            file_path: PathBuf::from("/src/committed.rs"),
            start_offset: 0,
            end_offset: 17,
            chunk_index: 0,
        };
        index
            .add_chunks(&[committed_chunk], "test-session")
            .unwrap();
        index.commit().unwrap();

        // Add a chunk WITHOUT commit: the reader must not see it
        let pending_chunk = Chunk {
            text: "pending content".to_string(),
            file_path: PathBuf::from("/src/pending.rs"),
            start_offset: 0,
            end_offset: 15,
            chunk_index: 0,
        };
        index.add_chunks(&[pending_chunk], "test-session").unwrap();

        let read_handle = TantivyReader::open(&index_dir).unwrap();
        let reader = read_handle.reader().unwrap();
        let searcher = reader.searcher();

        assert_eq!(
            searcher.num_docs(),
            1,
            "Reader must serve the last committed state only"
        );
    }

    #[test]
    fn test_reader_debug_impl() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("reader_debug_index");
        let mut index = TantivyIndex::create(&index_dir, "test-session").unwrap();
        index.commit().unwrap();
        drop(index);

        let read_handle = TantivyReader::open(&index_dir).unwrap();
        let debug_str = format!("{:?}", read_handle);
        assert!(
            debug_str.contains("TantivyReader"),
            "Debug output should contain struct name"
        );
    }

    // --- Phase 2: writer-lock retry tests ---
    //
    // The Tantivy writer lock is an OS advisory lock, so only a live
    // IndexWriter holds it. A live TantivyIndex therefore injects the
    // contention inside one process.

    #[test]
    fn test_writer_retry_succeeds_after_lock_release() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("retry_index");

        let holder = TantivyIndex::create(&index_dir, "test-session").unwrap();

        let releaser = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            drop(holder);
        });

        let index = Index::open_in_dir(&index_dir).unwrap();
        let result = acquire_writer(&index, &index_dir, "test-session");
        releaser.join().unwrap();

        assert!(
            result.is_ok(),
            "Writer must acquire the lock after the competing holder drops: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_writer_retry_exhausts_budget() {
        let temp_dir = tempdir().unwrap();
        let index_dir = temp_dir.path().join("busy_index");

        // The holder keeps the advisory lock for the full retry budget
        let holder = TantivyIndex::create(&index_dir, "test-session").unwrap();

        let index = Index::open_in_dir(&index_dir).unwrap();
        let result = acquire_writer(&index, &index_dir, "busy-session");
        drop(holder);

        match result {
            Err(ShebeError::IndexLocked {
                session,
                lock_path: reported_path,
                attempts,
                waited_ms,
            }) => {
                assert_eq!(session, "busy-session");
                assert_eq!(attempts, WRITER_LOCK_ATTEMPTS);
                assert!(waited_ms > 0, "Exhausted retry must report a wait time");
                assert!(
                    reported_path.ends_with(".tantivy-writer.lock"),
                    "Error must name the lock file: {reported_path}"
                );
            }
            Err(other) => panic!("Expected IndexLocked, got: {other:?}"),
            Ok(_) => panic!("Expected IndexLocked, got a writer"),
        }
    }

    #[test]
    fn test_backoff_delay_within_jitter_bounds() {
        for attempt in 1..=4u32 {
            let base = WRITER_LOCK_BASE_DELAY_MS << (attempt - 1);
            let delay = backoff_delay_ms(attempt);
            assert!(
                delay >= base && delay <= base + base / 4,
                "Attempt {attempt}: delay {delay} outside [{base}, {}]",
                base + base / 4
            );
        }
    }
}
