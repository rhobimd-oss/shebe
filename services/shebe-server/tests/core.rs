//! Core module integration tests
//!
//! Tests for protocol-agnostic functionality including:
//! - Storage: Session management and persistence
//! - Search: BM25 search functionality
//! - Indexer: UTF-8 safe chunking and file processing

mod common;

// Core submodules - tests/core/ directory
mod core {
    pub mod indexer;
    pub mod search;
    pub mod storage;
}
