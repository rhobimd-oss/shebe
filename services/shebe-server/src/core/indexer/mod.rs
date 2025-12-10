//! Document indexing module.
//!
//! Handles text chunking and file traversal for building search
//! indexes. Key features:
//!
//! - UTF-8 safe character-based chunking
//! - Configurable chunk size and overlap
//! - File system walking with pattern matching
//! - Indexing pipeline orchestration
//!
//! # Safety
//!
//! The chunker uses character-based slicing via `char_indices()`
//! to ensure UTF-8 safety. This prevents panics when processing
//! files containing emojis, multi-byte characters, or other
//! special Unicode sequences.

pub mod chunker;
pub mod pipeline;
pub mod walker;

pub use chunker::Chunker;
pub use pipeline::IndexingPipeline;
pub use walker::FileWalker;
