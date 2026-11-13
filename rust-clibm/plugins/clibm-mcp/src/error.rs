//! MCP and JSON-RPC 2.0 Error representations and codes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Code used when a tool ran but failed; reported to clients as an `isError` result.
pub const TOOL_EXECUTION_ERROR: i64 = -32000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_data(code: i64, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn parse_error(detail: impl Into<String>) -> Self {
        Self::new(-32700, format!("Parse error: {}", detail.into()))
    }

    pub fn invalid_request(detail: impl Into<String>) -> Self {
        Self::new(-32600, format!("Invalid Request: {}", detail.into()))
    }

    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(-32601, format!("Method not found: '{}'", method.into()))
    }

    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self::new(-32602, format!("Invalid params: {}", detail.into()))
    }

    #[allow(dead_code)]
    pub fn internal_error(detail: impl Into<String>) -> Self {
        Self::new(-32603, format!("Internal error: {}", detail.into()))
    }

    pub fn permission_denied(detail: impl Into<String>) -> Self {
        Self::new(-32003, format!("Permission denied: {}", detail.into()))
    }

    pub fn tool_execution_error(detail: impl Into<String>) -> Self {
        Self::new(TOOL_EXECUTION_ERROR, detail.into())
    }

    pub fn resource_not_found(uri: impl Into<String>) -> Self {
        Self::new(-32002, format!("Resource not found: '{}'", uri.into()))
    }

    pub fn prompt_not_found(name: impl Into<String>) -> Self {
        Self::new(-32004, format!("Prompt not found: '{}'", name.into()))
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error [{}]: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}
