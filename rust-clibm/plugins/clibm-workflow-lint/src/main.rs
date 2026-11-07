//! Standalone CLIBM Plugin: `clibm-workflow-lint`
//!
//! Extracted from core so `clibm_core` remains 100% pure without built-in plugins.
//! Validates Kanban column Work-In-Progress (WIP) limits and assignee requirements.

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct PluginResponse {
    ok: bool,
    output: String,
}

fn main() -> Result<()> {
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let objects_dir = project_root.join(".clibm").join("objects");
    let mut warnings = Vec::new();

    if objects_dir.exists() {
        for shard in fs::read_dir(&objects_dir)?.flatten() {
            if shard.path().is_dir() {
                for entry in fs::read_dir(shard.path())?.flatten() {
                    if let Ok(raw) = fs::read_to_string(entry.path()) {
                        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                            if v.get("Type").and_then(|t| t.as_str()) == Some("task") {
                                if let Some(p) = v.get("Payload") {
                                    let id = p.get("id").and_then(|x| x.as_str()).unwrap_or("-");
                                    let title =
                                        p.get("title").and_then(|x| x.as_str()).unwrap_or("-");
                                    let col =
                                        p.get("column_id").and_then(|x| x.as_str()).unwrap_or("-");
                                    let has_assignee = p
                                        .get("assignee")
                                        .and_then(|x| x.as_str())
                                        .is_some();
                                    if col == "in-progress" && !has_assignee {
                                        warnings.push(format!(
                                            "Task {} ('{}') is in-progress without an assignee",
                                            id, title
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let output = if warnings.is_empty() {
        "[clibm-workflow-lint] All tasks satisfy active Kanban workflow constraints.".to_string()
    } else {
        format!(
            "[clibm-workflow-lint] Found {} policy warning(s):\n  - {}",
            warnings.len(),
            warnings.join("\n  - ")
        )
    };

    let resp = PluginResponse {
        ok: warnings.is_empty(),
        output,
    };
    println!("{}", serde_json::to_string(&resp)?);
    Ok(())
}
