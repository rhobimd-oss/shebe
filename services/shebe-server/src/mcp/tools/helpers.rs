//! Helper functions for MCP tools

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Detect programming language from file extension
pub fn detect_language(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("jsx") => "javascript",
        Some("ts") => "typescript",
        Some("tsx") => "typescript",
        Some("java") => "java",
        Some("go") => "go",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") => "cpp",
        Some("c") | Some("h") => "c",
        Some("php") => "php",
        Some("rb") => "ruby",
        Some("sh") | Some("bash") => "bash",
        Some("sql") => "sql",
        Some("md") => "markdown",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("xml") => "xml",
        Some("html") | Some("htm") => "html",
        Some("css") => "css",
        Some("scss") | Some("sass") => "scss",
        Some("swift") => "swift",
        Some("kt") | Some("kts") => "kotlin",
        Some("cs") => "csharp",
        Some("ex") | Some("exs") => "elixir",
        Some("erl") | Some("hrl") => "erlang",
        Some("hs") => "haskell",
        Some("scala") | Some("sc") => "scala",
        Some("clj") | Some("cljs") | Some("cljc") => "clojure",
        Some("vim") => "vim",
        Some("lua") => "lua",
        Some("pl") | Some("pm") => "perl",
        Some("r") => "r",
        Some("jl") => "julia",
        _ => "",
    }
}

/// Truncate text if it exceeds max length
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    // Truncate at character boundary
    let truncated: String = text.chars().take(max_chars).collect();
    format!(
        "{}...\n\n[Truncated {} chars]",
        truncated,
        text.len() - max_chars
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_b() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(2048), "2.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(5242880), "5.00 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(2147483648), "2.00 GB");
    }

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(detect_language("main.rs"), "rust");
        assert_eq!(detect_language("/path/to/lib.rs"), "rust");
    }

    #[test]
    fn test_detect_language_python() {
        assert_eq!(detect_language("script.py"), "python");
    }

    #[test]
    fn test_detect_language_javascript() {
        assert_eq!(detect_language("app.js"), "javascript");
        assert_eq!(detect_language("component.jsx"), "javascript");
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language("app.ts"), "typescript");
        assert_eq!(detect_language("component.tsx"), "typescript");
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(detect_language("unknown.xyz"), "");
        assert_eq!(detect_language("no_extension"), "");
    }

    #[test]
    fn test_detect_language_various() {
        assert_eq!(detect_language("code.cpp"), "cpp");
        assert_eq!(detect_language("code.php"), "php");
        assert_eq!(detect_language("script.sh"), "bash");
        assert_eq!(detect_language("data.json"), "json");
        assert_eq!(detect_language("config.toml"), "toml");
    }

    #[test]
    fn test_truncate_text_no_truncation() {
        let text = "Hello, world!";
        assert_eq!(truncate_text(text, 100), text);
    }

    #[test]
    fn test_truncate_text_exact_length() {
        let text = "Hello!";
        assert_eq!(truncate_text(text, 6), text);
    }

    #[test]
    fn test_truncate_text_with_truncation() {
        let text = "Hello, world!";
        let result = truncate_text(text, 5);
        assert!(result.starts_with("Hello"));
        assert!(result.contains("...\n\n[Truncated 8 chars]"));
    }

    #[test]
    fn test_truncate_text_utf8_safe() {
        let text = "Hello 世界"; // 9 chars (not bytes!)
        let result = truncate_text(text, 7);
        assert!(result.chars().count() > 7); // Includes truncation message
        assert!(result.contains("Hello 世"));
    }
}
