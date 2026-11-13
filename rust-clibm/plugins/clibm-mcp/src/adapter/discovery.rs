//! Registry-based dynamic discovery for plugin MCP tools.

use crate::adapter::tools::{ToolDefinition, ToolHandler};
use crate::adapter::validator::SchemaValidator;
use crate::backend::ClibmBridge;
use crate::protocol::{McpToolInfo, PluginRegistryEntry};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub struct PluginToolDiscovery;

impl PluginToolDiscovery {
    /// Registry files to inspect: `[global, project]`. The global registry
    /// (`~/.clibm/plugins/registry.json`) is the list of installed plugins; the project
    /// registry may only switch them on or off (`{"name": .., "enabled": ..}`).
    pub fn candidate_registry_files(project_root: &Path, home_dir: Option<&Path>) -> Vec<PathBuf> {
        let home = home_dir.map(Path::to_path_buf).or_else(|| {
            env::var("CLIBM_HOME")
                .or_else(|_| env::var("HOME"))
                .or_else(|_| env::var("USERPROFILE"))
                .ok()
                .map(PathBuf::from)
        });
        let global = home
            .map(|h| h.join(".clibm").join("plugins").join("registry.json"))
            .unwrap_or_else(|| PathBuf::from(".clibm-global-registry-unavailable"));
        let project = project_root.join(".clibm").join("plugins").join("registry.json");
        vec![global, project]
    }

    /// Computes a composite fingerprint (mtime + size) for active registry files.
    pub fn compute_fingerprint(files: &[PathBuf]) -> u64 {
        let mut hash: u64 = 14695981039346656037; // FNV-1a 64-bit basis
        for path in files {
            if let Ok(meta) = fs::metadata(path) {
                let len = meta.len();
                let mtime_sec = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let mtime_nsec = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.subsec_nanos() as u64)
                    .unwrap_or(0);

                // Mix file metrics
                for byte in len
                    .to_le_bytes()
                    .iter()
                    .chain(mtime_sec.to_le_bytes().iter())
                    .chain(mtime_nsec.to_le_bytes().iter())
                {
                    hash ^= *byte as u64;
                    hash = hash.wrapping_mul(1099511628211);
                }
            }
        }
        hash
    }

    /// Reads registry files, validates plugin tool declarations, and builds MCP ToolDefinitions.
    #[allow(dead_code)]
    pub fn discover_tools(project_root: &Path) -> (Vec<ToolDefinition>, u64) {
        let files = Self::candidate_registry_files(project_root, None);
        Self::discover_tools_from_files(&files, project_root)
    }

    /// Discovers tools from explicit candidate registry files.
    pub fn discover_tools_from_files(
        files: &[PathBuf],
        _project_root: &Path,
    ) -> (Vec<ToolDefinition>, u64) {
        let fingerprint = Self::compute_fingerprint(files);
        let read_entries = |path: &PathBuf| -> Vec<serde_json::Value> {
            fs::read_to_string(path)
                .ok()
                .and_then(|raw| {
                    let clean = raw.strip_prefix('\u{feff}').unwrap_or(&raw).to_string();
                    serde_json::from_str::<Vec<serde_json::Value>>(&clean).ok()
                })
                .unwrap_or_default()
        };

        // 1. Installed plugins come from the global registry (first file) only; project
        //    registries (remaining files) can only override `enabled`.
        let mut plugins_by_name: HashMap<String, PluginRegistryEntry> = HashMap::new();
        if let Some(global) = files.first() {
            for value in read_entries(global) {
                if let Ok(entry) = serde_json::from_value::<PluginRegistryEntry>(value) {
                    plugins_by_name.entry(entry.name.clone()).or_insert(entry);
                }
            }
        }
        for overrides in files.iter().skip(1) {
            for value in read_entries(overrides) {
                let name = value.get("name").and_then(|n| n.as_str());
                let enabled = value.get("enabled").and_then(|e| e.as_bool());
                if let (Some(name), Some(enabled)) = (name, enabled) {
                    if let Some(entry) = plugins_by_name.get_mut(name) {
                        entry.enabled = enabled;
                    }
                }
            }
        }

        // 2. Build tools. Names are always `<plugin>__<tool>` (letters, digits, '_' and
        //    '-' only, at most 64 characters), so a plugin can neither shadow a native
        //    tool nor claim another plugin's tool name.
        let mut discovered = Vec::new();
        let mut registered_names: HashSet<String> = HashSet::new();

        for (plugin_name, manifest) in plugins_by_name {
            if !manifest.enabled {
                continue;
            }
            // Only installed plugins: the executable must exist.
            let installed = manifest
                .executable
                .as_deref()
                .map(|e| Path::new(e).is_file())
                .unwrap_or(false);
            if !installed {
                continue;
            }

            let mut raw_tools = manifest.tools.clone();
            if let Some(mcp) = manifest.mcp {
                raw_tools.extend(mcp.tools);
            }
            let plugin_prefix = Self::clean_plugin_prefix(&plugin_name);

            for tool_decl in raw_tools {
                let tool_part = Self::sanitize_name(tool_decl.name.trim());
                if tool_part.is_empty() || plugin_prefix.is_empty() {
                    continue;
                }
                let qualified_name = format!("{}__{}", plugin_prefix, tool_part);
                if qualified_name.len() > 64 {
                    eprintln!(
                        "[clibm-mcp] Warning: tool name '{}' from plugin '{}' is longer than 64 characters; skipping.",
                        qualified_name, plugin_name
                    );
                    continue;
                }
                if !registered_names.insert(qualified_name.clone()) {
                    eprintln!(
                        "[clibm-mcp] Warning: duplicate tool name '{}' from plugin '{}'; skipping.",
                        qualified_name, plugin_name
                    );
                    continue;
                }

                let description = tool_decl.description.unwrap_or_else(|| {
                    format!("Tool '{}' provided by plugin '{}'", tool_part, plugin_name)
                });

                discovered.push(Self::build_tool_definition(
                    qualified_name,
                    description,
                    tool_decl.input_schema,
                    tool_decl.is_mutating,
                    plugin_name.clone(),
                    tool_decl.execution_ref,
                ));
            }
        }

        // Sort discovered tools deterministically
        discovered.sort_by(|a, b| a.info.name.cmp(&b.info.name));

        (discovered, fingerprint)
    }

    /// Normalizes a plugin package name into a clean prefix (e.g. `clibm-dep-audit` -> `dep_audit`).
    pub fn clean_plugin_prefix(name: &str) -> String {
        let trimmed = name.trim();
        let stripped = trimmed.strip_prefix("clibm-").unwrap_or(trimmed);
        Self::sanitize_name(stripped).replace('-', "_")
    }

    /// Keeps letters, digits, '_' and '-' (MCP clients accept `^[a-zA-Z0-9_-]{1,64}$`);
    /// anything else becomes '_'.
    pub fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect()
    }

    /// Builds a ToolDefinition with the dispatch closure.
    fn build_tool_definition(
        name: String,
        description: String,
        input_schema: Value,
        is_mutating: bool,
        plugin_name: String,
        execution_ref: Option<String>,
    ) -> ToolDefinition {
        let schema_for_handler = input_schema.clone();
        let plugin_for_handler = plugin_name.clone();
        let exec_ref_for_handler = execution_ref.clone();

        let handler: ToolHandler = Arc::new(move |args: &Value, bridge: &ClibmBridge| {
            // 1. Validate input against declared schema
            SchemaValidator::validate(args, &schema_for_handler)?;

            // 2. Prepare CLI arguments for `clibm plugin run <plugin_name> -- [args]`
            let mut cmd_args: Vec<String> = vec!["plugin".to_string(), "run".to_string(), plugin_for_handler.clone()];

            let mut pass_args: Vec<String> = Vec::new();
            if let Some(ref eref) = exec_ref_for_handler {
                if !eref.is_empty() && eref != "run" {
                    pass_args.push(eref.clone());
                }
            }

            // Convert JSON arguments to CLI flags
            if let Value::Object(map) = args {
                for (key, val) in map {
                    let flag = format!("--{}", key.replace('_', "-"));
                    match val {
                        Value::Bool(b) => {
                            if *b {
                                pass_args.push(flag);
                            }
                        }
                        Value::String(s) => {
                            pass_args.push(flag);
                            pass_args.push(s.clone());
                        }
                        Value::Number(n) => {
                            pass_args.push(flag);
                            pass_args.push(n.to_string());
                        }
                        Value::Array(arr) => {
                            for item in arr {
                                if let Value::String(s) = item {
                                    pass_args.push(flag.clone());
                                    pass_args.push(s.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !pass_args.is_empty() {
                cmd_args.push("--".to_string());
                cmd_args.extend(pass_args);
            }

            let cmd_slices: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
            bridge.execute(&cmd_slices)
        });

        ToolDefinition {
            info: McpToolInfo {
                name,
                description,
                input_schema,
            },
            is_mutating,
            handler,
        }
    }
}
