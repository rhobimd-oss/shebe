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
        assert_eq!(tools.len(), 12); // search, list, info, index, server_info, config, read, delete, list_dir, find, preview, reindex
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
}
