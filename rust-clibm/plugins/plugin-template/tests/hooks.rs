//! Lifecycle-hook integration test: the core engine fires hooks to this template plugin.
//!
//! Everything runs in one test function because it points `CLIBM_HOME` (a process-wide
//! environment variable) at a temporary folder.

use clibm_core::engine::ClibmEngine;
use clibm_core::models::TaskPriority;
use clibm_core::plugin::PluginManager;
use std::path::PathBuf;

fn temp_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("clibm-hooks-{}-{}", std::process::id(), nanos));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add_task(engine: &ClibmEngine, title: &str) -> String {
    let (task, _, _) = engine
        .create_task(
            title,
            "",
            TaskPriority::Medium,
            "todo",
            None,
            None,
            vec![],
            vec![],
            false,
        )
        .unwrap();
    task.id
}

#[test]
fn engine_fires_hooks_and_pre_hooks_can_block() {
    let root = temp_root();
    let home = root.join("home");
    let project = root.join("project");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    let log = root.join("events.log");

    std::env::set_var("CLIBM_HOME", &home);
    std::env::remove_var("CLIBM_NO_HOOKS");
    std::env::remove_var("CLIBM_IN_HOOK");
    std::env::remove_var("CLIBM_TEMPLATE_DENY");
    std::env::set_var("CLIBM_TEMPLATE_LOG", &log);

    // Install the template plugin: register its own manifest with the hooks under test.
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_clibm-plugin-template"));
    let manifest = PluginManager::query_manifest(&exe, Some("pre-move,on-task-move,post-commit"))
        .expect("template prints its manifest");
    assert_eq!(manifest.name, "clibm-plugin-template");
    PluginManager::register_installed_plugin(&manifest).unwrap();

    let engine = ClibmEngine::open_exact(&project);
    engine.init("ws", "APP", "App").unwrap();

    // post-commit fires after every auto-commit.
    let id = add_task(&engine, "Hooked");
    let events = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(events.contains("post-commit"), "events: {}", events);

    // pre-move + on-task-move fire around a move, with the move details as context.
    engine.move_task(&id, "in-progress", false).unwrap();
    let events = std::fs::read_to_string(&log).unwrap();
    assert!(events.contains("pre-move"), "events: {}", events);
    assert!(events.contains("on-task-move"), "events: {}", events);
    assert!(events.contains("\"to\":\"in-progress\""), "events: {}", events);

    // A pre-* hook answering ok:false blocks the action and nothing is written.
    std::env::set_var("CLIBM_TEMPLATE_DENY", "pre-move");
    let err = engine.move_task(&id, "review", false).unwrap_err().to_string();
    assert!(err.contains("Blocked by plugin 'clibm-plugin-template'"), "{}", err);
    let (_, tree) = engine.load_working_tree().unwrap();
    assert_eq!(engine.load_task(&tree, &id).unwrap().column_id, "in-progress");

    // Hooks can be skipped: --no-hooks (engine) or the environment.
    let unhooked = ClibmEngine::open_exact(&project).without_hooks();
    unhooked.move_task(&id, "review", false).unwrap();
    std::env::set_var("CLIBM_NO_HOOKS", "1");
    engine.move_task(&id, "in-progress", false).unwrap();
    std::env::remove_var("CLIBM_NO_HOOKS");

    // Disabling the plugin for this project stops its hooks.
    PluginManager::set_plugin_enabled(&engine.cas, "template", false).unwrap();
    engine.move_task(&id, "review", false).unwrap();
    PluginManager::set_plugin_enabled(&engine.cas, "template", true).unwrap();

    // A post-* hook answering ok:false only warns.
    std::env::set_var("CLIBM_TEMPLATE_DENY", "post-commit");
    add_task(&engine, "Still created");

    // Uninstalling removes the registry entry: hooks stop firing.
    std::env::remove_var("CLIBM_TEMPLATE_DENY");
    let global = PluginManager::load_global_registry();
    assert!(global.iter().any(|p| p.name == "clibm-plugin-template"));
    let without: Vec<_> = global.into_iter().filter(|p| p.name != "clibm-plugin-template").collect();
    PluginManager::save_global_registry(&without).unwrap();
    std::env::set_var("CLIBM_TEMPLATE_DENY", "pre-move");
    engine.move_task(&id, "in-progress", false).unwrap();

    std::env::remove_var("CLIBM_TEMPLATE_DENY");
    std::env::remove_var("CLIBM_TEMPLATE_LOG");
    let _ = std::fs::remove_dir_all(&root);
}
