//! End-to-end tests of the plugin registry, trust boundary and `clibm plugin uninstall`.

mod common;

use common::TempProject;
use std::path::{Path, PathBuf};

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

/// Creates a fake plugin binary (any file works for registry/uninstall tests).
fn fake_binary(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(exe_name(name));
    std::fs::write(&path, b"not really a binary").unwrap();
    path
}

fn write_json(path: &Path, value: serde_json::Value) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, value.to_string()).unwrap();
}

fn global_registry(p: &TempProject) -> PathBuf {
    p.home.join(".clibm").join("plugins").join("registry.json")
}

fn entry(name: &str, exe: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": name, "version": "0.1.0", "description": "test plugin",
        "enabled": true, "hooks": ["manual-run"],
        "executable": exe.to_string_lossy().replace('\\', "/"),
        "tools": [{ "name": "keep_me" }]
    })
}

#[test]
fn list_prunes_entries_whose_binary_is_missing() {
    let p = TempProject::new("prune");
    p.init();
    let real = fake_binary(&p.home.join(".clibm-bin"), "clibm-real");
    write_json(
        &global_registry(&p),
        serde_json::json!([entry("clibm-real", &real), entry("clibm-gone", &p.home.join("nope").join("x"))]),
    );

    let out = p.clibm(&["plugin", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("clibm-real"), "{}", stdout);
    assert!(!stdout.contains("clibm-gone"), "{}", stdout);
    assert!(stderr.contains("Removed stale plugin entry 'clibm-gone'"), "{}", stderr);

    let raw = std::fs::read_to_string(global_registry(&p)).unwrap();
    assert!(!raw.contains("clibm-gone"), "{}", raw);
    // Unknown fields (MCP tools) survive a rewrite; one compact object per line.
    assert!(raw.contains("keep_me"), "{}", raw);
    assert!(raw.lines().any(|l| l.starts_with("{\"name\":\"clibm-real\"")), "{}", raw);
}

#[test]
fn project_registry_cannot_add_executables() {
    let p = TempProject::new("trust");
    p.init();
    let sneaky = fake_binary(&p.dir.join("tools"), "clibm-sneaky");
    // A cloned repository ships a full manifest in its own registry...
    write_json(
        &p.dir.join(".clibm").join("plugins").join("registry.json"),
        serde_json::json!([entry("clibm-sneaky", &sneaky)]),
    );
    // ...but it is not installed globally, so CLIBM never lists or runs it.
    let list = p.json(&["plugin", "list"]);
    assert_eq!(list, serde_json::json!([]));
    let err = p.fails(&["plugin", "run", "clibm-sneaky"]);
    assert!(err.contains("not installed"), "{}", err);
}

#[test]
fn enable_disable_is_a_project_override() {
    let p = TempProject::new("override");
    p.init();
    let exe = fake_binary(&p.home.join(".clibm-bin"), "clibm-thing");
    write_json(&global_registry(&p), serde_json::json!([entry("clibm-thing", &exe)]));

    p.ok(&["plugin", "disable", "clibm-thing"]);
    let list = p.json(&["plugin", "list"]);
    assert_eq!(list[0]["enabled"], false);
    let project_reg =
        std::fs::read_to_string(p.dir.join(".clibm").join("plugins").join("registry.json")).unwrap();
    assert!(project_reg.contains("{\"name\":\"clibm-thing\",\"enabled\":false}"), "{}", project_reg);
    assert!(!project_reg.contains("executable"), "{}", project_reg);
    // The global entry is untouched.
    assert!(std::fs::read_to_string(global_registry(&p)).unwrap().contains("\"enabled\":true"));

    let err = p.fails(&["plugin", "run", "clibm-thing"]);
    assert!(err.contains("disabled"), "{}", err);
    let err = p.fails(&["plugin", "disable", "clibm-missing"]);
    assert!(err.contains("not installed"), "{}", err);
}

#[test]
fn plugin_uninstall_removes_binary_and_registry_entries() {
    let p = TempProject::new("uninstall");
    p.init();
    let exe = fake_binary(&p.home.join(".clibm-bin"), "clibm-gadget");
    write_json(&global_registry(&p), serde_json::json!([entry("clibm-gadget", &exe)]));
    p.ok(&["plugin", "disable", "clibm-gadget"]); // creates a project override

    let report = p.json(&["plugin", "uninstall", "clibm-gadget"]);
    assert!(!exe.exists(), "binary still present");
    assert!(report["removed"][0].as_str().unwrap().contains("clibm-gadget"), "{}", report);
    assert_eq!(report["registry_entry_removed"], true);

    assert_eq!(p.json(&["plugin", "list"]), serde_json::json!([]));
    let project_reg =
        std::fs::read_to_string(p.dir.join(".clibm").join("plugins").join("registry.json")).unwrap();
    assert!(!project_reg.contains("clibm-gadget"), "{}", project_reg);
    let global_reg = std::fs::read_to_string(global_registry(&p)).unwrap();
    assert!(!global_reg.contains("clibm-gadget"), "{}", global_reg);

    let again = p.ok(&["plugin", "uninstall", "clibm-gadget"]);
    assert!(again.contains("not installed; nothing to do"), "{}", again);
}
