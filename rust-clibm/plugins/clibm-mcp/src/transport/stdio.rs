//! Stdio JSON-RPC 2.0 Transport for CLIBM MCP.

use crate::adapter::McpAdapter;
use crate::error::JsonRpcError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

pub struct StdioTransport;

impl StdioTransport {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    pub fn run(adapter: Arc<McpAdapter>) -> Result<()> {
        let doc_link = "https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md#ai-agent-mcp-configuration-json";
        eprintln!("====================================================================");
        eprintln!(" CLIBM-MCP v0.2.0 — Self-Hosted AI Model Context Protocol Server");
        eprintln!(" Transport Config : STDIO JSON-RPC 2.0");
        eprintln!(" Active Project   : {}", adapter.project_root().display());
        eprintln!(" Read-Only Mode   : {}", adapter.config().read_only);
        eprintln!(" Documentation    : {}", doc_link);
        eprintln!(" Agent Setup      : Configure your agent JSON using the documentation link above");
        eprintln!("====================================================================");

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break, // stdin closed or broken pipe
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let resp = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => {
                    // Notifications have no id and MUST NOT receive a response (JSON-RPC 2.0 §4.1)
                    if req.id.is_none() {
                        let _ = adapter.handle_request(req);
                        continue;
                    }
                    adapter.handle_request(req)
                }
                Err(err) => JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(format!("Malformed JSON: {}", err)),
                ),
            };

            let out_json = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", out_json)?;
            stdout.flush()?;
        }

        Ok(())
    }
}
