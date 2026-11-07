//! Headless Core Engine (Architectural Principles #1 & #2)
//!
//! Coordinates Workspaces, Projects, Tasks, Kanban Boards, Versioning, Configuration,
//! Automatic Per-Modification Commits, and Cross-Workspace Task Coordination.

use crate::cas::{CasObject, CasStore};
use crate::index::ComputedIndex;
use crate::models::*;
use crate::versioning::VersioningEngine;
use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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

pub struct ClibmEngine {
    pub cas: CasStore,
}

impl ClibmEngine {
    pub fn open(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            cas: CasStore::new(workspace_root),
        }
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
    pub fn init(&self, workspace_name: &str, project_code: &str, project_name: &str) -> Result<String> {
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
            code: project_code.to_uppercase(),
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

    /// Saves the updated working tree and, if `config.auto_commit` is enabled (the default),
    /// automatically records an individual immutable BLAKE3 commit on the active branch.
    pub fn record_mutation(
        &self,
        tree: &TreeSnapshotPayload,
        commit_message: &str,
    ) -> Result<(String, Option<String>)> {
        let tree_hash = self.save_working_tree(tree)?;
        let cfg = self.cas.load_config()?;
        if cfg.auto_commit {
            let commit_hash = VersioningEngine::create_commit(
                &self.cas,
                &tree_hash,
                &cfg.author_name,
                commit_message,
                None,
            )?;
            return Ok((tree_hash, Some(commit_hash)));
        }
        Ok((tree_hash, None))
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
        let next_idx = tree.workspaces.len() + 1;
        let ws_id = format!("WS-{:02}", next_idx);
        let prj_id = format!("PRJ-{:02}", tree.projects.len() + 1);
        let brd_id = format!("BRD-{:02}", tree.boards.len() + 1);
        let now = Utc::now().to_rfc3339();

        let derived_code = code
            .map(|c| c.to_uppercase())
            .unwrap_or_else(|| {
                let letters: String = name
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(4)
                    .collect();
                if letters.is_empty() {
                    format!("WS{}", next_idx)
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

        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!(
                "auto(workspace): switch active workspace to {} ({})",
                chosen.id, chosen.name
            ),
        )?;

        Ok((chosen, auto_commit))
    }

    /// Transfers (or copies if `copy_mode` is true) a task to a target workspace.
    pub fn transfer_task(
        &self,
        task_id: &str,
        target_workspace: &str,
        copy_mode: bool,
    ) -> Result<(TaskPayload, String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let task_hash = tree
            .tasks
            .get(task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;
        let mut task: TaskPayload = self.cas.get_typed_payload(task_hash, OBJ_TASK)?;

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
        let target_board_id = prj
            .board_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "BRD-01".to_string());

        let now = Utc::now().to_rfc3339();
        if copy_mode {
            let new_id = format!("{}-{}", prj.code, tree.tasks.len() + 101);
            task.id = new_id;
            task.created_at = now.clone();
        }
        task.project_id = target_prj_id;
        task.board_id = target_board_id;
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
    ) -> Result<(TaskPayload, String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let prj_id = tree.active_project_id.clone();
        let prj_hash = tree
            .projects
            .get(&prj_id)
            .ok_or_else(|| anyhow!("Active project '{}' not found", prj_id))?;
        let prj: ProjectPayload = self.cas.get_typed_payload(prj_hash, OBJ_PROJECT)?;
        let board_id = prj
            .board_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "BRD-01".to_string());

        let next_num = tree.tasks.len() + 101;
        let task_id = format!("{}-{}", prj.code, next_num);
        let now = Utc::now().to_rfc3339();

        let task = TaskPayload {
            id: task_id.clone(),
            project_id: prj_id,
            board_id,
            column_id: column_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            priority,
            assignee,
            due_date,
            created_at: now.clone(),
            updated_at: now,
            tags,
            depends_on,
            comments: Vec::new(),
            metadata: BTreeMap::from([(
                "workspace_id".to_string(),
                tree.active_workspace_id.clone(),
            )]),
        };

        let task_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task_id.clone(), task_hash.clone());
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): add {} \"{}\"", task_id, title),
        )?;

        Ok((task, task_hash, auto_commit))
    }

    pub fn move_task(
        &self,
        task_id: &str,
        target_column: &str,
        force: bool,
    ) -> Result<(String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let cfg = self.cas.load_config()?;
        let task_hash = tree
            .tasks
            .get(task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;
        let mut task: TaskPayload = self.cas.get_typed_payload(task_hash, OBJ_TASK)?;

        let board_hash = tree
            .boards
            .get(&task.board_id)
            .ok_or_else(|| anyhow!("Board '{}' not found", task.board_id))?;
        let board: KanbanBoardPayload = self.cas.get_typed_payload(board_hash, OBJ_BOARD)?;

        let col = board
            .columns
            .iter()
            .find(|c| c.id == target_column || c.name.eq_ignore_ascii_case(target_column))
            .ok_or_else(|| {
                anyhow!(
                    "Column '{}' does not exist on board '{}'",
                    target_column,
                    board.name
                )
            })?;

        if !force
            && cfg.enforce_dependencies
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

        task.column_id = col.id.clone();
        task.updated_at = Utc::now().to_rfc3339();

        let new_task_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_task_hash.clone());
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): move {} to {}", task.id, col.id),
        )?;

        Ok((new_task_hash, auto_commit))
    }

    pub fn add_comment(
        &self,
        task_id: &str,
        author: &str,
        body: &str,
    ) -> Result<(String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        let task_hash = tree
            .tasks
            .get(task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;
        let mut task: TaskPayload = self.cas.get_typed_payload(task_hash, OBJ_TASK)?;

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
        Ok((new_hash, auto_commit))
    }

    pub fn add_dependency(
        &self,
        task_id: &str,
        depends_on_id: &str,
    ) -> Result<(String, Option<String>)> {
        let (_, mut tree) = self.load_working_tree()?;
        if !tree.tasks.contains_key(depends_on_id) {
            bail!("Target dependency task '{}' does not exist", depends_on_id);
        }

        let mut all_tasks = BTreeMap::new();
        for (id, h) in &tree.tasks {
            let t: TaskPayload = self.cas.get_typed_payload(h, OBJ_TASK)?;
            all_tasks.insert(id.clone(), t);
        }

        ComputedIndex::validate_new_dependency(&all_tasks, task_id, depends_on_id)?;

        let mut task = all_tasks
            .remove(task_id)
            .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?;

        if !task.depends_on.contains(&depends_on_id.to_string()) {
            task.depends_on.push(depends_on_id.to_string());
            task.updated_at = Utc::now().to_rfc3339();
        }

        let new_hash = self.cas.put_object(OBJ_TASK, &task)?;
        tree.tasks.insert(task.id.clone(), new_hash.clone());
        let (_, auto_commit) = self.record_mutation(
            &tree,
            &format!("auto(task): depend {} -> {}", task.id, depends_on_id),
        )?;
        Ok((new_hash, auto_commit))
    }

    pub fn checkout(&self, target_ref: &str) -> Result<String> {
        if let Some(branch_commit) = self.cas.get_branch_commit(target_ref)? {
            self.cas.set_head_branch(target_ref)?;
            let commit: CommitPayload = self.cas.get_typed_payload(&branch_commit, OBJ_COMMIT)?;
            let tree: TreeSnapshotPayload =
                self.cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
            self.save_working_tree(&tree)?;
            return Ok(branch_commit);
        }

        let commit_hash = self.cas.resolve_ref(target_ref)?;
        self.cas.set_detached_head(&commit_hash)?;
        let commit: CommitPayload = self.cas.get_typed_payload(&commit_hash, OBJ_COMMIT)?;
        let tree: TreeSnapshotPayload = self.cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
        self.save_working_tree(&tree)?;
        Ok(commit_hash)
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
