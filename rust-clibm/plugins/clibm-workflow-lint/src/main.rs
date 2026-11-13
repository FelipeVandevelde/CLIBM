//! Standalone CLIBM Plugin: `clibm-workflow-lint`
//!
//! Checks the current project state against the Kanban workflow:
//! - violations (`ok: false`): a column over its WIP limit, a task in progress without
//!   the assignee its board requires, a completed task whose dependencies are still open;
//! - warnings: in-progress tasks without an assignee, overdue tasks, dependencies on
//!   tasks that no longer exist.
//!
//! Default hook `post-commit`: after every change it reports new violations (they can
//! only appear when someone used `--force`). Subscribed to `pre-move` it vetoes a move
//! that would break those rules, unless the move was forced.

use anyhow::{bail, Result};
use clibm_core::models::{KanbanBoardPayload, KanbanColumn, ProjectPayload, TaskPayload, OBJ_BOARD, OBJ_PROJECT, OBJ_TASK};
use clibm_core::plugin_sdk::PluginRun;
use std::collections::BTreeMap;
use std::env;

const PLUGIN_NAME: &str = "clibm-workflow-lint";
const PLUGIN_DESCRIPTION: &str = "Validates Kanban WIP limits, assignees, open dependencies and due dates";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["post-commit"];

const USAGE: &str = "clibm-workflow-lint - Kanban workflow checks for the current CLIBM project

USAGE:
  clibm plugin run workflow-lint [-- --all]
  clibm-workflow-lint [--all]

OPTIONS:
  --all        Check every project, not only the active one
  --manifest   Print this plugin's registry entry (used by installers)";

struct Project {
    tasks: BTreeMap<String, TaskPayload>,
    boards: BTreeMap<String, KanbanBoardPayload>,
    active: ProjectPayload,
}

impl Project {
    fn load(run: &PluginRun) -> Result<Self> {
        let engine = run.engine();
        if !engine.cas.is_initialized() {
            bail!("no CLIBM project found in {}", run.project_root.display());
        }
        let (_, tree) = engine.load_working_tree()?;
        let mut tasks = BTreeMap::new();
        for (id, hash) in &tree.tasks {
            tasks.insert(id.clone(), engine.cas.get_typed_payload::<TaskPayload>(hash, OBJ_TASK)?);
        }
        let mut boards = BTreeMap::new();
        for (id, hash) in &tree.boards {
            boards.insert(id.clone(), engine.cas.get_typed_payload::<KanbanBoardPayload>(hash, OBJ_BOARD)?);
        }
        let active_hash = tree
            .projects
            .get(&tree.active_project_id)
            .ok_or_else(|| anyhow::anyhow!("active project not found"))?;
        let active: ProjectPayload = engine.cas.get_typed_payload(active_hash, OBJ_PROJECT)?;
        Ok(Self { tasks, boards, active })
    }

    fn column<'a>(&'a self, task: &TaskPayload) -> Option<&'a KanbanColumn> {
        self.boards
            .get(&task.board_id)?
            .columns
            .iter()
            .find(|c| c.id == task.column_id)
    }

    fn is_done(&self, task: &TaskPayload) -> bool {
        self.column(task).map(|c| c.is_completed_state).unwrap_or(false) || task.column_id == "done"
    }

    fn open_dependencies(&self, task: &TaskPayload) -> Vec<String> {
        task.depends_on
            .iter()
            .filter(|d| self.tasks.get(*d).map(|t| !self.is_done(t)).unwrap_or(false))
            .cloned()
            .collect()
    }

    /// Full lint: (violations, warnings).
    fn lint(&self, all_projects: bool) -> (Vec<String>, Vec<String>) {
        let today = chrono::Utc::now().date_naive();
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let in_scope = |t: &TaskPayload| all_projects || t.project_id == self.active.id;

        for board in self.boards.values() {
            for col in &board.columns {
                let count = self
                    .tasks
                    .values()
                    .filter(|t| in_scope(t) && t.board_id == board.id && t.column_id == col.id)
                    .count();
                if let Some(limit) = col.wip_limit {
                    if count > limit {
                        violations.push(format!(
                            "Column '{}' on {} holds {} tasks (WIP limit {})",
                            col.id, board.id, count, limit
                        ));
                    }
                }
            }
        }

        for task in self.tasks.values().filter(|t| in_scope(t)) {
            let required = self
                .boards
                .get(&task.board_id)
                .map(|b| b.workflow.require_assignee_in_progress)
                .unwrap_or(false);
            let unassigned = task.assignee.as_deref().map(str::trim).unwrap_or("").is_empty();
            if task.column_id == "in-progress" && unassigned {
                if required {
                    violations.push(format!("{} is in progress without an assignee (required by its board)", task.id));
                } else {
                    warnings.push(format!("{} is in progress without an assignee", task.id));
                }
            }
            if self.is_done(task) {
                let open = self.open_dependencies(task);
                if !open.is_empty() {
                    violations.push(format!("{} is done but depends on open task(s) {}", task.id, open.join(", ")));
                }
            } else if let Some(due) = task.due_date.as_deref() {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(due.get(..10).unwrap_or(due), "%Y-%m-%d") {
                    if date < today {
                        warnings.push(format!("{} is overdue (due {})", task.id, date));
                    }
                }
            }
            for dep in &task.depends_on {
                if !self.tasks.contains_key(dep) {
                    warnings.push(format!("{} depends on {}, which does not exist", task.id, dep));
                }
            }
        }
        (violations, warnings)
    }

    /// Checks one proposed move (`pre-move` hook context). Returns the reasons to refuse it.
    fn check_move(&self, task_id: &str, to: &str) -> Vec<String> {
        let Some(task) = self.tasks.get(task_id) else {
            return Vec::new();
        };
        let Some(board) = self.boards.get(&task.board_id) else {
            return Vec::new();
        };
        let Some(target) = board.columns.iter().find(|c| c.id == to) else {
            return Vec::new();
        };
        let mut reasons = Vec::new();
        if let Some(limit) = target.wip_limit {
            let count = self
                .tasks
                .values()
                .filter(|t| t.id != task.id && t.board_id == board.id && t.column_id == target.id)
                .count();
            if count >= limit {
                reasons.push(format!("column '{}' is at its WIP limit ({}/{})", target.id, count, limit));
            }
        }
        if target.id == "in-progress"
            && board.workflow.require_assignee_in_progress
            && task.assignee.as_deref().map(str::trim).unwrap_or("").is_empty()
        {
            reasons.push(format!("{} needs an assignee before it can be in progress", task.id));
        }
        if target.is_completed_state {
            let open = self.open_dependencies(task);
            if !open.is_empty() {
                reasons.push(format!("{} still depends on open task(s) {}", task.id, open.join(", ")));
            }
        }
        reasons
    }
}

fn run_lint(run: &PluginRun) -> Result<(bool, String)> {
    let project = Project::load(run)?;
    let ctx = run.context();

    if run.event() == "pre-move" {
        if ctx.get("force").and_then(|f| f.as_bool()) == Some(true) {
            return Ok((true, String::new()));
        }
        let task_id = ctx.get("task_id").and_then(|v| v.as_str()).unwrap_or_default();
        let to = ctx.get("to").and_then(|v| v.as_str()).unwrap_or_default();
        let reasons = project.check_move(task_id, to);
        return Ok(if reasons.is_empty() {
            (true, String::new())
        } else {
            (false, format!("workflow policy: {}", reasons.join("; ")))
        });
    }

    let all = run.has_flag("--all");
    let (violations, warnings) = project.lint(all);
    let scope = if all {
        "all projects".to_string()
    } else {
        format!("{} ({})", project.active.name, project.active.code)
    };
    let mut out = String::new();
    if violations.is_empty() && warnings.is_empty() {
        out.push_str(&format!("[{}] {}: all workflow rules are satisfied.", PLUGIN_NAME, scope));
    } else {
        out.push_str(&format!(
            "[{}] {}: {} violation(s), {} warning(s)",
            PLUGIN_NAME,
            scope,
            violations.len(),
            warnings.len()
        ));
        for v in &violations {
            out.push_str(&format!("\n  VIOLATION: {}", v));
        }
        for w in &warnings {
            out.push_str(&format!("\n  warning: {}", w));
        }
    }
    Ok((violations.is_empty(), out))
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
    let (ok, output) = match run_lint(&run) {
        Ok(r) => r,
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
            "name": "lint_workflow",
            "description": "Check Kanban WIP limits, required assignees, open dependencies and overdue tasks",
            "input_schema": { "type": "object", "properties": { "all": { "type": "boolean", "description": "Check every project" } } },
            "is_mutating": false,
            "execution_ref": "lint"
        }]),
    })
    .to_string()
}
