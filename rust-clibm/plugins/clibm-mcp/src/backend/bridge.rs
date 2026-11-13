//! Bridge to the local CLIBM CLI and repository.

use crate::error::JsonRpcError;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ClibmBridge {
    project_root: PathBuf,
}

/// The `clibm` binary installed next to this `clibm-mcp` (AI clients often start MCP
/// servers with a minimal PATH), else `clibm` from PATH.
fn clibm_binary() -> PathBuf {
    let name = if cfg!(windows) { "clibm.exe" } else { "clibm" };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("clibm"))
}

impl ClibmBridge {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Runs `clibm <args>` in the project. A failing command becomes a tool execution
    /// error carrying CLIBM's own message.
    pub fn execute(&self, args: &[&str]) -> Result<String, JsonRpcError> {
        let output = Command::new(clibm_binary())
            .args(args)
            .current_dir(&self.project_root)
            .env("CLIBM_PROJECT_ROOT", &self.project_root)
            .output()
            .map_err(|e| {
                JsonRpcError::tool_execution_error(format!(
                    "Failed to run 'clibm {}' (is CLIBM installed?): {}",
                    args.join(" "),
                    e
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim_end().to_string();

        if output.status.success() {
            if stdout.is_empty() {
                Ok(format!("'clibm {}' succeeded with no output", args.join(" ")))
            } else {
                Ok(stdout)
            }
        } else {
            let message = match (stdout.is_empty(), stderr.is_empty()) {
                (true, false) => stderr,
                (false, false) => format!("{}\n{}", stdout, stderr),
                (false, true) => stdout,
                (true, true) => format!("'clibm {}' failed with exit code {:?}", args.join(" "), output.status.code()),
            };
            Err(JsonRpcError::tool_execution_error(message))
        }
    }

    /// Reads a file inside the project. Absolute paths and `..` are refused, and the
    /// resolved path must stay inside the project folder.
    pub fn read_repo_file(&self, relative_path: &str) -> Result<String, JsonRpcError> {
        let rel = Path::new(relative_path);
        let escapes = rel.components().any(|c| {
            matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        });
        if escapes || rel.is_absolute() {
            return Err(JsonRpcError::permission_denied(
                "Only paths inside the project are allowed",
            ));
        }
        let full_path = self.project_root.join(rel);
        if let (Ok(root), Ok(resolved)) = (self.project_root.canonicalize(), full_path.canonicalize()) {
            if !resolved.starts_with(&root) {
                return Err(JsonRpcError::permission_denied(
                    "Only paths inside the project are allowed",
                ));
            }
        }
        std::fs::read_to_string(&full_path)
            .map_err(|e| JsonRpcError::resource_not_found(format!("{}: {}", relative_path, e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_outside_the_project_are_refused() {
        let bridge = ClibmBridge::new(std::env::temp_dir());
        for bad in ["../secret.txt", "a/../../b", "/etc/passwd", "C:\\Windows\\win.ini", "\\\\server\\share\\x"] {
            let err = bridge.read_repo_file(bad).unwrap_err();
            assert_eq!(err.code, -32003, "{}", bad);
        }
        // A normal missing file is "not found", not "denied".
        assert_eq!(bridge.read_repo_file("definitely-missing.json").unwrap_err().code, -32002);
    }
}
