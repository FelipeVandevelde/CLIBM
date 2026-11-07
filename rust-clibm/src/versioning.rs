//! Versioning & History Engine (Specification 2.1.4)
//!
//! Provides Git-style commit creation, history log traversal, structural diffs
//! between any two commits/branches, individual task history lookup, and branch
//! merges without duplicating unchanged objects.

use crate::cas::CasStore;
use crate::models::*;
use anyhow::{bail, Result};
use chrono::Utc;
use std::collections::{BTreeSet, VecDeque};

pub struct VersioningEngine;

impl VersioningEngine {
    /// Creates an immutable commit pointing to `tree_hash` and updates `HEAD` / active branch.
    pub fn create_commit(
        cas: &CasStore,
        tree_hash: &str,
        author: &str,
        message: &str,
        extra_parent: Option<String>,
    ) -> Result<String> {
        let mut parents = Vec::new();
        if let Some(current_head) = cas.resolve_head_commit()? {
            parents.push(current_head);
        }
        if let Some(p2) = extra_parent {
            if !parents.contains(&p2) {
                parents.push(p2);
            }
        }

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
                    changes.push(EntityChange {
                        entity_type: OBJ_BOARD.to_string(),
                        entity_id: bid.clone(),
                        change_kind: "modified".to_string(),
                        old_hash: Some(old_h.clone()),
                        new_hash: Some(new_h.clone()),
                        details: vec!["Updated Kanban columns or workflow rules".to_string()],
                    });
                }
                _ => {}
            }
        }

        Ok(DiffReport {
            from_ref: from_label.to_string(),
            to_ref: to_label.to_string(),
            from_tree_hash: from_tree_hash.to_string(),
            to_tree_hash: to_tree_hash.to_string(),
            changes,
        })
    }

    /// Merges `source_branch` into the currently checked-out branch using structural CAS tree merging.
    pub fn merge_branch(cas: &CasStore, source_branch: &str, author: &str) -> Result<String> {
        let target_commit_hash = cas
            .resolve_head_commit()?
            .ok_or_else(|| anyhow::anyhow!("Cannot merge into an unborn HEAD"))?;
        let source_commit_hash = cas
            .get_branch_commit(source_branch)?
            .ok_or_else(|| anyhow::anyhow!("Branch '{}' does not exist", source_branch))?;

        if target_commit_hash == source_commit_hash {
            bail!("Branch '{}' is already up-to-date with HEAD", source_branch);
        }

        let target_commit: CommitPayload = cas.get_typed_payload(&target_commit_hash, OBJ_COMMIT)?;
        let source_commit: CommitPayload = cas.get_typed_payload(&source_commit_hash, OBJ_COMMIT)?;

        let mut merged_tree: TreeSnapshotPayload =
            cas.get_typed_payload(&target_commit.tree_hash, OBJ_TREE)?;
        let source_tree: TreeSnapshotPayload =
            cas.get_typed_payload(&source_commit.tree_hash, OBJ_TREE)?;

        for (k, v) in source_tree.workspaces {
            merged_tree.workspaces.insert(k, v);
        }
        for (k, v) in source_tree.projects {
            merged_tree.projects.insert(k, v);
        }
        for (k, v) in source_tree.boards {
            merged_tree.boards.insert(k, v);
        }
        for (k, v) in source_tree.tasks {
            merged_tree.tasks.insert(k, v);
        }

        let merged_tree_hash = cas.put_object(OBJ_TREE, &merged_tree)?;
        cas.set_staged_tree_hash(&merged_tree_hash)?;

        let msg = format!("Merge branch '{}'", source_branch);
        let merge_commit_hash = Self::create_commit(
            cas,
            &merged_tree_hash,
            author,
            &msg,
            Some(source_commit_hash),
        )?;

        Ok(merge_commit_hash)
    }
}
