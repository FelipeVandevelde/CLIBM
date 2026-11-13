use clibm_core::engine::ClibmEngine;
use clibm_core::models::TaskPriority;
use std::io::Write;
use std::process::{Command, Stdio};

fn ask(exe: &str, dir: &std::path::Path, request: serde_json::Value) -> serde_json::Value {
    let mut child = Command::new(exe)
        .env("CLIBM_PROJECT_ROOT", dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(request.to_string().as_bytes()).unwrap();
    serde_json::from_slice(&child.wait_with_output().unwrap().stdout).unwrap()
}

#[test]
fn lint_reports_violations_and_vetoes_moves() {
    let dir = std::env::temp_dir().join(format!("clibm-workflow-lint-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let engine = ClibmEngine::open_exact(&dir).without_hooks();
    engine.init("ws", "APP", "App").unwrap();
    let add = |title: &str, col: &str, due: Option<&str>| {
        engine
            .create_task(title, "", TaskPriority::Medium, col, None, due.map(String::from), vec![], vec![], true)
            .unwrap();
    };
    add("Overdue", "todo", Some("2000-01-01"));
    // review has a WIP limit of 3: force a fourth task in.
    for t in ["R1", "R2", "R3", "R4"] {
        add(t, "review", None);
    }

    let exe = env!("CARGO_BIN_EXE_clibm-workflow-lint");
    let out = Command::new(exe).env("CLIBM_PROJECT_ROOT", &dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "violations must fail: {}", text);
    assert!(text.contains("VIOLATION: Column 'review'"), "{}", text);
    assert!(text.contains("APP-101 is overdue"), "{}", text);

    // pre-move: the move into the full column is refused...
    let veto = ask(exe, &dir, serde_json::json!({
        "contract": "CLIBM_PLUGIN_CONTRACT_V1", "event": "pre-move", "tree_hash": "x", "args": [],
        "context": { "task_id": "APP-101", "from": "todo", "to": "review", "force": false }
    }));
    assert_eq!(veto["ok"], false, "{}", veto);
    assert!(veto["output"].as_str().unwrap().contains("WIP limit"), "{}", veto);
    // ...unless it was forced.
    let forced = ask(exe, &dir, serde_json::json!({
        "contract": "CLIBM_PLUGIN_CONTRACT_V1", "event": "pre-move", "tree_hash": "x", "args": [],
        "context": { "task_id": "APP-101", "from": "todo", "to": "review", "force": true }
    }));
    assert_eq!(forced["ok"], true, "{}", forced);
    let _ = std::fs::remove_dir_all(&dir);
}
