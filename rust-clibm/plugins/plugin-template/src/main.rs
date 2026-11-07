//! CLIBM Official Plugin Starter Template (`clibm-plugin-template`)
//!
//! Demonstrates how to write a standalone CLIBM plugin that can be:
//! 1. Built and installed strictly from its own folder (`./install.sh` or `clibm plugin install .`)
//! 2. Executed inside any ongoing CLIBM project via `clibm plugin run <name>` or direct binary invocation
//! 3. Invoked by CLIBM lifecycle hooks (`pre-commit`, `post-commit`, `on-task-move`, `on-workspace-switch`)
//!    using the `CLIBM_PLUGIN_CONTRACT_V1` JSON stdin/stdout protocol.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct PluginInvocationRequest {
    pub contract: String,
    pub event: String,
    pub tree_hash: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PluginInvocationResponse {
    pub ok: bool,
    pub output: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let clibm_dir = project_root.join(".clibm");

    // Check if stdin contains a JSON contract request from `clibm plugin run`
    let mut stdin_buf = String::new();
    if !atty_is_terminal() {
        let _ = io::stdin().read_to_string(&mut stdin_buf);
    }

    if let Ok(req) = serde_json::from_str::<PluginInvocationRequest>(stdin_buf.trim()) {
        let summary = inspect_project(&clibm_dir, Some(&req));
        let resp = PluginInvocationResponse {
            ok: true,
            output: summary,
        };
        println!("{}", serde_json::to_string(&resp)?);
        return Ok(());
    }

    // Direct CLI execution inside an ongoing project
    println!("{}", inspect_project(&clibm_dir, None));
    if !args.is_empty() {
        println!("  Plugin arguments: {:?}", args);
    }

    Ok(())
}

fn inspect_project(clibm_dir: &PathBuf, req: Option<&PluginInvocationRequest>) -> String {
    let head_path = clibm_dir.join("HEAD");
    let head_ref = fs::read_to_string(&head_path)
        .unwrap_or_else(|_| "uninitialized".to_string())
        .trim()
        .to_string();

    let objects_dir = clibm_dir.join("objects");
    let object_count = fs::read_dir(&objects_dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    match req {
        Some(r) => format!(
            "[clibm-plugin-template] Contract={} | Event={} | Tree={} | HEAD={} | CAS Objects={}",
            r.contract,
            r.event,
            &r.tree_hash[..12.min(r.tree_hash.len())],
            head_ref,
            object_count
        ),
        None => format!(
            "[clibm-plugin-template] Active Project: {} | HEAD: {} | CAS Objects: {}",
            clibm_dir.display(),
            head_ref,
            object_count
        ),
    }
}

fn atty_is_terminal() -> bool {
    // Minimal check: if CLIBM_PLUGIN_CONTRACT_STDIN=1 is not set and stdin is interactive
    env::var("CLIBM_PLUGIN_CONTRACT_STDIN").is_err()
}
