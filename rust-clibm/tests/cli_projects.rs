//! End-to-end tests of the project and board-configuration commands.

mod common;

use common::TempProject;

fn column_ids(p: &TempProject) -> Vec<String> {
    p.json(&["board", "column", "list"])["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn project_create_switch_and_task_numbering() {
    let p = TempProject::new("prj-create");
    p.init();
    p.ok(&["task", "add", "App task"]);

    // Not active unless --switch.
    p.ok(&["project", "create", "Payments API", "-c", "api"]);
    let st = p.json(&["status"]);
    assert_eq!(st["active_project_code"], "APP");

    p.ok(&["project", "switch", "API"]);
    let created = p.json(&["task", "add", "Charge card"]);
    assert_eq!(created["id"], "API-101");
    assert_eq!(created["task"]["column_id"], "todo");

    // task list is scoped to the active project.
    let tasks = p.json(&["task", "list"]);
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    let all = p.json(&["task", "list", "--all-workspaces"]);
    assert_eq!(all.as_array().unwrap().len(), 2);

    let list = p.json(&["project", "list"]);
    let rows = list.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let api = rows.iter().find(|r| r["project"]["code"] == "API").unwrap();
    assert_eq!(api["active"], true);
    assert_eq!(api["tasks"], 1);

    // Each project has its own board.
    let board = p.json(&["board", "show"]);
    assert_eq!(board["project"]["code"], "API");
    assert_ne!(board["board"]["id"], "BRD-01");

    // Switch by name and by ID.
    p.ok(&["project", "switch", "app"]);
    assert_eq!(p.json(&["status"])["active_project_code"], "APP");
    p.ok(&["project", "switch", "PRJ-02"]);
    assert_eq!(p.json(&["status"])["active_project_code"], "API");
}

#[test]
fn project_codes_are_unique_and_validated() {
    let p = TempProject::new("prj-codes");
    p.init();
    let err = p.fails(&["project", "create", "Again", "-c", "app"]);
    assert!(err.contains("already used"), "{}", err);
    let err = p.fails(&["workspace", "create", "backend", "-c", "APP"]);
    assert!(err.contains("already used"), "{}", err);
    let err = p.fails(&["project", "create", "Bad", "-c", "A-B"]);
    assert!(err.contains("Invalid project code"), "{}", err);
}

#[test]
fn project_switch_across_workspaces() {
    let p = TempProject::new("prj-ws");
    p.init();
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);
    p.ok(&["project", "switch", "BE"]);
    let st = p.json(&["status"]);
    assert_eq!(st["active_project_code"], "BE");
    assert_eq!(st["active_workspace_name"], "backend");
    // project list shows only the active workspace unless asked.
    assert_eq!(p.json(&["project", "list"]).as_array().unwrap().len(), 1);
    assert_eq!(
        p.json(&["project", "list", "--all-workspaces"]).as_array().unwrap().len(),
        2
    );
}

#[test]
fn project_edit_and_delete() {
    let p = TempProject::new("prj-delete");
    p.init();
    let err = p.fails(&["project", "delete", "APP"]);
    assert!(err.contains("only project"), "{}", err);

    p.ok(&["project", "create", "Design", "-c", "DS", "--switch"]);
    p.ok(&["project", "edit", "DS", "--name", "Design System", "-d", "Tokens and components"]);
    let rows = p.json(&["project", "list"]);
    let ds = rows.as_array().unwrap().iter().find(|r| r["id"] == "PRJ-02").unwrap().clone();
    assert_eq!(ds["project"]["name"], "Design System");
    assert_eq!(ds["project"]["code"], "DS");

    p.ok(&["task", "add", "Tokens"]);
    p.ok(&["board", "column", "add", "Design QA"]);
    p.ok(&["board", "move", "DS-101", "design-qa"]);

    let err = p.fails(&["project", "delete", "DS"]);
    assert!(err.contains("--move-to"), "{}", err);

    let res = p.json(&["project", "delete", "DS", "--move-to", "APP"]);
    assert_eq!(res["moved_tasks"][0], "DS-101");
    assert_eq!(res["active_project_id"], "PRJ-01");
    // The task keeps its ID; its column did not exist on APP's board.
    let t = p.json(&["task", "show", "DS-101"]);
    assert_eq!(t["project_id"], "PRJ-01");
    assert_eq!(t["column_id"], "todo");
    assert_eq!(p.json(&["project", "list"]).as_array().unwrap().len(), 1);
    // A new DS project does not reuse DS-101.
    p.ok(&["project", "create", "Design 2", "-c", "DS", "--switch"]);
    assert_eq!(p.json(&["task", "add", "Next"])["id"], "DS-102");
    p.ok(&["fsck"]);
}

#[test]
fn board_columns_add_edit_move_remove() {
    let p = TempProject::new("cols");
    p.init();
    // Default position: before the first completed column.
    p.ok(&["board", "column", "add", "QA Check", "--wip", "2"]);
    assert_eq!(column_ids(&p), ["backlog", "todo", "in-progress", "review", "qa-check", "done"]);
    p.ok(&["board", "column", "add", "Icebox", "--position", "0"]);
    p.ok(&["board", "column", "add", "Ready", "--id", "ready", "--after", "todo"]);
    assert_eq!(
        column_ids(&p),
        ["icebox", "backlog", "todo", "ready", "in-progress", "review", "qa-check", "done"]
    );
    let err = p.fails(&["board", "column", "add", "qa check"]);
    assert!(err.contains("already exists"), "{}", err);

    p.ok(&["board", "column", "edit", "QA Check", "--name", "QA", "--wip", "none"]);
    let cols = p.json(&["board", "column", "list"]);
    let qa = cols["columns"].as_array().unwrap().iter().find(|c| c["id"] == "qa-check").unwrap().clone();
    assert_eq!(qa["name"], "QA");
    assert!(qa["wip_limit"].is_null());

    p.ok(&["board", "column", "move", "icebox", "99"]);
    assert_eq!(column_ids(&p).last().unwrap(), "icebox");

    // Remove: refused with tasks, works with --move-to.
    p.ok(&["task", "add", "Sits in ready", "-c", "ready"]);
    let err = p.fails(&["board", "column", "remove", "ready"]);
    assert!(err.contains("--move-to"), "{}", err);
    p.ok(&["board", "column", "remove", "ready", "--move-to", "todo"]);
    assert_eq!(p.json(&["task", "show", "101"])["column_id"], "todo");
    assert!(!column_ids(&p).contains(&"ready".to_string()));

    // The last completed column is protected.
    let err = p.fails(&["board", "column", "edit", "done", "--done", "false"]);
    assert!(err.contains("completed column"), "{}", err);
    let err = p.fails(&["board", "column", "remove", "done"]);
    assert!(err.contains("completed column"), "{}", err);

    p.ok(&["board", "rename", "Delivery"]);
    assert_eq!(p.json(&["board", "show"])["board"]["name"], "Delivery");
    p.ok(&["fsck"]);
}

#[test]
fn wip_limit_edit_refused_below_task_count() {
    let p = TempProject::new("wip-edit");
    p.init();
    p.ok(&["task", "add", "One"]);
    p.ok(&["task", "add", "Two"]);
    let err = p.fails(&["board", "column", "edit", "todo", "--wip", "1"]);
    assert!(err.contains("more than the new limit"), "{}", err);
    p.ok(&["board", "column", "edit", "todo", "--wip", "1", "--force"]);
    let err = p.fails(&["task", "add", "Three"]);
    assert!(err.contains("WIP limit"), "{}", err);
}

#[test]
fn default_column_and_dependency_rule_follow_the_board() {
    let p = TempProject::new("default-col");
    p.init();
    p.ok(&["board", "column", "add", "Ready", "--after", "backlog"]);
    p.ok(&["board", "column", "remove", "todo"]);
    // Without todo, new tasks go to the first open non-backlog column.
    let t = p.json(&["task", "add", "Base"]);
    assert_eq!(t["task"]["column_id"], "ready");
    p.ok(&["task", "add", "Blocked", "--depends-on", "101"]);
    // backlog and ready (the default column) accept tasks with open dependencies...
    p.ok(&["board", "move", "102", "backlog"]);
    p.ok(&["board", "move", "102", "ready"]);
    // ...columns after it do not.
    let err = p.fails(&["board", "move", "102", "in-progress"]);
    assert!(err.contains("Blocked by incomplete dependency"), "{}", err);
}

#[test]
fn board_rules_show_and_change() {
    let p = TempProject::new("rules");
    p.init();
    let rules = p.json(&["board", "rules"]);
    assert_eq!(rules["require_assignee_in_progress"], false);

    p.ok(&["board", "rules", "--require-assignee", "true"]);
    p.ok(&["task", "add", "Nobody"]);
    let err = p.fails(&["board", "move", "101", "in-progress"]);
    assert!(err.contains("needs an assignee"), "{}", err);
    p.ok(&["task", "edit", "101", "-a", "ana"]);
    p.ok(&["board", "move", "101", "in-progress"]);

    p.ok(&["board", "rules", "--enforce-wip", "off"]);
    assert_eq!(p.json(&["board", "rules"])["enforce_wip_limits"], false);
    let err = p.fails(&["board", "rules", "--enforce-deps", "maybe"]);
    assert!(err.contains("Invalid boolean"), "{}", err);
}

#[test]
fn transfer_into_a_named_project() {
    let p = TempProject::new("transfer-prj");
    p.init();
    p.ok(&["workspace", "create", "backend", "-c", "BE"]);
    p.ok(&["project", "switch", "BE"]);
    p.ok(&["project", "create", "Billing", "-c", "BILL"]);
    p.ok(&["project", "switch", "APP"]);
    p.ok(&["task", "add", "Move me"]);

    let err = p.fails(&["task", "transfer", "101", "backend", "--project", "APP"]);
    assert!(err.contains("belongs to workspace"), "{}", err);

    let moved = p.json(&["task", "transfer", "101", "backend", "--project", "BILL"]);
    assert_eq!(moved["task"]["project_id"], "PRJ-03");
    let copy = p.json(&["task", "transfer", "APP-101", "backend", "--project", "Billing", "--copy"]);
    assert_eq!(copy["id"], "BILL-101");
}

#[test]
fn diff_reports_projects_and_column_changes() {
    let p = TempProject::new("diff-prj");
    p.init();
    let before = p.json(&["log"])[0]["hash"].as_str().unwrap().to_string();
    p.ok(&["project", "create", "Api", "-c", "API"]);
    p.ok(&["board", "column", "add", "QA"]);
    p.ok(&["board", "column", "edit", "in-progress", "--wip", "5"]);
    p.ok(&["board", "rules", "--require-assignee", "true"]);

    let diff = p.json(&["diff", &before[..12], "HEAD"]);
    assert_eq!(diff["changes"], p.json(&["diff", "HEAD~4", "HEAD"])["changes"]);
    let changes = diff["changes"].as_array().unwrap();
    let find = |ty: &str, id: &str| {
        changes
            .iter()
            .find(|c| c["entity_type"] == ty && c["entity_id"] == id)
            .unwrap_or_else(|| panic!("no {} {} in {:#}", ty, id, diff))
            .clone()
    };
    assert_eq!(find("project", "PRJ-02")["change_kind"], "added");
    assert_eq!(find("workspace", "WS-01")["change_kind"], "modified");
    let board = find("board", "BRD-01");
    let details: Vec<&str> = board["details"].as_array().unwrap().iter().map(|d| d.as_str().unwrap()).collect();
    assert!(details.iter().any(|d| d.contains("added column qa")), "{:?}", details);
    assert!(details.iter().any(|d| d.contains("wip Some(4) -> Some(5)")), "{:?}", details);
    assert!(details.iter().any(|d| d.contains("require_assignee: false -> true")), "{:?}", details);
}

#[test]
fn unchanged_board_edit_makes_no_commit() {
    let p = TempProject::new("noop-board");
    p.init();
    let res = p.json(&["board", "column", "edit", "todo", "--wip", "8"]);
    assert_eq!(res["changed"], false);
    assert!(res["commit"].is_null());
    let res = p.json(&["board", "column", "move", "todo", "1"]);
    assert_eq!(res["changed"], false);
}
