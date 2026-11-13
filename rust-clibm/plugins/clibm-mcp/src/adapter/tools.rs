//! Tool registry and execution dispatch for CLIBM MCP.

use crate::adapter::discovery::PluginToolDiscovery;
use crate::adapter::permissions::PermissionManager;
use crate::backend::ClibmBridge;
use crate::error::JsonRpcError;
use crate::protocol::McpToolInfo;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type ToolHandler = Arc<dyn Fn(&Value, &ClibmBridge) -> Result<String, JsonRpcError> + Send + Sync>;

pub struct ToolDefinition {
    pub info: McpToolInfo,
    pub is_mutating: bool,
    pub handler: ToolHandler,
}

pub struct ToolRegistry {
    native_tools: HashMap<String, ToolDefinition>,
    dynamic_tools: HashMap<String, ToolDefinition>,
    registry_fingerprint: u64,
    home_dir: Option<PathBuf>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            native_tools: HashMap::new(),
            dynamic_tools: HashMap::new(),
            registry_fingerprint: 0,
            home_dir: None,
        };
        registry.register_default_tools();
        registry
    }

    pub fn new_with_discovery(project_root: &Path, home_dir: Option<PathBuf>) -> Self {
        let mut registry = Self::new();
        registry.home_dir = home_dir;
        registry.check_and_reload(project_root);
        registry
    }

    pub fn register(&mut self, definition: ToolDefinition) {
        self.native_tools.insert(definition.info.name.clone(), definition);
    }

    /// Checks whether any plugin registry files have been modified or added/removed.
    /// If changes are detected, re-discovers dynamic tools and updates the registry.
    /// Returns `true` if dynamic tools were reloaded, `false` otherwise.
    pub fn check_and_reload(&mut self, project_root: &Path) -> bool {
        let files = PluginToolDiscovery::candidate_registry_files(project_root, self.home_dir.as_deref());
        self.reload_with_files(&files, project_root)
    }

    /// Reloads dynamic plugin tools from explicit candidate files.
    pub fn reload_with_files(&mut self, files: &[PathBuf], project_root: &Path) -> bool {
        let (tools, new_fingerprint) = PluginToolDiscovery::discover_tools_from_files(files, project_root);
        if new_fingerprint != self.registry_fingerprint || (tools.is_empty() && !self.dynamic_tools.is_empty()) {
            let mut map = HashMap::new();
            for def in tools {
                map.insert(def.info.name.clone(), def);
            }
            self.dynamic_tools = map;
            self.registry_fingerprint = new_fingerprint;
            true
        } else {
            false
        }
    }

    pub fn list_tools(&self) -> Vec<McpToolInfo> {
        let mut list: Vec<McpToolInfo> = self
            .native_tools
            .values()
            .chain(self.dynamic_tools.values())
            .map(|d| d.info.clone())
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn execute(
        &self,
        name: &str,
        args: &Value,
        bridge: &ClibmBridge,
        permissions: &PermissionManager,
    ) -> Result<String, JsonRpcError> {
        let tool = self
            .native_tools
            .get(name)
            .or_else(|| self.dynamic_tools.get(name))
            .or_else(|| {
                // Secondary fallback: if requested name has no dot, match if exactly one dynamic tool ends with .name
                let matches: Vec<&ToolDefinition> = self
                    .dynamic_tools
                    .iter()
                    .filter(|(k, _)| k.ends_with(&format!(".{}", name)))
                    .map(|(_, v)| v)
                    .collect();
                if matches.len() == 1 {
                    Some(matches[0])
                } else {
                    None
                }
            })
            .ok_or_else(|| JsonRpcError::method_not_found(format!("Tool '{}' not found", name)))?;

        permissions.check_tool(&tool.info.name, tool.is_mutating)?;
        (tool.handler)(args, bridge)
    }

    fn register_default_tools(&mut self) {
        // 1. clibm_status
        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_status".to_string(),
                description: "Inspect active workspace, branch, HEAD commit, and BLAKE3 CAS object count".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            is_mutating: false,
            handler: Arc::new(|_args, bridge| {
                bridge.execute(&["board", "show"])
            }),
        });

        // 2. clibm_task_list
        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_task_list".to_string(),
                description: "List tasks in active workspace or across all workspaces".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "all_workspaces": { "type": "boolean" },
                        "priority": { "type": "string" }
                    }
                }),
            },
            is_mutating: false,
            handler: Arc::new(|args, bridge| {
                let mut cmd_args = vec!["task", "list"];
                if args.get("all_workspaces").and_then(|b| b.as_bool()) == Some(true) {
                    cmd_args.push("--all-workspaces");
                }
                bridge.execute(&cmd_args)
            }),
        });

        // 3. clibm_task_add
        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_task_add".to_string(),
                description: "Create a new immutable task in the active workspace (auto-committed with BLAKE3)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["title"],
                    "properties": {
                        "title": { "type": "string" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                        "column": { "type": "string" }
                    }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| {
                let title = args.get("title").and_then(|s| s.as_str()).unwrap_or("AI-generated task");
                let prio = args.get("priority").and_then(|s| s.as_str()).unwrap_or("high");
                let col = args.get("column").and_then(|s| s.as_str()).unwrap_or("todo");
                bridge.execute(&["task", "add", title, "-p", prio, "-c", col])
            }),
        });

        // 4. clibm_board_move
        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_board_move".to_string(),
                description: "Move a task to a target Kanban column with dependency validation".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["task_id", "column"],
                    "properties": {
                        "task_id": { "type": "string" },
                        "column": { "type": "string" }
                    }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| {
                let tid = args.get("task_id").and_then(|s| s.as_str()).unwrap_or("");
                let col = args.get("column").and_then(|s| s.as_str()).unwrap_or("todo");
                bridge.execute(&["board", "move", tid, col])
            }),
        });

        // 5. clibm_workspace_switch
        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_workspace_switch".to_string(),
                description: "Switch active workspace by ID/name or omit target to cycle alphabetically (A->Z)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "target": { "type": "string" }
                    }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| {
                let mut cmd_args = vec!["workspace", "switch"];
                if let Some(target) = args.get("target").and_then(|s| s.as_str()) {
                    if !target.is_empty() {
                        cmd_args.push(target);
                    }
                }
                bridge.execute(&cmd_args)
            }),
        });
    }
}

