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

        match self.process_message(line).await {
            Ok(response) => {
                self.transport.send_response(response).await?;
            }
            Err(e) => {
                error!("Error processing message: {}", e);
                let error_response =
                    self.create_error_response(None, INTERNAL_ERROR, e.to_string());
                self.transport.send_response(error_response).await?;
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
            "initialized" => self.handlers.handle_initialized(request).await,
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
