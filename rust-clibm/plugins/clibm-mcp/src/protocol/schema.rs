//! MCP Schema Definitions for Tools, Resources, Prompts, and Capabilities (2024-11-05).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceInfo {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptInfo {
    pub name: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessageContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    pub role: String,
    pub content: McpPromptMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolDeclaration {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(
        rename = "input_schema",
        alias = "inputSchema",
        default = "default_tool_input_schema"
    )]
    pub input_schema: Value,
    #[serde(rename = "is_mutating", alias = "isMutating", default)]
    pub is_mutating: bool,
    #[serde(
        rename = "execution_ref",
        alias = "executionRef",
        alias = "handler",
        default
    )]
    pub execution_ref: Option<String>,
}

fn default_tool_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMcpSection {
    #[serde(default)]
    pub tools: Vec<PluginToolDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistryEntry {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_entry_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub tools: Vec<PluginToolDeclaration>,
    #[serde(default)]
    pub mcp: Option<PluginMcpSection>,
}

fn default_entry_enabled() -> bool {
    true
}


