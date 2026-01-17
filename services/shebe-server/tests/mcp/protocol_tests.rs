//! MCP protocol unit tests

#[cfg(test)]
mod tests {
    use serde_json::json;
    use shebe::mcp::protocol::*;

    #[test]
    fn test_parse_initialize_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.jsonrpc, "2.0");
        assert!(req.id.is_some());
        assert!(req.params.is_some());
    }

    #[test]
    fn test_parse_tools_list_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_serialize_initialize_response() {
        let response = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "shebe-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["protocolVersion"], "2024-11-05");
        assert_eq!(json["serverInfo"]["name"], "shebe-mcp");
        assert_eq!(json["capabilities"]["tools"]["listChanged"], false);
    }

    #[test]
    fn test_error_response() {
        let error = JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: "Unknown method".to_string(),
            data: None,
        };

        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Unknown method");
    }

    #[test]
    fn test_json_rpc_response_with_result() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_json_rpc_response_with_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: None,
            error: Some(JsonRpcError {
                code: INTERNAL_ERROR,
                message: "Internal error".to_string(),
                data: None,
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"code\":-32603"));
        assert!(!json.contains("\"result\""));
    }
}
