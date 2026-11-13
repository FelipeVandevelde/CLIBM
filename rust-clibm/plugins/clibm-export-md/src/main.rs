//! Standalone CLIBM Plugin: `clibm-export-md`
//!
//! Writes a Markdown report of the current project state (not of every historical
//! object): one table per board, in column order.
//!
//! - `clibm plugin run export-md [-- --all] [-- --output report.md]`
//! - `on-export` hook: `clibm export -o bundle.json` also writes `bundle.md`.

use anyhow::{bail, Result};
use clibm_core::models::{KanbanBoardPayload, ProjectPayload, TaskPayload, OBJ_BOARD, OBJ_PROJECT, OBJ_TASK};
use clibm_core::plugin_sdk::PluginRun;
use std::env;
use std::path::PathBuf;

const PLUGIN_NAME: &str = "clibm-export-md";
const PLUGIN_DESCRIPTION: &str = "Generates portable Markdown sprint reports from the current project tree";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["on-export"];

const USAGE: &str = "clibm-export-md - Markdown report of the current CLIBM project

USAGE:
  clibm plugin run export-md [-- --all] [-- --output <file.md>]
  clibm-export-md [--all] [--output <file.md>]

OPTIONS:
  --all              Report every project, not only the active one
  --output <file>    Write the report to a file instead of printing it
  --manifest         Print this plugin's registry entry (used by installers)";

/// Makes a value safe inside a Markdown table cell.
fn cell(s: &str) -> String {
    let flat = s.replace(['\r', '\n'], " ").replace('|', "\\|");
    if flat.trim().is_empty() {
        "-".to_string()
    } else {
        flat.trim().to_string()
    }
}

fn build_report(run: &PluginRun, all_projects: bool) -> Result<String> {
    let engine = run.engine();
    if !engine.cas.is_initialized() {
        bail!("no CLIBM project found in {}", run.project_root.display());
    }
    let (tree_hash, tree) = engine.load_working_tree()?;
    let active = engine.active_project(&tree)?;
    let branch = engine
        .cas
        .current_branch()?
        .unwrap_or_else(|| "detached HEAD".to_string());

    let mut projects: Vec<ProjectPayload> = Vec::new();
    for hash in tree.projects.values() {
        let p: ProjectPayload = engine.cas.get_typed_payload(hash, OBJ_PROJECT)?;
        if all_projects || p.id == active.id {
            projects.push(p);
        }
    }
    let mut tasks: Vec<TaskPayload> = Vec::new();
    for hash in tree.tasks.values() {
        tasks.push(engine.cas.get_typed_payload(hash, OBJ_TASK)?);
    }

    let mut md = String::new();
    md.push_str(&format!(
        "# CLIBM task report - {}\n\nBranch `{}`, tree `{}`, generated {}.\n",
        if all_projects {
            "all projects".to_string()
        } else {
            format!("{} ({})", active.name, active.code)
        },
        branch,
        &tree_hash[..12.min(tree_hash.len())],
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));

    for project in &projects {
        for board_id in &project.board_ids {
            let Some(board_hash) = tree.boards.get(board_id) else {
                continue;
            };
            let board: KanbanBoardPayload = engine.cas.get_typed_payload(board_hash, OBJ_BOARD)?;
            let board_tasks: Vec<&TaskPayload> = tasks
                .iter()
                .filter(|t| t.project_id == project.id && t.board_id == board.id)
                .collect();
            md.push_str(&format!("\n## {} ({}) - {}\n\n", project.name, project.code, board.name));
            let summary: Vec<String> = board
                .columns
                .iter()
                .map(|c| {
                    let n = board_tasks.iter().filter(|t| t.column_id == c.id).count();
                    format!("{}: {}", c.name, n)
                })
                .collect();
            md.push_str(&format!("{}\n\n", summary.join(" · ")));
            md.push_str("| ID | Title | Priority | Column | Assignee | Due | Depends on | Tags |\n");
            md.push_str("|---|---|---|---|---|---|---|---|\n");
            for col in &board.columns {
                for t in board_tasks.iter().filter(|t| t.column_id == col.id) {
                    md.push_str(&format!(
                        "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                        cell(&t.id),
                        cell(&t.title),
                        t.priority.as_str(),
                        cell(&col.name),
                        cell(t.assignee.as_deref().unwrap_or("")),
                        cell(t.due_date.as_deref().unwrap_or("")),
                        cell(&t.depends_on.join(", ")),
                        cell(&t.tags.join(", "))
                    ));
                }
            }
            if board_tasks.is_empty() {
                md.push_str("| - | (no tasks) | - | - | - | - | - | - |\n");
            }
        }
    }
    Ok(md)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&args));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", USAGE);
        return;
    }
    let run = PluginRun::from_env();

    // Where to write: --output, or next to the bundle of `clibm export` (on-export hook).
    let target: Option<PathBuf> = run.value_of("--output").map(PathBuf::from).or_else(|| {
        if run.event() == "on-export" {
            run.context()
                .get("output")
                .and_then(|o| o.as_str())
                .map(|o| PathBuf::from(o).with_extension("md"))
        } else {
            None
        }
    });

    let report = match build_report(&run, run.has_flag("--all")) {
        Ok(r) => r,
        Err(e) => run.finish(false, format!("[{}] {}", PLUGIN_NAME, e)),
    };
    match target {
        Some(path) => match std::fs::write(&path, &report) {
            Ok(()) => run.finish(true, format!("[{}] Wrote report to {}", PLUGIN_NAME, path.display())),
            Err(e) => run.finish(false, format!("[{}] Cannot write {}: {}", PLUGIN_NAME, path.display(), e)),
        },
        None => run.finish(true, report),
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
        "tools": serde_json::json!([{
            "name": "export_project",
            "description": "Markdown report of the current project's tasks (one table per board, in column order)",
            "input_schema": { "type": "object", "properties": { "all": { "type": "boolean", "description": "Report every project" } } },
            "is_mutating": false,
            "execution_ref": "export"
        }]),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::cell;

    #[test]
    fn table_cells_are_escaped() {
        assert_eq!(cell("a|b\nc"), "a\\|b c");
        assert_eq!(cell("  "), "-");
    }
}
