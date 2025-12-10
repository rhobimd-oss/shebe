//! Integration tests for the Shebe REST API
//!
//! Tests the complete end-to-end workflow including indexing,
//! searching, and session management.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{delete, get, post},
    Router,
};
use serde_json::json;
use shebe::core::config::Config;
use shebe::core::services::Services;
use shebe::core::types::*;
use shebe::http::{self, middleware as http_middleware};
use tempfile::TempDir;
use tower::ServiceExt as TowerServiceExt;
use tower_http::cors::CorsLayer;

/// Create a test application with temporary storage
fn create_test_app() -> (Router, TempDir) {
    let temp_dir = TempDir::new().unwrap();

    let mut config = Config::default();
    config.storage.index_dir = temp_dir.path().to_path_buf();

    let state = Arc::new(Services::new(config));

    let app = Router::new()
        .route("/health", get(http::health_handler))
        .route("/api/v1/index", post(http::index_handler))
        .route("/api/v1/search", post(http::search_handler))
        .route("/api/v1/sessions", get(http::list_sessions_handler))
        .route(
            "/api/v1/sessions/:session_id",
            delete(http::delete_session_handler),
        )
        .layer(middleware::from_fn(http_middleware::log_request))
        .layer(CorsLayer::permissive())
        .with_state(state);

    (app, temp_dir)
}

/// Create test files in a directory
fn create_test_files(dir: &std::path::Path) {
    use std::fs;

    // Create some test files
    fs::write(
        dir.join("file1.txt"),
        "This is a test file with some content about Rust",
    )
    .unwrap();

    fs::write(
        dir.join("file2.txt"),
        "Another file containing information about async programming",
    )
    .unwrap();

    fs::write(
        dir.join("file3.txt"),
        "Final test file with data about web frameworks and Axum",
    )
    .unwrap();
}

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _temp) = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let health: HealthResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(health.status, "ok");
    assert!(!health.version.is_empty());
}

#[tokio::test]
async fn test_end_to_end_workflow() {
    let (app, temp_dir) = create_test_app();

    // Create test files in a subdirectory
    let test_data_dir = temp_dir.path().join("test_data");
    std::fs::create_dir(&test_data_dir).unwrap();
    create_test_files(&test_data_dir);

    // Step 1: Index the test data
    let index_req = json!({
        "path": test_data_dir.to_str().unwrap(),
        "session": "test-session",
        "include_patterns": ["*.txt"],
        "exclude_patterns": []
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/index")
                .header("content-type", "application/json")
                .body(Body::from(index_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let index_response: IndexResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(index_response.files_indexed, 3);
    assert!(index_response.chunks_created > 0);
    assert_eq!(index_response.session, "test-session");

    // Step 2: Search the indexed content
    let search_req = json!({
        "query": "Rust",
        "session": "test-session",
        "k": 10
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from(search_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let search_response: SearchResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(search_response.query, "Rust");
    assert!(search_response.count > 0);
    assert!(!search_response.results.is_empty());

    // Step 3: List sessions
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let sessions_response: SessionsResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(sessions_response.sessions.len(), 1);
    assert_eq!(sessions_response.sessions[0].id, "test-session");

    // Step 4: Delete the session
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/sessions/test-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let delete_response: DeleteResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(delete_response.status, "deleted");
    assert_eq!(delete_response.session, "test-session");

    // Step 5: Verify session is gone
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let sessions_response: SessionsResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(sessions_response.sessions.len(), 0);
}

#[tokio::test]
async fn test_index_nonexistent_path() {
    let (app, _temp) = create_test_app();

    let index_req = json!({
        "path": "/nonexistent/path",
        "session": "test-session",
        "include_patterns": [],
        "exclude_patterns": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/index")
                .header("content-type", "application/json")
                .body(Body::from(index_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Path not found should return NOT_FOUND (404)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_index_duplicate_session() {
    let (app, temp_dir) = create_test_app();

    let test_data_dir = temp_dir.path().join("test_data");
    std::fs::create_dir(&test_data_dir).unwrap();
    create_test_files(&test_data_dir);

    let index_req = json!({
        "path": test_data_dir.to_str().unwrap(),
        "session": "duplicate-test",
        "include_patterns": [],
        "exclude_patterns": []
    });

    // First index should succeed
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/index")
                .header("content-type", "application/json")
                .body(Body::from(index_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Second index with same session should fail
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/index")
                .header("content-type", "application/json")
                .body(Body::from(index_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_search_empty_query() {
    let (app, _temp) = create_test_app();

    let search_req = json!({
        "query": "   ",
        "session": "test-session",
        "k": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from(search_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_search_nonexistent_session() {
    let (app, _temp) = create_test_app();

    let search_req = json!({
        "query": "test",
        "session": "nonexistent-session",
        "k": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from(search_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_session() {
    let (app, _temp) = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/sessions/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
