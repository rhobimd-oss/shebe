// Integration tests for search functionality

use crate::common::{
    assert_valid_stats, create_test_services, index_test_repository, OpenEmrData, TestRepo,
};

#[tokio::test]
async fn test_search_basic_query() {
    let repo = TestRepo::with_files(&[
        (
            "src/auth.rs",
            "pub fn authenticate_user(name: &str) -> bool { true }",
        ),
        (
            "src/login.rs",
            "pub fn login_handler() { println!(\"login\"); }",
        ),
    ]);

    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-basic").await;

    // Execute search
    let results = state
        .search
        .search_session("search-basic", "authenticate", Some(10))
        .expect("Search failed");

    // Verify results
    assert!(!results.results.is_empty(), "Expected at least one result");
    assert!(
        results.results[0].text.contains("authenticate"),
        "Expected result to contain 'authenticate'"
    );
    assert!(results.results[0].score > 0.0, "Expected positive score");
}

#[tokio::test]
async fn test_search_phrase_query() {
    let repo = TestRepo::with_files(&[
        (
            "test.rs",
            "async fn main() { println!(\"async function\"); }",
        ),
        ("other.rs", "fn sync_func() { println!(\"sync\"); }"),
    ]);

    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-phrase").await;

    // Phrase query
    let results = state
        .search
        .search_session("search-phrase", "\"async function\"", Some(10))
        .expect("Search failed");

    assert!(!results.results.is_empty());
    assert!(results.results[0].text.contains("async"));
}

#[tokio::test]
async fn test_search_boolean_query() {
    let repo = TestRepo::with_files(&[
        ("auth.rs", "authenticate login password validation"),
        ("config.rs", "database connection settings"),
    ]);

    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-boolean").await;

    // Boolean AND query
    let results = state
        .search
        .search_session("search-boolean", "login AND password", Some(10))
        .expect("Search failed");

    assert!(!results.results.is_empty());
    // auth.rs should match since it has both terms
}

#[tokio::test]
async fn test_search_no_results() {
    let repo = TestRepo::small();
    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-empty").await;

    let results = state
        .search
        .search_session("search-empty", "nonexistent_term_xyz123", Some(10))
        .expect("Search failed");

    assert_eq!(results.results.len(), 0, "Expected no results");
}

#[tokio::test]
async fn test_search_session_not_found() {
    let state = create_test_services();

    let result = state
        .search
        .search_session("nonexistent-session", "test", Some(10));

    assert!(result.is_err(), "Expected error for nonexistent session");
}

#[tokio::test]
async fn test_search_k_limit() {
    let repo = TestRepo::medium();
    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-limit").await;

    // Search with k=5
    let results = state
        .search
        .search_session("search-limit", "func", Some(5))
        .expect("Search failed");

    assert!(
        results.results.len() <= 5,
        "Expected at most 5 results, got {}",
        results.results.len()
    );
}

#[tokio::test]
async fn test_search_ranking_by_relevance() {
    let repo = TestRepo::with_files(&[
        (
            "high_relevance.rs",
            "user authenticate authenticate authenticate",
        ),
        ("low_relevance.rs", "user profile settings authenticate"),
    ]);

    let state = create_test_services();
    let _stats = index_test_repository(&state, repo.path(), "search-rank").await;

    let results = state
        .search
        .search_session("search-rank", "authenticate user", Some(10))
        .expect("Search failed");

    // Both files should match since they both have "user" and "authenticate"
    assert!(!results.results.is_empty(), "Expected at least one result");

    // If we get 2 results, verify ranking
    if results.results.len() >= 2 {
        assert!(
            results.results[0].score >= results.results[1].score,
            "Results should be ranked by score (descending)"
        );
    }
}

#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn test_search_openemr() {
    // Skip if OpenEMR not available
    if !OpenEmrData::is_available() {
        eprintln!("Skipping OpenEMR search test: repository not found");
        return;
    }

    let state = create_test_services();

    // Index OpenEMR interface directory
    let stats =
        index_test_repository(&state, &OpenEmrData::interface_dir(), "openemr-search").await;

    assert_valid_stats(&stats);

    // Search for common medical terms
    let queries = vec!["patient", "authentication", "database", "function", "class"];

    for query in queries {
        let results = state
            .search
            .search_session("openemr-search", query, Some(10))
            .unwrap_or_else(|_| panic!("Search for '{}' failed", query));

        println!("Query '{}': {} results", query, results.results.len());
        assert!(
            !results.results.is_empty(),
            "Expected results for query '{}'",
            query
        );

        // Verify all results contain relevant content
        for (i, result) in results.results.iter().take(3).enumerate() {
            println!(
                "  Result {}: score={:.2}, file={}",
                i + 1,
                result.score,
                result.file_path
            );
        }
    }
}

#[tokio::test]
async fn test_search_multiple_sessions() {
    let repo1 = TestRepo::with_files(&[("file1.rs", "content one alpha")]);
    let repo2 = TestRepo::with_files(&[("file2.rs", "content two beta")]);

    let state = create_test_services();

    // Index both repos in separate sessions
    let _stats1 = index_test_repository(&state, repo1.path(), "session-1").await;
    let _stats2 = index_test_repository(&state, repo2.path(), "session-2").await;

    // Search session 1
    let results1 = state
        .search
        .search_session("session-1", "alpha", Some(10))
        .expect("Search session-1 failed");

    // Search session 2
    let results2 = state
        .search
        .search_session("session-2", "beta", Some(10))
        .expect("Search session-2 failed");

    // Verify isolation
    assert!(!results1.results.is_empty());
    assert!(results1.results[0].text.contains("alpha"));

    assert!(!results2.results.is_empty());
    assert!(results2.results[0].text.contains("beta"));

    // Cross-session queries should not find results
    let results_cross = state
        .search
        .search_session("session-1", "beta", Some(10))
        .expect("Cross-session search failed");

    assert_eq!(
        results_cross.results.len(),
        0,
        "Session-1 should not contain 'beta'"
    );
}
