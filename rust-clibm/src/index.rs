//! Computed Index Engine (Specification 2.2.1)
//!
//! Accelerates queries across immutable CAS objects without requiring a relational
//! database. Computes secondary lookups by project, Kanban column, priority, tags,
//! assignee, and validates the task dependency DAG for cycles.

use crate::cas::CasStore;
use crate::models::*;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputedIndex {
    pub tree_hash: String,
    pub by_project: BTreeMap<String, Vec<String>>,
    pub by_column: BTreeMap<String, Vec<String>>,
    pub by_priority: BTreeMap<String, Vec<String>>,
    pub by_tag: BTreeMap<String, Vec<String>>,
    pub by_assignee: BTreeMap<String, Vec<String>>,
    /// Maps a task ID to the list of task IDs that depend on it (reverse dependencies)
    pub blocks_tasks: BTreeMap<String, Vec<String>>,
    pub dependency_cycles: Vec<Vec<String>>,
}

impl ComputedIndex {
    /// Recomputes the index from a `TreeSnapshotPayload` and persists it to `.clibm/index/computed.json`.
    pub fn build_and_save(
        cas: &CasStore,
        tree_hash: &str,
        tree: &TreeSnapshotPayload,
    ) -> Result<Self> {
        let mut idx = ComputedIndex {
            tree_hash: tree_hash.to_string(),
            ..Default::default()
        };

        let mut loaded_tasks: BTreeMap<String, TaskPayload> = BTreeMap::new();

        for (task_id, obj_hash) in &tree.tasks {
            let task: TaskPayload = cas.get_typed_payload(obj_hash, OBJ_TASK)?;
            idx.by_project
                .entry(task.project_id.clone())
                .or_default()
                .push(task_id.clone());

            let col_key = format!("{}:{}", task.board_id, task.column_id);
            idx.by_column
                .entry(col_key)
                .or_default()
                .push(task_id.clone());

            idx.by_priority
                .entry(task.priority.as_str().to_string())
                .or_default()
                .push(task_id.clone());

            for tag in &task.tags {
                idx.by_tag
                    .entry(tag.to_lowercase())
                    .or_default()
                    .push(task_id.clone());
            }

            if let Some(ref assignee) = task.assignee {
                idx.by_assignee
                    .entry(assignee.to_lowercase())
                    .or_default()
                    .push(task_id.clone());
            }

            for dep_id in &task.depends_on {
                idx.blocks_tasks
                    .entry(dep_id.clone())
                    .or_default()
                    .push(task_id.clone());
            }

            loaded_tasks.insert(task_id.clone(), task);
        }

        idx.dependency_cycles = detect_cycles(&loaded_tasks);

        let index_file = cas.index_dir().join("computed.json");
        let serialized = serde_json::to_string_pretty(&idx)?;
        fs::write(index_file, serialized)?;

        Ok(idx)
    }

    /// Checks whether adding `depends_on_id` to `task_id` would create a cycle in the task graph.
    pub fn validate_new_dependency(
        tasks: &BTreeMap<String, TaskPayload>,
        task_id: &str,
        depends_on_id: &str,
    ) -> Result<()> {
        if task_id == depends_on_id {
            bail!("Task '{}' cannot depend on itself", task_id);
        }
        // DFS from `depends_on_id` to see if `task_id` is reachable
        let mut visited = BTreeSet::new();
        let mut stack = vec![depends_on_id.to_string()];
        while let Some(curr) = stack.pop() {
            if curr == task_id {
                bail!(
                    "Circular dependency detected: '{}' already transitively depends on '{}'",
                    depends_on_id,
                    task_id
                );
            }
            if visited.insert(curr.clone()) {
                if let Some(t) = tasks.get(&curr) {
                    for next in &t.depends_on {
                        stack.push(next.clone());
                    }
                }
            }
        }
        Ok(())
    }
}

fn detect_cycles(tasks: &BTreeMap<String, TaskPayload>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = BTreeSet::new();
    let mut on_stack = BTreeSet::new();
    let mut path = Vec::new();

    for task_id in tasks.keys() {
        if !visited.contains(task_id) {
            dfs_cycle(
                task_id,
                tasks,
                &mut visited,
                &mut on_stack,
                &mut path,
                &mut cycles,
            );
        }
    }
    cycles
}

fn dfs_cycle(
    node: &str,
    tasks: &BTreeMap<String, TaskPayload>,
    visited: &mut BTreeSet<String>,
    on_stack: &mut BTreeSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    on_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(task) = tasks.get(node) {
        for dep in &task.depends_on {
            if !visited.contains(dep) {
                dfs_cycle(dep, tasks, visited, on_stack, path, cycles);
            } else if on_stack.contains(dep) {
                if let Some(pos) = path.iter().position(|x| x == dep) {
                    let mut cycle: Vec<String> = path[pos..].to_vec();
                    cycle.push(dep.clone());
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    on_stack.remove(node);
}
