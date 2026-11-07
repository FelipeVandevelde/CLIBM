//! Standalone CLIBM Self-Hosted MCP Server Plugin (`clibm-mcp`)
//!
//! Exposes any ongoing `.clibm` repository to AI agents and IDEs (Claude Desktop, Cursor,
//! Windsurf, custom LLM agents) via the Model Context Protocol (`2024-11-05`).
//!
//! Supports configurable transport modes selected during installation:
//!   - `both` (Default): Supports both `stdio` JSON-RPC 2.0 and Local HTTP + SSE (`127.0.0.1:7879`)
//!   - `stdio`: Stdio JSON-RPC 2.0 transport only
//!   - `http`: Local HTTP + SSE JSON-RPC 2.0 server only (`127.0.0.1:7879`)

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn load_configured_transport() -> String {
    if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
        let cfg_file = PathBuf::from(home).join(".clibm").join("mcp-config.json");
        if let Ok(raw) = fs::read_to_string(cfg_file) {
            if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                if let Some(t) = v.get("transport").and_then(|x| x.as_str()) {
                    return t.to_string();
                }
            }
        }
    }
    "both".to_string()
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let configured_transport = load_configured_transport();
    let want_http = args.iter().any(|a| a == "--http") || configured_transport == "http";
    let want_stdio = args.iter().any(|a| a == "--stdio") || configured_transport == "stdio";

    if configured_transport == "stdio" && args.iter().any(|a| a == "--http") {
        bail!(
            "clibm-mcp was installed with transport='stdio' only. Re-run ./install.sh to enable 'both' or 'http'."
        );
    }
    if configured_transport == "http" && args.iter().any(|a| a == "--stdio") {
        bail!(
            "clibm-mcp was installed with transport='http' only. Re-run ./install.sh to enable 'both' or 'stdio'."
        );
    }

    let port = args
        .windows(2)
        .find(|w| w[0] == "--port" || w[0] == "-p")
        .and_then(|w| w[1].parse::<u16>().ok())
        .unwrap_or(7879);

    if want_http && !want_stdio {
        return run_http_server(&project_root, port, &configured_transport);
    }

    // Default stdio JSON-RPC 2.0 loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
            let resp = handle_rpc(&project_root, req, &configured_transport);
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

fn run_http_server(project_root: &PathBuf, port: u16, transport_mode: &str) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind MCP HTTP server on {}: {}", addr, e))?;

    println!("====================================================================");
    println!(" CLIBM-MCP v0.1.0 — Self-Hosted AI Model Context Protocol Server");
    println!(" Transport Config : {}", transport_mode);
    println!(" HTTP/SSE Endpoint: http://{}", addr);
    println!(" Active Project   : {}", project_root.display());
    println!("====================================================================");

    for mut request in server.incoming_requests() {
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);
        let response_json = if let Ok(rpc_req) = serde_json::from_str::<JsonRpcRequest>(&body) {
            serde_json::to_string(&handle_rpc(project_root, rpc_req, transport_mode))?
        } else {
            serde_json::to_string(&mcp_capabilities(transport_mode))?
        };
        let header =
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
        let _ = request.respond(tiny_http::Response::from_string(response_json).with_header(header));
    }
    Ok(())
}

fn mcp_capabilities(transport_mode: &str) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "clibm-mcp",
            "version": "0.1.0",
            "configuredTransport": transport_mode
        },
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": true, "listChanged": true }
        }
    })
}

fn handle_rpc(project_root: &PathBuf, req: JsonRpcRequest, transport_mode: &str) -> JsonRpcResponse {
    let id = req.id.unwrap_or(Value::Null);
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(mcp_capabilities(transport_mode)),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "clibm_status",
                        "description": "Inspect active workspace, branch, HEAD commit, and BLAKE3 CAS object count",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "clibm_task_list",
                        "description": "List tasks in active workspace or across all workspaces",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "all_workspaces": { "type": "boolean" },
                                "priority": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "clibm_task_add",
                        "description": "Create a new immutable task in the active workspace (auto-committed with BLAKE3)",
                        "inputSchema": {
                            "type": "object",
                            "required": ["title"],
                            "properties": {
                                "title": { "type": "string" },
                                "priority": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                                "column": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "clibm_board_move",
                        "description": "Move a task to a target Kanban column with dependency validation",
                        "inputSchema": {
                            "type": "object",
                            "required": ["task_id", "column"],
                            "properties": {
                                "task_id": { "type": "string" },
                                "column": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "clibm_workspace_switch",
                        "description": "Switch active workspace by ID/name or omit target to cycle alphabetically (A->Z)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target": { "type": "string" }
                            }
                        }
                    }
                ]
            })),
            error: None,
        },
        "resources/list" => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "resources": [
                    {
                        "uri": "clibm://tree/staged",
                        "name": "Staged BLAKE3 Tree Snapshot",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "clibm://config",
                        "name": "Repository Configuration (.clibm/config.json)",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "clibm://board/active",
                        "name": "Active Workspace Kanban Board",
                        "mimeType": "text/plain"
                    }
                ]
            })),
            error: None,
        },
        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("clibm_status");
            let args = req.params.get("arguments").cloned().unwrap_or(json!({}));
            let out = dispatch_clibm_tool(project_root, tool_name, &args);
            JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "content": [{ "type": "text", "text": out }]
                })),
                error: None,
            }
        }
        other => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": format!("Unknown MCP method '{}'", other)
            })),
        },
    }
}

fn dispatch_clibm_tool(project_root: &PathBuf, tool_name: &str, args: &Value) -> String {
    let mut cmd_args: Vec<String> = match tool_name {
        "clibm_task_list" => {
            let mut v = vec!["task".to_string(), "list".to_string()];
            if args.get("all_workspaces").and_then(|b| b.as_bool()) == Some(true) {
                v.push("--all-workspaces".to_string());
            }
            v
        }
        "clibm_task_add" => {
            let title = args
                .get("title")
                .and_then(|s| s.as_str())
                .unwrap_or("AI-generated task");
            let prio = args
                .get("priority")
                .and_then(|s| s.as_str())
                .unwrap_or("high");
            let col = args.get("column").and_then(|s| s.as_str()).unwrap_or("todo");
            vec![
                "task".to_string(),
                "add".to_string(),
                title.to_string(),
                "-p".to_string(),
                prio.to_string(),
                "-c".to_string(),
                col.to_string(),
            ]
        }
        "clibm_board_move" => {
            let tid = args.get("task_id").and_then(|s| s.as_str()).unwrap_or("");
            let col = args.get("column").and_then(|s| s.as_str()).unwrap_or("todo");
            vec![
                "board".to_string(),
                "move".to_string(),
                tid.to_string(),
                col.to_string(),
            ]
        }
        "clibm_workspace_switch" => {
            let mut v = vec!["workspace".to_string(), "switch".to_string()];
            if let Some(t) = args.get("target").and_then(|s| s.as_str()) {
                if !t.is_empty() {
                    v.push(t.to_string());
                }
            }
            v
        }
        _ => vec!["board".to_string(), "show".to_string()],
    };

    if let Ok(out) = Command::new("clibm")
        .args(&cmd_args)
        .current_dir(project_root)
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        if !stdout.trim().is_empty() {
            return stdout;
        }
    }
    cmd_args.insert(0, "clibm".to_string());
    format!("Executed via MCP bridge: {}", cmd_args.join(" "))
}
