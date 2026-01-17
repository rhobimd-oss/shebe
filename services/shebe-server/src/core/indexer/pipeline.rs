//! Indexing pipeline orchestration.
//!
//! Coordinates the end-to-end indexing workflow:
//! 1. Walk directory tree
//! 2. Read file contents
//! 3. Chunk text
//! 4. Prepare chunks for storage

use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::core::error::{Result, ShebeError};
use crate::core::indexer::{Chunker, FileWalker};
use crate::core::types::{Chunk, IndexStats};

/// Orchestrates the indexing pipeline
pub struct IndexingPipeline {
    walker: FileWalker,
    chunker: Chunker,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of chunks in characters
    /// * `overlap` - Overlap between chunks in characters
    /// * `include_patterns` - Glob patterns for files to include
    /// * `exclude_patterns` - Glob patterns for files to exclude
    /// * `max_file_size_mb` - Maximum file size in megabytes
    ///
    /// # Returns
    ///
    /// A new `IndexingPipeline` instance
    pub fn new(
        chunk_size: usize,
        overlap: usize,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        max_file_size_mb: usize,
    ) -> Result<Self> {
        let walker = FileWalker::new(include_patterns, exclude_patterns, max_file_size_mb)?;
        let chunker = Chunker::new(chunk_size, overlap);

        Ok(Self { walker, chunker })
    }

    /// Index a directory and return chunks + stats
    ///
    /// Walks the directory tree, reads files, chunks content,
    /// and collects statistics. Errors reading individual files
    /// are logged but don't stop the process.
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory to index
    ///
    /// # Returns
    ///
    /// A tuple of (chunks, statistics) or an error
    pub fn index_directory(&self, root: &Path) -> Result<(Vec<Chunk>, IndexStats)> {
        let start = Instant::now();

        // Step 1: Collect files
        tracing::info!("Starting file collection from {:?}", root);
        let files = self.walker.collect_files(root)?;
        tracing::info!("Found {} files to index", files.len());

        // Step 2: Read and chunk files
        let mut all_chunks = Vec::new();
        let mut files_indexed = 0;
        let mut files_skipped = 0;

        for (idx, file_path) in files.iter().enumerate() {
            if idx % 100 == 0 && idx > 0 {
                tracing::info!("Progress: {}/{} files processed", idx, files.len());
            }

            match self.process_file(file_path) {
                Ok(chunks) => {
                    let chunk_count = chunks.len();
                    all_chunks.extend(chunks);
                    files_indexed += 1;

                    tracing::debug!("Indexed {:?} ({} chunks)", file_path, chunk_count);
                }
                Err(e) => {
                    tracing::warn!("Failed to process {:?}: {}", file_path, e);
                    files_skipped += 1;
                    // Continue processing other files
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            "Indexing complete: {} files indexed, {} skipped, \
             {} chunks created in {}ms",
            files_indexed,
            files_skipped,
            all_chunks.len(),
            duration_ms
        );

        let stats = IndexStats {
            files_indexed,
            chunks_created: all_chunks.len(),
            duration_ms,
            session: String::new(), // Filled by caller
        };

        Ok((all_chunks, stats))
    }

    /// Process a single file: read contents and chunk
    fn process_file(&self, path: &Path) -> Result<Vec<Chunk>> {
        // Read file contents
        let contents = fs::read_to_string(path).map_err(|e| {
            // Check if it's a UTF-8 error (likely binary file)
            if e.kind() == std::io::ErrorKind::InvalidData {
                ShebeError::IndexingFailed(format!("Skipping non-UTF-8 file: {path:?}"))
            } else {
                ShebeError::IndexingFailed(format!("Failed to read {path:?}: {e}"))
            }
        })?;

        // Skip empty files
        if contents.is_empty() {
            tracing::debug!("Skipping empty file: {:?}", path);
            return Ok(Vec::new());
        }

        // Chunk the text
        let chunks = self.chunker.chunk_text(&contents, path);

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir_with_files(files: &[(&str, &str)]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        for (path, content) in files {
            let full_path = temp_dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full_path, content).unwrap();
        }
        temp_dir
    }

    #[test]
    fn test_pipeline_simple_directory() {
        let temp_dir = create_test_dir_with_files(&[(
            "test.rs",
            "fn main() { println!(\"Hello, world!\"); }",
        )]);

        let pipeline = IndexingPipeline::new(
            20,                       // chunk_size
            5,                        // overlap
            vec!["*.rs".to_string()], // include
            vec![],                   // exclude
            10,                       // max_file_size_mb
        )
        .unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert!(!chunks.is_empty());
        assert_eq!(stats.chunks_created, chunks.len());
    }

    #[test]
    fn test_pipeline_multiple_files() {
        let temp_dir = create_test_dir_with_files(&[
            ("file1.rs", "fn test1() {}"),
            ("file2.rs", "fn test2() {}"),
            ("file3.txt", "ignored"),
        ]);

        let pipeline = IndexingPipeline::new(10, 2, vec!["*.rs".to_string()], vec![], 10).unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 2); // Only .rs files
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_pipeline_nested_directories() {
        let temp_dir = create_test_dir_with_files(&[
            ("src/main.rs", "fn main() {}"),
            ("src/lib.rs", "pub fn lib() {}"),
            ("tests/test.rs", "#[test] fn test() {}"),
        ]);

        let pipeline = IndexingPipeline::new(10, 2, vec!["*.rs".to_string()], vec![], 10).unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 3);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_pipeline_exclude_patterns() {
        let temp_dir = create_test_dir_with_files(&[
            ("src/main.rs", "fn main() {}"),
            ("target/debug/main.rs", "fn main() {}"),
            ("target/release/main.rs", "fn main() {}"),
        ]);

        let pipeline = IndexingPipeline::new(
            10,
            2,
            vec!["*.rs".to_string()],
            vec!["**/target/**".to_string()],
            10,
        )
        .unwrap();

        let (_chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        // Only src/main.rs should be indexed
        assert_eq!(stats.files_indexed, 1);
    }

    #[test]
    fn test_pipeline_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let pipeline = IndexingPipeline::new(10, 2, vec![], vec![], 10).unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 0);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_pipeline_empty_file() {
        let temp_dir = create_test_dir_with_files(&[("empty.rs", "")]);

        let pipeline = IndexingPipeline::new(10, 2, vec!["*.rs".to_string()], vec![], 10).unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        // Empty files are skipped but counted as indexed
        assert!(stats.files_indexed <= 1);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_pipeline_chunk_metadata() {
        let temp_dir =
            create_test_dir_with_files(&[("test.rs", "This is a test file with some content")]);

        let pipeline = IndexingPipeline::new(10, 2, vec!["*.rs".to_string()], vec![], 10).unwrap();

        let (chunks, _stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        // Verify chunk metadata
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
            assert!(chunk.file_path.ends_with("test.rs"));
            assert!(chunk.start_offset < chunk.end_offset);
        }
    }

    #[test]
    fn test_pipeline_utf8_content() {
        let temp_dir =
            create_test_dir_with_files(&[("unicode.rs", "// 中文注释\nfn test() {} // 🔥")]);

        let pipeline = IndexingPipeline::new(20, 5, vec!["*.rs".to_string()], vec![], 10).unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert!(!chunks.is_empty());
        // Verify UTF-8 content preserved
        let all_text: String = chunks.iter().map(|c| c.text.as_str()).collect();
        assert!(all_text.contains("中文"));
        assert!(all_text.contains("🔥"));
    }

    #[test]
    fn test_pipeline_large_file_handling() {
        // Create a file with repeated content
        let content = "x".repeat(1024 * 10); // 10KB
        let temp_dir = create_test_dir_with_files(&[("large.rs", &content)]);

        let pipeline = IndexingPipeline::new(
            512,
            64,
            vec!["*.rs".to_string()],
            vec![],
            10, // 10MB limit
        )
        .unwrap();

        let (chunks, stats) = pipeline.index_directory(temp_dir.path()).unwrap();

        assert_eq!(stats.files_indexed, 1);
        assert!(!chunks.is_empty());
    }
}
