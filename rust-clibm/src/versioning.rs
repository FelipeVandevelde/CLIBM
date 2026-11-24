//! Versioning & History Engine (Specification 2.1.4)
//!
//! Provides commit creation, history log traversal, structural diffs between any two
//! commits, and individual task history lookup. History is one straight line of commits:
//! CLIBM has no branches or merges (go back to an older state with `clibm restore`).

use crate::cas::CasStore;
use crate::models::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub struct VersioningEngine;

impl VersioningEngine {
    /// Creates an immutable commit pointing to `tree_hash` on top of HEAD and moves HEAD's
    /// line (`refs/heads/main`, or a leftover ref / detached HEAD in old repositories).
    pub fn create_commit(cas: &CasStore, tree_hash: &str, author: &str, message: &str) -> Result<String> {
        let parents: Vec<String> = cas.resolve_head_commit()?.into_iter().collect();

        let commit = CommitPayload {
            tree_hash: tree_hash.to_string(),
            parent_hashes: parents,
            author: author.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            message: message.to_string(),
        };

        let commit_hash = cas.put_object(OBJ_COMMIT, &commit)?;

        if let Some(branch) = cas.current_branch()? {
            cas.set_branch_commit(&branch, &commit_hash)?;
        } else {
            cas.set_detached_head(&commit_hash)?;
        }

        Ok(commit_hash)
    }

    /// Traverses the commit DAG in reverse chronological / topological order from `start_ref`.
    pub fn log_history(
        cas: &CasStore,
        start_ref: Option<&str>,
    ) -> Result<Vec<(String, CommitPayload)>> {
        let start_hash = match start_ref {
            Some(r) => cas.resolve_ref(r)?,
            None => match cas.resolve_head_commit()? {
                Some(h) => h,
                None => return Ok(Vec::new()),
            },
        };

        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        let mut commits = Vec::new();

        queue.push_back(start_hash);

        while let Some(curr_hash) = queue.pop_front() {
            if !visited.insert(curr_hash.clone()) {
                continue;
            }
            let commit: CommitPayload = cas.get_typed_payload(&curr_hash, OBJ_COMMIT)?;
            for parent in &commit.parent_hashes {
                queue.push_back(parent.clone());
            }
            commits.push((curr_hash, commit));
        }

        commits.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
        Ok(commits)
    }

    /// Extracts the individual modification history of a single Task across the commit graph.
    pub fn task_history(cas: &CasStore, task_id: &str) -> Result<Vec<TaskHistoryEntry>> {
        let commits = Self::log_history(cas, None)?;
        let mut history = Vec::new();
        let mut last_seen_hash: Option<String> = None;

        for (commit_hash, commit) in commits.into_iter().rev() {
            let tree: TreeSnapshotPayload = cas.get_typed_payload(&commit.tree_hash, OBJ_TREE)?;
            if let Some(task_obj_hash) = tree.tasks.get(task_id) {
                if last_seen_hash.as_deref() != Some(task_obj_hash.as_str()) {
                    let task_snapshot: TaskPayload =
                        cas.get_typed_payload(task_obj_hash, OBJ_TASK)?;
                    history.push(TaskHistoryEntry {
                        commit_hash,
                        commit_timestamp: commit.timestamp,
                        commit_author: commit.author,
                        commit_message: commit.message,
                        task_object_hash: task_obj_hash.clone(),
                        task_snapshot,
                    });
                    last_seen_hash = Some(task_obj_hash.clone());
                }
            } else {
                // Deleted here: if it comes back (e.g. `clibm restore`), list it again.
                last_seen_hash = None;
            }
        }
        history.reverse();
        Ok(history)
    }

    /// Computes a structural diff between two references (commits or branches).
    pub fn diff_refs(cas: &CasStore, from_ref: &str, to_ref: &str) -> Result<DiffReport> {
        let from_commit_hash = cas.resolve_ref(from_ref)?;
        let to_commit_hash = cas.resolve_ref(to_ref)?;

        let from_commit: CommitPayload = cas.get_typed_payload(&from_commit_hash, OBJ_COMMIT)?;
        let to_commit: CommitPayload = cas.get_typed_payload(&to_commit_hash, OBJ_COMMIT)?;

        let from_tree: TreeSnapshotPayload =
            cas.get_typed_payload(&from_commit.tree_hash, OBJ_TREE)?;
        let to_tree: TreeSnapshotPayload = cas.get_typed_payload(&to_commit.tree_hash, OBJ_TREE)?;

        Self::diff_trees(
            cas,
            from_ref,
            to_ref,
            &from_commit.tree_hash,
            &to_commit.tree_hash,
            &from_tree,
            &to_tree,
        )
    }

    pub fn diff_trees(
        cas: &CasStore,
        from_label: &str,
        to_label: &str,
        from_tree_hash: &str,
        to_tree_hash: &str,
        from_tree: &TreeSnapshotPayload,
        to_tree: &TreeSnapshotPayload,
    ) -> Result<DiffReport> {
        let mut changes = Vec::new();

        // Compare tasks
        let all_task_ids: BTreeSet<&String> =
            from_tree.tasks.keys().chain(to_tree.tasks.keys()).collect();
        for tid in all_task_ids {
            match (from_tree.tasks.get(tid), to_tree.tasks.get(tid)) {
                (None, Some(new_h)) => {
                    let t: TaskPayload = cas.get_typed_payload(new_h, OBJ_TASK)?;
                    changes.push(EntityChange {
                        entity_type: OBJ_TASK.to_string(),
                        entity_id: tid.clone(),
                        change_kind: "added".to_string(),
                        old_hash: None,
                        new_hash: Some(new_h.clone()),
                        details: vec![format!(
                            "Added task '{}' [priority={}, col={}]",
                            t.title,
                            t.priority.as_str(),
                            t.column_id
                        )],
                    });
                }
                (Some(old_h), None) => {
                    let t: TaskPayload = cas.get_typed_payload(old_h, OBJ_TASK)?;
                    changes.push(EntityChange {
                        entity_type: OBJ_TASK.to_string(),
                        entity_id: tid.clone(),
                        change_kind: "deleted".to_string(),
                        old_hash: Some(old_h.clone()),
                        new_hash: None,
                        details: vec![format!("Deleted task '{}'", t.title)],
                    });
                }
                (Some(old_h), Some(new_h)) if old_h != new_h => {
                    let old_t: TaskPayload = cas.get_typed_payload(old_h, OBJ_TASK)?;
                    let new_t: TaskPayload = cas.get_typed_payload(new_h, OBJ_TASK)?;
                    let mut details = Vec::new();
                    if old_t.title != new_t.title {
                        details.push(format!("title: '{}' -> '{}'", old_t.title, new_t.title));
                    }
                    if old_t.project_id != new_t.project_id {
                        details.push(format!(
                            "project: '{}' -> '{}'",
                            old_t.project_id, new_t.project_id
                        ));
                    }
                    if old_t.column_id != new_t.column_id {
                        details.push(format!(
                            "column: '{}' -> '{}'",
                            old_t.column_id, new_t.column_id
                        ));
                    }
                    if old_t.priority != new_t.priority {
                        details.push(format!(
                            "priority: '{}' -> '{}'",
                            old_t.priority.as_str(),
                            new_t.priority.as_str()
                        ));
                    }
                    if old_t.assignee != new_t.assignee {
                        details.push(format!(
                            "assignee: {:?} -> {:?}",
                            old_t.assignee, new_t.assignee
                        ));
                    }
                    if old_t.due_date != new_t.due_date {
                        details.push(format!(
                            "due_date: {:?} -> {:?}",
                            old_t.due_date, new_t.due_date
                        ));
                    }
                    if old_t.tags != new_t.tags {
                        details.push(format!("tags: {:?} -> {:?}", old_t.tags, new_t.tags));
                    }
                    if old_t.depends_on != new_t.depends_on {
                        details.push(format!(
                            "depends_on: {:?} -> {:?}",
                            old_t.depends_on, new_t.depends_on
                        ));
                    }
                    if old_t.comments.len() != new_t.comments.len() {
                        details.push(format!(
                            "comments: {} -> {}",
                            old_t.comments.len(),
                            new_t.comments.len()
                        ));
                    }
                    changes.push(EntityChange {
                        entity_type: OBJ_TASK.to_string(),
                        entity_id: tid.clone(),
                        change_kind: "modified".to_string(),
                        old_hash: Some(old_h.clone()),
                        new_hash: Some(new_h.clone()),
                        details,
                    });
                }
                _ => {}
            }
        }

        // Compare boards
        let all_board_ids: BTreeSet<&String> = from_tree
            .boards
            .keys()
            .chain(to_tree.boards.keys())
            .collect();
        for bid in all_board_ids {
            match (from_tree.boards.get(bid), to_tree.boards.get(bid)) {
                (None, Some(new_h)) => {
                    changes.push(EntityChange {
                        entity_type: OBJ_BOARD.to_string(),
                        entity_id: bid.clone(),
                        change_kind: "added".to_string(),
                        old_hash: None,
                        new_hash: Some(new_h.clone()),
                        details: vec!["Created Kanban board".to_string()],
                    });
                }
                (Some(old_h), None) => {
                    changes.push(EntityChange {
                        entity_type: OBJ_BOARD.to_string(),
                        entity_id: bid.clone(),
                        change_kind: "deleted".to_string(),
                        old_hash: Some(old_h.clone()),
                        new_hash: None,
                        details: vec!["Removed Kanban board".to_string()],
                    });
                }
                (Some(old_h), Some(new_h)) if old_h != new_h => {
                    let old_b: KanbanBoardPayload = cas.get_typed_payload(old_h, OBJ_BOARD)?;
                    let new_b: KanbanBoardPayload = cas.get_typed_payload(new_h, OBJ_BOARD)?;
                    let mut details = Self::board_details(&old_b, &new_b);
                    if details.is_empty() {
                        details.push("Updated Kanban columns or workflow rules".to_string());
                    }
                    changes.push(EntityChange {
                        entity_type: OBJ_BOARD.to_string(),
                        entity_id: bid.clone(),
                        change_kind: "modified".to_string(),
                        old_hash: Some(old_h.clone()),
                        new_hash: Some(new_h.clone()),
                        details,
                    });
                }
                _ => {}
            }
        }

        // Compare projects
        Self::diff_named::<ProjectPayload>(
            cas,
            OBJ_PROJECT,
            &from_tree.projects,
            &to_tree.projects,
            |p| format!("{} '{}'", p.code, p.name),
            |old, new| {
                let mut d = Vec::new();
                if old.name != new.name {
                    d.push(format!("name: '{}' -> '{}'", old.name, new.name));
                }
                if old.description != new.description {
                    d.push("description changed".to_string());
                }
                if old.board_ids != new.board_ids {
                    d.push(format!("boards: {:?} -> {:?}", old.board_ids, new.board_ids));
                }
                d
            },
            &mut changes,
        )?;

        // Compare workspaces
        Self::diff_named::<WorkspacePayload>(
            cas,
            OBJ_WORKSPACE,
            &from_tree.workspaces,
            &to_tree.workspaces,
            |w| format!("'{}'", w.name),
            |old, new| {
                let mut d = Vec::new();
                if old.name != new.name {
                    d.push(format!("name: '{}' -> '{}'", old.name, new.name));
                }
                if old.project_ids != new.project_ids {
                    d.push(format!("projects: {:?} -> {:?}", old.project_ids, new.project_ids));
                }
                if old.description != new.description {
                    d.push("description changed".to_string());
                }
                d
            },
            &mut changes,
        )?;

        Ok(DiffReport {
            from_ref: from_label.to_string(),
            to_ref: to_label.to_string(),
            from_tree_hash: from_tree_hash.to_string(),
            to_tree_hash: to_tree_hash.to_string(),
            changes,
        })
    }

    /// Added / deleted / modified entries of one entity map (projects, workspaces).
    #[allow(clippy::too_many_arguments)]
    fn diff_named<T: serde::de::DeserializeOwned>(
        cas: &CasStore,
        entity_type: &str,
        from: &BTreeMap<String, String>,
        to: &BTreeMap<String, String>,
        label: impl Fn(&T) -> String,
        details: impl Fn(&T, &T) -> Vec<String>,
        changes: &mut Vec<EntityChange>,
    ) -> Result<()> {
        let ids: BTreeSet<&String> = from.keys().chain(to.keys()).collect();
        for id in ids {
            let (kind, old_hash, new_hash, d) = match (from.get(id), to.get(id)) {
                (None, Some(new_h)) => {
                    let v: T = cas.get_typed_payload(new_h, entity_type)?;
                    ("added", None, Some(new_h), vec![format!("Created {} {}", entity_type, label(&v))])
                }
                (Some(old_h), None) => {
                    let v: T = cas.get_typed_payload(old_h, entity_type)?;
                    ("deleted", Some(old_h), None, vec![format!("Removed {} {}", entity_type, label(&v))])
                }
                (Some(old_h), Some(new_h)) if old_h != new_h => {
                    let old: T = cas.get_typed_payload(old_h, entity_type)?;
                    let new: T = cas.get_typed_payload(new_h, entity_type)?;
                    ("modified", Some(old_h), Some(new_h), details(&old, &new))
                }
                _ => continue,
            };
            changes.push(EntityChange {
                entity_type: entity_type.to_string(),
                entity_id: id.clone(),
                change_kind: kind.to_string(),
                old_hash: old_hash.cloned(),
                new_hash: new_hash.cloned(),
                details: d,
            });
        }
        Ok(())
    }

    /// Readable list of what changed between two versions of a board.
    fn board_details(old: &KanbanBoardPayload, new: &KanbanBoardPayload) -> Vec<String> {
        let mut d = Vec::new();
        if old.name != new.name {
            d.push(format!("name: '{}' -> '{}'", old.name, new.name));
        }
        for c in &new.columns {
            match old.columns.iter().find(|o| o.id == c.id) {
                None => d.push(format!("added column {} ('{}')", c.id, c.name)),
                Some(o) => {
                    if o.name != c.name {
                        d.push(format!("column {}: name '{}' -> '{}'", c.id, o.name, c.name));
                    }
                    if o.wip_limit != c.wip_limit {
                        d.push(format!("column {}: wip {:?} -> {:?}", c.id, o.wip_limit, c.wip_limit));
                    }
                    if o.is_completed_state != c.is_completed_state {
                        d.push(format!("column {}: done {} -> {}", c.id, o.is_completed_state, c.is_completed_state));
                    }
                }
            }
        }
        for o in &old.columns {
            if !new.columns.iter().any(|c| c.id == o.id) {
                d.push(format!("removed column {} ('{}')", o.id, o.name));
            }
        }
        let order = |b: &KanbanBoardPayload| {
            let mut cols: Vec<&KanbanColumn> = b.columns.iter().collect();
            cols.sort_by_key(|c| c.order);
            cols.into_iter().map(|c| c.id.clone()).collect::<Vec<_>>()
        };
        let (old_order, new_order) = (order(old), order(new));
        let kept_old: Vec<&String> = old_order.iter().filter(|id| new_order.contains(id)).collect();
        let kept_new: Vec<&String> = new_order.iter().filter(|id| old_order.contains(id)).collect();
        if kept_old != kept_new {
            d.push(format!("column order: {} -> {}", old_order.join(","), new_order.join(",")));
        }
        let (ow, nw) = (&old.workflow, &new.workflow);
        if ow.enforce_dependencies != nw.enforce_dependencies {
            d.push(format!("rule enforce_deps: {} -> {}", ow.enforce_dependencies, nw.enforce_dependencies));
        }
        if ow.enforce_wip_limits != nw.enforce_wip_limits {
            d.push(format!("rule enforce_wip: {} -> {}", ow.enforce_wip_limits, nw.enforce_wip_limits));
        }
        if ow.require_assignee_in_progress != nw.require_assignee_in_progress {
            d.push(format!(
                "rule require_assignee: {} -> {}",
                ow.require_assignee_in_progress, nw.require_assignee_in_progress
            ));
        }
        d
    }

}
