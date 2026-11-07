//! Standalone CLIBM Plugin: `clibm-export-md`
//!
//! Extracted from core so `clibm_core` remains 100% pure without built-in plugins.
//! Reads task objects from `.clibm/objects/` and outputs a deterministic Markdown report.

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
    let mut md = String::from("# CLIBM Project Snapshot Report\n\n");
    md.push_str("| ID | Title | Priority | Column | Assignee | Depends On |\n");
    md.push_str("|---|---|---|---|---|---|\n");

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
                                    let prio =
                                        p.get("priority").and_then(|x| x.as_str()).unwrap_or("-");
                                    let col =
                                        p.get("column_id").and_then(|x| x.as_str()).unwrap_or("-");
                                    let assignee = p
                                        .get("assignee")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("-");
                                    md.push_str(&format!(
                                        "| {} | {} | {} | {} | {} | - |\n",
                                        id, title, prio, col, assignee
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let resp = PluginResponse {
        ok: true,
        output: md,
    };
    println!("{}", serde_json::to_string(&resp)?);
    Ok(())
}
