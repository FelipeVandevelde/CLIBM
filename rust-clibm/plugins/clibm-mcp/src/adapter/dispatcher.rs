//! Request dispatcher routing incoming JSON-RPC 2.0 requests to the appropriate subsystems.

use crate::adapter::permissions::PermissionManager;
use crate::adapter::prompts::PromptRegistry;
use crate::adapter::resources::ResourceRegistry;
use crate::adapter::tools::ToolRegistry;
use crate::backend::ClibmBridge;
use crate::config::McpConfig;
use crate::error::{JsonRpcError, TOOL_EXECUTION_ERROR};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse, PROTOCOL_VERSION};
use serde_json::{json, Value};
use std::sync::RwLock;

pub struct RequestDispatcher;

impl RequestDispatcher {
    pub fn dispatch(
        req: JsonRpcRequest,
        config: &McpConfig,
        tools: &RwLock<ToolRegistry>,
        resources: &ResourceRegistry,
        prompts: &PromptRegistry,
        permissions: &PermissionManager,
        bridge: &ClibmBridge,
    ) -> JsonRpcResponse {
        let id = req.id.unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => {
                let caps = Self::capabilities(config);
                JsonRpcResponse::success(id, caps)
            }
            "notifications/initialized" => {
                JsonRpcResponse::success(id, json!({}))
            }
            "ping" => JsonRpcResponse::success(id, json!({})),

            // Tools
            "tools/list" => {
                if let Ok(mut lock) = tools.write() {
                    let _ = lock.check_and_reload(bridge.project_root());
                }
                let lock = tools.read().unwrap();
                let tool_list = lock.list_tools();
                JsonRpcResponse::success(id, json!({ "tools": tool_list }))
            }
            "tools/call" => {
                if let Ok(mut lock) = tools.write() {
                    let _ = lock.check_and_reload(bridge.project_root());
                }
                let lock = tools.read().unwrap();
                let tool_name = match req.params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            JsonRpcError::invalid_params("Missing required 'name' parameter"),
                        )
                    }
                };
                let args = req.params.get("arguments").unwrap_or(&Value::Null);

                match lock.execute(tool_name, args, bridge, permissions) {
                    Ok(output) => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [
                                { "type": "text", "text": output }
                            ]
                        }),
                    ),
                    // The tool ran but failed (e.g. a move blocked by a dependency): report
                    // it as a tool result so the agent can read the reason and react.
                    Err(err) if err.code == TOOL_EXECUTION_ERROR => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [
                                { "type": "text", "text": err.message }
                            ],
                            "isError": true
                        }),
                    ),
                    Err(err) => JsonRpcResponse::error(id, err),
                }
            }

            // Resources
            "resources/list" => {
                let res_list = resources.list_resources();
                JsonRpcResponse::success(id, json!({ "resources": res_list }))
            }
            "resources/read" => {
                let uri = match req.params.get("uri").and_then(|v| v.as_str()) {
                    Some(u) => u,
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            JsonRpcError::invalid_params("Missing required 'uri' parameter"),
                        )
                    }
                };

                match resources.read_resource(uri, bridge) {
                    Ok(content) => JsonRpcResponse::success(
                        id,
                        json!({
                            "contents": [content]
                        }),
                    ),
                    Err(err) => JsonRpcResponse::error(id, err),
                }
            }

            // Prompts
            "prompts/list" => {
                let prompt_list = prompts.list_prompts();
                JsonRpcResponse::success(id, json!({ "prompts": prompt_list }))
            }
            "prompts/get" => {
                let prompt_name = match req.params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            JsonRpcError::invalid_params("Missing required 'name' parameter"),
                        )
                    }
                };
                let args = req.params.get("arguments").unwrap_or(&Value::Null);

                match prompts.get_prompt(prompt_name, args) {
                    Ok(messages) => JsonRpcResponse::success(
                        id,
                        json!({
                            "description": format!("Prompt messages for {}", prompt_name),
                            "messages": messages
                        }),
                    ),
                    Err(err) => JsonRpcResponse::error(id, err),
                }
            }

            other => JsonRpcResponse::error(id, JsonRpcError::method_not_found(other)),
        }
    }

    pub fn capabilities(config: &McpConfig) -> Value {
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {
                "name": "clibm-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "configuredTransport": config.transport.as_str(),
                "readOnly": config.read_only,
                "documentation": "https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md#ai-agent-mcp-configuration-json"
            },
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false },
                "prompts": { "listChanged": false }
            }
        })
    }
}

