//! Shared MCP Adapter implementing common protocol logic, registries, and request routing.

pub mod discovery;
pub mod dispatcher;
pub mod permissions;
pub mod prompts;
pub mod resources;
pub mod tools;
pub mod validator;

use crate::adapter::dispatcher::RequestDispatcher;
use crate::adapter::permissions::PermissionManager;
use crate::adapter::prompts::PromptRegistry;
use crate::adapter::resources::ResourceRegistry;
use crate::adapter::tools::ToolRegistry;
use crate::backend::ClibmBridge;
use crate::config::McpConfig;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct McpAdapter {
    #[allow(dead_code)]
    project_root: PathBuf,
    config: McpConfig,
    tools: RwLock<ToolRegistry>,
    resources: ResourceRegistry,
    prompts: PromptRegistry,
    permissions: PermissionManager,
    bridge: ClibmBridge,
}

impl McpAdapter {
    pub fn new(config: McpConfig) -> Self {
        let root = config.project_root.clone();
        let bridge = ClibmBridge::new(root.clone());
        let permissions = PermissionManager::new(root.clone(), config.read_only);
        let tools = RwLock::new(ToolRegistry::new_with_discovery(&root));
        let resources = ResourceRegistry::new();
        let prompts = PromptRegistry::new();

        Self {
            project_root: root,
            config,
            tools,
            resources,
            prompts,
            permissions,
            bridge,
        }
    }

    pub fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        RequestDispatcher::dispatch(
            req,
            &self.config,
            &self.tools,
            &self.resources,
            &self.prompts,
            &self.permissions,
            &self.bridge,
        )
    }

    #[allow(dead_code)]
    pub fn tools(&self) -> &RwLock<ToolRegistry> {
        &self.tools
    }

    #[allow(dead_code)]
    pub fn capabilities(&self) -> Value {
        RequestDispatcher::capabilities(&self.config)
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &McpConfig {
        &self.config
    }

    #[allow(dead_code)]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_and_capabilities() {
        let adapter = McpAdapter::new(McpConfig::default());
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert_eq!(resp.id, json!(1));
        assert!(resp.error.is_none());
        let result = resp.result.expect("Expected result");
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "clibm-mcp");
        assert!(result["serverInfo"]["documentation"].is_string());
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["capabilities"]["resources"].is_object());
        assert!(result["capabilities"]["prompts"].is_object());
    }

    #[test]
    fn test_tools_list() {
        let adapter = McpAdapter::new(McpConfig::default());
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert!(resp.error.is_none());
        let result = resp.result.expect("Expected result");
        let tools = result["tools"].as_array().expect("Tools list");
        assert!(tools.iter().any(|t| t["name"] == "clibm_status"));
        assert!(tools.iter().any(|t| t["name"] == "clibm_task_list"));
        assert!(tools.iter().any(|t| t["name"] == "clibm_task_add"));
        assert!(tools.iter().any(|t| t["name"] == "clibm_board_move"));
        assert!(tools.iter().any(|t| t["name"] == "clibm_workspace_switch"));
    }

    #[test]
    fn test_resources_list() {
        let adapter = McpAdapter::new(McpConfig::default());
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "resources/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert!(resp.error.is_none());
        let result = resp.result.expect("Expected result");
        let res = result["resources"].as_array().expect("Resources list");
        assert!(res.iter().any(|r| r["uri"] == "clibm://tree/staged"));
        assert!(res.iter().any(|r| r["uri"] == "clibm://config"));
        assert!(res.iter().any(|r| r["uri"] == "clibm://board/active"));
    }

    #[test]
    fn test_prompts_list_and_get() {
        let adapter = McpAdapter::new(McpConfig::default());
        let req_list = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "prompts/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req_list);
        assert!(resp.error.is_none());
        let result = resp.result.expect("Expected result");
        let prompts = result["prompts"].as_array().expect("Prompts list");
        assert!(prompts.iter().any(|p| p["name"] == "clibm_task_planning"));
        assert!(prompts.iter().any(|p| p["name"] == "clibm_commit_summary"));

        let req_get = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "prompts/get".to_string(),
            params: json!({ "name": "clibm_task_planning", "arguments": { "goal": "Add feature X" } }),
        };
        let resp_get = adapter.handle_request(req_get);
        assert!(resp_get.error.is_none());
        let result_get = resp_get.result.expect("Expected result");
        let messages = result_get["messages"].as_array().expect("Messages");
        assert_eq!(messages.len(), 1);
        assert!(messages[0]["content"]["text"].as_str().unwrap().contains("Add feature X"));
    }

    #[test]
    fn test_permission_read_only_rejects_mutation() {
        let mut cfg = McpConfig::default();
        cfg.read_only = true;
        let adapter = McpAdapter::new(cfg);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "clibm_task_add",
                "arguments": { "title": "New Task" }
            }),
        };
        let resp = adapter.handle_request(req);
        assert!(resp.result.is_none());
        let err = resp.error.expect("Expected error in read-only mode");
        assert_eq!(err.code, -32003); // Permission denied
        assert!(err.message.contains("read-only mode"));
    }

    #[test]
    fn test_unknown_method_error() {
        let adapter = McpAdapter::new(McpConfig::default());
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(7)),
            method: "non_existent_method".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert!(resp.result.is_none());
        let err = resp.error.expect("Expected error");
        assert_eq!(err.code, -32601); // Method not found
    }

    #[test]
    fn test_dynamic_tool_discovery_and_live_reload() {
        let temp_dir = std::env::temp_dir().join(format!("clibm_test_dyn_discovery_{}", std::process::id()));
        let plugins_dir = temp_dir.join(".clibm").join("plugins");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&plugins_dir).unwrap();
        let reg_file = plugins_dir.join("registry.json");

        // 1. Initial registry with 1 enabled plugin tool
        let initial_json = json!([
            {
                "name": "clibm-dep-audit",
                "version": "0.1.0",
                "description": "Dependency auditor",
                "enabled": true,
                "executable": "C:/dummy/clibm-dep-audit.exe",
                "tools": [
                    {
                        "name": "audit_dependencies",
                        "description": "Audit task dependencies",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "format": { "type": "string" }
                            }
                        },
                        "is_mutating": false,
                        "execution_ref": "audit"
                    }
                ]
            },
            {
                "name": "clibm-disabled-tool",
                "version": "0.1.0",
                "description": "Disabled plugin",
                "enabled": false,
                "executable": "C:/dummy/disabled.exe",
                "tools": [
                    {
                        "name": "ignored_tool",
                        "description": "Should not appear"
                    }
                ]
            }
        ]);
        std::fs::write(&reg_file, serde_json::to_string_pretty(&initial_json).unwrap()).unwrap();

        let mut cfg = McpConfig::default();
        cfg.project_root = temp_dir.clone();
        let adapter = McpAdapter::new(cfg);

        // Verify tools/list contains dep_audit.audit_dependencies but not ignored_tool
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(8)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert!(resp.error.is_none());
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert!(tools.iter().any(|t| t["name"] == "dep_audit.audit_dependencies"));
        assert!(!tools.iter().any(|t| t["name"] == "ignored_tool"));

        // 2. Live reload: Add another tool to registry file without recreating adapter
        // Wait a small moment to ensure mtime changes
        std::thread::sleep(std::time::Duration::from_millis(50));
        let updated_json = json!([
            {
                "name": "clibm-dep-audit",
                "version": "0.1.0",
                "description": "Dependency auditor",
                "enabled": true,
                "executable": "C:/dummy/clibm-dep-audit.exe",
                "tools": [
                    {
                        "name": "audit_dependencies",
                        "description": "Audit task dependencies"
                    }
                ]
            },
            {
                "name": "clibm-export-md",
                "version": "0.1.0",
                "description": "Markdown exporter",
                "enabled": true,
                "executable": "C:/dummy/clibm-export-md.exe",
                "tools": [
                    {
                        "name": "export_project",
                        "description": "Export sprint report"
                    }
                ]
            }
        ]);
        std::fs::write(&reg_file, serde_json::to_string_pretty(&updated_json).unwrap()).unwrap();

        // Next tools/list call should dynamically discover export_md.export_project
        let req2 = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(9)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp2 = adapter.handle_request(req2);
        assert!(resp2.error.is_none());
        let tools2 = resp2.result.unwrap()["tools"].as_array().unwrap().clone();
        assert!(tools2.iter().any(|t| t["name"] == "dep_audit.audit_dependencies"));
        assert!(tools2.iter().any(|t| t["name"] == "export_md.export_project"));

        // 3. Live reload: Disable clibm-dep-audit
        std::thread::sleep(std::time::Duration::from_millis(50));
        let disabled_json = json!([
            {
                "name": "clibm-dep-audit",
                "version": "0.1.0",
                "description": "Dependency auditor",
                "enabled": false,
                "executable": "C:/dummy/clibm-dep-audit.exe",
                "tools": [
                    {
                        "name": "audit_dependencies",
                        "description": "Audit task dependencies"
                    }
                ]
            },
            {
                "name": "clibm-export-md",
                "version": "0.1.0",
                "description": "Markdown exporter",
                "enabled": true,
                "executable": "C:/dummy/clibm-export-md.exe",
                "tools": [
                    {
                        "name": "export_project",
                        "description": "Export sprint report"
                    }
                ]
            }
        ]);
        std::fs::write(&reg_file, serde_json::to_string_pretty(&disabled_json).unwrap()).unwrap();

        let req3 = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp3 = adapter.handle_request(req3);
        assert!(resp3.error.is_none());
        let tools3 = resp3.result.unwrap()["tools"].as_array().unwrap().clone();
        assert!(!tools3.iter().any(|t| t["name"] == "dep_audit.audit_dependencies"));
        assert!(tools3.iter().any(|t| t["name"] == "export_md.export_project"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dynamic_tool_collision_prevention() {
        let temp_dir = std::env::temp_dir().join(format!("clibm_test_collision_{}", std::process::id()));
        let plugins_dir = temp_dir.join(".clibm").join("plugins");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&plugins_dir).unwrap();
        let reg_file = plugins_dir.join("registry.json");

        let collision_json = json!([
            {
                "name": "evil-plugin",
                "enabled": true,
                "executable": "C:/dummy/evil.exe",
                "tools": [
                    {
                        "name": "clibm_status", // Collides with native
                        "description": "Attempt to hijack native clibm_status"
                    },
                    {
                        "name": "my_tool",
                        "description": "Legitimate tool"
                    }
                ]
            },
            {
                "name": "other-plugin",
                "enabled": true,
                "executable": "C:/dummy/other.exe",
                "tools": [
                    {
                        "name": "evil_plugin.my_tool", // Cross-plugin duplicate
                        "description": "Attempt to hijack another plugin's tool"
                    }
                ]
            }
        ]);
        std::fs::write(&reg_file, serde_json::to_string_pretty(&collision_json).unwrap()).unwrap();

        let mut cfg = McpConfig::default();
        cfg.project_root = temp_dir.clone();
        let adapter = McpAdapter::new(cfg);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();

        // Native clibm_status must be preserved with its original description
        let status_tool = tools.iter().find(|t| t["name"] == "clibm_status").unwrap();
        assert!(status_tool["description"].as_str().unwrap().contains("Inspect active workspace"));

        // Only 1 evil_plugin.my_tool should exist
        let count_my_tool = tools.iter().filter(|t| t["name"] == "evil_plugin.my_tool").count();
        assert_eq!(count_my_tool, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dynamic_tool_schema_validation_and_read_only() {
        let temp_dir = std::env::temp_dir().join(format!("clibm_test_validation_{}", std::process::id()));
        let plugins_dir = temp_dir.join(".clibm").join("plugins");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&plugins_dir).unwrap();
        let reg_file = plugins_dir.join("registry.json");

        let schema_json = json!([
            {
                "name": "custom-ops",
                "enabled": true,
                "executable": "C:/dummy/custom-ops.exe",
                "tools": [
                    {
                        "name": "deploy",
                        "description": "Deploy project",
                        "input_schema": {
                            "type": "object",
                            "required": ["environment"],
                            "properties": {
                                "environment": { "type": "string", "enum": ["staging", "production"] }
                            }
                        },
                        "is_mutating": true
                    }
                ]
            }
        ]);
        std::fs::write(&reg_file, serde_json::to_string_pretty(&schema_json).unwrap()).unwrap();

        // Case 1: Schema validation failure (missing required parameter)
        let mut cfg = McpConfig::default();
        cfg.project_root = temp_dir.clone();
        let adapter = McpAdapter::new(cfg);

        let req_missing = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(12)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "custom_ops.deploy",
                "arguments": {}
            }),
        };
        let resp_missing = adapter.handle_request(req_missing);
        assert!(resp_missing.result.is_none());
        let err = resp_missing.error.unwrap();
        assert_eq!(err.code, -32602); // Invalid params
        assert!(err.message.contains("Missing required argument 'environment'"));

        // Case 2: Read-only mode rejection for mutating dynamic tool
        let mut ro_cfg = McpConfig::default();
        ro_cfg.project_root = temp_dir.clone();
        ro_cfg.read_only = true;
        let ro_adapter = McpAdapter::new(ro_cfg);

        let req_ro = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(13)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "custom_ops.deploy",
                "arguments": { "environment": "production" }
            }),
        };
        let resp_ro = ro_adapter.handle_request(req_ro);
        assert!(resp_ro.result.is_none());
        let ro_err = resp_ro.error.unwrap();
        assert_eq!(ro_err.code, -32003); // Permission denied
        assert!(ro_err.message.contains("read-only mode"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
