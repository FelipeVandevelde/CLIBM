//! Bridge to the local CLIBM CLI and repository.

use crate::error::JsonRpcError;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ClibmBridge {
    project_root: PathBuf,
}

impl ClibmBridge {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    #[allow(dead_code)]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn execute(&self, args: &[&str]) -> Result<String, JsonRpcError> {
        let output = Command::new("clibm")
            .args(args)
            .current_dir(&self.project_root)
            .output()
            .map_err(|e| {
                JsonRpcError::tool_execution_error(format!(
                    "Failed to invoke 'clibm {}': {}",
                    args.join(" "),
                    e
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if !stdout.trim().is_empty() {
                Ok(stdout)
            } else {
                Ok(format!(
                    "Command 'clibm {}' succeeded with empty output",
                    args.join(" ")
                ))
            }
        } else {
            let err_msg = if !stderr.trim().is_empty() {
                stderr
            } else if !stdout.trim().is_empty() {
                stdout
            } else {
                format!("Process exited with status code {:?}", output.status.code())
            };
            Err(JsonRpcError::tool_execution_error(err_msg))
        }
    }

    pub fn read_repo_file(&self, relative_path: &str) -> Result<String, JsonRpcError> {
        let clean_path = relative_path
            .trim_start_matches('/')
            .trim_start_matches('\\');
        let full_path = self.project_root.join(clean_path);

        if !full_path.starts_with(&self.project_root) && full_path != self.project_root {
            return Err(JsonRpcError::permission_denied(
                "Path traversal outside repository is forbidden",
            ));
        }

        std::fs::read_to_string(&full_path)
            .map_err(|e| JsonRpcError::resource_not_found(format!("{}: {}", relative_path, e)))
    }
}
