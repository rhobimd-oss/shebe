// UTF-8 safety tests: Multibyte character handling
//
// Validates correct handling of CJK, Arabic, Hebrew, Cyrillic,
// and other multibyte UTF-8 characters.

use crate::common::{create_test_services, index_test_repository, TestRepo};

#[tokio::test]
async fn test_index_chinese_characters() {
    let repo = TestRepo::with_files(&[(
        "chinese.rs",
        "// 中文注释\nfn 测试() { println!(\"你好世界\"); }",
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "chinese-1").await;

    assert_eq!(stats.files_indexed, 1);
    assert!(stats.chunks_created > 0);
}

#[tokio::test]
async fn test_index_arabic_characters() {
    let repo =
        TestRepo::with_files(&[("arabic.rs", "// مرحبا بالعالم\nfn main() { /* العربية */ }")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "arabic-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_index_hebrew_characters() {
    let repo = TestRepo::with_files(&[("hebrew.rs", "// שלום עולם\nfn main() { /* עברית */ }")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "hebrew-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_index_cyrillic_characters() {
    let repo = TestRepo::with_files(&[(
        "russian.rs",
        "// Привет мир\nfn main() { println!(\"Русский\"); }",
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "cyrillic-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_index_japanese_characters() {
    let repo = TestRepo::with_files(&[("japanese.rs", "// こんにちは世界\nfn テスト() {}")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "japanese-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_index_korean_characters() {
    let repo = TestRepo::with_files(&[("korean.rs", "// 안녕하세요 세계\nfn 테스트() {}")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "korean-1").await;

    assert_eq!(stats.files_indexed, 1);
}

#[tokio::test]
async fn test_multibyte_at_chunk_boundary() {
    // Create content with multibyte chars near chunk boundaries
    let mut content = String::new();
    for i in 0..50 {
        content.push_str(&format!("// 中文{} 测试\n", i));
        content.push_str(&format!("fn test_{}() {{}}\n", i));
    }

    let repo = TestRepo::with_files(&[("boundary.rs", &content)]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "mb-boundary").await;

    // Should handle multibyte characters at boundaries
    assert_eq!(stats.files_indexed, 1);
    assert!(stats.chunks_created > 1);
}

#[tokio::test]
async fn test_search_multibyte_content() {
    let repo = TestRepo::with_files(&[("multi.rs", "// 中文 русский עברית العربية\nfn test() {}")]);

    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "mb-search").await;

    // Search for ASCII near multibyte
    let results = state
        .search
        .search_session("mb-search", "test", Some(10))
        .expect("Search failed");

    assert!(!results.results.is_empty());
}

#[tokio::test]
async fn test_all_unicode_planes() {
    let repo = TestRepo::with_files(&[("planes.txt", "BMP: 中文\nSMP: 𝕳𝖊𝖑𝖑𝖔\nAstral: 😀")]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "planes-1").await;

    // Should handle characters from different Unicode planes
    assert_eq!(stats.files_indexed, 1);
}
