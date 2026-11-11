//! Prompt registry and prompt template rendering for CLIBM MCP.

use crate::error::JsonRpcError;
use crate::protocol::{
    McpPromptArgument, McpPromptInfo, McpPromptMessage, McpPromptMessageContent,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub type PromptRenderer = Arc<dyn Fn(&Value) -> Result<Vec<McpPromptMessage>, JsonRpcError> + Send + Sync>;

pub struct PromptDefinition {
    pub info: McpPromptInfo,
    pub renderer: PromptRenderer,
}

pub struct PromptRegistry {
    prompts: HashMap<String, PromptDefinition>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            prompts: HashMap::new(),
        };
        registry.register_default_prompts();
        registry
    }

    pub fn register(&mut self, definition: PromptDefinition) {
        self.prompts.insert(definition.info.name.clone(), definition);
    }

    pub fn list_prompts(&self) -> Vec<McpPromptInfo> {
        let mut list: Vec<McpPromptInfo> = self.prompts.values().map(|p| p.info.clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn get_prompt(&self, name: &str, args: &Value) -> Result<Vec<McpPromptMessage>, JsonRpcError> {
        let prompt = self
            .prompts
            .get(name)
            .ok_or_else(|| JsonRpcError::prompt_not_found(name))?;

        (prompt.renderer)(args)
    }

    fn register_default_prompts(&mut self) {
        // 1. clibm_task_planning
        self.register(PromptDefinition {
            info: McpPromptInfo {
                name: "clibm_task_planning".to_string(),
                description: Some("Prompt template to analyze requirements and generate structured CLIBM tasks".to_string()),
                arguments: Some(vec![
                    McpPromptArgument {
                        name: "goal".to_string(),
                        description: Some("The high-level feature or objective to break down".to_string()),
                        required: Some(true),
                    },
                    McpPromptArgument {
                        name: "workspace".to_string(),
                        description: Some("Target workspace name or ID".to_string()),
                        required: Some(false),
                    },
                ]),
            },
            renderer: Arc::new(|args| {
                let goal = args
                    .get("goal")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Implement project milestone");
                let ws = args
                    .get("workspace")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                Ok(vec![
                    McpPromptMessage {
                        role: "user".to_string(),
                        content: McpPromptMessageContent {
                            content_type: "text".to_string(),
                            text: format!(
                                "You are a software architect working in CLIBM repository workspace '{}'.\n\
                                 Goal: {}\n\n\
                                 Please analyze the requirements, list the dependencies, and call `clibm_task_add` \
                                 to register the necessary actionable tasks with appropriate priority and column.",
                                ws, goal
                            ),
                        },
                    },
                ])
            }),
        });

        // 2. clibm_commit_summary
        self.register(PromptDefinition {
            info: McpPromptInfo {
                name: "clibm_commit_summary".to_string(),
                description: Some("Prompt template to summarize ongoing changes and create a deterministic BLAKE3 commit message".to_string()),
                arguments: Some(vec![
                    McpPromptArgument {
                        name: "scope".to_string(),
                        description: Some("Commit scope or module name".to_string()),
                        required: Some(false),
                    },
                ]),
            },
            renderer: Arc::new(|args| {
                let scope = args
                    .get("scope")
                    .and_then(|v| v.as_str())
                    .unwrap_or("workspace");

                Ok(vec![
                    McpPromptMessage {
                        role: "user".to_string(),
                        content: McpPromptMessageContent {
                            content_type: "text".to_string(),
                            text: format!(
                                "Please inspect the current CLIBM staged tree via `clibm://tree/staged` and active board via `clibm_status`.\n\
                                 Generate a concise, conventional commit message for scope '{}' following the pattern:\n\
                                 <type>({}): <summary>\n\n- Detailed task bullet points",
                                scope, scope
                            ),
                        },
                    },
                ])
            }),
        });
    }
}

