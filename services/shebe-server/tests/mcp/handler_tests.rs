//! MCP handler unit tests

#[cfg(test)]
mod tests {
    use serde_json::json;
    use shebe::core::config::Config;
    use shebe::core::services::Services;
    use shebe::mcp::handlers::ProtocolHandlers;
    use shebe::mcp::protocol::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_handlers() -> (ProtocolHandlers, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();
        let services = Arc::new(Services::new(config));
        (ProtocolHandlers::new(services), temp_dir)
    }

    #[tokio::test]
    async fn test_initialize_handler() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "test", "version": "1.0"}
            })),
        };

        let response = handlers.handle_initialize(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "shebe-mcp");
    }

    #[tokio::test]
    async fn test_initialized_handler() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: Some(json!({})),
        };

        let response = handlers.handle_initialized(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.id.is_none());
        assert!(response.result.is_none());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_tools_list_has_tools() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = handlers.handle_tools_list(request).await.unwrap();

        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        // search, list, info, index, server_info, config, read, delete, list_dir, find,
        // find_references, preview, reindex, upgrade
        assert_eq!(tools.len(), 14);
    }

    #[tokio::test]
    async fn test_tools_call_missing_params() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: None, // Missing params should error
        };

        let response = handlers.handle_tools_call(request).await.unwrap();

        // Should return a response with an error field (JSON-RPC 2.0 spec)
        assert!(response.error.is_some());
        assert!(response.result.is_none());

        let error = response.error.unwrap();
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(error.message.contains("Missing params"));
    }

    #[tokio::test]
    async fn test_tools_call_index_repository_nonexistent_path() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": "/nonexistent/path/that/does/not/exist",
                    "session": "test-session"
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();

        // Should return a response with an error field
        assert!(response.error.is_some());
        assert!(response.result.is_none());

        let error = response.error.unwrap();
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(
            error.message.contains("Path does not exist"),
            "Error message should indicate path doesn't exist, got: {}",
            error.message
        );
    }

    #[tokio::test]
    async fn test_tools_call_index_repository_file_not_directory() {
        use std::fs::File;

        let (handlers, temp) = create_test_handlers();

        // Create a file (not a directory)
        let file_path = temp.path().join("test_file.txt");
        File::create(&file_path).unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": file_path.to_str().unwrap(),
                    "session": "test-session"
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();

        // Should return a response with an error field
        assert!(response.error.is_some());
        assert!(response.result.is_none());

        let error = response.error.unwrap();
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(
            error.message.contains("directory"),
            "Error message should indicate path is not a directory, got: {}",
            error.message
        );
    }

    #[tokio::test]
    async fn test_notifications_initialized_sets_flag() {
        let (handlers, _temp) = create_test_handlers();

        // MCP spec method name: notifications/initialized
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: Some(json!({})),
        };

        let response = handlers.handle_initialized(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.id.is_none());
        assert!(response.result.is_none());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_cancelled_handler() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/cancelled".to_string(),
            params: Some(json!({
                "requestId": "123",
                "reason": "User requested cancellation"
            })),
        };

        let response = handlers.handle_cancelled(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.id.is_none());
        assert!(response.result.is_none());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_ping_handler() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "ping".to_string(),
            params: None,
        };

        let response = handlers.handle_ping(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    // --- Phase 2B: index_repository success path + validations ---

    #[tokio::test]
    async fn test_index_repository_success() {
        let (handlers, temp) = create_test_handlers();

        // Create a directory with some files to index
        let repo_dir = temp.path().join("test-repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::write(
            repo_dir.join("main.rs"),
            "fn main() { println!(\"hello world\"); }",
        )
        .unwrap();
        std::fs::write(
            repo_dir.join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        )
        .unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "test-idx"
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();

        assert!(
            response.error.is_none(),
            "Expected success, got error: {:?}",
            response.error
        );
        let result = response.result.unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(
            content.contains("Indexing complete"),
            "Expected 'Indexing complete', got: {content}"
        );
        assert!(content.contains("Files indexed: 2"));
    }

    #[tokio::test]
    async fn test_index_repository_force_false_existing() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-force");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::write(repo_dir.join("file.rs"), "fn test() {}").unwrap();

        // Index once
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "force-test"
                }
            })),
        };
        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_none());

        // Index again with force=false
        let request2 = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "force-test",
                    "force": false
                }
            })),
        };
        let response2 = handlers.handle_tools_call(request2).await.unwrap();

        assert!(response2.error.is_some());
        let err = response2.error.unwrap();
        assert!(
            err.message.contains("already exists"),
            "Expected 'already exists', got: {}",
            err.message
        );
    }

    #[tokio::test]
    async fn test_index_repository_force_true_reindex() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-reindex");
        std::fs::create_dir_all(&repo_dir).unwrap();
        std::fs::write(repo_dir.join("file.rs"), "fn test() {}").unwrap();

        // Index once
        let req1 = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "reindex-test"
                }
            })),
        };
        let r1 = handlers.handle_tools_call(req1).await.unwrap();
        assert!(r1.error.is_none());

        // Re-index with force=true (default)
        let req2 = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(14)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "reindex-test",
                    "force": true
                }
            })),
        };
        let r2 = handlers.handle_tools_call(req2).await.unwrap();
        assert!(
            r2.error.is_none(),
            "force=true should succeed, got: {:?}",
            r2.error
        );
    }

    #[tokio::test]
    async fn test_validate_session_invalid_chars() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-val");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(15)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "invalid session!"
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_validate_session_too_long() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-long");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let long_session = "a".repeat(65);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(16)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": long_session
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_validate_chunk_size_too_small() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-chunk-s");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(17)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "chunk-small",
                    "chunk_size": 50
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_validate_chunk_size_too_large() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-chunk-l");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(18)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "chunk-large",
                    "chunk_size": 3000
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_validate_overlap_too_large() {
        let (handlers, temp) = create_test_handlers();

        let repo_dir = temp.path().join("repo-overlap");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(19)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "index_repository",
                "arguments": {
                    "path": repo_dir.to_str().unwrap(),
                    "session": "overlap-test",
                    "overlap": 600
                }
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, INVALID_PARAMS);
    }

    // --- Phase 2D: unknown tool test ---

    #[tokio::test]
    async fn test_tools_call_unknown_tool() {
        let (handlers, _temp) = create_test_handlers();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(20)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "nonexistent_tool",
                "arguments": {}
            })),
        };

        let response = handlers.handle_tools_call(request).await.unwrap();
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, INVALID_REQUEST);
        assert!(
            err.message.contains("nonexistent_tool"),
            "Error should mention tool name, got: {}",
            err.message
        );
    }
}
