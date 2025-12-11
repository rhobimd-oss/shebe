// Test fixtures for integration testing

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// OpenEMR repository path (for real-world integration testing)
#[allow(dead_code)] // Used in integration tests
pub const OPENEMR_PATH: &str = "~/github/openemr/openemr";

/// Test repository fixture for creating synthetic test data
pub struct TestRepo {
    pub dir: TempDir,
    #[allow(dead_code)] // Reserved for future use
    pub files: Vec<PathBuf>,
}

impl TestRepo {
    /// Create a small test repository (10 files)
    #[allow(dead_code)] // Used in integration tests
    pub fn small() -> Self {
        Self::with_files(&[
            ("src/main.rs", "fn main() { println!(\"Hello\"); }"),
            ("src/lib.rs", "pub fn helper() -> u32 { 42 }"),
            (
                "src/utils.rs",
                "pub fn add(a: i32, b: i32) -> i32 { a + b }",
            ),
            ("README.md", "# Test Project\n\nThis is a test."),
            (
                "Cargo.toml",
                "[package]\nname = \"test\"\nversion = \"0.1.0\"",
            ),
            (
                "src/auth.rs",
                "pub fn authenticate(user: &str) -> bool { true }",
            ),
            (
                "src/db.rs",
                "pub fn connect() -> Result<(), String> { Ok(()) }",
            ),
            (
                "tests/test_main.rs",
                "#[test]\nfn it_works() { assert_eq!(2 + 2, 4); }",
            ),
            ("docs/api.md", "# API\n\n## Functions\n\n- `helper()`\n"),
            ("LICENSE", "MIT License\n\nCopyright (c) 2025"),
        ])
    }

    /// Create a medium test repository (50 files)
    #[allow(dead_code)] // Used in integration tests
    pub fn medium() -> Self {
        let mut files = Vec::new();

        // Add 50 Rust files with various content
        for i in 0..50 {
            let filename = format!("src/module_{}.rs", i);
            let content = format!(
                "// Module {}\npub fn func_{}() -> i32 {{\n    {}\n}}\n",
                i, i, i
            );
            files.push((filename, content));
        }

        Self::with_file_specs(
            files
                .iter()
                .map(|(f, c)| (f.as_str(), c.as_str()))
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }

    /// Create with custom files
    pub fn with_files(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().unwrap();
        let mut paths = Vec::new();

        for (path, content) in files {
            let full_path = dir.path().join(path);
            std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            std::fs::write(&full_path, content).unwrap();
            paths.push(full_path);
        }

        Self { dir, files: paths }
    }

    /// Create with dynamically generated file specs
    #[allow(dead_code)] // Used internally by medium()
    fn with_file_specs(files: &[(&str, &str)]) -> Self {
        Self::with_files(files)
    }

    /// Get path to the repository
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// UTF-8 test data for safety validation
#[allow(dead_code)] // Reserved for future UTF-8 tests
pub struct Utf8TestData {
    pub emoji: Vec<&'static str>,
    pub multibyte: Vec<&'static str>,
    pub mixed: Vec<&'static str>,
    pub edge_cases: Vec<&'static str>,
}

impl Utf8TestData {
    #[allow(dead_code)] // Reserved for future UTF-8 tests
    pub fn new() -> Self {
        Self {
            emoji: vec![
                "Hello 👋 World",
                "Rust 🦀 is awesome",
                "Testing 🧪 code",
                "🚀 Launch time",
                "Done ✅",
                "Error ❌",
                "Warning ⚠️",
                "Celebrate 🎉🎊🥳",
            ],
            multibyte: vec![
                "中文测试",        // Chinese
                "مرحبا بالعالم",   // Arabic
                "שלום עולם",       // Hebrew
                "Привет мир",      // Russian
                "こんにちは世界",  // Japanese
                "안녕하세요 세계", // Korean
                "Γειά σου κόσμε",  // Greek
                "สวัสดีโลก",         // Thai
            ],
            mixed: vec![
                "fn main() { // 🚀 Entry point",
                "// TODO: Fix 🐛 in auth module",
                "let greeting = \"Hello 👋\";",
                "// 中文注释 in Rust code",
                "error!(\"{} failed\", \"مرحبا\");",
                "/* שלום */ pub fn test() {}",
                "/// Документация на русском",
                "const MSG: &str = \"🎉 Success!\";",
            ],
            edge_cases: vec![
                "",       // Empty string
                " ",      // Single space
                "a",      // Single ASCII char
                "🦀",     // Single emoji
                "中",     // Single CJK
                "\n\n\n", // Multiple newlines
                "   \t  \n  ", // Whitespace mix
                          // Note: long strings tested separately to avoid lifetime issues
            ],
        }
    }
}

impl Default for Utf8TestData {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenEMR test data helpers
#[allow(dead_code)] // Used in integration tests
pub struct OpenEmrData;

impl OpenEmrData {
    /// Get path to OpenEMR repository
    #[allow(dead_code)] // Used in integration tests
    pub fn path() -> &'static Path {
        Path::new(OPENEMR_PATH)
    }

    /// Check if OpenEMR is available
    #[allow(dead_code)] // Used in integration tests
    pub fn is_available() -> bool {
        Path::new(OPENEMR_PATH).exists()
    }

    /// Get a subset of OpenEMR for faster tests
    /// Returns path to interface/ directory (smaller subset)
    #[allow(dead_code)] // Used in integration tests
    pub fn interface_dir() -> PathBuf {
        Path::new(OPENEMR_PATH).join("interface")
    }

    /// Get a smaller subset (just a few files for quick tests)
    #[allow(dead_code)] // Reserved for future tests
    pub fn small_subset() -> Vec<&'static str> {
        vec![
            "interface/main/main_screen.php",
            "interface/login/login.php",
            "library/sql.inc.php",
        ]
    }
}
