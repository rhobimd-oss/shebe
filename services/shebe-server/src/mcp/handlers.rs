//! MCP protocol method handlers

use crate::core::services::Services;
use crate::mcp::error::McpError;
use crate::mcp::protocol::*;
use crate::mcp::tools::{
    DeleteSessionHandler, FindFileHandler, GetServerInfoHandler, GetSessionInfoHandler,
    IndexRepositoryHandler, ListDirHandler, ListSessionsHandler, PreviewChunkHandler,
    ReadFileHandler, ReindexSessionHandler, SearchCodeHandler, ShowShebeConfigHandler,
    ToolRegistry,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

pub struct ProtocolHandlers {
    initialized: AtomicBool,
    tool_registry: ToolRegistry,
}

impl ProtocolHandlers {
    pub fn new(services: Arc<Services>) -> Self {
        let mut registry = ToolRegistry::new();

        // Register all available tools
        registry.register(Arc::new(SearchCodeHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(ListSessionsHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(GetSessionInfoHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(IndexRepositoryHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(GetServerInfoHandler::new()));
        registry.register(Arc::new(ShowShebeConfigHandler::new(Arc::clone(
            &services.config,
        ))));
        registry.register(Arc::new(ReadFileHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(DeleteSessionHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(ListDirHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(FindFileHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(PreviewChunkHandler::new(Arc::clone(&services))));
        registry.register(Arc::new(ReindexSessionHandler::new(Arc::clone(&services))));

        Self {
            initialized: AtomicBool::new(false),
            tool_registry: registry,
        }
    }

    /// Handle initialize request
    pub async fn handle_initialize(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        let _params: InitializeParams =
            serde_json::from_value(request.params.unwrap_or(Value::Null))?;

        info!("Client initialized");

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "shebe-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    /// Handle initialized notification
    pub async fn handle_initialized(
        &self,
        _request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        self.initialized.store(true, Ordering::SeqCst);
        info!("Server initialized");

        // Initialized is a notification, no response needed
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: None,
        })
    }

    /// Handle tools/list request
    pub async fn handle_tools_list(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        // Get tools from registry
        let tools = self.tool_registry.list();

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({ "tools": tools })),
            error: None,
        })
    }

    /// Handle tools/call request
    pub async fn handle_tools_call(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        // Parse params
        let params_value = match request.params.clone() {
            Some(v) => v,
            None => {
                return Ok(self.create_error_response(
                    request.id,
                    INVALID_PARAMS,
                    "Missing params".to_string(),
                ));
            }
        };

        let params: ToolCallParams = match serde_json::from_value(params_value) {
            Ok(p) => p,
            Err(e) => {
                return Ok(self.create_error_response(
                    request.id,
                    INVALID_PARAMS,
                    format!("Invalid params: {e}"),
                ));
            }
        };

        // Get tool handler from registry
        let handler = match self.tool_registry.get(&params.name) {
            Some(h) => h,
            None => {
                return Ok(self.create_error_response(
                    request.id,
                    INVALID_REQUEST,
                    format!("Tool not found: {}", params.name),
                ));
            }
        };

        // Execute tool and handle errors
        match handler.execute(params.arguments).await {
            Ok(result) => Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::to_value(result)?),
                error: None,
            }),
            Err(e) => {
                // Map McpError to proper JSON-RPC error code
                let (code, message) = match &e {
                    McpError::ParseError(msg) => (PARSE_ERROR, msg.clone()),
                    McpError::InvalidRequest(msg) => (INVALID_REQUEST, msg.clone()),
                    McpError::InvalidParams(msg) => (INVALID_PARAMS, msg.clone()),
                    McpError::InternalError(msg) => (INTERNAL_ERROR, msg.clone()),
                    McpError::ToolError(code, msg) => (*code, msg.clone()),
                    McpError::Io(e) => (INTERNAL_ERROR, format!("I/O error: {e}")),
                    McpError::Json(e) => (INTERNAL_ERROR, format!("JSON error: {e}")),
                };

                Ok(self.create_error_response(request.id, code, message))
            }
        }
    }

    /// Create an error response with proper structure
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

    /// Handle ping request
    pub async fn handle_ping(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({})),
            error: None,
        })
    }
}

// ProtocolHandlers now requires Services, so Default is not implemented
