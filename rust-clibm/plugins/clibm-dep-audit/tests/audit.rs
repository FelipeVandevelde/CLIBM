use clibm_core::engine::ClibmEngine;
use clibm_core::models::TaskPriority;
use std::process::Command;

#[test]
fn audit_finds_blockers_and_the_critical_path() {
    let dir = std::env::temp_dir().join(format!("clibm-dep-audit-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let engine = ClibmEngine::open_exact(&dir).without_hooks();
    engine.init("ws", "APP", "App").unwrap();
    let add = |title: &str, deps: Vec<String>| {
        engine
            .create_task(title, "", TaskPriority::Medium, "todo", None, None, vec![], deps, false)
            .unwrap();
    };
    add("Base", vec![]);
    add("Middle", vec!["101".into()]);
    add("Top", vec!["102".into()]);
    add("Side", vec!["101".into()]);

    let exe = env!("CARGO_BIN_EXE_clibm-dep-audit");
    let out = Command::new(exe).env("CLIBM_PROJECT_ROOT", &dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{}", text);
    assert!(text.contains("Cycles: none"), "{}", text);
    assert!(text.contains("APP-101 \"Base\" [todo] blocks 2: APP-102, APP-104"), "{}", text);
    assert!(text.contains("Critical path (3 open tasks): APP-101 -> APP-102 -> APP-103"), "{}", text);
    let _ = std::fs::remove_dir_all(&dir);
}
