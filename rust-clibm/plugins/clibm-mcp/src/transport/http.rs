//! Local HTTP transport for CLIBM MCP (a minimal "Streamable HTTP" endpoint).
//!
//! - `POST /mcp` (or `/`) with one JSON-RPC message or a batch; requests get
//!   `application/json` responses, notifications get `202 Accepted` with no body.
//! - `GET` answers `405` (no server-sent event stream is offered).
//! - Only `127.0.0.1` is bound, no CORS headers are sent, and requests whose `Host` or
//!   `Origin` is not this local server are refused (blocks other websites and DNS
//!   rebinding).

use crate::adapter::McpAdapter;
use crate::error::JsonRpcError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct HttpTransport;

/// Result of one HTTP exchange, independent of tiny_http.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub body: Option<String>,
}

fn is_local(value: &str, port: u16) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v == format!("127.0.0.1:{}", port) || v == format!("localhost:{}", port)
}

fn json_error(status: u16, message: &str) -> HttpReply {
    HttpReply {
        status,
        body: Some(serde_json::json!({ "error": message }).to_string()),
    }
}

fn handle_message(adapter: &McpAdapter, value: Value) -> Option<JsonRpcResponse> {
    match serde_json::from_value::<JsonRpcRequest>(value) {
        Ok(req) if req.id.is_none() => {
            // Notification: process it, never answer it.
            let _ = adapter.handle_request(req);
            None
        }
        Ok(req) => Some(adapter.handle_request(req)),
        Err(e) => Some(JsonRpcResponse::error(
            Value::Null,
            JsonRpcError::invalid_request(e.to_string()),
        )),
    }
}

/// Handles one HTTP request.
pub fn handle_http(
    adapter: &McpAdapter,
    port: u16,
    method: &str,
    path: &str,
    host: Option<&str>,
    origin: Option<&str>,
    body: &str,
) -> HttpReply {
    if !host.map(|h| is_local(h, port)).unwrap_or(false) {
        return json_error(403, "Forbidden: unexpected Host header");
    }
    if let Some(o) = origin {
        let allowed = o
            .trim()
            .to_ascii_lowercase()
            .strip_prefix("http://")
            .map(|rest| is_local(rest, port))
            .unwrap_or(false);
        if !allowed {
            return json_error(403, "Forbidden: cross-origin request");
        }
    }
    let path = path.split('?').next().unwrap_or("/");
    if path != "/" && path != "/mcp" {
        return json_error(404, "Not found: send JSON-RPC messages to POST /mcp");
    }
    if method != "POST" {
        return json_error(405, "Method not allowed: use POST /mcp");
    }

    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            let resp = JsonRpcResponse::error(
                Value::Null,
                JsonRpcError::parse_error(format!("Malformed JSON: {}", e)),
            );
            return HttpReply {
                status: 400,
                body: serde_json::to_string(&resp).ok(),
            };
        }
    };

    let answer = match parsed {
        Value::Array(items) => {
            let responses: Vec<JsonRpcResponse> =
                items.into_iter().filter_map(|m| handle_message(adapter, m)).collect();
            if responses.is_empty() {
                None
            } else {
                serde_json::to_string(&responses).ok()
            }
        }
        single => handle_message(adapter, single).and_then(|r| serde_json::to_string(&r).ok()),
    };
    match answer {
        Some(body) => HttpReply {
            status: 200,
            body: Some(body),
        },
        None => HttpReply {
            status: 202,
            body: None,
        },
    }
}

impl HttpTransport {
    pub fn run(
        adapter: Arc<McpAdapter>,
        port: u16,
        shutdown_signal: Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let server = tiny_http::Server::http(&addr)
            .map_err(|e| anyhow!("Failed to bind MCP HTTP server on {}: {}", addr, e))?;

        // Logs go to stderr so they never mix with a stdio transport running alongside.
        eprintln!("====================================================================");
        eprintln!(" clibm-mcp {} - HTTP transport", env!("CARGO_PKG_VERSION"));
        eprintln!(" Endpoint  : POST http://{}/mcp", addr);
        eprintln!(" Project   : {}", adapter.project_root().display());
        eprintln!(" Read-only : {}", adapter.config().read_only);
        eprintln!("====================================================================");

        loop {
            if let Some(ref sig) = shutdown_signal {
                if sig.load(Ordering::Relaxed) {
                    break;
                }
            }
            let mut request = match server.recv_timeout(Duration::from_millis(500)) {
                Ok(Some(rq)) => rq,
                Ok(None) => continue,
                Err(_) => break,
            };

            let method = request.method().as_str().to_uppercase();
            let path = request.url().to_string();
            let header = |name: &str| {
                request
                    .headers()
                    .iter()
                    .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
                    .map(|h| h.value.as_str().to_string())
            };
            let host = header("Host");
            let origin = header("Origin");
            let mut body = String::new();
            if method == "POST" {
                let _ = request.as_reader().read_to_string(&mut body);
            }

            let reply = handle_http(&adapter, port, &method, &path, host.as_deref(), origin.as_deref(), &body);
            let mut response = tiny_http::Response::from_string(reply.body.unwrap_or_default())
                .with_status_code(reply.status);
            response.add_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            );
            if reply.status == 405 {
                response.add_header(tiny_http::Header::from_bytes(&b"Allow"[..], &b"POST"[..]).unwrap());
            }
            let _ = request.respond(response);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpConfig;

    fn adapter() -> McpAdapter {
        let mut cfg = McpConfig::default();
        cfg.home_dir = Some(std::env::temp_dir().join("clibm-mcp-http-test-home"));
        McpAdapter::new(cfg)
    }

    const HOST: Option<&str> = Some("127.0.0.1:7879");

    #[test]
    fn requests_notifications_and_methods() {
        let a = adapter();
        let ping = handle_http(&a, 7879, "POST", "/mcp", HOST, None, r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        assert_eq!(ping.status, 200);
        assert!(ping.body.unwrap().contains("\"id\":1"));

        let note = handle_http(&a, 7879, "POST", "/mcp", HOST, None, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        assert_eq!(note, HttpReply { status: 202, body: None });

        let batch = handle_http(
            &a,
            7879,
            "POST",
            "/",
            HOST,
            None,
            r#"[{"jsonrpc":"2.0","id":7,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"}]"#,
        );
        assert_eq!(batch.status, 200);
        let v: Value = serde_json::from_str(&batch.body.unwrap()).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);

        assert_eq!(handle_http(&a, 7879, "GET", "/mcp", HOST, None, "").status, 405);
        assert_eq!(handle_http(&a, 7879, "POST", "/mcp", HOST, None, "{not json").status, 400);
        assert_eq!(handle_http(&a, 7879, "POST", "/other", HOST, None, "{}").status, 404);
    }

    #[test]
    fn foreign_hosts_and_origins_are_refused() {
        let a = adapter();
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        assert_eq!(handle_http(&a, 7879, "POST", "/mcp", Some("evil.example:7879"), None, body).status, 403);
        assert_eq!(handle_http(&a, 7879, "POST", "/mcp", None, None, body).status, 403);
        assert_eq!(
            handle_http(&a, 7879, "POST", "/mcp", HOST, Some("https://evil.example"), body).status,
            403
        );
        assert_eq!(
            handle_http(&a, 7879, "POST", "/mcp", Some("localhost:7879"), Some("http://localhost:7879"), body).status,
            200
        );
    }
}
