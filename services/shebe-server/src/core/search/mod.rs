//! Search module for BM25 full-text search.
//!
//! This module provides search functionality over indexed content
//! using Tantivy's BM25 ranking algorithm.

mod bm25;
mod query;

pub use bm25::SearchService;
pub use query::{preprocess_query, validate_query_fields};
