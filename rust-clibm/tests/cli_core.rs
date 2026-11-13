//! End-to-end tests of the `clibm` binary (core commands).

mod common;

use common::TempProject;

fn board_hash_prefix(board: &str, task_id: &str) -> String {
    let line = board
        .lines()
        .find(|l| l.contains(task_id))
        .unwrap_or_else(|| panic!("{} not on board:\n{}", task_id, board));
    let start = line.find("hash: ").expect("hash marker") + "hash: ".len();
    line[start..start + 8].to_string()
}

#[test]
fn init_refuses_existing_repository() {
    let p = TempProject::new("reinit");
    p.init();
    p.ok(&["task", "add", "Keep me"]);
    let err = p.fails(&["init"]);
    assert!(err.contains("already exists"), "{}", err);
    // The task survived.
    let tasks = p.json(&["task", "list"]);
    assert_eq!(tasks.as_array().unwrap().len(), 1);
}

#[test]
fn init_uses_installer_defaults() {
    let p = TempProject::new("installcfg");
    std::fs::create_dir_all(p.home.join(".clibm")).unwrap();
    std::fs::write(
        p.home.join(".clibm").join("install-config.json"),
        r#"{"language":"pt-BR","auto_commit":false,"install_path":"~/custom-bin"}"#,
    )
    .unwrap();
    p.init();
    let cfg = p.json(&["config", "show"]);
    assert_eq!(cfg["language"], "pt-BR");
    assert_eq!(cfg["auto_commit"], false);
    assert_eq!(cfg["install_path"], "~/custom-bin");
}

#[test]
fn task_reference_by_id_number_and_hash() {
    let p = TempProject::new("refs");
    p.init();
    p.ok(&["task", "add", "First"]);
    p.ok(&["task", "add", "Second"]);

    for reference in ["APP-102", "app-102", "102"] {
        let t = p.json(&["task", "show", reference]);
        assert_eq!(t["id"], "APP-102", "reference {}", reference);
    }

    let board = p.ok(&["board", "show"]);
    let prefix = board_hash_prefix(&board, "APP-101");
    let t = p.json(&["task", "show", &prefix]);
    assert_eq!(t["id"], "APP-101");

    // References work in mutations too.
    p.ok(&["task", "depend", "102", "101"]);
    let t = p.json(&["task", "show", "102"]);
    assert_eq!(t["depends_on"][0], "APP-101");

    let err = p.fails(&["task", "show", "999"]);
    assert!(err.contains("not found"), "{}", err);
}

#[test]
fn status_reports_branch_counts_and_dirty_flag() {
    let p = TempProject::new("status");
    p.init();
    p.ok(&["task", "add", "One", "-c", "todo"]);
    let st = p.json(&["status"]);
    assert_eq!(st["branch"], "main");
    assert_eq!(st["dirty"], false);
    assert_eq!(st["active_project_code"], "APP");
    assert_eq!(st["project_tasks"], 1);
    let todo = st["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "todo")
        .unwrap()
        .clone();
    assert_eq!(todo["tasks"], 1);

    p.ok(&["config", "set", "auto_commit", "false"]);
    p.ok(&["task", "add", "Two"]);
    assert_eq!(p.json(&["status"])["dirty"], true);
    let human = p.ok(&["status"]);
    assert!(human.contains("uncommitted changes"), "{}", human);

    p.ok(&["commit", "-m", "manual"]);
    assert_eq!(p.json(&["status"])["dirty"], false);
    let noop = p.ok(&["commit", "-m", "again"]);
    assert!(noop.contains("Nothing to commit"), "{}", noop);
}

#[test]
fn wip_limit_and_dependencies_are_enforced() {
    let p = TempProject::new("wip");
    p.init();
    // review has a WIP limit of 3
    for t in ["A", "B", "C"] {
        p.ok(&["task", "add", t, "-c", "review"]);
    }
    p.ok(&["task", "add", "D", "-c", "todo"]);
    let err = p.fails(&["board", "move", "104", "review"]);
    assert!(err.contains("WIP limit"), "{}", err);
    p.ok(&["board", "move", "104", "review", "--force"]);

    p.ok(&["task", "add", "Blocked", "--depends-on", "101"]);
    let err = p.fails(&["board", "move", "105", "in-progress"]);
    assert!(err.contains("incomplete dependency"), "{}", err);
}

#[test]
fn edit_and_delete_tasks() {
    let p = TempProject::new("editdel");
    p.init();
    p.ok(&["task", "add", "Base", "--tags", "ui"]);
    p.ok(&["task", "add", "Child", "--depends-on", "101"]);

    p.ok(&["task", "edit", "101", "--title", "Base v2", "-p", "high", "--add-tag", "core"]);
    let t = p.json(&["task", "show", "101"]);
    assert_eq!(t["title"], "Base v2");
    assert_eq!(t["priority"], "high");
    assert_eq!(t["tags"], serde_json::json!(["ui", "core"]));

    let noop = p.ok(&["task", "edit", "101", "--title", "Base v2"]);
    assert!(noop.contains("nothing changed"), "{}", noop);

    let err = p.fails(&["task", "delete", "101"]);
    assert!(err.contains("dependency of APP-102"), "{}", err);
    p.ok(&["task", "delete", "101", "--force"]);
    let child = p.json(&["task", "show", "APP-102"]);
    assert_eq!(child["depends_on"], serde_json::json!([]));

    // A deleted number is never reused.
    p.ok(&["task", "delete", "102"]);
    p.ok(&["task", "add", "Fresh"]);
    let tasks = p.json(&["task", "list"]);
    assert_eq!(tasks[0][1]["id"], "APP-103");
}

#[test]
fn checkout_refuses_uncommitted_changes() {
    let p = TempProject::new("checkout");
    p.init();
    p.ok(&["branch", "feature"]);
    p.ok(&["config", "set", "auto_commit", "false"]);
    p.ok(&["task", "add", "Unsaved"]);
    let err = p.fails(&["checkout", "feature"]);
    assert!(err.contains("uncommitted changes"), "{}", err);
    p.ok(&["checkout", "feature", "--force"]);
}

#[test]
fn branch_names_are_validated() {
    let p = TempProject::new("branchname");
    p.init();
    for bad in ["../../evil", "-x", "a..b", "HEAD", "x.lock", "a//b"] {
        let err = p.fails(&["branch", "--", bad]);
        assert!(err.contains("Invalid branch name"), "{}: {}", bad, err);
    }
    p.ok(&["branch", "feature/login"]);
    let err = p.fails(&["branch", "feature/login"]);
    assert!(err.contains("already exists"), "{}", err);
}

#[test]
fn merge_conflict_stops_and_theirs_resolves() {
    let p = TempProject::new("mergeconf");
    p.init();
    p.ok(&["task", "add", "Shared"]);
    p.ok(&["branch", "feat"]);
    p.ok(&["task", "edit", "101", "--title", "main title"]);
    p.ok(&["checkout", "feat"]);
    p.ok(&["task", "edit", "101", "--title", "feat title"]);
    p.ok(&["checkout", "main"]);

    let err = p.fails(&["merge", "feat"]);
    assert!(err.contains("conflict"), "{}", err);
    assert_eq!(p.json(&["task", "show", "101"])["title"], "main title");

    p.ok(&["merge", "feat", "--theirs"]);
    assert_eq!(p.json(&["task", "show", "101"])["title"], "feat title");
}

#[test]
fn merge_renumbers_same_id_and_propagates_deletions() {
    let p = TempProject::new("mergerenum");
    p.init();
    p.ok(&["task", "add", "Old"]); // APP-101 in base
    p.ok(&["branch", "feat"]);
    p.ok(&["task", "add", "Main task"]); // APP-102 on main
    p.ok(&["checkout", "feat"]);
    p.ok(&["task", "add", "Feat task"]); // APP-102 on feat (collision)
    p.ok(&["task", "add", "Feat child", "--depends-on", "102"]); // APP-103 -> APP-102
    p.ok(&["task", "delete", "101"]);
    p.ok(&["checkout", "main"]);

    let outcome = p.json(&["merge", "feat"]);
    assert_eq!(outcome["kind"], "merged");
    assert_eq!(outcome["renumbered"][0][0], "APP-102");
    let new_id = outcome["renumbered"][0][1].as_str().unwrap().to_string();

    let tasks = p.json(&["task", "list"]);
    let titles: Vec<(String, String)> = tasks
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row[1]["id"].as_str().unwrap().to_string(),
                row[1]["title"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert!(!titles.iter().any(|(id, _)| id == "APP-101"), "deletion not applied: {:?}", titles);
    assert!(titles.contains(&("APP-102".to_string(), "Main task".to_string())));
    assert!(titles.contains(&(new_id.clone(), "Feat task".to_string())));
    let child = p.json(&["task", "show", "APP-103"]);
    assert_eq!(child["title"], "Feat child");
    assert_eq!(child["depends_on"][0], new_id.as_str());
    assert_eq!(p.json(&["status"])["dirty"], false);
}

#[test]
fn merge_fast_forwards() {
    let p = TempProject::new("mergeff");
    p.init();
    p.ok(&["branch", "feat"]);
    p.ok(&["checkout", "feat"]);
    p.ok(&["task", "add", "On feat"]);
    p.ok(&["checkout", "main"]);
    let outcome = p.json(&["merge", "feat"]);
    assert_eq!(outcome["kind"], "fast-forward");
    assert_eq!(p.json(&["task", "show", "101"])["title"], "On feat");
    let again = p.json(&["merge", "feat"]);
    assert_eq!(again["kind"], "up-to-date");
}

#[test]
fn purge_requires_confirmation_and_never_targets_parent() {
    let p = TempProject::new("purge");
    p.init();

    // Without --yes and without a terminal, nothing is deleted.
    let out = p.ok(&["uninstall", "--purge"]);
    assert!(out.contains("Not deleted"), "{}", out);
    assert!(p.dir.join(".clibm").exists());

    // From a subfolder, the parent repository is never deleted.
    let sub = p.dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let out = p.clibm_in(&sub, &["uninstall", "--purge", "--yes"]);
    assert!(out.status.success());
    assert!(p.dir.join(".clibm").exists());

    p.ok(&["uninstall", "--purge", "--yes"]);
    assert!(!p.dir.join(".clibm").exists());
}

#[test]
fn fsck_reports_legacy_plugin_objects_separately() {
    let p = TempProject::new("fscklegacy");
    p.init();
    // Simulate an object written by the old clibm-web hash scheme.
    let payload = serde_json::json!({"note": "legacy"});
    let preimage = format!("task:0.1.0:{}", serde_json::to_string(&payload).unwrap());
    let hash = blake3_hex(preimage.as_bytes());
    let dir = p.dir.join(".clibm").join("objects").join(&hash[..2]);
    std::fs::create_dir_all(&dir).unwrap();
    let envelope = serde_json::json!({"Type": "task", "Version": "0.1.0", "Hash": hash, "Payload": payload});
    std::fs::write(dir.join(format!("{}.json", &hash[2..])), envelope.to_string()).unwrap();

    let report = p.json(&["fsck"]);
    assert_eq!(report["legacy_hashes"][0], hash.as_str());
    assert_eq!(report["corrupted_hashes"], serde_json::json!([]));
}

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[test]
fn config_repairs_browser_style_repository_urls() {
    let p = TempProject::new("repourl");
    p.init();
    p.ok(&["config", "set", "github_repo_url", "https://github.com/FelipeVandevelde/CLIBM/tree/dev/rust-clibm"]);
    let cfg = p.json(&["config", "show"]);
    assert_eq!(cfg["github_repo_url"], "https://github.com/FelipeVandevelde/CLIBM.git");
    assert_eq!(cfg["update_branch"], "dev");
    let err = p.fails(&["config", "set", "github_repo_url", "nonsense"]);
    assert!(err.contains("not a repository URL"), "{}", err);
}
