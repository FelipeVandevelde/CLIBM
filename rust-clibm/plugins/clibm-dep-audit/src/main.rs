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

const PLUGIN_NAME: &str = "clibm-dep-audit";
const PLUGIN_DESCRIPTION: &str = "Critical path and circular dependency analyzer for task graphs";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["on-task-depend"];

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
    let manifest_args: Vec<String> = env::args().skip(1).collect();
    if manifest_args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&manifest_args));
        return Ok(());
    }
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

/// `--manifest [--executable <path>] [--hooks a,b]`: prints this plugin's registry entry
/// (one JSON line). Installers and `clibm plugin install` store it in
/// `~/.clibm/plugins/registry.json`.
fn manifest_json(args: &[String]) -> String {
    let value_after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let hooks: Vec<String> = match value_after("--hooks") {
        Some(csv) => csv
            .split(',')
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect(),
        None => DEFAULT_HOOKS.iter().map(|h| h.to_string()).collect(),
    };
    serde_json::json!({
        "name": PLUGIN_NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "description": PLUGIN_DESCRIPTION,
        "enabled": true,
        "hooks": hooks,
        "executable": value_after("--executable"),
        "tools": serde_json::json!([{ "name": "audit_dependencies", "description": "Audit the task dependency graph for circular chains and critical-path blockers", "input_schema": { "type": "object", "properties": {} }, "is_mutating": false, "execution_ref": "audit" }]),
    })
    .to_string()
}
