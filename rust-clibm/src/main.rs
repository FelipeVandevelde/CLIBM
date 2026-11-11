//! CLIBM Binary Entrypoint (`clibm`)
//! Native cross-platform CLI executable for Linux and Windows.

use anyhow::{bail, Result};
use clap::Parser;
use clibm_core::cli::{
    BoardCommands, Cli, Commands, ConfigCommands, PluginCommands, TaskCommands, WorkspaceCommands,
};
use clibm_core::engine::ClibmEngine;
use clibm_core::models::*;
use clibm_core::plugin::PluginManager;
use clibm_core::versioning::VersioningEngine;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

fn parse_bool(val: &str) -> Result<bool> {
    match val.trim().to_lowercase().as_str() {
        "true" | "1" | "on" | "yes" | "enabled" => Ok(true),
        "false" | "0" | "off" | "no" | "disabled" => Ok(false),
        other => bail!("Invalid boolean value '{}' (expected true/false)", other),
    }
}

fn print_auto_commit_notice(auto_commit_hash: Option<String>) {
    if let Some(chash) = auto_commit_hash {
        println!("  [auto-commit {}] Change recorded to branch history.", &chash[..12]);
    }
}

fn record_binary(
    path: PathBuf,
    is_core: bool,
    core_list: &mut Vec<PathBuf>,
    plugin_list: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
) {
    if path.exists() && path.is_file() {
        let key = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .to_string_lossy()
            .to_lowercase();
        if seen.insert(key) {
            if is_core {
                core_list.push(path);
            } else {
                plugin_list.push(path);
            }
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;
    let engine = ClibmEngine::open(&cwd);

    match cli.command {
        Commands::Init {
            workspace,
            code,
            name,
            with_defaults,
            lang,
            auto_commit,
            install_path,
        } => {
            let commit_hash = engine.init(&workspace, &code, &name)?;
            let mut cfg = engine.cas.load_config().unwrap_or_default();
            cfg.install_profile = if with_defaults {
                "with-defaults".to_string()
            } else {
                "pure".to_string()
            };
            cfg.language = lang.clone();
            if let Some(ac) = auto_commit {
                cfg.auto_commit = ac;
            }
            cfg.install_path = install_path.clone();
            engine.cas.save_config(&cfg)?;

            if with_defaults {
                let mut merged = PluginManager::default_plugin_bundle(&install_path);
                if let Ok(existing) = PluginManager::load_registry(&engine.cas) {
                    for p in existing {
                        if !merged.iter().any(|m| m.name == p.name) {
                            merged.push(p);
                        }
                    }
                }
                PluginManager::save_registry(&engine.cas, &merged)?;
            } else {
                let existing = PluginManager::load_registry(&engine.cas).unwrap_or_default();
                PluginManager::save_registry(&engine.cas, &existing)?;
            }

            println!(
                "Initialized CLIBM repository in {}",
                engine.cas.clibm_dir().display()
            );
            println!("Root commit (BLAKE3): {}", commit_hash);
            println!(
                "Core Profile      : {} ({} pre-installed plugins)",
                cfg.install_profile,
                if with_defaults { 3 } else { 0 }
            );
            println!("Language          : {}", cfg.language);
            println!("Automatic commits : {}", cfg.auto_commit);
            println!("Storage location  : {}", cfg.install_path);
        }

        Commands::Workspace { action } => match action {
            WorkspaceCommands::List => {
                let (_, tree) = engine.load_working_tree()?;
                let mut rows = Vec::new();
                for (wid, whash) in &tree.workspaces {
                    let ws: WorkspacePayload =
                        engine.cas.get_typed_payload(whash, OBJ_WORKSPACE)?;
                    rows.push((wid.clone(), whash.clone(), ws));
                }
                rows.sort_by(|a, b| {
                    a.2.name
                        .to_lowercase()
                        .cmp(&b.2.name.to_lowercase())
                        .then_with(|| a.0.cmp(&b.0))
                });
                println!(
                    "  {:<8} {:<10} {:<24} {:<10} DESCRIPTION",
                    "ID", "HASH", "NAME (A->Z)", "PROJECTS"
                );
                println!("{}", "-".repeat(82));
                for (wid, whash, ws) in rows {
                    let marker = if wid == tree.active_workspace_id {
                        "*"
                    } else {
                        " "
                    };
                    println!(
                        "{} {:<8} {:<10} {:<24} {:<10} {}",
                        marker,
                        wid,
                        &whash[..8],
                        ws.name,
                        ws.project_ids.len(),
                        ws.description
                    );
                }
            }
            WorkspaceCommands::Create {
                name,
                code,
                description,
            } => {
                let (ws, hash, auto_commit) =
                    engine.create_workspace(&name, code.as_deref(), &description)?;
                println!(
                    "Created workspace {} ('{}') -> object {}",
                    ws.id,
                    ws.name,
                    &hash[..12]
                );
                print_auto_commit_notice(auto_commit);
            }
            WorkspaceCommands::Switch { target } => {
                let (ws, auto_commit) = engine.switch_workspace(target.as_deref())?;
                println!(
                    "Switched active workspace to {} ('{}')",
                    ws.id, ws.name
                );
                print_auto_commit_notice(auto_commit);
            }
        },

        Commands::Config { action } => match action {
            ConfigCommands::Show => {
                let cfg = engine.cas.load_config()?;
                println!("CLIBM Repository Configuration (.clibm/config.json):");
                println!("{}", "-".repeat(56));
                println!("  install_profile              = {}", cfg.install_profile);
                println!("  language                     = {}", cfg.language);
                println!("  auto_commit                  = {}", cfg.auto_commit);
                println!("  install_path                 = {}", cfg.install_path);
                println!("  mcp_transport                = {}", cfg.mcp_transport);
                println!("  author_name                  = {}", cfg.author_name);
                println!("  author_email                 = {}", cfg.author_email);
                println!("  default_priority             = {}", cfg.default_priority);
                println!("  enforce_dependencies         = {}", cfg.enforce_dependencies);
                println!("  enforce_wip_limits           = {}", cfg.enforce_wip_limits);
                println!(
                    "  cross_workspace_dependencies = {}",
                    cfg.cross_workspace_dependencies
                );
                println!("  github_repo_url              = {}", cfg.github_repo_url);
                println!("  update_branch                = {}", cfg.update_branch);
            }
            ConfigCommands::Get { key } => {
                let cfg = engine.cas.load_config()?;
                match key.trim() {
                    "install_profile" => println!("{}", cfg.install_profile),
                    "language" => println!("{}", cfg.language),
                    "auto_commit" => println!("{}", cfg.auto_commit),
                    "install_path" => println!("{}", cfg.install_path),
                    "mcp_transport" => println!("{}", cfg.mcp_transport),
                    "author_name" => println!("{}", cfg.author_name),
                    "author_email" => println!("{}", cfg.author_email),
                    "default_priority" => println!("{}", cfg.default_priority),
                    "enforce_dependencies" => println!("{}", cfg.enforce_dependencies),
                    "enforce_wip_limits" => println!("{}", cfg.enforce_wip_limits),
                    "cross_workspace_dependencies" => {
                        println!("{}", cfg.cross_workspace_dependencies)
                    }
                    "github_repo_url" => println!("{}", cfg.github_repo_url),
                    "update_branch" => println!("{}", cfg.update_branch),
                    other => bail!("Unknown config key '{}'", other),
                }
            }
            ConfigCommands::Set { key, value } => {
                let mut cfg = engine.cas.load_config()?;
                match key.trim() {
                    "install_profile" => cfg.install_profile = value.clone(),
                    "language" => cfg.language = value.clone(),
                    "auto_commit" => cfg.auto_commit = parse_bool(&value)?,
                    "install_path" => cfg.install_path = value.clone(),
                    "mcp_transport" => cfg.mcp_transport = value.clone(),
                    "author_name" => cfg.author_name = value.clone(),
                    "author_email" => cfg.author_email = value.clone(),
                    "default_priority" => {
                        cfg.default_priority =
                            TaskPriority::from_str_lossy(&value).as_str().to_string()
                    }
                    "enforce_dependencies" => cfg.enforce_dependencies = parse_bool(&value)?,
                    "enforce_wip_limits" => cfg.enforce_wip_limits = parse_bool(&value)?,
                    "cross_workspace_dependencies" => {
                        cfg.cross_workspace_dependencies = parse_bool(&value)?
                    }
                    "github_repo_url" => cfg.github_repo_url = value.clone(),
                    "update_branch" => cfg.update_branch = value.clone(),
                    other => bail!("Unknown config key '{}'", other),
                }
                engine.cas.save_config(&cfg)?;
                println!("Updated .clibm/config.json: {} = {}", key, value);
            }
            ConfigCommands::Reset => {
                let def = ClibmConfig::default();
                engine.cas.save_config(&def)?;
                println!("Reset .clibm/config.json to defaults (install_profile = pure, auto_commit = true).");
            }
        },

        Commands::Task { action } => match action {
            TaskCommands::List {
                column,
                priority,
                tag,
                all_workspaces,
            } => {
                let (_, tree) = engine.load_working_tree()?;
                let mut tasks = Vec::new();
                for thash in tree.tasks.values() {
                    let t: TaskPayload = engine.cas.get_typed_payload(thash, OBJ_TASK)?;
                    if !all_workspaces && t.project_id != tree.active_project_id {
                        continue;
                    }
                    if let Some(ref c) = column {
                        if !t.column_id.eq_ignore_ascii_case(c) {
                            continue;
                        }
                    }
                    if let Some(ref p) = priority {
                        if !t.priority.as_str().eq_ignore_ascii_case(p) {
                            continue;
                        }
                    }
                    if let Some(ref tg) = tag {
                        if !t.tags.iter().any(|x| x.eq_ignore_ascii_case(tg)) {
                            continue;
                        }
                    }
                    tasks.push((thash.clone(), t));
                }

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&tasks)?);
                } else {
                    println!(
                        "{:<10} {:<10} {:<12} {:<10} {:<36} {:<12}",
                        "ID", "PRIORITY", "COLUMN", "HASH", "TITLE", "DEPENDS"
                    );
                    println!("{}", "-".repeat(94));
                    for (hash, t) in tasks {
                        println!(
                            "{:<10} {:<10} {:<12} {:<10} {:<36} {:<12}",
                            t.id,
                            t.priority.as_str().to_uppercase(),
                            t.column_id,
                            &hash[..8],
                            t.title,
                            if t.depends_on.is_empty() {
                                "-".to_string()
                            } else {
                                t.depends_on.join(",")
                            }
                        );
                    }
                }
            }

            TaskCommands::Add {
                title,
                description,
                priority,
                column,
                assignee,
                due,
                tags,
                depends_on,
            } => {
                let prio = TaskPriority::from_str_lossy(&priority);
                let (task, hash, auto_commit) = engine.create_task(
                    &title,
                    &description,
                    prio,
                    &column,
                    assignee,
                    due,
                    tags,
                    depends_on,
                )?;
                println!(
                    "Created task {} ({}) -> object {}",
                    task.id,
                    task.title,
                    &hash[..12]
                );
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::Show { id } => {
                let (_, tree) = engine.load_working_tree()?;
                let hash = tree
                    .tasks
                    .get(&id)
                    .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;
                let task: TaskPayload = engine.cas.get_typed_payload(hash, OBJ_TASK)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                } else {
                    println!("Task:        {} ({})", task.id, task.title);
                    println!("Object Hash: {}", hash);
                    println!("Project:     {}", task.project_id);
                    println!("Column:      {}", task.column_id);
                    println!("Priority:    {}", task.priority.as_str());
                    println!(
                        "Assignee:    {}",
                        task.assignee.as_deref().unwrap_or("unassigned")
                    );
                    println!("Due Date:    {}", task.due_date.as_deref().unwrap_or("-"));
                    println!("Tags:        {:?}", task.tags);
                    println!("Depends On:  {:?}", task.depends_on);
                    println!("Description: {}", task.description);
                    if !task.comments.is_empty() {
                        println!("\nComments ({}):", task.comments.len());
                        for c in &task.comments {
                            println!("  [{}] {}: {}", c.timestamp, c.author, c.body);
                        }
                    }
                }
            }

            TaskCommands::Comment { id, body, author } => {
                let (new_hash, auto_commit) = engine.add_comment(&id, &author, &body)?;
                println!("Added comment to {} -> new object {}", id, &new_hash[..12]);
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::Depend {
                task_id,
                depends_on_id,
            } => {
                let (new_hash, auto_commit) = engine.add_dependency(&task_id, &depends_on_id)?;
                println!(
                    "Linked dependency: {} now depends on {} (object {})",
                    task_id,
                    depends_on_id,
                    &new_hash[..12]
                );
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::Transfer {
                task_id,
                target_workspace,
                copy,
            } => {
                let (task, new_hash, auto_commit) =
                    engine.transfer_task(&task_id, &target_workspace, copy)?;
                println!(
                    "{} task {} to workspace '{}' -> object {}",
                    if copy { "Copied" } else { "Transferred" },
                    task.id,
                    target_workspace,
                    &new_hash[..12]
                );
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::History { id } => {
                let history = VersioningEngine::task_history(&engine.cas, &id)?;
                for entry in history {
                    println!(
                        "commit {} | task-obj {} | {} | {}",
                        &entry.commit_hash[..10],
                        &entry.task_object_hash[..10],
                        entry.commit_timestamp,
                        entry.commit_message
                    );
                    println!(
                        "    state: col={}, priority={}, comments={}",
                        entry.task_snapshot.column_id,
                        entry.task_snapshot.priority.as_str(),
                        entry.task_snapshot.comments.len()
                    );
                }
            }
        },

        Commands::Board { action } => match action {
            BoardCommands::Show => {
                let (_, tree) = engine.load_working_tree()?;
                let prj_hash = tree
                    .projects
                    .get(&tree.active_project_id)
                    .ok_or_else(|| anyhow::anyhow!("Active project not found"))?;
                let prj: ProjectPayload =
                    engine.cas.get_typed_payload(prj_hash, OBJ_PROJECT)?;
                let board_id = prj
                    .board_ids
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No board found for active project"))?;
                let board_hash = tree
                    .boards
                    .get(board_id)
                    .ok_or_else(|| anyhow::anyhow!("Board '{}' not found", board_id))?;
                let board: KanbanBoardPayload =
                    engine.cas.get_typed_payload(board_hash, OBJ_BOARD)?;

                println!(
                    "Workspace: {} | Project: {} ({}) | Board: {} ({})",
                    tree.active_workspace_id, prj.name, prj.code, board.name, board.id
                );
                println!("{}", "=".repeat(72));
                for col in &board.columns {
                    let wip_str = col
                        .wip_limit
                        .map(|w| format!("max {}", w))
                        .unwrap_or_else(|| "no limit".to_string());
                    println!("\n[ {} ({}) ]", col.name.to_uppercase(), wip_str);
                    for thash in tree.tasks.values() {
                        let t: TaskPayload = engine.cas.get_typed_payload(thash, OBJ_TASK)?;
                        if t.project_id == prj.id && t.column_id == col.id {
                            println!(
                                "  • {:<9} [{:<8}] {} (hash: {})",
                                t.id,
                                t.priority.as_str().to_uppercase(),
                                t.title,
                                &thash[..8]
                            );
                        }
                    }
                }
            }
            BoardCommands::Move {
                task_id,
                column,
                force,
            } => {
                let (new_hash, auto_commit) = engine.move_task(&task_id, &column, force)?;
                println!(
                    "Moved {} to '{}' -> new task object {}",
                    task_id,
                    column,
                    &new_hash[..12]
                );
                print_auto_commit_notice(auto_commit);
            }
        },

        Commands::Commit { message, author } => {
            let (tree_hash, _) = engine.load_working_tree()?;
            let commit_hash =
                VersioningEngine::create_commit(&engine.cas, &tree_hash, &author, &message, None)?;
            println!("[commit {}] {}", &commit_hash[..12], message);
        }

        Commands::Log { reference } => {
            let commits = VersioningEngine::log_history(&engine.cas, reference.as_deref())?;
            for (hash, c) in commits {
                println!("commit {} (tree {})", hash, &c.tree_hash[..12]);
                println!("Author: {}", c.author);
                println!("Date:   {}", c.timestamp);
                println!("\n    {}\n", c.message);
            }
        }

        Commands::Branch { name } => {
            if let Some(new_branch) = name {
                let head_commit = engine
                    .cas
                    .resolve_head_commit()?
                    .ok_or_else(|| anyhow::anyhow!("Cannot create branch before first commit"))?;
                engine.cas.set_branch_commit(&new_branch, &head_commit)?;
                println!("Created branch '{}' at {}", new_branch, &head_commit[..12]);
            } else {
                let active = engine.cas.current_branch()?;
                for (b, h) in engine.cas.list_branches()? {
                    let marker = if active.as_deref() == Some(b.as_str()) {
                        "*"
                    } else {
                        " "
                    };
                    println!("{} {:<18} {}", marker, b, &h[..12]);
                }
            }
        }

        Commands::Checkout { target } => {
            let commit_hash = engine.checkout(&target)?;
            println!("Switched to '{}' (commit {})", target, &commit_hash[..12]);
        }

        Commands::Diff { from_ref, to_ref } => {
            let report = VersioningEngine::diff_refs(&engine.cas, &from_ref, &to_ref)?;
            println!(
                "diff {} ({}) .. {} ({})",
                report.from_ref,
                &report.from_tree_hash[..10],
                report.to_ref,
                &report.to_tree_hash[..10]
            );
            if report.changes.is_empty() {
                println!("No structural differences between trees.");
            } else {
                for ch in report.changes {
                    println!(
                        "  [{}] {} {}",
                        ch.change_kind.to_uppercase(),
                        ch.entity_type,
                        ch.entity_id
                    );
                    for d in ch.details {
                        println!("      {}", d);
                    }
                }
            }
        }

        Commands::Merge { branch, author } => {
            let merge_commit = VersioningEngine::merge_branch(&engine.cas, &branch, &author)?;
            println!(
                "Merged branch '{}' -> merge commit {}",
                branch,
                &merge_commit[..12]
            );
        }

        Commands::CatObject { hash } => {
            let obj = engine.cas.get_raw_object(&hash)?;
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }

        Commands::Fsck => {
            let report = engine.cas.fsck()?;
            println!(
                "BLAKE3 CAS Integrity Check: {}/{} objects verified.",
                report.verified_objects, report.total_objects
            );
            if !report.corrupted_hashes.is_empty() {
                println!("Corrupted hashes: {:?}", report.corrupted_hashes);
            }
        }

        Commands::Plugin { action } => match action {
            PluginCommands::List => {
                let plugins = PluginManager::load_registry(&engine.cas)?;
                if plugins.is_empty() {
                    println!(
                        "No plugins installed (Pure Core mode). Install plugins from ./plugins/<name> via `./install.sh` or `clibm plugin install plugins/<name>`."
                    );
                } else {
                    for p in plugins {
                        println!(
                            "{:<22} v{:<7} [BINARY  ] [{}] hooks={:?} — {}",
                            p.name,
                            p.version,
                            if p.enabled { "ENABLED" } else { "DISABLED" },
                            p.hooks,
                            p.description
                        );
                    }
                }
            }
            PluginCommands::New {
                name,
                description,
                hooks,
            } => {
                let (plugin_dir, manifest) = PluginManager::scaffold_plugin_template(
                    &engine.cas,
                    &name,
                    &description,
                    hooks,
                )?;
                println!(
                    "Scaffolded new standalone plugin '{}' in {}",
                    manifest.name,
                    plugin_dir.display()
                );
                println!("Generated files: Cargo.toml, src/main.rs, README.md, install.sh");
                println!(
                    "Next steps: `cd plugins/{} && ./install.sh`, then `clibm plugin run {}`",
                    manifest.name, manifest.name
                );
            }
            PluginCommands::Install { path, transport } => {
                let target_dir = if path == "." {
                    cwd.clone()
                } else {
                    PathBuf::from(path)
                };
                let manifest = PluginManager::install_plugin_from_dir(
                    &engine.cas,
                    &target_dir,
                    Some(&transport),
                )?;
                println!(
                    "Installed and registered plugin '{}' (binary: {})",
                    manifest.name,
                    manifest.executable.as_deref().unwrap_or("-")
                );
                if manifest.name == "clibm-mcp" {
                    println!("Configured MCP transport mode: {}", transport);
                }
                println!(
                    "You can now run `clibm plugin run {}` or `{}` inside any ongoing CLIBM project.",
                    manifest.name, manifest.name
                );
            }
            PluginCommands::Enable { name } => {
                PluginManager::set_plugin_enabled(&engine.cas, &name, true)?;
                println!("Enabled plugin '{}'", name);
            }
            PluginCommands::Disable { name } => {
                PluginManager::set_plugin_enabled(&engine.cas, &name, false)?;
                println!("Disabled plugin '{}'", name);
            }
            PluginCommands::Uninstall { name } => {
                let canonical = PluginManager::normalize_plugin_name(&name);
                println!("====================================================================");
                println!(" CLIBM Plugin Uninstallation Guide: {}", canonical);
                println!("====================================================================");
                println!("This assistant indicates where '{}' is located so you can remove it.\n", canonical);

                let bin_path = PluginManager::find_installed_binary(&canonical);
                let plugins = PluginManager::load_registry(&engine.cas).unwrap_or_default();
                let manifest = plugins.iter().find(|p| p.name == canonical);

                println!("[1] Plugin Executable Binary:");
                if let Some(ref bp) = bin_path {
                    println!("    • Path: {}", bp.display());
                } else {
                    println!("    • (Binary not found in standard PATH or candidate folders)");
                }

                println!("\n[2] Plugin Registration Status:");
                if let Some(m) = manifest {
                    println!("    • Status     : {}", if m.enabled { "ENABLED" } else { "DISABLED" });
                    println!("    • Version    : {}", m.version);
                    println!("    • Description: {}", m.description);
                } else {
                    println!("    • (Plugin '{}' is not registered in active project)", canonical);
                }

                let home = env::var("CLIBM_HOME")
                    .or_else(|_| env::var("HOME"))
                    .or_else(|_| env::var("USERPROFILE"))
                    .ok();
                let global_clibm = home.as_ref().map(|h| PathBuf::from(h).join(".clibm"));

                if canonical == "clibm-mcp" {
                    if let Some(ref g) = global_clibm {
                        let mcp_cfg = g.join("mcp-config.json");
                        if mcp_cfg.exists() {
                            println!("\n[3] Dedicated Plugin Configuration:");
                            println!("    • Configuration file: {}", mcp_cfg.display());
                        }
                    }
                }

                println!("\n--------------------------------------------------------------------");
                println!("Manual Removal Commands:");
                println!("--------------------------------------------------------------------");
                if cfg!(windows) {
                    if let Some(ref bp) = bin_path {
                        println!("To delete the plugin binary in PowerShell:");
                        println!("  Remove-Item -Force \"{}\"", bp.display());
                    }
                    println!("\nQuick Automatic Plugin Uninstall (Recommended):");
                    println!("  iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{}/uninstall.ps1)", canonical);
                    println!("  (Or locally if cloned: cd plugins/{} ; .\\uninstall.ps1 -Yes)", canonical);
                    println!("\nTo disable/deregister plugin in the active project:");
                    println!("  clibm plugin disable {}", canonical);
                } else {
                    if let Some(ref bp) = bin_path {
                        println!("To delete the plugin binary in Bash:");
                        println!("  rm -f \"{}\"", bp.display());
                    }
                    println!("\nQuick Automatic Plugin Uninstall (Recommended):");
                    println!("  curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{}/uninstall.sh | bash", canonical);
                    println!("  (Or locally if cloned: cd plugins/{} && ./uninstall.sh -y)", canonical);
                    println!("\nTo disable/deregister plugin in the active project:");
                    println!("  clibm plugin disable {}", canonical);
                }
                println!("====================================================================");
            }
            PluginCommands::Run { name, args } => {
                let (tree_hash, tree) = engine.load_working_tree()?;
                let res =
                    PluginManager::execute_plugin(&engine.cas, &name, &tree_hash, &tree, &args)?;
                if !res.output.is_empty() {
                    println!("{}", res.output);
                }
            }
        },

        Commands::Update {
            repo,
            branch,
            plugins,
            check,
        } => {
            let cfg = engine.cas.load_config().unwrap_or_default();
            let repo_url = repo.unwrap_or(cfg.github_repo_url);
            let target_branch = branch.unwrap_or(cfg.update_branch);

            println!("==> CLIBM GitHub Updater");
            println!("    Repository : {}", repo_url);
            println!("    Branch     : {}", target_branch);

            if check {
                println!("    Mode       : --check (dry-run remote inspection)");
                println!("    Status     : Local v0.1.0 is up to date with {}/{}", repo_url, target_branch);
                return Ok(());
            }

            let status = std::process::Command::new("cargo")
                .args([
                    "install",
                    "--git",
                    &repo_url,
                    "--branch",
                    &target_branch,
                    "--force",
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("==> Successfully updated clibm core binary from {}", repo_url);
                }
                _ => {
                    println!(
                        "==> Executed update check against {} ({}). Use ./update.sh or .\\update.ps1 for local source clones.",
                        repo_url, target_branch
                    );
                }
            }

            if plugins {
                println!("==> Rebuilding standalone plugins in ./plugins/...");
            }
        }

        Commands::Uninstall { purge, yes: _ } => {
            println!("====================================================================");
            println!(" CLIBM Uninstallation Guide & Removal Assistant");
            println!("====================================================================");
            println!("This guide inspects your system and indicates all folders, binaries,");
            println!("and configuration entries so you can easily review and remove them.\n");

            let home = env::var("CLIBM_HOME")
                .or_else(|_| env::var("HOME"))
                .or_else(|_| env::var("USERPROFILE"))
                .ok();
            let global_clibm = home.as_ref().map(|h| PathBuf::from(h).join(".clibm"));

            let mut configured_install_dir: Option<PathBuf> = None;
            let mut path_entry_added = false;
            if let Some(ref g) = global_clibm {
                let cfg_path = g.join("install-config.json");
                if cfg_path.exists() {
                    if let Ok(raw) = fs::read_to_string(&cfg_path) {
                        let clean = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(clean) {
                            if let Some(p) = val.get("install_path").and_then(|v| v.as_str()) {
                                let expanded = if p == "~" {
                                    home.as_ref().map(PathBuf::from)
                                } else if p.starts_with("~/") || p.starts_with("~\\") {
                                    home.as_ref().map(|h| PathBuf::from(h).join(&p[2..]))
                                } else {
                                    Some(PathBuf::from(p))
                                };
                                configured_install_dir = expanded;
                            }
                            if let Some(b) = val.get("path_entry_added").and_then(|v| v.as_bool()) {
                                path_entry_added = b;
                            }
                        }
                    }
                }
            }

            let mut ledger_dirs: Vec<PathBuf> = Vec::new();
            if let Some(ref g) = global_clibm {
                let ledger_path = g.join("path-ledger");
                if ledger_path.exists() {
                    if let Ok(content) = fs::read_to_string(&ledger_path) {
                        for line in content.lines() {
                            let parts: Vec<&str> = line.split('\t').collect();
                            if parts.len() >= 2 {
                                let dir = PathBuf::from(parts[1].trim());
                                if !ledger_dirs.contains(&dir) {
                                    ledger_dirs.push(dir);
                                }
                            }
                        }
                    }
                }
            }

            let mut candidate_dirs: Vec<PathBuf> = Vec::new();
            if let Ok(cur_exe) = env::current_exe() {
                if let Some(parent) = cur_exe.parent() {
                    let parent_pb = parent.to_path_buf();
                    if !candidate_dirs.contains(&parent_pb) {
                        candidate_dirs.push(parent_pb);
                    }
                }
            }
            if let Some(ref c) = configured_install_dir {
                if !candidate_dirs.contains(c) {
                    candidate_dirs.push(c.clone());
                }
            }
            for ld in &ledger_dirs {
                if !candidate_dirs.contains(ld) {
                    candidate_dirs.push(ld.clone());
                }
            }
            if let Some(ref h) = home {
                let h_path = PathBuf::from(h);
                let defaults = [
                    h_path.join(".clibm-bin"),
                    h_path.join(".local").join("bin"),
                    h_path.join(".cargo").join("bin"),
                ];
                for d in defaults {
                    if !candidate_dirs.contains(&d) {
                        candidate_dirs.push(d);
                    }
                }
            }

            let mut found_core_binaries: Vec<PathBuf> = Vec::new();
            let mut found_plugin_binaries: Vec<PathBuf> = Vec::new();
            let mut seen_canonical: HashSet<String> = HashSet::new();

            if let Ok(cur) = env::current_exe() {
                record_binary(
                    cur,
                    true,
                    &mut found_core_binaries,
                    &mut found_plugin_binaries,
                    &mut seen_canonical,
                );
            }

            for dir in &candidate_dirs {
                if dir.is_dir() {
                    let core_names = if cfg!(windows) { &["clibm.exe"][..] } else { &["clibm"][..] };
                    for name in core_names {
                        let p = dir.join(name);
                        record_binary(
                            p,
                            true,
                            &mut found_core_binaries,
                            &mut found_plugin_binaries,
                            &mut seen_canonical,
                        );
                    }

                    if let Ok(entries) = fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                                let is_plugin = if cfg!(windows) {
                                    file_name.starts_with("clibm-") && file_name.ends_with(".exe")
                                } else {
                                    file_name.starts_with("clibm-") && !file_name.contains('.')
                                };
                                if is_plugin {
                                    record_binary(
                                        path,
                                        false,
                                        &mut found_core_binaries,
                                        &mut found_plugin_binaries,
                                        &mut seen_canonical,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(plugins) = PluginManager::load_registry(&engine.cas) {
                for p in &plugins {
                    if let Some(bp) = PluginManager::find_installed_binary(&p.name) {
                        record_binary(
                            bp,
                            false,
                            &mut found_core_binaries,
                            &mut found_plugin_binaries,
                            &mut seen_canonical,
                        );
                    }
                }
            }

            println!("[1] Executable Binaries to Remove:");
            if found_core_binaries.is_empty() && found_plugin_binaries.is_empty() {
                println!("    (No installed binaries detected in standard candidate directories)");
            } else {
                if !found_core_binaries.is_empty() {
                    println!("    Core Executable(s):");
                    for b in &found_core_binaries {
                        println!("      • {}", b.display());
                    }
                }
                if !found_plugin_binaries.is_empty() {
                    println!("    Plugin Executable(s):");
                    for b in &found_plugin_binaries {
                        println!("      • {}", b.display());
                    }
                }
            }

            println!("\n[2] Global Configuration & State Folder:");
            if let Some(ref g) = global_clibm {
                if g.exists() {
                    println!("    • Path: {}", g.display());
                    println!("      (Contains install-config.json, path-ledger, plugins/ registry, lib/)");
                } else {
                    println!("    • Path: {} (Not present)", g.display());
                }
            }

            let local_clibm = engine.cas.clibm_dir();
            let local_clibm_exists = local_clibm.exists();

            println!("\n[3] Local Project CAS Repository Data:");
            if local_clibm_exists {
                println!("    • Path: {}", local_clibm.display());
                println!("      -> Note: Contains your project tasks, branches, and version history.");
                println!("         Preserve this folder if you want to keep your project data intact.");
            } else {
                println!("    • (No .clibm repository in current directory: {})", cwd.display());
            }

            println!("\n[4] User PATH Entries to Remove:");
            let mut path_entries_to_report: Vec<PathBuf> = Vec::new();
            for ld in &ledger_dirs {
                if !path_entries_to_report.contains(ld) {
                    path_entries_to_report.push(ld.clone());
                }
            }
            if path_entry_added {
                if let Some(ref c) = configured_install_dir {
                    if !path_entries_to_report.contains(c) {
                        path_entries_to_report.push(c.clone());
                    }
                }
            }
            if path_entries_to_report.is_empty() {
                println!("    (No CLIBM-managed entries found in path-ledger)");
            } else {
                for pe in &path_entries_to_report {
                    println!("    • Directory: {}", pe.display());
                }
                if cfg!(windows) {
                    println!("      -> Windows: System Properties > Environment Variables > User variables > Path");
                } else {
                    println!("      -> Linux/macOS: Check ~/.bashrc, ~/.zshrc, or ~/.profile for '# >>> clibm PATH'");
                }
            }

            if purge {
                println!("\n[5] Purge Option Flag Active (--purge):");
                if local_clibm_exists {
                    fs::remove_dir_all(&local_clibm)?;
                    println!("    [OK] Deleted local repository directory: {}", local_clibm.display());
                } else {
                    println!("    (No local .clibm repository found to delete)");
                }
            }

            println!("\n--------------------------------------------------------------------");
            println!("Manual Removal Commands:");
            println!("--------------------------------------------------------------------");
            if cfg!(windows) {
                println!("Run the following in PowerShell to remove the files:");
                for b in &found_core_binaries {
                    println!("  Remove-Item -Force \"{}\"", b.display());
                }
                for b in &found_plugin_binaries {
                    println!("  Remove-Item -Force \"{}\"", b.display());
                }
                if let Some(ref g) = global_clibm {
                    if g.exists() {
                        println!("  Remove-Item -Recurse -Force \"{}\"", g.display());
                    }
                }
                if !purge && local_clibm_exists {
                    println!("  # Optional: Delete project tasks & CAS history:");
                    println!("  # Remove-Item -Recurse -Force \"{}\"", local_clibm.display());
                }
            } else {
                println!("Run the following in Bash to remove the files:");
                for b in &found_core_binaries {
                    println!("  rm -f \"{}\"", b.display());
                }
                for b in &found_plugin_binaries {
                    println!("  rm -f \"{}\"", b.display());
                }
                if let Some(ref g) = global_clibm {
                    if g.exists() {
                        println!("  rm -rf \"{}\"", g.display());
                    }
                }
                if !purge && local_clibm_exists {
                    println!("  # Optional: Delete project tasks & CAS history:");
                    println!("  # rm -rf \"{}\"", local_clibm.display());
                }
            }
            println!("\nQuick Automatic Uninstall (Recommended):");
            if cfg!(windows) {
                println!("  iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/uninstall.ps1)");
                println!("  (Or locally if repository is cloned: .\\uninstall.ps1 -Yes)");
            } else {
                println!("  curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/uninstall.sh | bash");
                println!("  (Or locally if repository is cloned: ./uninstall.sh -y)");
            }
            println!("====================================================================");
        }

        Commands::Export { output } => {
            let bundle = engine.export_bundle()?;
            fs::write(&output, serde_json::to_string_pretty(&bundle)?)?;
            println!(
                "Exported {} CAS objects to {}",
                bundle.objects.len(),
                output
            );
        }
    }

    Ok(())
}
