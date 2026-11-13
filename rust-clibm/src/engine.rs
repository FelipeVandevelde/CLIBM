//! Headless Core Engine (Architectural Principles #1 & #2)
//!
//! Coordinates Workspaces, Projects, Tasks, Kanban Boards, Versioning, Configuration,
//! Automatic Per-Modification Commits, and Cross-Workspace Task Coordination.

use crate::cas::{CasObject, CasStore};
use crate::index::ComputedIndex;
use crate::models::*;
use crate::plugin::PluginManager;
use crate::versioning::{MergeOutcome, MergeStrategy, VersioningEngine};
use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableBundle {
    pub format: String,
    pub version: String,
    pub exported_at: String,
    pub head_branch: Option<String>,
    pub branches: Vec<(String, String)>,
    pub staged_tree_hash: Option<String>,
    pub objects: Vec<CasObject>,
}

/// Per-column task count reported by `clibm status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatus {
    pub id: String,
    pub name: String,
    pub tasks: usize,
    pub wip_limit: Option<usize>,
}

/// Snapshot of the repository returned by [`ClibmEngine::status`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub repository: String,
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub head_tree: Option<String>,
    pub staged_tree: String,
    /// `true` when the staged tree differs from the tree of the HEAD commit.
    pub dirty: bool,
    pub auto_commit: bool,
    pub active_workspace_id: String,
    pub active_workspace_name: String,
    pub active_project_id: String,
    pub active_project_code: String,
    pub active_project_name: String,
    pub board_id: Option<String>,
    pub board_name: Option<String>,
    pub columns: Vec<ColumnStatus>,
    pub project_tasks: usize,
    pub total_tasks: usize,
    pub workspaces: usize,
    pub branches: usize,
    pub objects: usize,
}

/// Field changes applied by [`ClibmEngine::edit_task`]. `None` leaves a field untouched;
/// `Some(None)` clears an optional field (assignee, due date).
#[derive(Debug, Clone, Default)]
pub struct TaskEdit {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<TaskPriority>,
    pub assignee: Option<Option<String>>,
    pub due_date: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

/// Result of a mutation on an existing task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    /// Resolved task ID (e.g. `CLIBM-101`).
    pub id: String,
    /// Hash of the task object after the mutation.
    pub hash: String,
    /// Auto-commit created for the mutation, if any.
    pub commit: Option<String>,
    /// `false` when the request was a no-op (nothing written, no commit).
    pub changed: bool,
}

/// Result of [`ClibmEngine::delete_task`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    pub id: String,
    /// Tasks whose `depends_on` lost a reference to the deleted task (`--force`).
    pub unlinked_dependents: Vec<String>,
    pub commit: Option<String>,
}

fn clean_tags(tags: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim().to_string();
        if !t.is_empty() && !out.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

pub struct ClibmEngine {
    pub cas: CasStore,
    /// Whether lifecycle hooks are fired to plugins (see [`ClibmEngine::without_hooks`]).
    hooks: bool,
}

impl ClibmEngine {
    /// Opens the repository in `workspace_root` or in one of its parent directories.
    pub fn open(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            cas: CasStore::new(workspace_root),
            hooks: true,
        }
    }

    /// Targets `<workspace_root>/.clibm` exactly (no parent-directory discovery).
    pub fn open_exact(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            cas: CasStore::at_root(workspace_root),
            hooks: true,
        }
    }

    /// Disables plugin lifecycle hooks for this engine (`clibm --no-hooks`).
    pub fn without_hooks(mut self) -> Self {
        self.hooks = false;
        self
    }

    /// Fires a lifecycle hook to subscribed plugins. `pre-*` hooks may return an error
    /// that blocks the action; see [`PluginManager::fire_hook`].
    pub fn fire_hook(&self, event: &str, tree_hash: &str, context: serde_json::Value) -> Result<()> {
        if !self.hooks {
            return Ok(());
        }
        let timeout = self
            .cas
            .load_config()
            .map(|c| c.hook_timeout_secs)
            .unwrap_or(10)
            .max(1);
        PluginManager::fire_hook(&self.cas, event, tree_hash, context, Duration::from_secs(timeout))
    }

    fn default_columns() -> Vec<KanbanColumn> {
        vec![
            KanbanColumn {
                id: "backlog".to_string(),
                name: "Backlog".to_string(),
                order: 0,
                wip_limit: None,
                is_completed_state: false,
            },
            KanbanColumn {
                id: "todo".to_string(),
                name: "To Do".to_string(),
                order: 1,
                wip_limit: Some(8),
                is_completed_state: false,
            },
            KanbanColumn {
                id: "in-progress".to_string(),
                name: "In Progress".to_string(),
                order: 2,
                wip_limit: Some(4),
                is_completed_state: false,
            },
            KanbanColumn {
                id: "review".to_string(),
                name: "Review".to_string(),
                order: 3,
                wip_limit: Some(3),
                is_completed_state: false,
            },
            KanbanColumn {
                id: "done".to_string(),
                name: "Done".to_string(),
                order: 4,
                wip_limit: None,
                is_completed_state: true,
            },
        ]
    }

    /// Initializes a new `.clibm` repository with a default workspace, project, Kanban board,
    /// and `.clibm/config.json` (`auto_commit = true` by default).
    ///
    /// Refuses to run when the repository already exists, so existing tasks are never
    /// replaced by an empty tree.
    pub fn init(&self, workspace_name: &str, project_code: &str, project_name: &str) -> Result<String> {
        if self.cas.is_initialized() {
            bail!(
                "A CLIBM repository already exists at {}. Nothing was changed.",
                self.cas.clibm_dir().display()
            );
        }
        let code = project_code.trim().to_uppercase();
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            bail!(
                "Invalid project code '{}': use letters, digits or '_' (e.g. APP)",
                project_code
            );
        }
        self.cas.init()?;

        let now = Utc::now().to_rfc3339();
        let ws_id = "WS-01".to_string();
        let prj_id = "PRJ-01".to_string();
        let brd_id = "BRD-01".to_string();

        let workspace = WorkspacePayload {
            id: ws_id.clone(),
            name: workspace_name.to_string(),
            description: "Default local-first CLIBM workspace".to_string(),
            created_at: now.clone(),
            properties: BTreeMap::from([
                ("hash_algorithm".to_string(), "BLAKE3-256".to_string()),
                ("storage_format".to_string(), "canonical-json-v0.1.0".to_string()),
            ]),
            project_ids: vec![prj_id.clone()],
        };

        let project = ProjectPayload {
            id: prj_id.clone(),
            workspace_id: ws_id.clone(),
            code,
            name: project_name.to_string(),
            description: "Primary project managed via CLIBM".to_string(),
            created_at: now.clone(),
            participants: vec![Participant {
                id: "USR-01".to_string(),
                name: "Local Maintainer".to_string(),
                email: "maintainer@local.clibm".to_string(),
                role: "architect".to_string(),
            }],
            metadata: BTreeMap::new(),
            board_ids: vec![brd_id.clone()],
        };

        let board = KanbanBoardPayload {
            id: brd_id.clone(),
            project_id: prj_id.clone(),
            name: format!("{} Kanban Board", project_name),
            columns: Self::default_columns(),
            workflow: WorkflowConfig {
                enforce_dependencies: true,
                enforce_wip_limits: true,
                require_assignee_in_progress: false,
            },
        };

        let ws_hash = self.cas.put_object(OBJ_WORKSPACE, &workspace)?;
        let prj_hash = self.cas.put_object(OBJ_PROJECT, &project)?;
        let brd_hash = self.cas.put_object(OBJ_BOARD, &board)?;

        let mut tree = TreeSnapshotPayload {
            active_workspace_id: ws_id.clone(),
            active_project_id: prj_id.clone(),
            ..Default::default()
        };
        tree.workspaces.insert(ws_id, ws_hash);
        tree.projects.insert(prj_id, prj_hash);
        tree.boards.insert(brd_id, brd_hash);

        let tree_hash = self.save_working_tree(&tree)?;
        let cfg = self.cas.load_config()?;
        let commit_hash = VersioningEngine::create_commit(
            &self.cas,
            &tree_hash,
            &cfg.author_name,
            "Initialize CLIBM repository with workspace, project, and Kanban board",
            None,
        )?;

        Ok(commit_hash)
    }

    pub fn load_working_tree(&self) -> Result<(String, TreeSnapshotPayload)> {
        if let Some(staged_hash) = self.cas.get_staged_tree_hash()? {
            let tree: TreeSnapshotPayload = self.cas.get_typed_payload(&staged_hash, OBJ_TREE)?;
            return Ok((staged_hash, tree));
        }
        if let Some(head_commit_hash) = self.cas.resolve_head_commit()? {
            let commit: CommitPayload = self.cas.get_typed_payload(&head_commit_hash, OBJ_COMMIT)?;
            let tree: TreeSnapshotPayload =
                self.cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
            return Ok((commit.tree_hash, tree));
        }
        bail!("CLIBM repository is not initialized. Run `clibm init` first.");
    }

    pub fn save_working_tree(&self, tree: &TreeSnapshotPayload) -> Result<String> {
        let tree_hash = self.cas.put_object(OBJ_TREE, tree)?;
        self.cas.set_staged_tree_hash(&tree_hash)?;
        ComputedIndex::build_and_save(&self.cas, &tree_hash, tree)?;
        Ok(tree_hash)
    }

    /// Tree hash of the HEAD commit, or `None` before the first commit.
    pub fn head_tree_hash(&self) -> Result<Option<String>> {
        match self.cas.resolve_head_commit()? {
            Some(commit_hash) => {
                let commit: CommitPayload = self.cas.get_typed_payload(&commit_hash, OBJ_COMMIT)?;
                Ok(Some(commit.tree_hash))
            }
            None => Ok(None),
        }
    }

    /// `true` when the staged tree contains changes that are not in the HEAD commit.
    pub fn is_dirty(&self) -> Result<bool> {
        let staged = self.cas.get_staged_tree_hash()?;
        let head = self.head_tree_hash()?;
        Ok(match (staged, head) {
            (Some(s), Some(h)) => s != h,
            (Some(_), None) => true,
            _ => false,
        })
    }

    /// Saves the updated working tree and, if `config.auto_commit` is enabled (the default),
    /// automatically records an individual immutable BLAKE3 commit on the active branch.
    pub fn record_mutation(
        &self,
        tree: &TreeSnapshotPayload,
        commit_message: &str,
    ) -> Result<(String, Option<String>)> {
        let cfg = self.cas.load_config()?;
        if cfg.auto_commit {
            // Let pre-commit hooks veto the change before anything is staged.
            let tree_hash = self.cas.put_object(OBJ_TREE, tree)?;
            self.fire_hook(
                "pre-commit",
                &tree_hash,
                json!({ "message": commit_message, "auto": true }),
            )?;
        }
        let tree_hash = self.save_working_tree(tree)?;
        if cfg.auto_commit {
            let commit_hash = VersioningEngine::create_commit(
                &self.cas,
                &tree_hash,
                &cfg.author_name,
                commit_message,
                None,
            )?;
            self.fire_hook(
                "post-commit",
                &tree_hash,
                json!({ "message": commit_message, "commit": commit_hash, "auto": true }),
            )?;
            return Ok((tree_hash, Some(commit_hash)));
        }
        Ok((tree_hash, None))
    }

    // --- Lookups ---

    pub fn active_project(&self, tree: &TreeSnapshotPayload) -> Result<ProjectPayload> {
        let prj_hash = tree
            .projects
            .get(&tree.active_project_id)
            .ok_or_else(|| anyhow!("Active project '{}' not found", tree.active_project_id))?;
        self.cas.get_typed_payload(prj_hash, OBJ_PROJECT)
    }

    /// First board of `project`.
    pub fn project_board(
        &self,
        tree: &TreeSnapshotPayload,
        project: &ProjectPayload,
    ) -> Result<KanbanBoardPayload> {
        let board_id = project
            .board_ids
            .first()
            .ok_or_else(|| anyhow!("Project '{}' has no Kanban board", project.id))?;
        let board_hash = tree
            .boards
            .get(board_id)
            .ok_or_else(|| anyhow!("Board '{}' not found", board_id))?;
        self.cas.get_typed_payload(board_hash, OBJ_BOARD)
    }

    pub fn load_task(&self, tree: &TreeSnapshotPayload, task_id: &str) -> Result<TaskPayload> {
        let hash = tree
            .tasks
            .get(task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;
        self.cas.get_typed_payload(hash, OBJ_TASK)
    }

    fn load_all_tasks(&self, tree: &TreeSnapshotPayload) -> Result<BTreeMap<String, TaskPayload>> {
        let mut out = BTreeMap::new();
        for (id, h) in &tree.tasks {
            out.insert(id.clone(), self.cas.get_typed_payload(h, OBJ_TASK)?);
        }
        Ok(out)
    }

    /// Finds a column by ID or display name (case-insensitive).
    pub fn resolve_column<'a>(
        board: &'a KanbanBoardPayload,
        column: &str,
    ) -> Result<&'a KanbanColumn> {
        let q = column.trim();
        board
            .columns
            .iter()
            .find(|c| c.id == q)
            .or_else(|| {
                board
                    .columns
                    .iter()
                    .find(|c| c.id.eq_ignore_ascii_case(q) || c.name.eq_ignore_ascii_case(q))
            })
            .ok_or_else(|| {
                let valid: Vec<&str> = board.columns.iter().map(|c| c.id.as_str()).collect();
                anyhow!(
                    "Column '{}' does not exist on board '{}' (valid: {})",
                    column,
                    board.name,
                    valid.join(", ")
                )
            })
    }

    /// Resolves a user-supplied task reference to a task ID in `tree`. Accepted forms:
    /// 1. the full ID, ignoring case (`CLIBM-101`, `clibm-101`);
    /// 2. the task number (`101`): the active project's task first, otherwise the only
    ///    task with that number (an error lists the candidates when it is ambiguous);
    /// 3. a prefix of at least 4 hex characters of the task's current object hash, as
    ///    shown by `clibm board show`. The hash changes whenever the task is edited.
    pub fn resolve_task_ref(&self, tree: &TreeSnapshotPayload, input: &str) -> Result<String> {
        let q = input.trim();
        if q.is_empty() {
            bail!("Task reference cannot be empty");
        }
        if tree.tasks.contains_key(q) {
            return Ok(q.to_string());
        }
        if let Some(k) = tree.tasks.keys().find(|k| k.eq_ignore_ascii_case(q)) {
            return Ok(k.clone());
        }

        if q.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = q.parse::<u64>() {
                let candidates: Vec<String> = tree
                    .tasks
                    .keys()
                    .filter(|k| split_entity_id(k).map(|(_, num)| num) == Some(n))
                    .cloned()
                    .collect();
                if !candidates.is_empty() {
                    if let Ok(prj) = self.active_project(tree) {
                        if let Some(k) = candidates.iter().find(|k| {
                            split_entity_id(k)
                                .map(|(p, _)| p.eq_ignore_ascii_case(&prj.code))
                                .unwrap_or(false)
                        }) {
                            return Ok(k.clone());
                        }
                    }
                    if candidates.len() == 1 {
                        return Ok(candidates[0].clone());
                    }
                    bail!(
                        "Task number '{}' is ambiguous: it matches {}. Use the full task ID.",
                        q,
                        candidates.join(", ")
                    );
                }
            }
        }

        if q.len() >= 4 && q.chars().all(|c| c.is_ascii_hexdigit()) {
            let lower = q.to_ascii_lowercase();
            let matches: Vec<String> = tree
                .tasks
                .iter()
                .filter(|(_, h)| h.starts_with(&lower))
                .map(|(k, _)| k.clone())
                .collect();
            match matches.len() {
                0 => {}
                1 => return Ok(matches[0].clone()),
                _ => bail!(
                    "Hash prefix '{}' is ambiguous: it matches tasks {}. Use more characters or the task ID.",
                    q,
                    matches.join(", ")
                ),
            }
        }

        bail!(
            "Task '{}' not found. Use the full ID (e.g. CLIBM-101), the task number (101), \
             or a hash prefix of at least 4 characters from `clibm board show`.",
            q
        )
    }

    /// Enforces dependency, WIP-limit and assignee rules for placing `task` in `col`.
    fn check_column_rules(
        &self,
        tree: &TreeSnapshotPayload,
        board: &KanbanBoardPayload,
        task: &TaskPayload,
        col: &KanbanColumn,
        cfg: &ClibmConfig,
    ) -> Result<()> {
        if cfg.enforce_dependencies
            && board.workflow.enforce_dependencies
            && col.id != "backlog"
            && col.id != "todo"
        {
            for dep_id in &task.depends_on {
                if let Some(dep_hash) = tree.tasks.get(dep_id) {
                    let dep_task: TaskPayload = self.cas.get_typed_payload(dep_hash, OBJ_TASK)?;
                    let dep_col = board.columns.iter().find(|c| c.id == dep_task.column_id);
                    let is_done = dep_col.map(|c| c.is_completed_state).unwrap_or(false)
                        || dep_task.column_id == "done";
                    if !is_done {
                        bail!(
                            "Blocked by incomplete dependency '{}' (currently in column '{}'). Pass --force to override.",
                            dep_id,
                            dep_task.column_id
                        );
                    }
                }
            }
        }

        if cfg.enforce_wip_limits && board.workflow.enforce_wip_limits {
            if let Some(limit) = col.wip_limit {
                let mut count = 0usize;
                for (id, h) in &tree.tasks {
                    if id == &task.id {
                        continue;
                    }
                    let t: TaskPayload = self.cas.get_typed_payload(h, OBJ_TASK)?;
                    if t.board_id == board.id && t.column_id == col.id {
                        count += 1;
                    }
                }
                if count >= limit {
                    bail!(
                        "WIP limit reached: column '{}' already has {}/{} tasks. Pass --force to override.",
                        col.id,
                        count,
                        limit
                    );
                }
            }
        }

        if board.workflow.require_assignee_in_progress
            && col.id == "in-progress"
            && task.assignee.as_deref().map(str::trim).unwrap_or("").is_empty()
        {
            bail!(
                "Task '{}' needs an assignee before it can move to '{}'. Pass --force to override.",
                task.id,
                col.id
            );
        }
        Ok(())
    }

    // --- Status ---

    pub fn status(&self) -> Result<StatusReport> {
        let (staged_tree, tree) = self.load_working_tree()?;
        let cfg = self.cas.load_config()?;
        let head_commit = self.cas.resolve_head_commit()?;
        let head_tree = self.head_tree_hash()?;

        let ws_name = tree
            .workspaces
            .get(&tree.active_workspace_id)
            .and_then(|h| self.cas.get_typed_payload::<WorkspacePayload>(h, OBJ_WORKSPACE).ok())
            .map(|w| w.name)
            .unwrap_or_default();
        let project = self.active_project(&tree).ok();
        let board = project
            .as_ref()
            .and_then(|p| self.project_board(&tree, p).ok());

        let tasks = self.load_all_tasks(&tree)?;
        let project_tasks: Vec<&TaskPayload> = tasks
            .values()
            .filter(|t| t.project_id == tree.active_project_id)
            .collect();

        let columns = board
            .as_ref()
            .map(|b| {
                b.columns
                    .iter()
                    .map(|c| ColumnStatus {
                        id: c.id.clone(),
                        name: c.name.clone(),
                        tasks: project_tasks
                            .iter()
                            .filter(|t| t.board_id == b.id && t.column_id == c.id)
                            .count(),
                        wip_limit: c.wip_limit,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let dirty = match &head_tree {
            Some(h) => h != &staged_tree,
            None => true,
        };

        Ok(StatusReport {
            repository: self.cas.clibm_dir().display().to_string(),
            branch: self.cas.current_branch()?,
            head_commit,
            head_tree,
            staged_tree,
            dirty,
            auto_commit: cfg.auto_commit,
            active_workspace_id: tree.active_workspace_id.clone(),
            active_workspace_name: ws_name,
            active_project_id: tree.active_project_id.clone(),
            active_project_code: project.as_ref().map(|p| p.code.clone()).unwrap_or_default(),
            active_project_name: project.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
            board_id: board.as_ref().map(|b| b.id.clone()),
            board_name: board.as_ref().map(|b| b.name.clone()),
            columns,
            project_tasks: project_tasks.len(),
            total_tasks: tasks.len(),
            workspaces: tree.workspaces.len(),
            branches: self.cas.list_branches()?.len(),
            objects: self.cas.count_objects(),
        })
    }

    // --- Workspace Selection & Cross-Workspace Management ---

    /// Creates a new workspace along with its default project and Kanban board.
    pub fn create_workspace(
        &self,
        name: &str,
        code: Option<&str>,
        description: &str,
    ) -> Result<(WorkspacePayload, String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let ws_id = next_entity_id(tree.workspaces.keys(), "WS");
        let prj_id = next_entity_id(tree.projects.keys(), "PRJ");
        let brd_id = next_entity_id(tree.boards.keys(), "BRD");
        let now = Utc::now().to_rfc3339();

        let derived_code = code
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| {
                let letters: String = name
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(4)
                    .collect();
                if letters.is_empty() {
                    ws_id.replace('-', "")
                } else {
                    letters.to_uppercase()
                }
            });

        let workspace = WorkspacePayload {
            id: ws_id.clone(),
            name: name.to_string(),
            description: if description.is_empty() {
                format!("Workspace {}", name)
            } else {
                description.to_string()
            },
            created_at: now.clone(),
            properties: BTreeMap::from([("hash_algorithm".to_string(), "BLAKE3-256".to_string())]),
            project_ids: vec![prj_id.clone()],
        };

        let project = ProjectPayload {
            id: prj_id.clone(),
            workspace_id: ws_id.clone(),
            code: derived_code,
            name: format!("{} Project", name),
            description: format!("Default project in workspace {}", name),
            created_at: now,
            participants: Vec::new(),
            metadata: BTreeMap::new(),
            board_ids: vec![brd_id.clone()],
        };

        let board = KanbanBoardPayload {
            id: brd_id.clone(),
            project_id: prj_id.clone(),
            name: format!("{} Board", name),
            columns: Self::default_columns(),
            workflow: WorkflowConfig {
                enforce_dependencies: true,
                enforce_wip_limits: true,
                require_assignee_in_progress: false,
            },
        };

        let ws_hash = self.cas.put_object(OBJ_WORKSPACE, &workspace)?;
        let prj_hash = self.cas.put_object(OBJ_PROJECT, &project)?;
        let brd_hash = self.cas.put_object(OBJ_BOARD, &board)?;

        tree.workspaces.insert(ws_id.clone(), ws_hash.clone());
        tree.projects.insert(prj_id, prj_hash);
        tree.boards.insert(brd_id, brd_hash);

        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(workspace): create {} \"{}\"", ws_id, name),
        )?;

        Ok((workspace, ws_hash, auto_commit))
    }

    /// Switches the active workspace.
    /// If `target` is `None`, cycles to the next workspace in alphabetical order by workspace name.
    pub fn switch_workspace(
        &self,
        target: Option<&str>,
    ) -> Result<(WorkspacePayload, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let mut loaded_workspaces: Vec<WorkspacePayload> = Vec::new();
        for whash in tree.workspaces.values() {
            let ws: WorkspacePayload = self.cas.get_typed_payload(whash, OBJ_WORKSPACE)?;
            loaded_workspaces.push(ws);
        }

        if loaded_workspaces.is_empty() {
            bail!("No workspaces exist in the repository.");
        }

        // Sort alphabetically by name (case-insensitive), then by ID
        loaded_workspaces.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });

        let chosen = if let Some(q) = target {
            let clean = q.trim();
            loaded_workspaces
                .into_iter()
                .find(|w| w.id.eq_ignore_ascii_case(clean) || w.name.eq_ignore_ascii_case(clean))
                .ok_or_else(|| anyhow!("Workspace '{}' not found", clean))?
        } else {
            // Cycle to the next workspace in alphabetical order
            let current_idx = loaded_workspaces
                .iter()
                .position(|w| w.id == tree.active_workspace_id)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % loaded_workspaces.len();
            loaded_workspaces.remove(next_idx)
        };

        tree.active_workspace_id = chosen.id.clone();
        if let Some(first_prj) = chosen.project_ids.first() {
            tree.active_project_id = first_prj.clone();
        }

        let (tree_hash, auto_commit) = self.record_mutation(
            &tree,
            &format!(
                "auto(workspace): switch active workspace to {} ({})",
                chosen.id, chosen.name
            ),
        )?;
        self.fire_hook(
            "on-workspace-switch",
            &tree_hash,
            json!({ "workspace_id": chosen.id, "workspace_name": chosen.name, "commit": auto_commit }),
        )?;

        Ok((chosen, auto_commit))
    }

    /// Transfers (or copies if `copy_mode` is true) a task to a target workspace.
    pub fn transfer_task(
        &self,
        task_ref: &str,
        target_workspace: &str,
        copy_mode: bool,
    ) -> Result<(TaskPayload, String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let mut task = self.load_task(&tree, &task_id)?;

        let mut target_ws: Option<WorkspacePayload> = None;
        for whash in tree.workspaces.values() {
            let ws: WorkspacePayload = self.cas.get_typed_payload(whash, OBJ_WORKSPACE)?;
            if ws.id.eq_ignore_ascii_case(target_workspace)
                || ws.name.eq_ignore_ascii_case(target_workspace)
            {
                target_ws = Some(ws);
                break;
            }
        }
        let ws = target_ws.ok_or_else(|| anyhow!("Target workspace '{}' not found", target_workspace))?;
        let target_prj_id = ws
            .project_ids
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("Workspace '{}' has no projects", ws.id))?;
        let prj_hash = tree
            .projects
            .get(&target_prj_id)
            .ok_or_else(|| anyhow!("Project '{}' not found", target_prj_id))?;
        let prj: ProjectPayload = self.cas.get_typed_payload(prj_hash, OBJ_PROJECT)?;
        let target_board = self.project_board(&tree, &prj)?;

        let now = Utc::now().to_rfc3339();
        if copy_mode {
            let number = tree.next_task_number(&prj.code);
            tree.record_task_number(&prj.code, number);
            task.id = format!("{}-{}", prj.code, number);
            task.created_at = now.clone();
        }
        task.project_id = target_prj_id;
        task.board_id = target_board.id.clone();
        if !target_board.columns.iter().any(|c| c.id == task.column_id) {
            task.column_id = target_board
                .columns
                .iter()
                .find(|c| !c.is_completed_state && c.id != "backlog")
                .or_else(|| target_board.columns.first())
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "todo".to_string());
        }
        task.updated_at = now;
        task.metadata
            .insert("workspace_id".to_string(), ws.id.clone());

        let new_task_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_task_hash.clone());

        let action_verb = if copy_mode { "copy" } else { "transfer" };
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): {} {} to workspace {}", action_verb, task.id, ws.id),
        )?;

        Ok((task, new_task_hash, auto_commit))
    }

    // --- Task Operations ---

    /// Creates a task in the active project. `column_id` may be a column ID or name.
    /// `depends_on` entries are resolved like any task reference. Dependency, WIP-limit
    /// and assignee rules apply to the target column unless `force` is set.
    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
        &self,
        title: &str,
        description: &str,
        priority: TaskPriority,
        column_id: &str,
        assignee: Option<String>,
        due_date: Option<String>,
        tags: Vec<String>,
        depends_on: Vec<String>,
        force: bool,
    ) -> Result<(TaskPayload, String, Option<String>)> {
        if title.trim().is_empty() {
            bail!("Task title cannot be empty");
        }
        let (_, mut tree) = self.load_working_tree()?;
        let cfg = self.cas.load_config()?;
        let prj = self.active_project(&tree)?;
        let board = self.project_board(&tree, &prj)?;
        let col = Self::resolve_column(&board, column_id)?.clone();

        let mut deps: Vec<String> = Vec::new();
        for raw in depends_on.iter().map(|d| d.trim()).filter(|d| !d.is_empty()) {
            let dep = self.resolve_task_ref(&tree, raw)?;
            if !deps.contains(&dep) {
                deps.push(dep);
            }
        }

        let number = tree.next_task_number(&prj.code);
        let task_id = format!("{}-{}", prj.code, number);
        let now = Utc::now().to_rfc3339();

        let task = TaskPayload {
            id: task_id.clone(),
            project_id: prj.id.clone(),
            board_id: board.id.clone(),
            column_id: col.id.clone(),
            title: title.trim().to_string(),
            description: description.to_string(),
            priority,
            assignee: assignee.filter(|a| !a.trim().is_empty()),
            due_date: due_date.filter(|d| !d.trim().is_empty()),
            created_at: now.clone(),
            updated_at: now,
            tags: clean_tags(tags),
            depends_on: deps,
            comments: Vec::new(),
            metadata: BTreeMap::from([(
                "workspace_id".to_string(),
                tree.active_workspace_id.clone(),
            )]),
        };

        if !force {
            self.check_column_rules(&tree, &board, &task, &col, &cfg)?;
        }

        let task_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task_id.clone(), task_hash.clone());
        tree.record_task_number(&prj.code, number);
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): add {} \"{}\"", task_id, task.title),
        )?;

        Ok((task, task_hash, auto_commit))
    }

    pub fn move_task(&self, task_ref: &str, target_column: &str, force: bool) -> Result<MutationResult> {
        let (_, mut tree) = self.load_working_tree()?;
        let cfg = self.cas.load_config()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let task_hash = tree.tasks[&task_id].clone();
        let mut task: TaskPayload = self.cas.get_typed_payload(&task_hash, OBJ_TASK)?;

        let board_hash = tree
            .boards
            .get(&task.board_id)
            .ok_or_else(|| anyhow!("Board '{}' not found", task.board_id))?;
        let board: KanbanBoardPayload = self.cas.get_typed_payload(board_hash, OBJ_BOARD)?;
        let col = Self::resolve_column(&board, target_column)?.clone();

        if task.column_id == col.id {
            return Ok(MutationResult {
                id: task_id,
                hash: task_hash,
                commit: None,
                changed: false,
            });
        }

        if !force {
            self.check_column_rules(&tree, &board, &task, &col, &cfg)?;
        }

        let from_column = task.column_id.clone();
        let (current_tree_hash, _) = self.load_working_tree()?;
        self.fire_hook(
            "pre-move",
            &current_tree_hash,
            json!({ "task_id": task_id, "board_id": board.id, "from": from_column, "to": col.id, "force": force }),
        )?;

        task.column_id = col.id.clone();
        task.updated_at = Utc::now().to_rfc3339();

        let new_task_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_task_hash.clone());
        let (tree_hash, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): move {} to {}", task.id, col.id),
        )?;
        self.fire_hook(
            "on-task-move",
            &tree_hash,
            json!({ "task_id": task_id, "board_id": board.id, "from": from_column, "to": col.id, "commit": auto_commit }),
        )?;

        Ok(MutationResult {
            id: task_id,
            hash: new_task_hash,
            commit: auto_commit,
            changed: true,
        })
    }

    pub fn add_comment(&self, task_ref: &str, author: &str, body: &str) -> Result<MutationResult> {
        if body.trim().is_empty() {
            bail!("Comment cannot be empty");
        }
        let (_, mut tree) = self.load_working_tree()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let mut task = self.load_task(&tree, &task_id)?;

        let comment_id = format!("CMT-{:02}", task.comments.len() + 1);
        task.comments.push(TaskComment {
            id: comment_id,
            author: author.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            body: body.to_string(),
        });
        task.updated_at = Utc::now().to_rfc3339();

        let new_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_hash.clone());
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): comment on {}", task.id),
        )?;
        Ok(MutationResult {
            id: task_id,
            hash: new_hash,
            commit: auto_commit,
            changed: true,
        })
    }

    pub fn add_dependency(&self, task_ref: &str, depends_on_ref: &str) -> Result<MutationResult> {
        let (_, mut tree) = self.load_working_tree()?;
        let cfg = self.cas.load_config()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let depends_on_id = self.resolve_task_ref(&tree, depends_on_ref)?;

        let mut all_tasks = self.load_all_tasks(&tree)?;
        ComputedIndex::validate_new_dependency(&all_tasks, &task_id, &depends_on_id)?;

        if !cfg.cross_workspace_dependencies {
            let a = &all_tasks[&task_id];
            let b = &all_tasks[&depends_on_id];
            if a.project_id != b.project_id {
                bail!(
                    "Cross-workspace dependencies are disabled (cross_workspace_dependencies = false): \
                     '{}' and '{}' belong to different projects.",
                    task_id,
                    depends_on_id
                );
            }
        }

        let mut task = all_tasks
            .remove(&task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;

        if task.depends_on.contains(&depends_on_id) {
            return Ok(MutationResult {
                id: task_id.clone(),
                hash: tree.tasks[&task_id].clone(),
                commit: None,
                changed: false,
            });
        }
        task.depends_on.push(depends_on_id.clone());
        task.updated_at = Utc::now().to_rfc3339();

        let new_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_hash.clone());
        let (tree_hash, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): depend {} -> {}", task.id, depends_on_id),
        )?;
        self.fire_hook(
            "on-task-depend",
            &tree_hash,
            json!({ "task_id": task_id, "depends_on": depends_on_id, "commit": auto_commit }),
        )?;
        Ok(MutationResult {
            id: task_id,
            hash: new_hash,
            commit: auto_commit,
            changed: true,
        })
    }

    /// Edits task fields. Moving between columns stays with [`ClibmEngine::move_task`].
    pub fn edit_task(&self, task_ref: &str, edit: TaskEdit) -> Result<MutationResult> {
        let (_, mut tree) = self.load_working_tree()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let original = self.load_task(&tree, &task_id)?;
        let mut task = original.clone();

        if let Some(title) = edit.title {
            if title.trim().is_empty() {
                bail!("Task title cannot be empty");
            }
            task.title = title.trim().to_string();
        }
        if let Some(desc) = edit.description {
            task.description = desc;
        }
        if let Some(p) = edit.priority {
            task.priority = p;
        }
        if let Some(a) = edit.assignee {
            task.assignee = a.filter(|v| !v.trim().is_empty());
        }
        if let Some(d) = edit.due_date {
            task.due_date = d.filter(|v| !v.trim().is_empty());
        }
        if let Some(tags) = edit.tags {
            task.tags = clean_tags(tags);
        }
        if !edit.add_tags.is_empty() {
            let merged = task.tags.iter().cloned().chain(edit.add_tags);
            task.tags = clean_tags(merged);
        }
        if !edit.remove_tags.is_empty() {
            task.tags
                .retain(|t| !edit.remove_tags.iter().any(|r| r.trim().eq_ignore_ascii_case(t)));
        }

        if task == original {
            return Ok(MutationResult {
                id: task_id.clone(),
                hash: tree.tasks[&task_id].clone(),
                commit: None,
                changed: false,
            });
        }

        task.updated_at = Utc::now().to_rfc3339();
        let new_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_hash.clone());
        let (_, auto_commit) =
            self.record_mutation(&tree, &format!("auto(task): edit {}", task.id))?;
        Ok(MutationResult {
            id: task_id,
            hash: new_hash,
            commit: auto_commit,
            changed: true,
        })
    }

    /// Removes a task from the working tree (its history stays in earlier commits).
    /// Refuses while other tasks depend on it, unless `force` is set, in which case those
    /// references are removed from the dependent tasks.
    pub fn delete_task(&self, task_ref: &str, force: bool) -> Result<DeleteResult> {
        let (_, mut tree) = self.load_working_tree()?;
        let task_id = self.resolve_task_ref(&tree, task_ref)?;
        let all_tasks = self.load_all_tasks(&tree)?;

        let dependents: Vec<&TaskPayload> = all_tasks
            .values()
            .filter(|t| t.id != task_id && t.depends_on.contains(&task_id))
            .collect();
        if !dependents.is_empty() && !force {
            let ids: Vec<&str> = dependents.iter().map(|t| t.id.as_str()).collect();
            bail!(
                "Task '{}' is a dependency of {}. Pass --force to delete it and remove those links.",
                task_id,
                ids.join(", ")
            );
        }

        let now = Utc::now().to_rfc3339();
        let mut unlinked = Vec::new();
        for dep in dependents {
            let mut t = dep.clone();
            t.depends_on.retain(|d| d != &task_id);
            t.updated_at = now.clone();
            let h = self.cas.put_object(OBJ_TASK, &t)?;
            tree.tasks.insert(t.id.clone(), h);
            unlinked.push(t.id.clone());
        }
        tree.tasks.remove(&task_id);

        let (_, auto_commit) =
            self.record_mutation(&tree, &format!("auto(task): delete {}", task_id))?;
        Ok(DeleteResult {
            id: task_id,
            unlinked_dependents: unlinked,
            commit: auto_commit,
        })
    }

    // --- Versioning wrappers ---

    /// Creates a manual commit of the staged tree. Returns `None` ("nothing to commit")
    /// when the staged tree already matches the HEAD commit.
    pub fn commit(&self, message: &str, author: Option<&str>) -> Result<Option<String>> {
        if message.trim().is_empty() {
            bail!("Commit message cannot be empty");
        }
        let (tree_hash, _) = self.load_working_tree()?;
        if self.head_tree_hash()?.as_deref() == Some(tree_hash.as_str()) {
            return Ok(None);
        }
        let cfg = self.cas.load_config()?;
        let author = author
            .map(str::to_string)
            .unwrap_or_else(|| cfg.author_name.clone());
        self.fire_hook("pre-commit", &tree_hash, json!({ "message": message, "auto": false }))?;
        let commit_hash =
            VersioningEngine::create_commit(&self.cas, &tree_hash, &author, message, None)?;
        self.fire_hook(
            "post-commit",
            &tree_hash,
            json!({ "message": message, "commit": commit_hash, "auto": false }),
        )?;
        Ok(Some(commit_hash))
    }

    /// Creates branch `name` at the HEAD commit.
    pub fn create_branch(&self, name: &str) -> Result<String> {
        crate::cas::validate_ref_name(name)?;
        if self.cas.get_branch_commit(name)?.is_some() {
            bail!("Branch '{}' already exists", name);
        }
        let head_commit = self
            .cas
            .resolve_head_commit()?
            .ok_or_else(|| anyhow!("Cannot create a branch before the first commit"))?;
        self.cas.set_branch_commit(name, &head_commit)?;
        Ok(head_commit)
    }

    /// Switches to a branch or commit. Refuses when the working tree has uncommitted
    /// changes, unless `force` is set (those changes are then discarded).
    pub fn checkout(&self, target_ref: &str, force: bool) -> Result<String> {
        if !force && self.is_dirty()? {
            bail!(
                "You have uncommitted changes. Commit them with `clibm commit -m <msg>` \
                 or pass --force to discard them."
            );
        }

        if let Some(branch_commit) = self.cas.get_branch_commit(target_ref)? {
            self.cas.set_head_branch(target_ref)?;
            let commit: CommitPayload = self.cas.get_typed_payload(&branch_commit, OBJ_COMMIT)?;
            let tree: TreeSnapshotPayload =
                self.cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
            self.save_working_tree(&tree)?;
            return Ok(branch_commit);
        }

        let commit_hash = self.cas.resolve_ref(target_ref)?;
        let commit: CommitPayload = self.cas.get_typed_payload(&commit_hash, OBJ_COMMIT)?;
        self.cas.set_detached_head(&commit_hash)?;
        let tree: TreeSnapshotPayload = self.cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
        self.save_working_tree(&tree)?;
        Ok(commit_hash)
    }

    /// Merges `branch` into HEAD (see [`VersioningEngine::merge_branch`]). Refuses when the
    /// working tree has uncommitted changes.
    pub fn merge(
        &self,
        branch: &str,
        author: Option<&str>,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        if self.is_dirty()? {
            bail!("You have uncommitted changes. Commit them before merging.");
        }
        let cfg = self.cas.load_config()?;
        let author = author
            .map(str::to_string)
            .unwrap_or_else(|| cfg.author_name.clone());
        VersioningEngine::merge_branch(&self.cas, branch, &author, strategy)
    }

    pub fn export_bundle(&self) -> Result<PortableBundle> {
        Ok(PortableBundle {
            format: "clibm-portable-bundle".to_string(),
            version: "0.1.0".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            head_branch: self.cas.current_branch()?,
            branches: self.cas.list_branches()?,
            staged_tree_hash: self.cas.get_staged_tree_hash()?,
            objects: self.cas.list_all_objects()?,
        })
    }
}
