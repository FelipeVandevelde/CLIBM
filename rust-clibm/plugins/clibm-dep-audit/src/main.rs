//! Standalone CLIBM Plugin: `clibm-dep-audit`
//!
//! Audits the task dependency graph of the current project state: circular chains,
//! dependencies on tasks that no longer exist, open tasks that block others, and the
//! longest chain of open dependencies (the critical path).
//!
//! `clibm plugin run dep-audit [-- --all]` (all projects) or the `on-task-depend` hook.

use anyhow::{bail, Result};
use clibm_core::index::detect_cycles;
use clibm_core::models::{KanbanBoardPayload, TaskPayload, OBJ_BOARD, OBJ_TASK};
use clibm_core::plugin_sdk::PluginRun;
use std::collections::{BTreeMap, BTreeSet};
use std::env;

const PLUGIN_NAME: &str = "clibm-dep-audit";
const PLUGIN_DESCRIPTION: &str = "Critical path and circular dependency analyzer for task graphs";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["on-task-depend"];

const USAGE: &str = "clibm-dep-audit - dependency audit of the current CLIBM project

USAGE:
  clibm plugin run dep-audit [-- --all]
  clibm-dep-audit [--all]

OPTIONS:
  --all        Audit every project, not only the active one
  --manifest   Print this plugin's registry entry (used by installers)";

struct Audit {
    scope_label: String,
    tree_hash: String,
    scope: Vec<TaskPayload>,
    all: BTreeMap<String, TaskPayload>,
    boards: BTreeMap<String, KanbanBoardPayload>,
}

impl Audit {
    fn load(run: &PluginRun, all_projects: bool) -> Result<Self> {
        let engine = run.engine();
        if !engine.cas.is_initialized() {
            bail!("no CLIBM project found in {}", run.project_root.display());
        }
        let (tree_hash, tree) = engine.load_working_tree()?;
        let project = engine.active_project(&tree)?;
        let mut all = BTreeMap::new();
        for (id, hash) in &tree.tasks {
            let task: TaskPayload = engine.cas.get_typed_payload(hash, OBJ_TASK)?;
            all.insert(id.clone(), task);
        }
        let mut boards = BTreeMap::new();
        for (id, hash) in &tree.boards {
            let board: KanbanBoardPayload = engine.cas.get_typed_payload(hash, OBJ_BOARD)?;
            boards.insert(id.clone(), board);
        }
        let scope: Vec<TaskPayload> = all
            .values()
            .filter(|t| all_projects || t.project_id == project.id)
            .cloned()
            .collect();
        Ok(Self {
            scope_label: if all_projects {
                "all projects".to_string()
            } else {
                format!("{} ({})", project.name, project.code)
            },
            tree_hash,
            scope,
            all,
            boards,
        })
    }

    fn is_done(&self, task: &TaskPayload) -> bool {
        self.boards
            .get(&task.board_id)
            .and_then(|b| b.columns.iter().find(|c| c.id == task.column_id))
            .map(|c| c.is_completed_state)
            .unwrap_or(false)
            || task.column_id == "done"
    }

    /// Longest chain of open tasks following `depends_on` (memoized DFS; cycles are
    /// reported separately and cut here).
    fn longest_open_chain(&self) -> Vec<String> {
        fn walk(
            id: &str,
            audit: &Audit,
            memo: &mut BTreeMap<String, Vec<String>>,
            visiting: &mut BTreeSet<String>,
        ) -> Vec<String> {
            if let Some(found) = memo.get(id) {
                return found.clone();
            }
            if !visiting.insert(id.to_string()) {
                return Vec::new();
            }
            let mut best: Vec<String> = Vec::new();
            if let Some(task) = audit.all.get(id) {
                for dep in &task.depends_on {
                    if let Some(dep_task) = audit.all.get(dep) {
                        if !audit.is_done(dep_task) {
                            let chain = walk(dep, audit, memo, visiting);
                            if chain.len() > best.len() {
                                best = chain;
                            }
                        }
                    }
                }
            }
            visiting.remove(id);
            best.push(id.to_string());
            memo.insert(id.to_string(), best.clone());
            best
        }
        let mut memo = BTreeMap::new();
        let mut longest = Vec::new();
        for task in self.scope.iter().filter(|t| !self.is_done(t)) {
            let chain = walk(&task.id, self, &mut memo, &mut BTreeSet::new());
            if chain.len() > longest.len() {
                longest = chain;
            }
        }
        longest
    }

    fn report(&self) -> (bool, String) {
        let mut out = format!(
            "[{}] Dependency audit of {}: {} task(s), tree {}\n",
            PLUGIN_NAME,
            self.scope_label,
            self.scope.len(),
            &self.tree_hash[..12.min(self.tree_hash.len())]
        );

        let cycles = detect_cycles(&self.all);
        if cycles.is_empty() {
            out.push_str("  Cycles: none\n");
        } else {
            out.push_str(&format!("  ALERT: {} circular dependency chain(s):\n", cycles.len()));
            for c in &cycles {
                out.push_str(&format!("    {}\n", c.join(" -> ")));
            }
        }

        let mut missing = Vec::new();
        for task in &self.scope {
            for dep in &task.depends_on {
                if !self.all.contains_key(dep) {
                    missing.push(format!("{} (needed by {})", dep, task.id));
                }
            }
        }
        if !missing.is_empty() {
            out.push_str(&format!("  Missing dependencies ({}):\n", missing.len()));
            for m in &missing {
                out.push_str(&format!("    {}\n", m));
            }
        }

        let mut blockers: Vec<(&TaskPayload, Vec<&str>)> = Vec::new();
        for task in self.scope.iter().filter(|t| !self.is_done(t)) {
            let waiting: Vec<&str> = self
                .all
                .values()
                .filter(|other| !self.is_done(other) && other.depends_on.contains(&task.id))
                .map(|other| other.id.as_str())
                .collect();
            if !waiting.is_empty() {
                blockers.push((task, waiting));
            }
        }
        blockers.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.id.cmp(&b.0.id)));
        if blockers.is_empty() {
            out.push_str("  Blockers: none (no open task is waiting on another)\n");
        } else {
            out.push_str("  Blockers (open tasks that others wait on):\n");
            for (task, waiting) in &blockers {
                out.push_str(&format!(
                    "    {} \"{}\" [{}] blocks {}: {}\n",
                    task.id,
                    task.title,
                    task.column_id,
                    waiting.len(),
                    waiting.join(", ")
                ));
            }
        }

        let chain = self.longest_open_chain();
        if chain.len() > 1 {
            out.push_str(&format!(
                "  Critical path ({} open tasks): {}\n",
                chain.len(),
                chain.join(" -> ")
            ));
        }

        (cycles.is_empty() && missing.is_empty(), out.trim_end().to_string())
    }
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
    // `audit` is the MCP execution_ref; it needs no handling.
    let (ok, output) = match Audit::load(&run, run.has_flag("--all")) {
        Ok(audit) => audit.report(),
        Err(e) => (false, format!("[{}] {}", PLUGIN_NAME, e)),
    };
    run.finish(ok, output);
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
            "name": "audit_dependencies",
            "description": "Audit the task dependency graph: cycles, missing dependencies, blockers and the critical path",
            "input_schema": { "type": "object", "properties": { "all": { "type": "boolean", "description": "Audit every project" } } },
            "is_mutating": false,
            "execution_ref": "audit"
        }]),
    })
    .to_string()
}
