// UTF-8 safety tests: Emoji handling
//
// Validates that the chunker never panics on emoji characters and
// correctly preserves emoji in indexed content.

use crate::common::{create_test_services, index_test_repository, TestRepo};

#[tokio::test]
async fn test_index_emoji_in_comments() {
    let repo = TestRepo::with_files(&[(
        "emoji.rs",
        "// 🚀 Rocket launch\nfn main() { /* 🎉 Party */ }",
    )]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "emoji-1").await;

    // Should index without panicking
    assert_eq!(stats.files_indexed, 1);
    assert!(stats.chunks_created > 0);

    // Search for emoji should work
    let results = state
        .search
        .search_session("emoji-1", "Rocket", Some(10))
        .expect("Search failed");

    assert!(!results.results.is_empty());
}

#[tokio::test]
async fn test_index_various_emoji() {
    let emojis = vec![
        "👋",     // Waving hand
        "🦀",     // Crab (Rust mascot)
        "🧪",     // Test tube
        "✅",     // Check mark
        "❌",     // Cross mark
        "⚠️",     // Warning
        "🎉🎊🥳", // Multiple emojis
    ];

    for emoji in emojis {
        let content = format!("// Test with {}\nfn test() {{}}", emoji);
        let repo = TestRepo::with_files(&[("test.rs", &content)]);

        let state = create_test_services();
        let session_id = format!("emoji-{}", emoji.chars().next().unwrap() as u32);
        let stats = index_test_repository(&state, repo.path(), &session_id).await;

        assert_eq!(stats.files_indexed, 1, "Failed to index emoji: {}", emoji);
    }
}

#[tokio::test]
async fn test_emoji_at_chunk_boundary() {
    // Create content that might split emoji at chunk boundaries
    let mut content = String::new();
    content.push_str("fn test() {\n");
    // Add content to approach chunk boundary
    for i in 0..60 {
        content.push_str(&format!("    let x{} = {}; // 🦀\n", i, i));
    }
    content.push_str("}\n");

    let repo = TestRepo::with_files(&[("boundary.rs", &content)]);

    let state = create_test_services();
    let stats = index_test_repository(&state, repo.path(), "emoji-boundary").await;

    // Should chunk without panicking on emoji boundaries
    assert_eq!(stats.files_indexed, 1);
    assert!(stats.chunks_created > 1); // Should create multiple chunks
}
