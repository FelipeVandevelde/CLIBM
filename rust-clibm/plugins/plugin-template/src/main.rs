//! CLIBM Official Plugin Starter Template (`clibm-plugin-template`)
//!
//! Shows how a standalone CLIBM plugin works:
//! 1. It is built and installed from its own folder (`./install.sh`, `.\install.ps1` or
//!    `clibm plugin install template`). The installer asks it for its registry entry with
//!    `clibm-plugin-template --manifest`.
//! 2. `clibm plugin run template` runs it inside the current project.
//! 3. CLIBM fires lifecycle hooks to it (`pre-commit`, `post-commit`, `pre-move`,
//!    `on-task-move`, `on-task-depend`, `on-workspace-switch`, `on-export`) when it is
//!    installed with those hooks. Each call sends a `CLIBM_PLUGIN_CONTRACT_V1` JSON
//!    request on stdin and expects `{"ok": bool, "output": "..."}` on stdout.
//!    Answering `ok: false` to a `pre-*` event blocks that action.
//!
//! Demo switches (handy for learning and used by the tests):
//! - `CLIBM_TEMPLATE_DENY=<event>[,<event>]` answers `ok: false` for those events.
//! - `CLIBM_TEMPLATE_LOG=<file>` appends one line per received event to `<file>`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "clibm-plugin-template";
const PLUGIN_DESCRIPTION: &str = "Official starter template for custom CLIBM plugins";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["manual-run"];

#[derive(Debug, Deserialize)]
struct PluginInvocationRequest {
    contract: String,
    event: String,
    tree_hash: String,
    #[serde(default)]
    args: Vec<String>,
    /// Event details (task, columns, commit...), present for lifecycle hooks.
    #[serde(default)]
    context: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct PluginInvocationResponse {
    ok: bool,
    output: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&args));
        return Ok(());
    }

    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let clibm_dir = project_root.join(".clibm");

    // CLIBM pipes a JSON request on stdin; when run by hand, stdin is the terminal.
    let mut stdin_buf = String::new();
    if !io::stdin().is_terminal() {
        let _ = io::stdin().read_to_string(&mut stdin_buf);
    }

    if let Ok(req) = serde_json::from_str::<PluginInvocationRequest>(stdin_buf.trim()) {
        log_event(&req);
        let denied = env::var("CLIBM_TEMPLATE_DENY")
            .map(|v| v.split(',').any(|e| e.trim() == req.event))
            .unwrap_or(false);
        let resp = if denied {
            PluginInvocationResponse {
                ok: false,
                output: format!(
                    "[{}] '{}' refused by the demo policy (CLIBM_TEMPLATE_DENY)",
                    PLUGIN_NAME, req.event
                ),
            }
        } else {
            PluginInvocationResponse {
                ok: true,
                output: inspect_project(&clibm_dir, Some(&req)),
            }
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

fn log_event(req: &PluginInvocationRequest) {
    if let Ok(path) = env::var("CLIBM_TEMPLATE_LOG") {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{} {}", req.event, req.context);
        }
    }
}

/// Number of object files in `.clibm/objects/<shard>/`.
fn count_objects(clibm_dir: &Path) -> usize {
    let Ok(shards) = fs::read_dir(clibm_dir.join("objects")) else {
        return 0;
    };
    shards
        .flatten()
        .filter(|s| s.path().is_dir())
        .filter_map(|s| fs::read_dir(s.path()).ok())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                .count()
        })
        .sum()
}

fn inspect_project(clibm_dir: &Path, req: Option<&PluginInvocationRequest>) -> String {
    let head_ref = fs::read_to_string(clibm_dir.join("HEAD"))
        .unwrap_or_else(|_| "uninitialized".to_string())
        .trim()
        .to_string();
    let object_count = count_objects(clibm_dir);

    match req {
        Some(r) => {
            let args_info = if r.args.is_empty() {
                String::new()
            } else {
                format!(" | Args={:?}", r.args)
            };
            format!(
                "[{}] Contract={} | Event={} | Tree={} | HEAD={} | CAS Objects={}{}",
                PLUGIN_NAME,
                r.contract,
                r.event,
                &r.tree_hash[..12.min(r.tree_hash.len())],
                head_ref,
                object_count,
                args_info
            )
        }
        None => format!(
            "[{}] Active Project: {} | HEAD: {} | CAS Objects: {}",
            PLUGIN_NAME,
            clibm_dir.display(),
            head_ref,
            object_count
        ),
    }
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
        "tools": serde_json::json!([]),
    })
    .to_string()
}
