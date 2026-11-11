//! HTTP JSON-RPC 2.0 Transport for CLIBM MCP.

use crate::adapter::McpAdapter;
use crate::error::JsonRpcError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct HttpTransport;

impl HttpTransport {
    pub fn run(
        adapter: Arc<McpAdapter>,
        port: u16,
        shutdown_signal: Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let server = tiny_http::Server::http(&addr)
            .map_err(|e| anyhow!("Failed to bind MCP HTTP server on {}: {}", addr, e))?;

        let doc_link = "https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md#ai-agent-mcp-configuration-json";
        let banner = format!(
            "====================================================================\n\
             CLIBM-MCP v0.2.0 — Self-Hosted AI Model Context Protocol Server\n\
             Transport Config : HTTP SSE\n\
             HTTP/SSE Endpoint: http://{}\n\
             Active Project   : {}\n\
             Read-Only Mode   : {}\n\
             Documentation    : {}\n\
             Agent Setup      : Configure your agent JSON using the documentation link above\n\
            ====================================================================",
            addr,
            adapter.project_root().display(),
            adapter.config().read_only,
            doc_link,
        );

        if shutdown_signal.is_some() {
            eprintln!("{}", banner);
        } else {
            println!("{}", banner);
        }

        loop {
            if let Some(ref sig) = shutdown_signal {
                if sig.load(Ordering::Relaxed) {
                    break;
                }
            }

            let mut request = match server.recv() {
                Ok(rq) => rq,
                Err(_) => break,
            };

            let method = request.method().as_str().to_uppercase();

            // Support CORS preflight
            if method == "OPTIONS" {
                let resp = tiny_http::Response::empty(200)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                            .unwrap(),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Methods"[..],
                            &b"POST, GET, OPTIONS"[..],
                        )
                        .unwrap(),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Headers"[..],
                            &b"Content-Type"[..],
                        )
                        .unwrap(),
                    );
                let _ = request.respond(resp);
                continue;
            }

            // Handle GET /: return capabilities JSON
            if method == "GET" {
                let caps = serde_json::to_string(&adapter.capabilities())?;
                let resp = tiny_http::Response::from_string(caps)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                            .unwrap(),
                    );
                let _ = request.respond(resp);
                continue;
            }

            // Handle POST /: JSON-RPC
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);

            let rpc_resp = match serde_json::from_str::<JsonRpcRequest>(&body) {
                Ok(req) => adapter.handle_request(req),
                Err(e) => JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(format!("Malformed JSON: {}", e)),
                ),
            };

            let response_json = serde_json::to_string(&rpc_resp)?;
            let header_type =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let header_cors =
                tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
            let resp = tiny_http::Response::from_string(response_json)
                .with_header(header_type)
                .with_header(header_cors);
            let _ = request.respond(resp);
        }

        Ok(())
    }
}

