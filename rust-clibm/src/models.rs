//! Domain & Versioning Models for CLIBM v0.1.0
//!
//! Defines serializable payloads for Workspaces, Projects, Tasks, Kanban Boards,
//! Tree Snapshots, Commits, Diffs, Configuration, and Portable Repository Bundles.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Object type constants used in the `Type` field of every CAS object
pub const OBJ_WORKSPACE: &str = "workspace";
pub const OBJ_PROJECT: &str = "project";
pub const OBJ_TASK: &str = "task";
pub const OBJ_BOARD: &str = "board";
pub const OBJ_TREE: &str = "tree";
pub const OBJ_COMMIT: &str = "commit";

fn default_true() -> bool {
    true
}

fn default_author_name() -> String {
    "Local Maintainer".to_string()
}

fn default_author_email() -> String {
    "maintainer@local.clibm".to_string()
}

fn default_priority_str() -> String {
    "medium".to_string()
}

fn default_github_repo_url() -> String {
    "https://github.com/FelipeVandevelde/CLIBM.git".to_string()
}

fn default_update_branch() -> String {
    "main".to_string()
}

fn default_install_profile() -> String {
    "pure".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_install_path() -> String {
    "~/.local/bin".to_string()
}

fn default_mcp_transport() -> String {
    "both".to_string()
}

/// Repository & machine installation configuration stored in `.clibm/config.json`.
/// Automatic commits are enabled by default (`auto_commit = true`) so every modification
/// is recorded individually without requiring manual commit commands.
/// Core starts pure (`install_profile = "pure"`, 0 built-in plugins) unless the user selects
/// `"with-defaults"` during installation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClibmConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,
    #[serde(default = "default_author_name")]
    pub author_name: String,
    #[serde(default = "default_author_email")]
    pub author_email: String,
    #[serde(default = "default_priority_str")]
    pub default_priority: String,
    #[serde(default = "default_true")]
    pub enforce_dependencies: bool,
    #[serde(default = "default_true")]
    pub enforce_wip_limits: bool,
    #[serde(default = "default_true")]
    pub cross_workspace_dependencies: bool,
    #[serde(default = "default_github_repo_url")]
    pub github_repo_url: String,
    #[serde(default = "default_update_branch")]
    pub update_branch: String,
    #[serde(default = "default_install_profile")]
    pub install_profile: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_install_path")]
    pub install_path: String,
    #[serde(default = "default_mcp_transport")]
    pub mcp_transport: String,
}

impl Default for ClibmConfig {
    fn default() -> Self {
        Self {
            auto_commit: true,
            author_name: default_author_name(),
            author_email: default_author_email(),
            default_priority: default_priority_str(),
            enforce_dependencies: true,
            enforce_wip_limits: true,
            cross_workspace_dependencies: true,
            github_repo_url: default_github_repo_url(),
            update_branch: default_update_branch(),
            install_profile: default_install_profile(),
            language: default_language(),
            install_path: default_install_path(),
            mcp_transport: default_mcp_transport(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspacePayload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub properties: BTreeMap<String, String>,
    pub project_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectPayload {
    pub id: String,
    pub workspace_id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub participants: Vec<Participant>,
    pub metadata: BTreeMap<String, String>,
    pub board_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskPriority::Low => "low",
            TaskPriority::Medium => "medium",
            TaskPriority::High => "high",
            TaskPriority::Critical => "critical",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "critical" | "crit" | "p0" => TaskPriority::Critical,
            "high" | "p1" => TaskPriority::High,
            "low" | "p3" => TaskPriority::Low,
            _ => TaskPriority::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskComment {
    pub id: String,
    pub author: String,
    pub timestamp: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskPayload {
    pub id: String,
    pub project_id: String,
    pub board_id: String,
    pub column_id: String,
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub depends_on: Vec<String>,
    pub comments: Vec<TaskComment>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KanbanColumn {
    pub id: String,
    pub name: String,
    pub order: usize,
    pub wip_limit: Option<usize>,
    pub is_completed_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowConfig {
    pub enforce_dependencies: bool,
    pub enforce_wip_limits: bool,
    pub require_assignee_in_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KanbanBoardPayload {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub columns: Vec<KanbanColumn>,
    pub workflow: WorkflowConfig,
}

/// Git-inspired Root Tree Snapshot mapping entity IDs to their immutable BLAKE3 object hashes.
/// Unmodified tasks, boards, and projects share the exact same CAS object across commits.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeSnapshotPayload {
    pub active_workspace_id: String,
    pub active_project_id: String,
    pub workspaces: BTreeMap<String, String>,
    pub projects: BTreeMap<String, String>,
    pub boards: BTreeMap<String, String>,
    pub tasks: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitPayload {
    pub tree_hash: String,
    pub parent_hashes: Vec<String>,
    pub author: String,
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChange {
    pub entity_type: String,
    pub entity_id: String,
    pub change_kind: String, // "added" | "modified" | "deleted"
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub from_ref: String,
    pub to_ref: String,
    pub from_tree_hash: String,
    pub to_tree_hash: String,
    pub changes: Vec<EntityChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryEntry {
    pub commit_hash: String,
    pub commit_timestamp: String,
    pub commit_author: String,
    pub commit_message: String,
    pub task_object_hash: String,
    pub task_snapshot: TaskPayload,
}
