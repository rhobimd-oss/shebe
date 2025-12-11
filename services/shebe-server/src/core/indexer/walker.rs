//! File system walker with pattern-based filtering.
//!
//! Traverses directory trees and filters files using glob patterns.
//! Handles errors gracefully (permission denied, etc.) without
//! crashing.

use glob::Pattern;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::core::error::{Result, ShebeError};

/// File system walker with pattern-based filtering
pub struct FileWalker {
    /// Patterns to include (e.g., "*.rs", "*.md")
    include_patterns: Vec<Pattern>,

    /// Patterns to exclude (e.g., "**/target/**", "**/.git/**")
    exclude_patterns: Vec<Pattern>,

    /// Maximum file size in bytes (skip larger files)
    max_file_size_bytes: u64,
}

impl FileWalker {
    /// Create a new file walker
    ///
    /// # Arguments
    ///
    /// * `include_patterns` - Glob patterns for files to include
    /// * `exclude_patterns` - Glob patterns for files to exclude
    /// * `max_file_size_mb` - Maximum file size in megabytes
    ///
    /// # Returns
    ///
    /// A new `FileWalker` instance or an error if patterns are
    /// invalid
    pub fn new(
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        max_file_size_mb: usize,
    ) -> Result<Self> {
        // Parse include patterns
        let include = include_patterns
            .into_iter()
            .map(|p| {
                Pattern::new(&p).map_err(|e| {
                    ShebeError::ConfigError(format!("Invalid include pattern '{p}': {e}"))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Parse exclude patterns
        let exclude = exclude_patterns
            .into_iter()
            .map(|p| {
                Pattern::new(&p).map_err(|e| {
                    ShebeError::ConfigError(format!("Invalid exclude pattern '{p}': {e}"))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            include_patterns: include,
            exclude_patterns: exclude,
            max_file_size_bytes: (max_file_size_mb as u64) * 1024 * 1024,
        })
    }

    /// Collect all matching files from a directory
    ///
    /// Traverses the directory tree, applies include/exclude
    /// patterns and filters by file size.
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory to start traversal
    ///
    /// # Returns
    ///
    /// A vector of file paths that match the criteria
    pub fn collect_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| self.should_process_entry(e, root))
        {
            match entry {
                Ok(entry) => {
                    if !entry.file_type().is_file() {
                        continue;
                    }

                    let path = entry.path();

                    // Check file size
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.len() > self.max_file_size_bytes {
                            tracing::debug!(
                                "Skipping large file: {:?} \
                                 ({} bytes)",
                                path,
                                metadata.len()
                            );
                            continue;
                        }
                    }

                    // Check patterns
                    if self.matches_patterns(path) {
                        files.push(path.to_path_buf());
                    }
                }
                Err(e) => {
                    tracing::warn!("Walk error: {}", e);
                    // Continue walking despite errors
                }
            }
        }

        Ok(files)
    }

    /// Determine if a directory entry should be processed
    ///
    /// Filters out hidden directories and excluded patterns.
    /// Never filters the root directory itself.
    fn should_process_entry(&self, entry: &DirEntry, root: &Path) -> bool {
        let path = entry.path();

        // Never filter the root directory
        if path == root {
            return true;
        }

        // Skip hidden directories (starting with '.')
        // but only if they're not the root
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && entry.file_type().is_dir() {
                return false;
            }
        }

        // Check exclude patterns for directories
        // (skip entire directory trees early)
        if entry.file_type().is_dir() {
            for pattern in &self.exclude_patterns {
                if pattern.matches_path(path) {
                    tracing::debug!("Skipping excluded directory: {:?}", path);
                    return false;
                }
            }
        }

        true
    }

    /// Check if a file path matches the include/exclude patterns
    fn matches_patterns(&self, path: &Path) -> bool {
        // Convert path to string for matching
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return false,
        };

        // If no include patterns, include all
        let matches_include = self.include_patterns.is_empty()
            || self.include_patterns.iter().any(|p| {
                // Match against both full path and filename
                p.matches(path_str)
                    || path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map(|f| p.matches(f))
                        .unwrap_or(false)
            });

        if !matches_include {
            return false;
        }

        // Must not match any exclude pattern
        !self
            .exclude_patterns
            .iter()
            .any(|p| p.matches(path_str) || p.matches_path(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files(files: &[&str]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        for file in files {
            let path = temp_dir.path().join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "test content").unwrap();
        }
        temp_dir
    }

    #[test]
    fn test_walker_no_patterns() {
        let temp_dir = create_test_files(&["file1.rs", "file2.md", "file3.txt"]);

        let walker = FileWalker::new(vec![], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_walker_include_patterns() {
        let temp_dir = create_test_files(&["file1.rs", "file2.md", "file3.txt"]);

        let walker = FileWalker::new(vec!["*.rs".to_string()], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("file1.rs"));
    }

    #[test]
    fn test_walker_exclude_patterns() {
        let temp_dir = create_test_files(&["file1.rs", "file2.md", "target/debug/file.rs"]);

        let walker = FileWalker::new(
            vec!["*.rs".to_string()],
            vec!["**/target/**".to_string()],
            10,
        )
        .unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("file1.rs"));
    }

    #[test]
    fn test_walker_multiple_include_patterns() {
        let temp_dir = create_test_files(&["file1.rs", "file2.md", "file3.txt"]);

        let walker =
            FileWalker::new(vec!["*.rs".to_string(), "*.md".to_string()], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_walker_hidden_directories() {
        let temp_dir = create_test_files(&["visible.rs", ".git/config", ".cache/data.txt"]);

        let walker = FileWalker::new(vec![], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        // Should skip .git and .cache directories
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("visible.rs"));
    }

    #[test]
    fn test_walker_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let walker = FileWalker::new(vec![], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_walker_invalid_pattern() {
        let result = FileWalker::new(vec!["[invalid".to_string()], vec![], 10);

        assert!(result.is_err());
    }

    #[test]
    fn test_walker_nested_directories() {
        let temp_dir =
            create_test_files(&["src/main.rs", "src/lib.rs", "tests/test.rs", "README.md"]);

        let walker = FileWalker::new(vec!["*.rs".to_string()], vec![], 10).unwrap();
        let files = walker.collect_files(temp_dir.path()).unwrap();

        assert_eq!(files.len(), 3);
    }
}
