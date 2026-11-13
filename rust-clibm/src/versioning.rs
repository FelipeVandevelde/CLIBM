//! Versioning & History Engine (Specification 2.1.4)
//!
//! Provides Git-style commit creation, history log traversal, structural diffs
//! between any two commits/branches, individual task history lookup, and branch
//! merges without duplicating unchanged objects.

use crate::cas::{validate_ref_name, CasStore};
use crate::index::ComputedIndex;
use crate::models::*;
use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

    /// All commits reachable from `start` (including `start`).
    pub fn ancestors(cas: &CasStore, start: &str) -> Result<BTreeSet<String>> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([start.to_string()]);
        while let Some(curr) = queue.pop_front() {
            if !seen.insert(curr.clone()) {
                continue;
            }
            let commit: CommitPayload = cas.get_typed_payload(&curr, OBJ_COMMIT)?;
            queue.extend(commit.parent_hashes);
        }
        Ok(seen)
    }

    /// Nearest commit reachable from `source` that is also in `target_ancestors`.
    fn merge_base(
        cas: &CasStore,
        target_ancestors: &BTreeSet<String>,
        source: &str,
    ) -> Result<Option<String>> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([source.to_string()]);
        while let Some(curr) = queue.pop_front() {
            if !seen.insert(curr.clone()) {
                continue;
            }
            if target_ancestors.contains(&curr) {
                return Ok(Some(curr));
            }
            let commit: CommitPayload = cas.get_typed_payload(&curr, OBJ_COMMIT)?;
            queue.extend(commit.parent_hashes);
        }
        Ok(None)
    }

    /// Three-way merge of one `id -> object hash` map.
    fn merge_map(
        entity_type: &str,
        base: &BTreeMap<String, String>,
        ours: &BTreeMap<String, String>,
        theirs: &BTreeMap<String, String>,
        strategy: MergeStrategy,
        conflicts: &mut Vec<MergeConflict>,
    ) -> BTreeMap<String, String> {
        let keys: BTreeSet<&String> = base.keys().chain(ours.keys()).chain(theirs.keys()).collect();
        let mut out = BTreeMap::new();
        for key in keys {
            let (b, o, t) = (base.get(key), ours.get(key), theirs.get(key));
            let pick = if o == t {
                o
            } else if o == b {
                t
            } else if t == b {
                o
            } else {
                let reason = match (b, o, t) {
                    (None, Some(_), Some(_)) => "added on both branches with different content",
                    (Some(_), None, Some(_)) => "deleted here but modified on the merged branch",
                    (Some(_), Some(_), None) => "modified here but deleted on the merged branch",
                    _ => "modified differently on both branches",
                };
                conflicts.push(MergeConflict {
                    entity_type: entity_type.to_string(),
                    entity_id: key.clone(),
                    reason: reason.to_string(),
                });
                match strategy {
                    MergeStrategy::Theirs => t,
                    _ => o,
                }
            };
            if let Some(h) = pick {
                out.insert(key.clone(), h.clone());
            }
        }
        out
    }

    /// Renumbers tasks that both branches added under the same ID with different content,
    /// so neither is lost. The incoming task gets the next free number for its project
    /// code, and `depends_on` references in the incoming side's new/changed tasks follow.
    fn renumber_task_collisions(
        cas: &CasStore,
        base: &BTreeMap<String, String>,
        ours: &BTreeMap<String, String>,
        theirs: &BTreeMap<String, String>,
        counters: &mut BTreeMap<String, u64>,
    ) -> Result<(BTreeMap<String, String>, Vec<(String, String)>)> {
        let mut theirs = theirs.clone();
        let collisions: Vec<String> = ours
            .iter()
            .filter(|(k, h)| !base.contains_key(*k) && theirs.get(*k).map_or(false, |t| t != *h))
            .map(|(k, _)| k.clone())
            .collect();

        let mut renumbered: Vec<(String, String)> = Vec::new();
        for old_id in collisions {
            let hash = theirs.remove(&old_id).expect("collision key present");
            let mut task: TaskPayload = cas.get_typed_payload(&hash, OBJ_TASK)?;
            let code = split_entity_id(&old_id)
                .map(|(p, _)| p.to_string())
                .unwrap_or_else(|| old_id.clone());
            let taken: Vec<String> = base
                .keys()
                .chain(ours.keys())
                .chain(theirs.keys())
                .cloned()
                .collect();
            let issued = counters
                .get(&code.to_uppercase())
                .map(|n| n + 1)
                .unwrap_or(FIRST_TASK_NUMBER);
            let number = next_task_number_in(taken.iter(), &code).max(issued);
            counters.insert(code.to_uppercase(), number);
            let new_id = format!("{}-{}", code, number);
            task.id = new_id.clone();
            let new_hash = cas.put_object(OBJ_TASK, &task)?;
            theirs.insert(new_id.clone(), new_hash);
            renumbered.push((old_id, new_id));
        }

        if !renumbered.is_empty() {
            let mapping: BTreeMap<&String, &String> =
                renumbered.iter().map(|(o, n)| (o, n)).collect();
            for (id, hash) in theirs.clone() {
                if base.get(&id) == Some(&hash) {
                    continue;
                }
                let mut task: TaskPayload = cas.get_typed_payload(&hash, OBJ_TASK)?;
                let mut changed = false;
                for dep in task.depends_on.iter_mut() {
                    if let Some(new_id) = mapping.get(dep) {
                        *dep = (*new_id).clone();
                        changed = true;
                    }
                }
                if changed {
                    theirs.insert(id, cas.put_object(OBJ_TASK, &task)?);
                }
            }
        }
        Ok((theirs, renumbered))
    }

    /// Merges `source_branch` into HEAD with a three-way merge against the nearest common
    /// ancestor:
    /// - a change made on only one side (including deletions) is applied;
    /// - a change made differently on both sides is a conflict: by default the merge stops
    ///   and lists the conflicts; [`MergeStrategy::Ours`] / [`MergeStrategy::Theirs`]
    ///   resolve them automatically;
    /// - tasks added on both sides under the same ID are renumbered, not lost;
    /// - when HEAD is an ancestor of the branch, the merge fast-forwards.
    ///
    /// Callers should refuse to merge over uncommitted changes (see `ClibmEngine::merge`).
    pub fn merge_branch(
        cas: &CasStore,
        source_branch: &str,
        author: &str,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        validate_ref_name(source_branch)?;
        let target_commit_hash = cas
            .resolve_head_commit()?
            .ok_or_else(|| anyhow::anyhow!("Cannot merge into an unborn HEAD"))?;
        let source_commit_hash = cas
            .get_branch_commit(source_branch)?
            .ok_or_else(|| anyhow::anyhow!("Branch '{}' does not exist", source_branch))?;

        let up_to_date = MergeOutcome {
            kind: MergeKind::UpToDate,
            commit: None,
            renumbered: Vec::new(),
            resolved_conflicts: Vec::new(),
        };
        if target_commit_hash == source_commit_hash {
            return Ok(up_to_date);
        }
        let target_ancestors = Self::ancestors(cas, &target_commit_hash)?;
        if target_ancestors.contains(&source_commit_hash) {
            return Ok(up_to_date);
        }

        let source_commit: CommitPayload = cas.get_typed_payload(&source_commit_hash, OBJ_COMMIT)?;
        let source_tree: TreeSnapshotPayload =
            cas.get_typed_payload(&source_commit.tree_hash, OBJ_TREE)?;

        // Fast-forward: HEAD is an ancestor of the branch.
        if Self::ancestors(cas, &source_commit_hash)?.contains(&target_commit_hash) {
            match cas.current_branch()? {
                Some(branch) => cas.set_branch_commit(&branch, &source_commit_hash)?,
                None => cas.set_detached_head(&source_commit_hash)?,
            }
            cas.set_staged_tree_hash(&source_commit.tree_hash)?;
            ComputedIndex::build_and_save(cas, &source_commit.tree_hash, &source_tree)?;
            return Ok(MergeOutcome {
                kind: MergeKind::FastForward,
                commit: Some(source_commit_hash),
                renumbered: Vec::new(),
                resolved_conflicts: Vec::new(),
            });
        }

        let target_commit: CommitPayload = cas.get_typed_payload(&target_commit_hash, OBJ_COMMIT)?;
        let target_tree: TreeSnapshotPayload =
            cas.get_typed_payload(&target_commit.tree_hash, OBJ_TREE)?;
        let base_tree: TreeSnapshotPayload =
            match Self::merge_base(cas, &target_ancestors, &source_commit_hash)? {
                Some(base_hash) => {
                    let base_commit: CommitPayload = cas.get_typed_payload(&base_hash, OBJ_COMMIT)?;
                    cas.get_typed_payload(&base_commit.tree_hash, OBJ_TREE)?
                }
                None => TreeSnapshotPayload::default(),
            };

        let mut conflicts = Vec::new();
        let mut merged = target_tree.clone();
        merged.workspaces = Self::merge_map(
            OBJ_WORKSPACE,
            &base_tree.workspaces,
            &target_tree.workspaces,
            &source_tree.workspaces,
            strategy,
            &mut conflicts,
        );
        merged.projects = Self::merge_map(
            OBJ_PROJECT,
            &base_tree.projects,
            &target_tree.projects,
            &source_tree.projects,
            strategy,
            &mut conflicts,
        );
        merged.boards = Self::merge_map(
            OBJ_BOARD,
            &base_tree.boards,
            &target_tree.boards,
            &source_tree.boards,
            strategy,
            &mut conflicts,
        );
        // Task counters only grow: keep the highest number issued on either side.
        let mut counters = target_tree.task_counters.clone();
        for (code, n) in base_tree.task_counters.iter().chain(source_tree.task_counters.iter()) {
            let entry = counters.entry(code.clone()).or_insert(0);
            *entry = (*entry).max(*n);
        }
        let (theirs_tasks, renumbered) = Self::renumber_task_collisions(
            cas,
            &base_tree.tasks,
            &target_tree.tasks,
            &source_tree.tasks,
            &mut counters,
        )?;
        merged.task_counters = counters;
        merged.tasks = Self::merge_map(
            OBJ_TASK,
            &base_tree.tasks,
            &target_tree.tasks,
            &theirs_tasks,
            strategy,
            &mut conflicts,
        );

        if !conflicts.is_empty() && strategy == MergeStrategy::Abort {
            let list: Vec<String> = conflicts
                .iter()
                .map(|c| format!("  {} {}: {}", c.entity_type, c.entity_id, c.reason))
                .collect();
            bail!(
                "Merge of '{}' stopped: {} conflict(s). Nothing was changed.\n{}\n\
                 Re-run with --ours (keep this branch's version) or --theirs (take '{}') to resolve them.",
                source_branch,
                conflicts.len(),
                list.join("\n"),
                source_branch
            );
        }

        let merged_tree_hash = cas.put_object(OBJ_TREE, &merged)?;
        cas.set_staged_tree_hash(&merged_tree_hash)?;
        ComputedIndex::build_and_save(cas, &merged_tree_hash, &merged)?;

        let mut msg = format!("Merge branch '{}'", source_branch);
        if !renumbered.is_empty() {
            let pairs: Vec<String> = renumbered.iter().map(|(o, n)| format!("{} -> {}", o, n)).collect();
            msg.push_str(&format!(" (renumbered: {})", pairs.join(", ")));
        }
        let merge_commit_hash = Self::create_commit(
            cas,
            &merged_tree_hash,
            author,
            &msg,
            Some(source_commit_hash),
        )?;

        Ok(MergeOutcome {
            kind: MergeKind::Merged,
            commit: Some(merge_commit_hash),
            renumbered,
            resolved_conflicts: conflicts,
        })
    }
}

/// How `clibm merge` handles entities changed differently on both branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Stop and list the conflicts (default).
    Abort,
    /// Keep the current branch's version.
    Ours,
    /// Take the merged branch's version.
    Theirs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflict {
    pub entity_type: String,
    pub entity_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeKind {
    UpToDate,
    FastForward,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub kind: MergeKind,
    pub commit: Option<String>,
    /// `(old_id, new_id)` for incoming tasks renumbered to avoid an ID collision.
    pub renumbered: Vec<(String, String)>,
    /// Conflicts resolved automatically by `--ours` / `--theirs`.
    pub resolved_conflicts: Vec<MergeConflict>,
}
