//! Tool registry and execution dispatch for CLIBM MCP.

use crate::adapter::discovery::PluginToolDiscovery;
use crate::adapter::permissions::PermissionManager;
use crate::adapter::validator::SchemaValidator;
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
                // Short form: `audit_dependencies` matches `dep_audit__audit_dependencies`
                // when exactly one plugin provides a tool with that name.
                let suffix = format!("__{}", name);
                let matches: Vec<&ToolDefinition> = self
                    .dynamic_tools
                    .iter()
                    .filter(|(k, _)| k.ends_with(&suffix))
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
        SchemaValidator::validate(args, &tool.info.input_schema)?;
        (tool.handler)(args, bridge)
    }

    fn register_default_tools(&mut self) {
        // Free text (titles, IDs, names) always goes after `--` so a value that starts
        // with '-' can never be read as a CLI flag.
        fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
            args.get(key).and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty())
        }

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_status".to_string(),
                description: "Project status as JSON: branch, HEAD commit, uncommitted changes, active workspace/project, task counts per column".to_string(),
                input_schema: json!({ "type": "object", "properties": {} }),
            },
            is_mutating: false,
            handler: Arc::new(|_args, bridge| bridge.execute(&["--json", "status"])),
        });

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_task_list".to_string(),
                description: "List tasks as JSON ([object hash, task] pairs) in the active project, or in all workspaces".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "all_workspaces": { "type": "boolean" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                        "column": { "type": "string", "description": "Column ID, e.g. todo, in-progress, done" },
                        "tag": { "type": "string" }
                    }
                }),
            },
            is_mutating: false,
            handler: Arc::new(|args, bridge| {
                let mut cmd: Vec<&str> = vec!["--json", "task", "list"];
                if args.get("all_workspaces").and_then(|b| b.as_bool()) == Some(true) {
                    cmd.push("--all-workspaces");
                }
                for (key, flag) in [("priority", "--priority"), ("column", "--column"), ("tag", "--tag")] {
                    if let Some(v) = text(args, key) {
                        cmd.push(flag);
                        cmd.push(v);
                    }
                }
                bridge.execute(&cmd)
            }),
        });

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_task_show".to_string(),
                description: "Show one task as JSON. `task` accepts the full ID (APP-101), the number (101) or a hash prefix".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["task"],
                    "properties": { "task": { "type": "string" } }
                }),
            },
            is_mutating: false,
            handler: Arc::new(|args, bridge| {
                let task = text(args, "task").unwrap_or_default();
                bridge.execute(&["--json", "task", "show", "--", task])
            }),
        });

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_task_add".to_string(),
                description: "Create a task in the active project (recorded as a commit when auto_commit is on). Priority defaults to the project setting".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["title"],
                    "properties": {
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                        "column": { "type": "string", "description": "Column ID (default: todo)" },
                        "assignee": { "type": "string" },
                        "depends_on": { "type": "string", "description": "Comma-separated task references, e.g. \"101,APP-102\"" }
                    }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| {
                let mut cmd: Vec<&str> = vec!["--json", "task", "add"];
                for (key, flag) in [
                    ("priority", "-p"),
                    ("column", "-c"),
                    ("description", "-d"),
                    ("assignee", "-a"),
                    ("depends_on", "--depends-on"),
                ] {
                    if let Some(v) = text(args, key) {
                        cmd.push(flag);
                        cmd.push(v);
                    }
                }
                cmd.push("--");
                cmd.push(text(args, "title").unwrap_or_default());
                bridge.execute(&cmd)
            }),
        });

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_board_move".to_string(),
                description: "Move a task to another Kanban column. Dependency, WIP-limit and plugin rules apply; the error explains a refusal".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["task_id", "column"],
                    "properties": {
                        "task_id": { "type": "string", "description": "Full ID (APP-101), number (101) or hash prefix" },
                        "column": { "type": "string", "description": "Column ID, e.g. in-progress" }
                    }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| {
                let task = text(args, "task_id").unwrap_or_default();
                let column = text(args, "column").unwrap_or_default();
                bridge.execute(&["--json", "board", "move", "--", task, column])
            }),
        });

        self.register(ToolDefinition {
            info: McpToolInfo {
                name: "clibm_workspace_switch".to_string(),
                description: "Switch the active workspace by ID or name; without a target, cycles alphabetically".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": { "target": { "type": "string" } }
                }),
            },
            is_mutating: true,
            handler: Arc::new(|args, bridge| match text(args, "target") {
                Some(target) => bridge.execute(&["workspace", "switch", "--", target]),
                None => bridge.execute(&["workspace", "switch"]),
            }),
        });
    }
}

