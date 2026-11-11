//! Permission management for MCP tools, resources, and execution.

use crate::error::JsonRpcError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPolicy {
    FullAccess,
    ReadOnly,
}

#[derive(Debug, Clone)]
pub struct PermissionManager {
    policy: PermissionPolicy,
    #[allow(dead_code)]
    allowed_root: PathBuf,
}

impl PermissionManager {
    pub fn new(allowed_root: PathBuf, read_only: bool) -> Self {
        Self {
            policy: if read_only {
                PermissionPolicy::ReadOnly
            } else {
                PermissionPolicy::FullAccess
            },
            allowed_root,
        }
    }

    pub fn check_tool(&self, tool_name: &str, is_mutating: bool) -> Result<(), JsonRpcError> {
        if self.policy == PermissionPolicy::ReadOnly && is_mutating {
            return Err(JsonRpcError::permission_denied(format!(
                "Tool '{}' performs mutations and cannot be executed in read-only mode",
                tool_name
            )));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn check_path(&self, target: &Path) -> Result<(), JsonRpcError> {
        if !target.starts_with(&self.allowed_root) && target != self.allowed_root {
            return Err(JsonRpcError::permission_denied(
                "Access outside repository boundary is denied",
            ));
        }
        Ok(())
    }
}
