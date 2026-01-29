//! MCP server implementation

use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::handlers::ProtocolHandlers;
use crate::mcp::protocol::*;
use crate::mcp::transport::StdioTransport;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error, info};

pub struct McpServer {
    transport: StdioTransport,
    handlers: Arc<ProtocolHandlers>,
}

impl McpServer {
    pub fn new(services: Arc<Services>) -> Self {
        Self {
            transport: StdioTransport::new(),
            handlers: Arc::new(ProtocolHandlers::new(services)),
        }
    }

    /// Run the MCP server (blocking)
    pub async fn run(&mut self) -> Result<(), McpError> {
        info!("Starting Shebe MCP server");

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();

        // Spawn signal handler
        let mut shutdown = tokio::spawn(async {
            tokio::signal::ctrl_c().await.ok();
        });

        // Main loop
        loop {
            tokio::select! {
                // Process stdin messages
                line = reader.next_line() => {
                    match line? {
                        Some(line) if !line.trim().is_empty() => {
                            self.process_and_respond(&line).await?;
                        }
                        None => break, // EOF
                        _ => continue,
                    }
                }

                // Handle Ctrl+C
                _ = &mut shutdown => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }

        info!("MCP server shutting down");
        Ok(())
    }

    async fn process_and_respond(&mut self, line: &str) -> Result<(), McpError> {
        debug!("Received: {}", line);

        // Check if this is a JSON-RPC notification (no id field).
        // Per JSON-RPC 2.0, notifications must not receive a response.
        let raw: serde_json::Value =
            serde_json::from_str(line).map_err(|e| McpError::ParseError(e.to_string()))?;
        let is_notification = raw.get("id").is_none();

        match self.process_message(line).await {
            Ok(response) => {
                if !is_notification {
                    self.transport.send_response(response).await?;
                }
            }
            Err(e) => {
                error!("Error processing message: {}", e);
                if !is_notification {
                    let error_response =
                        self.create_error_response(None, INTERNAL_ERROR, e.to_string());
                    self.transport.send_response(error_response).await?;
                }
            }
        }

        Ok(())
    }

    async fn process_message(&self, line: &str) -> Result<JsonRpcResponse, McpError> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest =
            serde_json::from_str(line).map_err(|e| McpError::ParseError(e.to_string()))?;

        // Route to handler
        match request.method.as_str() {
            "initialize" => self.handlers.handle_initialize(request).await,
            "initialized" | "notifications/initialized" => {
                self.handlers.handle_initialized(request).await
            }
            "notifications/cancelled" => self.handlers.handle_cancelled(request).await,
            "tools/list" => self.handlers.handle_tools_list(request).await,
            "tools/call" => self.handlers.handle_tools_call(request).await,
            "ping" => self.handlers.handle_ping(request).await,
            _ => Ok(self.create_error_response(
                request.id,
                METHOD_NOT_FOUND,
                format!("Unknown method: {}", request.method),
            )),
        }
    }

    fn create_error_response(
        &self,
        id: Option<Value>,
        code: i32,
        message: String,
    ) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

// McpServer now requires Services, so Default is not implemented

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use tempfile::TempDir;

    fn create_test_server() -> (McpServer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.storage.index_dir = temp_dir.path().to_path_buf();
        let services = Arc::new(Services::new(config));
        (McpServer::new(services), temp_dir)
    }

    #[tokio::test]
    async fn test_process_message_initialize() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn test_process_message_ping() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping"
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_process_message_tools_list() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 14);
    }

    #[tokio::test]
    async fn test_process_message_notifications_initialized() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        // Notification -- no error
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_process_message_notifications_cancelled() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": "42", "reason": "test"}
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_process_message_unknown_method() {
        let (server, _temp) = create_test_server();

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "nonexistent/method"
        })
        .to_string();

        let response = server.process_message(&msg).await.unwrap();
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, METHOD_NOT_FOUND);
        assert!(err.message.contains("nonexistent/method"));
    }

    #[tokio::test]
    async fn test_process_message_invalid_json() {
        let (server, _temp) = create_test_server();

        let result = server.process_message("not valid json{{{").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_error_response() {
        let (server, _temp) = create_test_server();

        let response = server.create_error_response(
            Some(serde_json::json!(99)),
            INTERNAL_ERROR,
            "test error".to_string(),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(serde_json::json!(99)));
        assert!(response.result.is_none());
        let err = response.error.unwrap();
        assert_eq!(err.code, INTERNAL_ERROR);
        assert_eq!(err.message, "test error");
    }
}
