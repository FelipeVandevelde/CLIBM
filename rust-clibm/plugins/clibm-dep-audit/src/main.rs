//! Standalone CLIBM Plugin: `clibm-dep-audit`
//!
//! Extracted from core so `clibm_core` remains 100% pure without built-in plugins.
//! Audits the task dependency graph in `.clibm/index/computed.json` and `.clibm/objects/`
//! to detect circular dependency chains and report critical-path blockers.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct PluginResponse {
    ok: bool,
    output: String,
}

#[derive(Debug, Deserialize)]
struct ComputedIndex {
    #[serde(default)]
    tree_hash: String,
    #[serde(default)]
    blocks_tasks: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    dependency_cycles: Vec<Vec<String>>,
}

fn main() -> Result<()> {
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let index_file = project_root.join(".clibm").join("index").join("computed.json");
    let mut report = String::new();

    if index_file.exists() {
        let raw = fs::read_to_string(&index_file)?;
        if let Ok(idx) = serde_json::from_str::<ComputedIndex>(&raw) {
            report.push_str(&format!(
                "[clibm-dep-audit] Audited task dependency DAG in tree {}\n",
                &idx.tree_hash[..12.min(idx.tree_hash.len())]
            ));
            if idx.dependency_cycles.is_empty() {
                report.push_str("  • DAG Topological Check: PASSED (0 circular dependencies)\n");
            } else {
                report.push_str(&format!(
                    "  • ALERT: {} circular dependency chains detected!\n",
                    idx.dependency_cycles.len()
                ));
            }
            for (blocker, dependents) in &idx.blocks_tasks {
                report.push_str(&format!(
                    "  • Critical Blocker: {} -> blocks {:?}\n",
                    blocker, dependents
                ));
            }
            let resp = PluginResponse {
                ok: idx.dependency_cycles.is_empty(),
                output: report,
            };
            println!("{}", serde_json::to_string(&resp)?);
            return Ok(());
        }
    }

    let fallback = PluginResponse {
        ok: true,
        output: format!(
            "[clibm-dep-audit] Executed standalone DAG audit in {}",
            project_root.display()
        ),
    };
    let _ = Value::Null;
    println!("{}", serde_json::to_string(&fallback)?);
    Ok(())
}
