//! Configuration management for the Shebe RAG service.
//!
//! This module handles loading configuration from TOML files and
//! environment variables, with sensible defaults for all settings.

use crate::error::{Result, ShebeError};
use crate::xdg::XdgDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub indexing: IndexingConfig,
    pub storage: StorageConfig,
    pub search: SearchConfig,
    pub limits: LimitsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Server bind address
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Logging level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Indexing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexingConfig {
    /// Characters per chunk (not bytes!)
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    /// Character overlap between consecutive chunks
    #[serde(default = "default_overlap")]
    pub overlap: usize,

    /// Maximum file size in MB (skip larger files)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: usize,

    /// File patterns to include (glob syntax)
    #[serde(default = "default_include_patterns")]
    pub include_patterns: Vec<String>,

    /// File patterns to exclude (glob syntax)
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Root directory for index storage
    #[serde(default = "default_index_dir")]
    pub index_dir: PathBuf,
}

/// Search configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    /// Default number of results to return
    #[serde(default = "default_k")]
    pub default_k: usize,

    /// Maximum results per query
    #[serde(default = "default_max_k")]
    pub max_k: usize,

    /// Maximum query string length
    #[serde(default = "default_max_query_length")]
    pub max_query_length: usize,
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_chunk_size() -> usize {
    512
}

fn default_overlap() -> usize {
    64
}

fn default_max_file_size() -> usize {
    10
}

fn default_index_dir() -> PathBuf {
    PathBuf::from("./data")
}

fn default_k() -> usize {
    10
}

fn default_max_k() -> usize {
    100
}

fn default_max_query_length() -> usize {
    500
}

fn default_include_patterns() -> Vec<String> {
    vec![
        "*.rs".to_string(),
        "*.toml".to_string(),
        "*.md".to_string(),
        "*.txt".to_string(),
        "*.php".to_string(),
        "*.js".to_string(),
        "*.ts".to_string(),
        "*.py".to_string(),
        "*.go".to_string(),
        "*.java".to_string(),
        "*.c".to_string(),
        "*.cpp".to_string(),
        "*.h".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        // Build artifacts and dependencies
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/vendor/**".to_string(),
        "**/.git/**".to_string(),
        "**/build/**".to_string(),
        "**/__pycache__/**".to_string(),
        "**/dist/**".to_string(),
        "**/.next/**".to_string(),
        // Image files
        "**/*.jpg".to_string(),
        "**/*.jpeg".to_string(),
        "**/*.png".to_string(),
        "**/*.gif".to_string(),
        "**/*.bmp".to_string(),
        "**/*.svg".to_string(),
        "**/*.webp".to_string(),
        "**/*.ico".to_string(),
        "**/*.tiff".to_string(),
        "**/*.tif".to_string(),
        // Audio files
        "**/*.mp3".to_string(),
        "**/*.wav".to_string(),
        "**/*.flac".to_string(),
        "**/*.ogg".to_string(),
        "**/*.m4a".to_string(),
        "**/*.aac".to_string(),
        "**/*.wma".to_string(),
        // Video files
        "**/*.mp4".to_string(),
        "**/*.avi".to_string(),
        "**/*.mov".to_string(),
        "**/*.mkv".to_string(),
        "**/*.webm".to_string(),
        "**/*.flv".to_string(),
        "**/*.wmv".to_string(),
        // Document formats (binary/structured)
        "**/*.pdf".to_string(),
        "**/*.doc".to_string(),
        "**/*.docx".to_string(),
        "**/*.xls".to_string(),
        "**/*.xlsx".to_string(),
        "**/*.ppt".to_string(),
        "**/*.pptx".to_string(),
        "**/*.odt".to_string(),
        "**/*.ods".to_string(),
        "**/*.odp".to_string(),
        // Archive files
        "**/*.zip".to_string(),
        "**/*.tar".to_string(),
        "**/*.gz".to_string(),
        "**/*.bz2".to_string(),
        "**/*.7z".to_string(),
        "**/*.rar".to_string(),
        "**/*.tgz".to_string(),
        // Executables and binaries
        "**/*.exe".to_string(),
        "**/*.dll".to_string(),
        "**/*.so".to_string(),
        "**/*.dylib".to_string(),
        "**/*.bin".to_string(),
        "**/*.o".to_string(),
        "**/*.a".to_string(),
        // Font files
        "**/*.ttf".to_string(),
        "**/*.otf".to_string(),
        "**/*.woff".to_string(),
        "**/*.woff2".to_string(),
        "**/*.eot".to_string(),
    ]
}

fn default_max_concurrent_indexes() -> usize {
    1
}

fn default_request_timeout() -> u64 {
    300
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
        }
    }
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            chunk_size: default_chunk_size(),
            overlap: default_overlap(),
            max_file_size_mb: default_max_file_size(),
            include_patterns: default_include_patterns(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            index_dir: default_index_dir(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_k: default_k(),
            max_k: default_max_k(),
            max_query_length: default_max_query_length(),
        }
    }
}

/// Limits configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitsConfig {
    /// Maximum concurrent indexing operations
    #[serde(default = "default_max_concurrent_indexes")]
    pub max_concurrent_indexes: usize,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_sec: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_concurrent_indexes: default_max_concurrent_indexes(),
            request_timeout_sec: default_request_timeout(),
        }
    }
}

impl Config {
    /// Load configuration from TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ShebeError::ConfigError(format!("Failed to read config file: {e}")))?;

        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Create default configuration
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load config with priority: env vars > TOML > defaults
    ///
    /// This method uses XDG Base Directory specification for file locations.
    pub fn load() -> Result<Self> {
        let xdg = XdgDirs::new();
        Self::load_with_xdg(&xdg)
    }

    /// Load config with explicit XDG directories
    ///
    /// Priority order:
    /// 1. SHEBE_CONFIG or SHEBE_CONFIG_FILE env var
    /// 2. XDG config file (~/.config/shebe/config.toml)
    /// 3. Legacy ./shebe.toml (for backward compatibility)
    /// 4. Defaults
    pub fn load_with_xdg(xdg: &XdgDirs) -> Result<Self> {
        // Start with defaults
        let mut config = if let Ok(config_path) = env::var("SHEBE_CONFIG") {
            // Load from file if SHEBE_CONFIG is set (legacy)
            Self::from_file(config_path)?
        } else {
            // Try XDG config file
            let xdg_config = xdg.config_file();
            if xdg_config.exists() {
                Self::from_file(xdg_config)?
            } else if Path::new("shebe.toml").exists() {
                // Fall back to legacy location for backward compatibility
                Self::from_file("shebe.toml")?
            } else {
                // Use defaults
                Self::default()
            }
        };

        // Override storage path with XDG data directory if not explicitly set
        if env::var("SHEBE_DATA_DIR").is_err() && config.storage.index_dir == default_index_dir() {
            config.storage.index_dir = xdg.sessions_dir();
        }

        // Override with environment variables
        config.merge_env();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Merge configuration with environment variables
    pub fn merge_env(&mut self) {
        // Server configuration
        if let Ok(host) = env::var("SHEBE_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = env::var("SHEBE_PORT") {
            if let Ok(p) = port.parse() {
                self.server.port = p;
            }
        }
        if let Ok(log_level) = env::var("SHEBE_LOG_LEVEL") {
            self.server.log_level = log_level;
        }

        // Indexing configuration
        if let Ok(chunk_size) = env::var("SHEBE_CHUNK_SIZE") {
            if let Ok(size) = chunk_size.parse() {
                self.indexing.chunk_size = size;
            }
        }
        if let Ok(overlap) = env::var("SHEBE_OVERLAP") {
            if let Ok(o) = overlap.parse() {
                self.indexing.overlap = o;
            }
        }
        if let Ok(max_size) = env::var("SHEBE_MAX_FILE_SIZE_MB") {
            if let Ok(size) = max_size.parse() {
                self.indexing.max_file_size_mb = size;
            }
        }

        // Storage configuration
        if let Ok(data_dir) = env::var("SHEBE_DATA_DIR") {
            self.storage.index_dir = PathBuf::from(data_dir).join("sessions");
        }

        // Search configuration
        if let Ok(default_k) = env::var("SHEBE_DEFAULT_K") {
            if let Ok(k) = default_k.parse() {
                self.search.default_k = k;
            }
        }
        if let Ok(max_k) = env::var("SHEBE_MAX_K") {
            if let Ok(k) = max_k.parse() {
                self.search.max_k = k;
            }
        }
        if let Ok(max_query_len) = env::var("SHEBE_MAX_QUERY_LENGTH") {
            if let Ok(len) = max_query_len.parse() {
                self.search.max_query_length = len;
            }
        }

        // Limits configuration
        if let Ok(max_concurrent) = env::var("SHEBE_MAX_CONCURRENT_INDEXES") {
            if let Ok(max) = max_concurrent.parse() {
                self.limits.max_concurrent_indexes = max;
            }
        }
        if let Ok(timeout) = env::var("SHEBE_REQUEST_TIMEOUT_SEC") {
            if let Ok(t) = timeout.parse() {
                self.limits.request_timeout_sec = t;
            }
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(ShebeError::ConfigError("Port must be non-zero".to_string()));
        }

        // Validate indexing config
        if self.indexing.chunk_size == 0 {
            return Err(ShebeError::ConfigError(
                "Chunk size must be non-zero".to_string(),
            ));
        }

        if self.indexing.overlap >= self.indexing.chunk_size {
            return Err(ShebeError::ConfigError(
                "Overlap must be less than chunk size".to_string(),
            ));
        }

        // Validate search config
        if self.search.default_k == 0 {
            return Err(ShebeError::ConfigError(
                "Default k must be non-zero".to_string(),
            ));
        }

        if self.search.default_k > self.search.max_k {
            return Err(ShebeError::ConfigError(
                "Default k cannot exceed max k".to_string(),
            ));
        }

        if self.search.max_query_length == 0 {
            return Err(ShebeError::ConfigError(
                "Max query length must be non-zero".to_string(),
            ));
        }

        // Validate limits config
        if self.limits.max_concurrent_indexes == 0 {
            return Err(ShebeError::ConfigError(
                "Max concurrent indexes must be non-zero".to_string(),
            ));
        }

        if self.limits.request_timeout_sec == 0 {
            return Err(ShebeError::ConfigError(
                "Request timeout must be non-zero".to_string(),
            ));
        }

        Ok(())
    }

    /// Log configuration (redacting sensitive values)
    pub fn log_config(&self) {
        tracing::info!("Configuration loaded:");
        tracing::info!("  Server: {}:{}", self.server.host, self.server.port);
        tracing::info!("  Log level: {}", self.server.log_level);
        tracing::info!("  Chunk size: {} chars", self.indexing.chunk_size);
        tracing::info!("  Overlap: {} chars", self.indexing.overlap);
        tracing::info!("  Max file size: {} MB", self.indexing.max_file_size_mb);
        tracing::info!(
            "  Include patterns: {} patterns",
            self.indexing.include_patterns.len()
        );
        tracing::info!(
            "  Exclude patterns: {} patterns",
            self.indexing.exclude_patterns.len()
        );
        tracing::info!("  Index dir: {:?}", self.storage.index_dir);
        tracing::info!("  Default k: {}", self.search.default_k);
        tracing::info!("  Max k: {}", self.search.max_k);
        tracing::info!("  Max query length: {}", self.search.max_query_length);
        tracing::info!(
            "  Max concurrent indexes: {}",
            self.limits.max_concurrent_indexes
        );
        tracing::info!("  Request timeout: {}s", self.limits.request_timeout_sec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.indexing.chunk_size, 512);
        assert_eq!(config.indexing.overlap, 64);
        assert_eq!(config.search.default_k, 10);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_overlap() {
        let mut config = Config::default();
        config.indexing.overlap = 600; // Greater than chunk_size
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_chunk_size() {
        let mut config = Config::default();
        config.indexing.chunk_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_var_override() {
        env::set_var("SHEBE_PORT", "8080");
        env::set_var("SHEBE_CHUNK_SIZE", "1024");

        let mut config = Config::default();
        config.merge_env();

        assert_eq!(config.server.port, 8080);
        assert_eq!(config.indexing.chunk_size, 1024);

        // Cleanup
        env::remove_var("SHEBE_PORT");
        env::remove_var("SHEBE_CHUNK_SIZE");
    }

    #[test]
    fn test_toml_deserialization() {
        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 8000
            log_level = "debug"

            [indexing]
            chunk_size = 256
            overlap = 32
            max_file_size_mb = 20

            [storage]
            index_dir = "/data/shebe"

            [search]
            default_k = 20
            max_k = 200
            max_query_length = 1000

            [limits]
            max_concurrent_indexes = 2
            request_timeout_sec = 600
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.indexing.chunk_size, 256);
        assert_eq!(config.search.default_k, 20);
        assert_eq!(config.search.max_query_length, 1000);
        assert_eq!(config.limits.max_concurrent_indexes, 2);
        assert_eq!(config.limits.request_timeout_sec, 600);
    }

    #[test]
    fn test_include_exclude_patterns() {
        let config = Config::default();
        assert!(!config.indexing.include_patterns.is_empty());
        assert!(!config.indexing.exclude_patterns.is_empty());
        assert!(config
            .indexing
            .include_patterns
            .contains(&"*.rs".to_string()));
        assert!(config
            .indexing
            .exclude_patterns
            .contains(&"**/target/**".to_string()));
    }

    #[test]
    fn test_limits_config() {
        let config = Config::default();
        assert_eq!(config.limits.max_concurrent_indexes, 1);
        assert_eq!(config.limits.request_timeout_sec, 300);
    }

    #[test]
    fn test_search_max_query_length() {
        let config = Config::default();
        assert_eq!(config.search.max_query_length, 500);
    }

    #[test]
    fn test_limits_validation() {
        let mut config = Config::default();
        config.limits.max_concurrent_indexes = 0;
        assert!(config.validate().is_err());

        config = Config::default();
        config.limits.request_timeout_sec = 0;
        assert!(config.validate().is_err());
    }
}
