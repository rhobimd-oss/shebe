//! UTF-8 safe text chunking.
//!
//! Implements character-based chunking that never panics on
//! multi-byte UTF-8 sequences. Unlike byte-based chunking,
//! which can split UTF-8 characters and cause panics, this
//! implementation uses `char_indices()` to ensure all chunk
//! boundaries fall on valid character boundaries.
//!
//! # Example
//!
//! ```
//! use shebe::indexer::Chunker;
//! use std::path::Path;
//!
//! let chunker = Chunker::new(512, 64);
//! let text = "Hello 👋 World 🌍";
//! let chunks = chunker.chunk_text(text, Path::new("test.txt"));
//!
//! // All chunks are valid UTF-8, never panics
//! for chunk in chunks {
//!     assert!(std::str::from_utf8(chunk.text.as_bytes()).is_ok());
//! }
//! ```

use crate::core::types::Chunk;
use std::path::Path;

/// UTF-8 safe text chunker.
///
/// Splits text into fixed-size chunks with configurable overlap.
/// All sizes are measured in **characters**, not bytes, ensuring
/// UTF-8 safety.
#[derive(Debug, Clone)]
pub struct Chunker {
    /// Number of characters per chunk
    chunk_size: usize,

    /// Number of characters to overlap between consecutive chunks
    overlap: usize,
}

impl Chunker {
    /// Create a new chunker with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Number of characters per chunk (must be
    ///   > 0)
    /// * `overlap` - Number of characters to overlap between
    ///   chunks
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0 or if `overlap >= chunk_size`.
    ///
    /// # Example
    ///
    /// ```
    /// use shebe::indexer::Chunker;
    ///
    /// let chunker = Chunker::new(512, 64);
    /// assert_eq!(chunker.chunk_size(), 512);
    /// assert_eq!(chunker.overlap(), 64);
    /// ```
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        assert!(chunk_size > 0, "chunk_size must be > 0");
        assert!(overlap < chunk_size, "overlap must be < chunk_size");

        Self {
            chunk_size,
            overlap,
        }
    }

    /// Get the chunk size in characters.
    #[allow(dead_code)]
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Get the overlap size in characters.
    #[allow(dead_code)]
    pub fn overlap(&self) -> usize {
        self.overlap
    }

    /// Chunk text into overlapping segments.
    ///
    /// # Safety
    ///
    /// This function **always** works on character boundaries by
    /// using `char_indices()`. It will never panic on valid UTF-8
    /// input, regardless of the presence of emojis, multi-byte
    /// characters, or other special Unicode sequences.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to chunk (must be valid UTF-8)
    /// * `file_path` - The source file path for metadata
    ///
    /// # Returns
    ///
    /// A vector of chunks, each containing:
    /// - The text content
    /// - The source file path
    /// - Byte offsets (start_offset, end_offset)
    /// - Sequential chunk index
    ///
    /// # Example
    ///
    /// ```
    /// use shebe::indexer::Chunker;
    /// use std::path::Path;
    ///
    /// let chunker = Chunker::new(10, 2);
    /// let text = "Hello 👋 World 🌍 Rust 🦀";
    /// let chunks = chunker.chunk_text(text, Path::new("test.txt"));
    ///
    /// // All chunks are valid UTF-8
    /// for chunk in chunks {
    ///     assert!(!chunk.text.is_empty());
    ///     assert!(chunk.start_offset < chunk.end_offset);
    /// }
    /// ```
    pub fn chunk_text(&self, text: &str, file_path: &Path) -> Vec<Chunk> {
        // Collect character indices (byte offset, char)
        // This is the key to UTF-8 safety - we never work with
        // raw byte indices
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();

        if char_indices.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut char_start_idx = 0;

        while char_start_idx < char_indices.len() {
            // Calculate end index (character-based)
            let char_end_idx = (char_start_idx + self.chunk_size).min(char_indices.len());

            // Get byte offsets for this chunk
            // Since we're using character indices, these byte
            // offsets are guaranteed to fall on character
            // boundaries
            let byte_start = char_indices[char_start_idx].0;
            let byte_end = if char_end_idx < char_indices.len() {
                char_indices[char_end_idx].0
            } else {
                text.len() // End of string
            };

            // Extract chunk (guaranteed valid UTF-8 slice)
            let chunk_text = &text[byte_start..byte_end];

            chunks.push(Chunk {
                text: chunk_text.to_string(),
                file_path: file_path.to_path_buf(),
                start_offset: byte_start,
                end_offset: byte_end,
                chunk_index: chunks.len(),
            });

            // Move forward with overlap
            // Step = chunk_size - overlap, but always advance at
            // least 1 character to prevent infinite loops
            let step = self.chunk_size.saturating_sub(self.overlap);
            char_start_idx += step.max(1);
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_chunker_new() {
        let chunker = Chunker::new(512, 64);
        assert_eq!(chunker.chunk_size(), 512);
        assert_eq!(chunker.overlap(), 64);
    }

    #[test]
    #[should_panic(expected = "chunk_size must be > 0")]
    fn test_chunker_zero_size_panics() {
        Chunker::new(0, 0);
    }

    #[test]
    #[should_panic(expected = "overlap must be < chunk_size")]
    fn test_chunker_overlap_too_large_panics() {
        Chunker::new(10, 10);
    }

    #[test]
    fn test_chunk_empty_string() {
        let chunker = Chunker::new(10, 2);
        let chunks = chunker.chunk_text("", Path::new("test.txt"));
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_basic_text() {
        let chunker = Chunker::new(10, 2);
        let text = "0123456789ABCDEFGHIJ";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        assert_eq!(chunks.len(), 3);

        // First chunk: chars 0-9
        assert_eq!(chunks[0].text, "0123456789");
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].start_offset, 0);

        // Second chunk: chars 8-17 (overlap of 2)
        assert_eq!(chunks[1].text, "89ABCDEFGH");
        assert_eq!(chunks[1].chunk_index, 1);

        // Third chunk: chars 16-19 (remaining)
        assert_eq!(chunks[2].text, "GHIJ");
        assert_eq!(chunks[2].chunk_index, 2);
    }

    #[test]
    fn test_chunk_with_emoji() {
        let chunker = Chunker::new(10, 2);
        let text = "Hello 👋 World 🌍";

        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        // Should not panic
        assert!(!chunks.is_empty());

        // All chunks should be valid UTF-8
        for chunk in chunks {
            assert!(std::str::from_utf8(chunk.text.as_bytes()).is_ok());
        }
    }

    #[test]
    fn test_chunk_multibyte_characters() {
        let chunker = Chunker::new(10, 2);

        // Chinese characters (3 bytes each in UTF-8)
        let text = "中文测试字符串";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        // Should not panic
        assert!(!chunks.is_empty());

        // All chunks should be valid UTF-8
        for chunk in chunks {
            assert!(std::str::from_utf8(chunk.text.as_bytes()).is_ok());
            // Verify each chunk contains valid Chinese characters
            assert!(chunk.text.chars().all(|c| !c.is_ascii()));
        }
    }

    #[test]
    fn test_chunk_mixed_content() {
        let chunker = Chunker::new(20, 5);
        let text = "fn main() { // 🚀 Rust code with emoji";

        let chunks = chunker.chunk_text(text, Path::new("test.rs"));

        // Should not panic
        assert!(!chunks.is_empty());

        // All chunks should be valid UTF-8
        for chunk in chunks {
            assert!(std::str::from_utf8(chunk.text.as_bytes()).is_ok());
        }
    }

    #[test]
    fn test_offset_tracking() {
        let chunker = Chunker::new(5, 1);
        let text = "ABCDEFGHIJ";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        // Verify offsets are sequential and correct
        for chunk in &chunks {
            let extracted = &text[chunk.start_offset..chunk.end_offset];
            assert_eq!(extracted, chunk.text);
        }
    }

    #[test]
    fn test_chunk_index_sequential() {
        let chunker = Chunker::new(10, 2);
        let text = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        // Verify chunk indices are sequential starting from 0
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn test_overlap_correctness() {
        let chunker = Chunker::new(10, 3);
        let text = "0123456789ABCDEFGHIJ";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        // With overlap=3, chunks should share 3 characters
        assert!(chunks[1].text.starts_with("789"));
    }

    #[test]
    fn test_file_path_preserved() {
        let chunker = Chunker::new(10, 2);
        let text = "Hello, world!";
        let path = Path::new("/test/path/file.rs");
        let chunks = chunker.chunk_text(text, path);

        for chunk in chunks {
            assert_eq!(chunk.file_path, path);
        }
    }

    #[test]
    fn test_single_character() {
        let chunker = Chunker::new(10, 2);
        let text = "A";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "A");
        assert_eq!(chunks[0].start_offset, 0);
        assert_eq!(chunks[0].end_offset, 1);
    }

    #[test]
    fn test_exact_chunk_size() {
        let chunker = Chunker::new(10, 0);
        let text = "0123456789";
        let chunks = chunker.chunk_text(text, Path::new("test.txt"));

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
    }
}
