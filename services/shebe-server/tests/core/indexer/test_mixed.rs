// UTF-8 safety tests: Mixed content
//
// Validates handling of files with mixed ASCII, emoji, and multibyte
// characters, as commonly found in real-world code.

use crate::common::{create_test_services, index_test_repository, TestRepo};

#[tokio::test]
async fn test_mixed_ascii_emoji_multibyte() {
    let repo = TestRepo::with_files(&[(
        "mixed.rs",
        r#"
            // 🚀 Launch function - 启动函数
            fn main() {
                println!("Hello 世界! 🌍");
            }
        "#,
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "mixed-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_code_with_international_comments() {
    let repo = TestRepo::with_files(&[(
        "i18n.rs",
        r#"
            // English comment
            // 中文注释
            // Русский комментарий
            // العربية تعليق
            fn test() { /* 🦀 */ }
        "#,
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "i18n-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_string_literals_with_unicode() {
    let repo = TestRepo::with_files(&[(
        "strings.rs",
        r#"
            const GREET_EN: &str = "Hello";
            const GREET_ZH: &str = "你好";
            const GREET_RU: &str = "Привет";
            const GREET_AR: &str = "مرحبا";
            const GREET_EMOJI: &str = "👋";
        "#,
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "strings-1").await;

    assert_eq!(stats.files_indexed, 1);

    // Search should work with Unicode in results
    let results = state
        .search
        .search_session("strings-1", "GREET", Some(10))
        .expect("Search failed");

    assert!(!results.results.is_empty());
}

#[tokio::test]
async fn test_rtl_and_ltr_mixed() {
    let repo =
        TestRepo::with_files(&[("rtl.rs", "// English then العربية then עברית\nfn test() {}")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "rtl-1").await;

    // Should handle RTL (right-to-left) text correctly
    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_complex_mixed_content() {
    let mut content = String::new();
    content.push_str("// Multi-language test file\n");
    content.push_str("// 🌍 International support\n\n");

    // Mix everything together
    for i in 0..20 {
        content.push_str(&format!("// Line {} - 中文 русский 🦀\n", i));
        content.push_str(&format!("fn test_{}() {{\n", i));
        content.push_str("    let msg = \"Hello مرحبا שלום 你好 🎉\";\n");
        content.push_str("}\n\n");
    }

    let repo = TestRepo::with_files(&[("complex.rs", &content)]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "complex-1").await;

    assert_eq!(stats.files_indexed, 1);
    assert!(stats.chunks_created > 1);
}

#[tokio::test]
async fn test_edge_case_whitespace_unicode() {
    let repo = TestRepo::with_files(&[(
        "whitespace.txt",
        "Normal\u{00A0}space\u{2003}em\u{3000}ideographic",
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "ws-1").await;

    // Should handle various Unicode whitespace characters
    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_empty_and_unicode() {
    let repo = TestRepo::with_files(&[
        ("empty1.txt", ""),
        ("unicode1.txt", "中文"),
        ("empty2.txt", ""),
        ("unicode2.txt", "🦀"),
    ]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "empty-uni").await;

    // Should handle mix of empty and Unicode files
    assert!(stats.files_indexed >= 2); // At least the non-empty files
}
