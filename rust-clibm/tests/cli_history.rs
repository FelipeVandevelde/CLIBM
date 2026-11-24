//! End-to-end tests of CLIBM's linear history: log, diff with HEAD~N, restore, and
//! repositories made when CLIBM still had branches.

mod common;

use common::TempProject;

fn commits(p: &TempProject) -> u64 {
    p.json(&["status"])["commits"].as_u64().unwrap()
}

fn task_ids(p: &TempProject) -> Vec<String> {
    p.json(&["task", "list", "--all-workspaces"])
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row[1]["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn restore_brings_back_a_deleted_task_as_a_new_commit() {
    let p = TempProject::new("restore-deleted");
    p.init();
    p.ok(&["task", "add", "Keep"]);
    p.ok(&["task", "add", "Oops"]);
    p.ok(&["task", "delete", "102"]);
    let before = commits(&p);

    let res = p.json(&["restore", "HEAD~1"]);
    assert_eq!(res["changed"], true);
    assert!(res["commit"].is_string());
    assert_eq!(task_ids(&p), ["APP-101", "APP-102"]);
    // History only grows: the delete is still there, followed by the restore.
    assert_eq!(commits(&p), before + 1);
    let log = p.ok(&["log"]);
    assert!(log.contains("restore state of"), "{}", log);
    assert!(log.contains("auto(task): delete APP-102"), "{}", log);
    // The task's own history shows it was added, and came back with the restore.
    let history = p.json(&["task", "history", "102"]);
    let messages: Vec<&str> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["commit_message"].as_str().unwrap())
        .collect();
    assert_eq!(messages.len(), 2, "{:?}", messages);
    assert!(messages[0].starts_with("restore state of"), "{:?}", messages);

    // Custom message.
    p.ok(&["task", "delete", "102"]);
    p.ok(&["restore", "HEAD~1", "-m", "undo delete"]);
    assert!(p.ok(&["log"]).contains("undo delete"));
}

#[test]
fn restore_never_reuses_task_numbers() {
    let p = TempProject::new("restore-numbers");
    p.init();
    p.ok(&["task", "add", "One"]);
    p.ok(&["task", "add", "Two"]);
    p.ok(&["task", "add", "Three"]);
    p.ok(&["restore", "HEAD~2"]); // state with APP-101 only
    assert_eq!(task_ids(&p), ["APP-101"]);
    let created = p.json(&["task", "add", "Four"]);
    assert_eq!(created["id"], "APP-104");
}

#[test]
fn restore_to_the_same_state_is_a_noop() {
    let p = TempProject::new("restore-noop");
    p.init();
    p.ok(&["task", "add", "One"]);
    let before = commits(&p);
    let res = p.json(&["restore", "HEAD"]);
    assert_eq!(res["changed"], false);
    assert!(res["commit"].is_null());
    assert_eq!(commits(&p), before);
}

#[test]
fn restore_refuses_uncommitted_changes_and_force_discards_them() {
    let p = TempProject::new("restore-dirty");
    p.init();
    p.ok(&["task", "add", "Saved"]);
    p.ok(&["config", "set", "auto_commit", "false"]);
    p.ok(&["task", "add", "Unsaved"]);
    let before = commits(&p);

    let err = p.fails(&["restore", "HEAD"]);
    assert!(err.contains("uncommitted changes"), "{}", err);

    let out = p.ok(&["restore", "HEAD", "--force"]);
    assert!(out.contains("Discarded the uncommitted changes"), "{}", out);
    assert_eq!(task_ids(&p), ["APP-101"]);
    assert_eq!(p.json(&["status"])["dirty"], false);
    assert_eq!(commits(&p), before);
}

#[test]
fn restore_with_auto_commit_off_is_staged() {
    let p = TempProject::new("restore-staged");
    p.init();
    p.ok(&["task", "add", "One"]);
    p.ok(&["task", "add", "Two"]);
    p.ok(&["config", "set", "auto_commit", "false"]);
    let before = commits(&p);
    let res = p.json(&["restore", "HEAD~1"]);
    assert!(res["commit"].is_null());
    assert_eq!(p.json(&["status"])["dirty"], true);
    assert_eq!(commits(&p), before);
    p.ok(&["commit", "-m", "back to one task"]);
    assert_eq!(commits(&p), before + 1);
    assert_eq!(task_ids(&p), ["APP-101"]);
}

#[test]
fn restore_keeps_the_active_project_when_it_still_exists() {
    let p = TempProject::new("restore-active");
    p.init();
    p.ok(&["project", "create", "Api", "-c", "API"]);
    p.ok(&["task", "add", "App task"]);
    p.ok(&["project", "switch", "API"]);
    p.ok(&["restore", "HEAD~2"]); // before "App task", API already existed
    assert_eq!(p.json(&["status"])["active_project_code"], "API");

    // Restoring a state without API falls back to that state's active project.
    p.ok(&["restore", "HEAD~4"]); // right after init
    assert_eq!(p.json(&["status"])["active_project_code"], "APP");
    assert_eq!(p.json(&["project", "list"]).as_array().unwrap().len(), 1);
}

#[test]
fn head_tilde_references_in_log_diff_and_restore() {
    let p = TempProject::new("head-tilde");
    p.init();
    p.ok(&["task", "add", "One"]);
    p.ok(&["task", "add", "Two"]);

    let diff = p.json(&["diff", "HEAD~1", "HEAD"]);
    let changes = diff["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["entity_id"], "APP-102");
    assert_eq!(changes[0]["change_kind"], "added");

    assert_eq!(p.json(&["log", "HEAD~1"]).as_array().unwrap().len(), 2);
    // HEAD~ alone means one step back.
    assert_eq!(p.json(&["log", "HEAD~"]).as_array().unwrap().len(), 2);

    let err = p.fails(&["restore", "HEAD~9"]);
    assert!(err.contains("goes past the first commit"), "{}", err);
    let err = p.fails(&["diff", "HEAD~x", "HEAD"]);
    assert!(err.contains("Invalid reference"), "{}", err);

    // A task object's hash is not a commit.
    let task_hash = p.json(&["task", "list"])[0][0].as_str().unwrap().to_string();
    let err = p.fails(&["restore", &task_hash[..12]]);
    assert!(err.contains("is not a commit"), "{}", err);
}

#[test]
fn branch_commands_no_longer_exist() {
    let p = TempProject::new("no-branches");
    p.init();
    for args in [&["branch", "feat"][..], &["checkout", "main"][..], &["merge", "feat"][..]] {
        let err = p.fails(args);
        assert!(err.contains("unrecognized subcommand"), "{:?}: {}", args, err);
    }
}

#[test]
fn old_repository_with_branches_keeps_working() {
    let p = TempProject::new("legacy-branches");
    p.init();
    p.ok(&["task", "add", "One"]);
    let clibm = p.dir.join(".clibm");
    let head_commit = std::fs::read_to_string(clibm.join("refs").join("heads").join("main"))
        .unwrap()
        .trim()
        .to_string();

    // An extra branch ref left over from an old CLIBM.
    std::fs::create_dir_all(clibm.join("refs").join("heads").join("feat")).unwrap();
    std::fs::write(clibm.join("refs").join("heads").join("feat").join("login"), format!("{}\n", head_commit)).unwrap();
    std::fs::write(clibm.join("refs").join("heads").join("old"), format!("{}\n", head_commit)).unwrap();
    let note = p.json(&["status"])["history_note"].as_str().unwrap().to_string();
    assert!(note.contains("unused branch refs remain: old"), "{}", note);
    assert!(p.ok(&["status"]).contains("Note"));
    assert!(p.ok(&["fsck"]).contains("CLIBM no longer uses branches"));

    // Detached HEAD on an old commit: work continues from there.
    std::fs::write(clibm.join("HEAD"), format!("{}\n", head_commit)).unwrap();
    let note = p.json(&["status"])["history_note"].as_str().unwrap().to_string();
    assert!(note.contains("detached"), "{}", note);
    p.ok(&["task", "add", "Two"]);
    assert_eq!(task_ids(&p), ["APP-101", "APP-102"]);
    p.ok(&["restore", "HEAD~1"]);
    assert_eq!(task_ids(&p), ["APP-101"]);
    p.ok(&["fsck"]);
}
