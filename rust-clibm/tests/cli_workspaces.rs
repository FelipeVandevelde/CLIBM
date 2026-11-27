//! End-to-end tests of workspace management: list, edit, delete, and creating projects
//! in a chosen workspace.

mod common;

use common::TempProject;
use serde_json::Value;

fn commits(p: &TempProject) -> u64 {
    p.json(&["status"])["commits"].as_u64().unwrap()
}

fn assert_fsck_clean(p: &TempProject) {
    let r = p.json(&["fsck"]);
    assert_eq!(r["verified_objects"], r["total_objects"], "{}", r);
    assert_eq!(r["corrupted_hashes"], serde_json::json!([]), "{}", r);
    assert_eq!(r["legacy_hashes"], serde_json::json!([]), "{}", r);
}

fn find<'a>(rows: &'a Value, key: &str, value: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r[key] == value)
        .unwrap_or_else(|| panic!("no row with {} = {} in {}", key, value, rows))
}

#[test]
fn workspace_list_has_counts_and_active_marker() {
    let p = TempProject::new("ws-list");
    p.init();
    p.ok(&["task", "add", "One"]);
    p.ok(&["task", "add", "Two"]);
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);

    let rows = p.json(&["workspace", "list"]);
    assert_eq!(rows.as_array().unwrap().len(), 2);
    let first = find(&rows, "id", "WS-01");
    assert_eq!(first["active"], true);
    assert_eq!(first["projects"], 1);
    assert_eq!(first["tasks"], 2);
    let be = find(&rows, "id", "WS-02");
    assert_eq!(be["active"], false);
    assert_eq!(be["tasks"], 0);
    assert_eq!(be["workspace"]["name"], "backend");
}

#[test]
fn workspace_names_are_unique() {
    let p = TempProject::new("ws-unique");
    p.init();
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);
    let err = p.fails(&["workspace", "create", "Backend", "-c", "BE2"]);
    assert!(err.contains("already used"), "{}", err);
    let err = p.fails(&["workspace", "create", "  ", "-c", "XX"]);
    assert!(err.contains("cannot be empty"), "{}", err);
}

#[test]
fn workspace_edit() {
    let p = TempProject::new("ws-edit");
    p.init();
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);

    let before = commits(&p);
    let res = p.json(&["workspace", "edit", "backend", "--name", "Server", "-d", "API team"]);
    assert_eq!(res["id"], "WS-02");
    assert_eq!(res["changed"], true);
    assert_eq!(commits(&p), before + 1);
    let ws = find(&p.json(&["workspace", "list"]), "id", "WS-02").clone();
    assert_eq!(ws["workspace"]["name"], "Server");
    assert_eq!(ws["workspace"]["description"], "API team");

    // Same values again: nothing written, no commit.
    let res = p.json(&["workspace", "edit", "WS-02", "--name", "Server"]);
    assert_eq!(res["changed"], false);
    assert_eq!(commits(&p), before + 1);

    // Renaming to its own name in another case is fine; to another workspace's name is not.
    p.ok(&["workspace", "edit", "WS-02", "--name", "SERVER"]);
    let err = p.fails(&["workspace", "edit", "WS-02", "--name", "default-workspace"]);
    assert!(err.contains("already used"), "{}", err);
    let err = p.fails(&["workspace", "edit", "WS-02", "--name", " "]);
    assert!(err.contains("cannot be empty"), "{}", err);
    let err = p.fails(&["workspace", "edit", "nowhere", "--name", "x"]);
    assert!(err.contains("not found"), "{}", err);
    assert_fsck_clean(&p);
}

#[test]
fn workspace_delete_refusals() {
    let p = TempProject::new("ws-del-refuse");
    p.init();
    let err = p.fails(&["workspace", "delete", "WS-01"]);
    assert!(err.contains("only workspace"), "{}", err);

    p.ok(&["workspace", "create", "backend", "-c", "BE"]);
    p.ok(&["project", "switch", "BE"]);
    p.ok(&["task", "add", "Server work"]);
    let err = p.fails(&["workspace", "delete", "backend"]);
    assert!(err.contains("still has 1 task"), "{}", err);
    let err = p.fails(&["workspace", "delete", "backend", "--move-to", "WS-02"]);
    assert!(err.contains("different workspace"), "{}", err);
    // Nothing was deleted.
    assert_eq!(p.json(&["workspace", "list"]).as_array().unwrap().len(), 2);
}

#[test]
fn workspace_delete_empty_removes_its_projects() {
    let p = TempProject::new("ws-del-empty");
    p.init();
    p.ok(&["workspace", "create", "scratch", "-c", "SCR"]);
    p.ok(&["project", "create", "Extra", "-c", "EXT", "-w", "scratch"]);
    // Delete it while it is the active workspace.
    p.ok(&["workspace", "switch", "scratch"]);

    let before = commits(&p);
    let res = p.json(&["workspace", "delete", "scratch"]);
    assert_eq!(res["id"], "WS-02");
    assert_eq!(res["deleted_projects"].as_array().unwrap().len(), 2);
    assert_eq!(res["moved_projects"].as_array().unwrap().len(), 0);
    assert_eq!(res["active_workspace_id"], "WS-01");
    assert_eq!(res["active_project_id"], "PRJ-01");
    assert_eq!(commits(&p), before + 1);

    let st = p.json(&["status"]);
    assert_eq!(st["active_workspace_id"], "WS-01");
    assert_eq!(st["active_project_code"], "APP");
    assert_eq!(st["workspaces"], 1);
    let projects = p.json(&["project", "list", "--all-workspaces"]);
    assert_eq!(projects.as_array().unwrap().len(), 1);
    // The deleted project's code is free again.
    p.ok(&["project", "create", "Scratch again", "-c", "SCR"]);
    assert_fsck_clean(&p);
}

#[test]
fn workspace_delete_with_move_to_keeps_projects_and_tasks() {
    let p = TempProject::new("ws-del-move");
    p.init();
    p.ok(&["task", "add", "App task"]);
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);
    p.ok(&["project", "switch", "BE"]);
    p.ok(&["task", "add", "Base"]);
    p.ok(&["task", "add", "Depends", "--depends-on", "BE-101"]);
    p.ok(&["project", "create", "Jobs", "-c", "JOB", "--switch"]);
    p.ok(&["task", "add", "Cron"]);

    let before = commits(&p);
    let res = p.json(&["workspace", "delete", "backend", "--move-to", "default-workspace"]);
    assert_eq!(res["moved_projects"], serde_json::json!(["PRJ-02", "PRJ-03"]));
    assert_eq!(res["deleted_projects"].as_array().unwrap().len(), 0);
    // The active project moved along and stays active.
    assert_eq!(res["active_workspace_id"], "WS-01");
    assert_eq!(res["active_project_id"], "PRJ-03");
    assert_eq!(commits(&p), before + 1);

    let projects = p.json(&["project", "list", "--all-workspaces"]);
    assert_eq!(projects.as_array().unwrap().len(), 3);
    for r in projects.as_array().unwrap() {
        assert_eq!(r["workspace_id"], "WS-01", "{}", r);
    }
    assert_eq!(find(&projects, "id", "PRJ-02")["tasks"], 2);

    // Task IDs and dependencies are unchanged.
    let tasks = p.json(&["task", "list", "--all-workspaces"]);
    let ids: Vec<String> = tasks
        .as_array()
        .unwrap()
        .iter()
        // Each row is a [object hash, task] pair.
        .map(|t| t[1]["id"].as_str().unwrap().to_string())
        .collect();
    for id in ["APP-101", "BE-101", "BE-102", "JOB-101"] {
        assert!(ids.iter().any(|i| i == id), "{} missing from {:?}", id, ids);
    }
    let shown = p.json(&["task", "show", "BE-102"]);
    let text = shown.to_string();
    assert!(text.contains("BE-101"), "{}", text);
    assert!(text.contains("WS-01"), "{}", text);

    let ws = p.json(&["workspace", "list"]);
    assert_eq!(ws.as_array().unwrap().len(), 1);
    assert_eq!(ws[0]["projects"], 3);
    assert_eq!(ws[0]["tasks"], 4);
    assert_fsck_clean(&p);
}

#[test]
fn project_create_in_another_workspace() {
    let p = TempProject::new("prj-in-ws");
    p.init();
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);

    // Created in backend, but the active project does not change.
    let created = p.json(&["project", "create", "Payments", "-c", "PAY", "--workspace", "backend"]);
    assert_eq!(created["project"]["workspace_id"], "WS-02");
    let st = p.json(&["status"]);
    assert_eq!(st["active_workspace_id"], "WS-01");
    assert_eq!(st["active_project_code"], "APP");
    assert_eq!(find(&p.json(&["workspace", "list"]), "id", "WS-02")["projects"], 2);

    // With --switch, both the project and its workspace become active.
    p.ok(&["project", "create", "Billing", "-c", "BIL", "-w", "WS-02", "--switch"]);
    let st = p.json(&["status"]);
    assert_eq!(st["active_workspace_id"], "WS-02");
    assert_eq!(st["active_project_code"], "BIL");

    let err = p.fails(&["project", "create", "Lost", "-c", "LOST", "-w", "nowhere"]);
    assert!(err.contains("not found"), "{}", err);
    assert_fsck_clean(&p);
}
