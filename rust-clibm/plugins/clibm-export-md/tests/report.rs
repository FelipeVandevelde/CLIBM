use clibm_core::engine::{ClibmEngine, TaskEdit};
use clibm_core::models::TaskPriority;
use std::process::Command;

#[test]
fn report_lists_current_tasks_once_and_writes_next_to_exports() {
    let dir = std::env::temp_dir().join(format!("clibm-export-md-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let engine = ClibmEngine::open_exact(&dir).without_hooks();
    engine.init("ws", "APP", "App").unwrap();
    engine
        .create_task("First | piped", "", TaskPriority::High, "todo", None, None, vec![], vec![], false)
        .unwrap();
    engine
        .create_task("Second", "", TaskPriority::Low, "todo", None, None, vec![], vec!["101".into()], false)
        .unwrap();
    // Several historical versions of APP-101: only the current one may appear.
    for title in ["v2", "v3", "Final title"] {
        engine
            .edit_task("101", TaskEdit { title: Some(title.into()), ..Default::default() })
            .unwrap();
    }

    let exe = env!("CARGO_BIN_EXE_clibm-export-md");
    let out = Command::new(exe).env("CLIBM_PROJECT_ROOT", &dir).output().unwrap();
    assert!(out.status.success());
    let md = String::from_utf8_lossy(&out.stdout);
    assert_eq!(md.lines().filter(|l| l.starts_with("| APP-101 |")).count(), 1, "{}", md);
    assert!(md.contains("Final title"), "{}", md);
    assert!(!md.contains("v2"), "{}", md);
    assert!(md.contains("| APP-102 | Second | low | To Do | - | - | APP-101 | - |"), "{}", md);

    // on-export hook: writes <bundle>.md next to the exported bundle.
    let bundle = dir.join("bundle.json");
    let request = serde_json::json!({
        "contract": "CLIBM_PLUGIN_CONTRACT_V1", "event": "on-export", "tree_hash": "abc", "args": [],
        "context": { "output": bundle.display().to_string() }
    });
    let mut child = Command::new(exe)
        .env("CLIBM_PROJECT_ROOT", &dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(request.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let reply: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(reply["ok"], true, "{}", reply);
    assert!(dir.join("bundle.md").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
