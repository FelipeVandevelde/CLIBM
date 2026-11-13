//! Resource registry and URI resolution for CLIBM MCP.

use crate::backend::ClibmBridge;
use crate::error::JsonRpcError;
use crate::protocol::{McpResourceContent, McpResourceInfo};
use std::collections::HashMap;
use std::sync::Arc;

pub type ResourceReader = Arc<dyn Fn(&str, &ClibmBridge) -> Result<McpResourceContent, JsonRpcError> + Send + Sync>;

pub struct ResourceDefinition {
    pub info: McpResourceInfo,
    pub reader: ResourceReader,
}

pub struct ResourceRegistry {
    resources: HashMap<String, ResourceDefinition>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            resources: HashMap::new(),
        };
        registry.register_default_resources();
        registry
    }

    pub fn register(&mut self, definition: ResourceDefinition) {
        self.resources.insert(definition.info.uri.clone(), definition);
    }

    pub fn list_resources(&self) -> Vec<McpResourceInfo> {
        let mut list: Vec<McpResourceInfo> = self.resources.values().map(|r| r.info.clone()).collect();
        list.sort_by(|a, b| a.uri.cmp(&b.uri));
        list
    }

    pub fn read_resource(&self, uri: &str, bridge: &ClibmBridge) -> Result<McpResourceContent, JsonRpcError> {
        let res = self
            .resources
            .get(uri)
            .ok_or_else(|| JsonRpcError::resource_not_found(uri))?;

        (res.reader)(uri, bridge)
    }

    fn register_default_resources(&mut self) {
        // 1. clibm://tree/staged
        self.register(ResourceDefinition {
            info: McpResourceInfo {
                uri: "clibm://tree/staged".to_string(),
                name: "Staged tree snapshot".to_string(),
                description: Some("The current (staged) tree object: IDs of workspaces, projects, boards and tasks mapped to their object hashes".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            reader: Arc::new(|uri, bridge| {
                let staged = bridge.read_repo_file(".clibm/STAGE_TREE")?;
                let hash = staged.trim();
                if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(JsonRpcError::resource_not_found("no staged tree recorded in .clibm/STAGE_TREE"));
                }
                let content = bridge.execute(&["cat-object", hash])?;
                Ok(McpResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(content),
                    blob: None,
                })
            }),
        });

        // 2. clibm://config
        self.register(ResourceDefinition {
            info: McpResourceInfo {
                uri: "clibm://config".to_string(),
                name: "Repository Configuration (.clibm/config.json)".to_string(),
                description: Some("Repository-level settings, branch heads, and active workspace metadata".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            reader: Arc::new(|uri, bridge| {
                let content = bridge.read_repo_file(".clibm/config.json")?;
                Ok(McpResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(content),
                    blob: None,
                })
            }),
        });

        // 3. clibm://board/active
        self.register(ResourceDefinition {
            info: McpResourceInfo {
                uri: "clibm://board/active".to_string(),
                name: "Active Workspace Kanban Board".to_string(),
                description: Some("Current ASCII Kanban board visualization for active tasks and columns".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
            reader: Arc::new(|uri, bridge| {
                let content = bridge.execute(&["board", "show"])?;
                Ok(McpResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(content),
                    blob: None,
                })
            }),
        });
    }
}

