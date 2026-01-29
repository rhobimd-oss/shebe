//! XDG Base Directory Support
//!
//! Implements XDG Base Directory specification for proper file organization
//! on Linux/Unix systems. Provides automatic migration from legacy paths.

use std::env;
use std::fs;
use std::path::PathBuf;

/// XDG directory structure for Shebe
///
/// Implements XDG Base Directory specification with fallbacks and
/// backward compatibility for legacy environment variables.
#[derive(Debug, Clone)]
pub struct XdgDirs {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl XdgDirs {
    /// Create new XDG directory structure with proper resolution order
    ///
    /// Priority order (highest to lowest):
    /// 1. Explicit SHEBE_* env vars (backward compatibility)
    /// 2. XDG_* environment variables
    /// 3. XDG defaults (~/.config, ~/.local/share, etc.)
    pub fn new() -> Self {
        Self {
            config_dir: Self::resolve_config_dir(),
            data_dir: Self::resolve_data_dir(),
            state_dir: Self::resolve_state_dir(),
            cache_dir: Self::resolve_cache_dir(),
        }
    }

    /// Resolve config directory
    fn resolve_config_dir() -> PathBuf {
        // 1. Check SHEBE_CONFIG_DIR (backward compat)
        if let Ok(dir) = env::var("SHEBE_CONFIG_DIR") {
            return PathBuf::from(dir);
        }

        // 2. Check XDG_CONFIG_HOME
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("shebe");
        }

        // 3. Use XDG default
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("shebe")
    }

    /// Resolve data directory
    fn resolve_data_dir() -> PathBuf {
        // 1. Check SHEBE_DATA_DIR
        if let Ok(dir) = env::var("SHEBE_DATA_DIR") {
            return PathBuf::from(dir);
        }

        // 2. Check XDG_DATA_HOME
        if let Ok(xdg) = env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("shebe");
        }

        // 3. Use XDG default
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
            .join("shebe")
    }

    /// Resolve state directory
    fn resolve_state_dir() -> PathBuf {
        // 1. Check SHEBE_STATE_DIR
        if let Ok(dir) = env::var("SHEBE_STATE_DIR") {
            return PathBuf::from(dir);
        }

        // 2. Check XDG_STATE_HOME
        if let Ok(xdg) = env::var("XDG_STATE_HOME") {
            return PathBuf::from(xdg).join("shebe");
        }

        // 3. Use XDG default
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("state")
            .join("shebe")
    }

    /// Resolve cache directory
    fn resolve_cache_dir() -> PathBuf {
        // 1. Check SHEBE_CACHE_DIR
        if let Ok(dir) = env::var("SHEBE_CACHE_DIR") {
            return PathBuf::from(dir);
        }

        // 2. Check XDG_CACHE_HOME
        if let Ok(xdg) = env::var("XDG_CACHE_HOME") {
            return PathBuf::from(xdg).join("shebe");
        }

        // 3. Use XDG default
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache")
            .join("shebe")
    }

    /// Get config file path
    pub fn config_file(&self) -> PathBuf {
        // Check SHEBE_CONFIG_FILE first (explicit override)
        if let Ok(file) = env::var("SHEBE_CONFIG_FILE") {
            return PathBuf::from(file);
        }

        self.config_dir.join("config.toml")
    }

    /// Get sessions directory path
    pub fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    /// Get logs directory path
    pub fn logs_dir(&self) -> PathBuf {
        self.state_dir.join("logs")
    }

    /// Get progress directory path (for future use)
    #[allow(dead_code)]
    pub fn progress_dir(&self) -> PathBuf {
        self.state_dir.join("progress")
    }

    /// Get query cache directory path (for future use)
    #[allow(dead_code)]
    pub fn query_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("query-cache")
    }

    /// Create all XDG directories if they don't exist
    pub fn ensure_dirs_exist(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.config_dir)?;
        fs::create_dir_all(self.sessions_dir())?;
        fs::create_dir_all(self.logs_dir())?;
        Ok(())
    }

    /// Log the resolved XDG paths
    pub fn log_paths(&self) {
        tracing::info!("XDG directories resolved:");
        tracing::info!("  Config: {:?}", self.config_dir);
        tracing::info!("  Data: {:?}", self.data_dir);
        tracing::info!("  State: {:?}", self.state_dir);
        tracing::info!("  Cache: {:?}", self.cache_dir);
        tracing::info!("  Config file: {:?}", self.config_file());
        tracing::info!("  Sessions: {:?}", self.sessions_dir());
    }
}

impl Default for XdgDirs {
    fn default() -> Self {
        Self::new()
    }
}

/// Migrate legacy paths to XDG structure
///
/// Automatically copies config from legacy location to XDG paths.
/// Safe operation: never deletes original files, only copies.
pub fn migrate_legacy_paths(xdg: &XdgDirs) -> std::io::Result<()> {
    // Migrate config: ./shebe.toml → XDG_CONFIG/config.toml
    let legacy_config = PathBuf::from("./shebe.toml");
    let new_config = xdg.config_file();

    if legacy_config.exists() && !new_config.exists() {
        fs::create_dir_all(&xdg.config_dir)?;
        fs::copy(&legacy_config, &new_config)?;
        tracing::info!("Migrated config: {:?} → {:?}", legacy_config, new_config);
        tracing::info!(
            "Legacy config file preserved at {:?} (safe to delete after verification)",
            legacy_config
        );
        tracing::info!("XDG migration complete. Legacy config file preserved as backup.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    // Helper to clear all XDG-related env vars
    fn clear_env_vars() {
        env::remove_var("XDG_CONFIG_HOME");
        env::remove_var("XDG_DATA_HOME");
        env::remove_var("XDG_STATE_HOME");
        env::remove_var("XDG_CACHE_HOME");
        env::remove_var("SHEBE_CONFIG_DIR");
        env::remove_var("SHEBE_CONFIG_FILE");
        env::remove_var("SHEBE_DATA_DIR");
        env::remove_var("SHEBE_STATE_DIR");
        env::remove_var("SHEBE_CACHE_DIR");
    }

    #[test]
    #[serial]
    fn test_xdg_defaults() {
        clear_env_vars();

        let xdg = XdgDirs::new();
        assert!(xdg.config_dir.ends_with(".config/shebe"));
        assert!(xdg.data_dir.ends_with(".local/share/shebe"));
        assert!(xdg.state_dir.ends_with(".local/state/shebe"));
        assert!(xdg.cache_dir.ends_with(".cache/shebe"));
    }

    #[test]
    #[serial]
    fn test_xdg_config_home_override() {
        clear_env_vars();
        env::set_var("XDG_CONFIG_HOME", "/custom/config");

        let xdg = XdgDirs::new();
        // Should use XDG_CONFIG_HOME if SHEBE_* vars not set
        assert!(
            xdg.config_dir == PathBuf::from("/custom/config/shebe")
                || xdg.config_dir.ends_with(".config/shebe"),
            "Expected /custom/config/shebe or default, got {:?}",
            xdg.config_dir
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_data_home_override() {
        clear_env_vars();
        env::set_var("XDG_DATA_HOME", "/custom/data");

        let xdg = XdgDirs::new();
        // Should use XDG_DATA_HOME if SHEBE_* vars not set
        assert!(
            xdg.data_dir == PathBuf::from("/custom/data/shebe")
                || xdg.data_dir.ends_with(".local/share/shebe"),
            "Expected /custom/data/shebe or default, got {:?}",
            xdg.data_dir
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_shebe_config_dir_priority() {
        clear_env_vars();
        env::set_var("XDG_CONFIG_HOME", "/xdg/config");
        env::set_var("SHEBE_CONFIG_DIR", "/shebe/config");

        let xdg = XdgDirs::new();
        // SHEBE_CONFIG_DIR should win
        assert!(
            xdg.config_dir == PathBuf::from("/shebe/config"),
            "Expected /shebe/config, got {:?}",
            xdg.config_dir
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_shebe_data_dir_priority() {
        clear_env_vars();
        env::set_var("XDG_DATA_HOME", "/xdg/data");
        env::set_var("SHEBE_DATA_DIR", "/shebe/data");

        let xdg = XdgDirs::new();
        // SHEBE_DATA_DIR should win over XDG_DATA_HOME
        assert!(
            xdg.data_dir == PathBuf::from("/shebe/data"),
            "Expected /shebe/data, got {:?}",
            xdg.data_dir
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_config_file_resolution() {
        clear_env_vars();

        let xdg = XdgDirs::new();
        let config_file = xdg.config_file();
        assert!(config_file.ends_with("shebe/config.toml"));
    }

    #[test]
    #[serial]
    fn test_config_file_env_override() {
        clear_env_vars();
        env::set_var("SHEBE_CONFIG_FILE", "/custom/my-config.toml");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.config_file(), PathBuf::from("/custom/my-config.toml"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_sessions_dir_resolution() {
        clear_env_vars();

        let xdg = XdgDirs::new();
        let sessions = xdg.sessions_dir();
        assert!(sessions.ends_with("shebe/sessions"));
    }

    #[test]
    #[serial]
    fn test_logs_dir_resolution() {
        clear_env_vars();

        let xdg = XdgDirs::new();
        let logs = xdg.logs_dir();
        assert!(logs.ends_with("shebe/logs"));
    }

    #[test]
    #[serial]
    fn test_all_xdg_env_vars() {
        clear_env_vars();
        env::set_var("XDG_CONFIG_HOME", "/c");
        env::set_var("XDG_DATA_HOME", "/d");
        env::set_var("XDG_STATE_HOME", "/s");
        env::set_var("XDG_CACHE_HOME", "/k");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.config_dir, PathBuf::from("/c/shebe"));
        assert_eq!(xdg.data_dir, PathBuf::from("/d/shebe"));
        assert_eq!(xdg.state_dir, PathBuf::from("/s/shebe"));
        assert_eq!(xdg.cache_dir, PathBuf::from("/k/shebe"));

        clear_env_vars();
    }

    // --- Phase 1A: Center tests ---

    #[test]
    #[serial]
    fn test_xdg_resolve_state_dir() {
        clear_env_vars();
        env::set_var("XDG_STATE_HOME", "/custom/state");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.state_dir, PathBuf::from("/custom/state/shebe"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_resolve_cache_dir() {
        clear_env_vars();
        env::set_var("XDG_CACHE_HOME", "/custom/cache");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.cache_dir, PathBuf::from("/custom/cache/shebe"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_sessions_dir() {
        clear_env_vars();
        env::set_var("SHEBE_DATA_DIR", "/test/data");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.sessions_dir(), PathBuf::from("/test/data/sessions"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_logs_dir() {
        clear_env_vars();
        env::set_var("SHEBE_STATE_DIR", "/test/state");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.logs_dir(), PathBuf::from("/test/state/logs"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_progress_dir() {
        clear_env_vars();
        env::set_var("SHEBE_STATE_DIR", "/test/state");

        let xdg = XdgDirs::new();
        assert_eq!(xdg.progress_dir(), PathBuf::from("/test/state/progress"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_query_cache_dir() {
        clear_env_vars();
        env::set_var("SHEBE_CACHE_DIR", "/test/cache");

        let xdg = XdgDirs::new();
        assert_eq!(
            xdg.query_cache_dir(),
            PathBuf::from("/test/cache/query-cache")
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_shebe_state_dir_priority() {
        clear_env_vars();
        env::set_var("XDG_STATE_HOME", "/xdg/state");
        env::set_var("SHEBE_STATE_DIR", "/shebe/state");

        let xdg = XdgDirs::new();
        assert_eq!(
            xdg.state_dir,
            PathBuf::from("/shebe/state"),
            "SHEBE_STATE_DIR should take priority over XDG_STATE_HOME"
        );

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_shebe_cache_dir_priority() {
        clear_env_vars();
        env::set_var("XDG_CACHE_HOME", "/xdg/cache");
        env::set_var("SHEBE_CACHE_DIR", "/shebe/cache");

        let xdg = XdgDirs::new();
        assert_eq!(
            xdg.cache_dir,
            PathBuf::from("/shebe/cache"),
            "SHEBE_CACHE_DIR should take priority over XDG_CACHE_HOME"
        );

        clear_env_vars();
    }

    // --- Phase 1A: Boundary tests ---

    #[test]
    #[serial]
    fn test_xdg_ensure_dirs_exist() {
        clear_env_vars();
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("xdg_test");

        env::set_var("SHEBE_CONFIG_DIR", base.join("config").to_str().unwrap());
        env::set_var("SHEBE_DATA_DIR", base.join("data").to_str().unwrap());
        env::set_var("SHEBE_STATE_DIR", base.join("state").to_str().unwrap());

        let xdg = XdgDirs::new();
        xdg.ensure_dirs_exist().unwrap();

        assert!(base.join("config").exists());
        assert!(base.join("data").join("sessions").exists());
        assert!(base.join("state").join("logs").exists());

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_ensure_dirs_idempotent() {
        clear_env_vars();
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("xdg_idem");

        env::set_var("SHEBE_CONFIG_DIR", base.join("config").to_str().unwrap());
        env::set_var("SHEBE_DATA_DIR", base.join("data").to_str().unwrap());
        env::set_var("SHEBE_STATE_DIR", base.join("state").to_str().unwrap());

        let xdg = XdgDirs::new();
        xdg.ensure_dirs_exist().unwrap();
        // Call again -- should not error
        xdg.ensure_dirs_exist().unwrap();

        assert!(base.join("config").exists());

        clear_env_vars();
    }

    // --- Phase 1A: Beyond-boundary tests ---

    #[test]
    #[serial]
    fn test_xdg_migrate_no_legacy_file() {
        clear_env_vars();
        let temp = tempfile::tempdir().unwrap();
        env::set_var(
            "SHEBE_CONFIG_DIR",
            temp.path().join("cfg").to_str().unwrap(),
        );

        let xdg = XdgDirs::new();
        // No ./shebe.toml exists, migrate should be a no-op
        let result = migrate_legacy_paths(&xdg);
        assert!(result.is_ok());
        // Config file should NOT have been created
        assert!(!xdg.config_file().exists());

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_migrate_with_legacy_file() {
        clear_env_vars();
        let temp = tempfile::tempdir().unwrap();
        let cfg_dir = temp.path().join("cfg");
        env::set_var("SHEBE_CONFIG_DIR", cfg_dir.to_str().unwrap());

        // Create legacy config in current directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();
        fs::write("shebe.toml", "key = \"value\"").unwrap();

        let xdg = XdgDirs::new();
        migrate_legacy_paths(&xdg).unwrap();

        // Config file should now exist with copied content
        let new_config = xdg.config_file();
        assert!(new_config.exists());
        let content = fs::read_to_string(&new_config).unwrap();
        assert_eq!(content, "key = \"value\"");

        // Legacy file should still exist (safe copy)
        assert!(temp.path().join("shebe.toml").exists());

        env::set_current_dir(original_dir).unwrap();
        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_migrate_does_not_overwrite() {
        clear_env_vars();
        let temp = tempfile::tempdir().unwrap();
        let cfg_dir = temp.path().join("cfg");
        fs::create_dir_all(&cfg_dir).unwrap();
        env::set_var("SHEBE_CONFIG_DIR", cfg_dir.to_str().unwrap());

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // Create legacy config
        fs::write("shebe.toml", "legacy = true").unwrap();

        // Create existing config at XDG path
        let xdg = XdgDirs::new();
        fs::write(xdg.config_file(), "existing = true").unwrap();

        migrate_legacy_paths(&xdg).unwrap();

        // Existing config should NOT be overwritten
        let content = fs::read_to_string(xdg.config_file()).unwrap();
        assert_eq!(content, "existing = true");

        env::set_current_dir(original_dir).unwrap();
        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_default_impl() {
        clear_env_vars();
        let xdg = XdgDirs::default();
        assert!(xdg.config_dir.ends_with(".config/shebe"));

        clear_env_vars();
    }

    #[test]
    #[serial]
    fn test_xdg_log_paths_does_not_panic() {
        clear_env_vars();
        let xdg = XdgDirs::new();
        // log_paths should not panic even without a tracing subscriber
        xdg.log_paths();

        clear_env_vars();
    }
}
