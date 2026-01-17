//! Stdio transport for MCP protocol

use crate::mcp::error::McpError;
use crate::mcp::protocol::JsonRpcResponse;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::debug;

pub struct StdioTransport {
    stdout: BufWriter<tokio::io::Stdout>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdout: BufWriter::new(tokio::io::stdout()),
        }
    }

    /// Send JSON-RPC response to stdout
    pub async fn send_response(&mut self, response: JsonRpcResponse) -> Result<(), McpError> {
        // Skip responses for notifications (no id)
        if response.id.is_none() && response.result.is_none() && response.error.is_none() {
            return Ok(());
        }

        let json = serde_json::to_string(&response)?;
        debug!("Sending: {}", json);

        // Write JSON + newline
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;

        Ok(())
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}
