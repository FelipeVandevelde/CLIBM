//! CLIBM Binary Entrypoint (`clibm`)
//! Native cross-platform CLI executable for Linux and Windows.

use anyhow::{bail, Result};
use clap::Parser;
use clibm_core::cli::{
    BoardCommands, Cli, Commands, ConfigCommands, PluginCommands, TaskCommands, WorkspaceCommands,
};
use clibm_core::engine::{ClibmEngine, MutationResult, TaskEdit};
use clibm_core::install::{clibm_home, InstallConfig};
use clibm_core::models::*;
use clibm_core::plugin::PluginManager;
use clibm_core::versioning::{MergeKind, MergeStrategy, VersioningEngine};
use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}

/// Prints the standard line for a task mutation, or a no-op notice.
fn report_mutation(res: &MutationResult, done: &str, noop: &str, json: bool) -> Result<()> {
    if json {
        return print_json(res);
    }
    if res.changed {
        println!("{} {} -> task object {}", done, res.id, short(&res.hash));
        print_auto_commit_notice(res.commit.clone());
    } else {
        println!("{} {} (nothing changed)", noop, res.id);
    }
    Ok(())
}

/// Asks a yes/no question on the terminal. Returns `false` when stdin is not a terminal.
fn confirm(prompt: &str) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("{} ", prompt);
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

fn parse_bool(val: &str) -> Result<bool> {
    match val.trim().to_lowercase().as_str() {
        "true" | "1" | "on" | "yes" | "enabled" => Ok(true),
        "false" | "0" | "off" | "no" | "disabled" => Ok(false),
        other => bail!("Invalid boolean value '{}' (expected true/false)", other),
    }
}

fn print_auto_commit_notice(auto_commit_hash: Option<String>) {
    if let Some(chash) = auto_commit_hash {
        println!("  [auto-commit {}] Change recorded to branch history.", short(&chash));
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
    let engine = if cli.no_hooks {
        ClibmEngine::open(&cwd).without_hooks()
    } else {
        ClibmEngine::open(&cwd)
    };

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
            // Always initialize in the current directory, never in a repository
            // discovered in a parent folder.
            let engine = ClibmEngine::open_exact(&cwd);
            let machine = InstallConfig::load();
            let lang = lang
                .or_else(|| machine.language.clone())
                .unwrap_or_else(|| "en".to_string());
            let install_path = install_path
                .or_else(|| machine.install_path.clone())
                .unwrap_or_else(|| ClibmConfig::default().install_path);
            let auto_commit = auto_commit.or(machine.auto_commit);

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

            let installed = PluginManager::load_registry(&engine.cas).unwrap_or_default();

            println!(
                "Initialized CLIBM repository in {}",
                engine.cas.clibm_dir().display()
            );
            println!("Root commit (BLAKE3): {}", commit_hash);
            println!(
                "Core Profile      : {} ({} plugin(s) installed on this machine)",
                cfg.install_profile,
                installed.len()
            );
            if with_defaults {
                for default in ["clibm-dep-audit", "clibm-export-md", "clibm-workflow-lint"] {
                    if !installed.iter().any(|p| p.name == default) {
                        println!(
                            "  Default plugin '{}' is not installed yet: run `clibm plugin install {}`",
                            default, default
                        );
                    }
                }
            }
            println!("Language          : {}", cfg.language);
            println!("Automatic commits : {}", cfg.auto_commit);
            println!("Storage location  : {}", cfg.install_path);
        }

        Commands::Status => {
            let st = engine.status()?;
            if cli.json {
                return print_json(&st);
            }
            match (&st.branch, &st.head_commit) {
                (Some(b), _) => println!("On branch {}", b),
                (None, Some(h)) => println!("HEAD detached at {}", short(h)),
                (None, None) => println!("No commits yet"),
            }
            println!("Repository : {}", st.repository);
            println!(
                "HEAD       : {}",
                st.head_commit.as_deref().map(short).unwrap_or("(none)")
            );
            println!("Staged tree: {}", short(&st.staged_tree));
            if st.dirty {
                println!("Changes    : uncommitted changes (run `clibm commit -m <msg>`)");
            } else {
                println!("Changes    : working tree clean");
            }
            println!(
                "Workspace  : {} ({})",
                st.active_workspace_id, st.active_workspace_name
            );
            println!(
                "Project    : {} {} ({})",
                st.active_project_id, st.active_project_code, st.active_project_name
            );
            if let (Some(id), Some(name)) = (&st.board_id, &st.board_name) {
                println!("Board      : {} ({})", id, name);
            }
            for c in &st.columns {
                let limit = c.wip_limit.map(|l| format!("/{}", l)).unwrap_or_default();
                println!("  {:<12} {}{}", c.id, c.tasks, limit);
            }
            println!(
                "Tasks      : {} in active project, {} total",
                st.project_tasks, st.total_tasks
            );
            println!(
                "Repository : {} workspace(s), {} branch(es), {} objects, auto_commit = {}",
                st.workspaces, st.branches, st.objects, st.auto_commit
            );
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
                if cli.json {
                    let out: Vec<serde_json::Value> = rows
                        .iter()
                        .map(|(wid, whash, ws)| {
                            serde_json::json!({
                                "id": wid,
                                "hash": whash,
                                "active": wid == &tree.active_workspace_id,
                                "workspace": ws,
                            })
                        })
                        .collect();
                    return print_json(&out);
                }
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
                if cli.json {
                    return print_json(&cfg);
                }
                println!("CLIBM Repository Configuration (.clibm/config.json):");
                println!("{}", "-".repeat(56));
                println!("  install_profile              = {}", cfg.install_profile);
                println!("  language                     = {}", cfg.language);
                println!("  auto_commit                  = {}", cfg.auto_commit);
                println!("  install_path                 = {}", cfg.install_path);
                println!("  mcp_transport                = {}", cfg.mcp_transport);
                println!("  hook_timeout_secs            = {}", cfg.hook_timeout_secs);
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
                    "hook_timeout_secs" => println!("{}", cfg.hook_timeout_secs),
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
                    "hook_timeout_secs" => {
                        cfg.hook_timeout_secs = value
                            .trim()
                            .parse::<u64>()
                            .ok()
                            .filter(|n| *n > 0)
                            .ok_or_else(|| anyhow::anyhow!("hook_timeout_secs must be a positive number of seconds"))?
                    }
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
                force,
            } => {
                let cfg = engine.cas.load_config()?;
                let prio = TaskPriority::from_str_lossy(
                    priority.as_deref().unwrap_or(&cfg.default_priority),
                );
                let (task, hash, auto_commit) = engine.create_task(
                    &title,
                    &description,
                    prio,
                    &column,
                    assignee,
                    due,
                    tags,
                    depends_on,
                    force,
                )?;
                if cli.json {
                    return print_json(&serde_json::json!({
                        "id": task.id, "hash": hash, "commit": auto_commit, "task": task
                    }));
                }
                println!(
                    "Created task {} ({}) -> object {}",
                    task.id,
                    task.title,
                    short(&hash)
                );
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::Show { id } => {
                let (_, tree) = engine.load_working_tree()?;
                let id = engine.resolve_task_ref(&tree, &id)?;
                let hash = &tree.tasks[&id];
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

            TaskCommands::Edit {
                id,
                title,
                description,
                priority,
                assignee,
                due,
                tags,
                add_tag,
                remove_tag,
            } => {
                let edit = TaskEdit {
                    title,
                    description,
                    priority: priority.as_deref().map(TaskPriority::from_str_lossy),
                    assignee: assignee.map(Some),
                    due_date: due.map(Some),
                    tags,
                    add_tags: add_tag,
                    remove_tags: remove_tag,
                };
                let res = engine.edit_task(&id, edit)?;
                report_mutation(&res, "Updated", "Task", cli.json)?;
            }

            TaskCommands::Delete { id, force } => {
                let res = engine.delete_task(&id, force)?;
                if cli.json {
                    return print_json(&res);
                }
                println!("Deleted task {} (it remains in earlier commits)", res.id);
                if !res.unlinked_dependents.is_empty() {
                    println!(
                        "  Removed the dependency link from: {}",
                        res.unlinked_dependents.join(", ")
                    );
                }
                print_auto_commit_notice(res.commit);
            }

            TaskCommands::Comment { id, body, author } => {
                let author = match author {
                    Some(a) => a,
                    None => engine.cas.load_config()?.author_name,
                };
                let res = engine.add_comment(&id, &author, &body)?;
                report_mutation(&res, "Added comment to", "Task", cli.json)?;
            }

            TaskCommands::Depend {
                task_id,
                depends_on_id,
            } => {
                let res = engine.add_dependency(&task_id, &depends_on_id)?;
                if cli.json {
                    return print_json(&res);
                }
                if res.changed {
                    let (_, tree) = engine.load_working_tree()?;
                    let dep = engine.resolve_task_ref(&tree, &depends_on_id)?;
                    println!(
                        "Linked dependency: {} now depends on {} (object {})",
                        res.id,
                        dep,
                        short(&res.hash)
                    );
                    print_auto_commit_notice(res.commit);
                } else {
                    println!("{} already has that dependency (nothing changed)", res.id);
                }
            }

            TaskCommands::Transfer {
                task_id,
                target_workspace,
                copy,
            } => {
                let (task, new_hash, auto_commit) =
                    engine.transfer_task(&task_id, &target_workspace, copy)?;
                if cli.json {
                    return print_json(&serde_json::json!({
                        "id": task.id, "hash": new_hash, "commit": auto_commit, "task": task
                    }));
                }
                println!(
                    "{} task {} to workspace '{}' -> object {}",
                    if copy { "Copied" } else { "Transferred" },
                    task.id,
                    target_workspace,
                    short(&new_hash)
                );
                print_auto_commit_notice(auto_commit);
            }

            TaskCommands::History { id } => {
                // Resolve against the current tree; fall back to the raw input so the
                // history of a deleted task can still be traced by its full ID.
                let id = match engine.load_working_tree() {
                    Ok((_, tree)) => engine.resolve_task_ref(&tree, &id).unwrap_or(id),
                    Err(_) => id,
                };
                let history = VersioningEngine::task_history(&engine.cas, &id)?;
                if cli.json {
                    return print_json(&history);
                }
                if history.is_empty() {
                    println!("No history found for task '{}'", id);
                }
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
                let res = engine.move_task(&task_id, &column, force)?;
                if cli.json {
                    return print_json(&res);
                }
                if res.changed {
                    println!(
                        "Moved {} to '{}' -> new task object {}",
                        res.id,
                        column,
                        short(&res.hash)
                    );
                    print_auto_commit_notice(res.commit);
                } else {
                    println!("{} is already in '{}' (nothing changed)", res.id, column);
                }
            }
        },

        Commands::Commit { message, author } => {
            match engine.commit(&message, author.as_deref())? {
                Some(commit_hash) => {
                    if cli.json {
                        return print_json(&serde_json::json!({ "commit": commit_hash }));
                    }
                    println!("[commit {}] {}", short(&commit_hash), message);
                }
                None => {
                    if cli.json {
                        return print_json(&serde_json::json!({ "commit": null }));
                    }
                    println!("Nothing to commit: the working tree matches HEAD.");
                }
            }
        }

        Commands::Log { reference } => {
            let commits = VersioningEngine::log_history(&engine.cas, reference.as_deref())?;
            if cli.json {
                let out: Vec<serde_json::Value> = commits
                    .iter()
                    .map(|(h, c)| serde_json::json!({ "hash": h, "commit": c }))
                    .collect();
                return print_json(&out);
            }
            for (hash, c) in commits {
                println!("commit {} (tree {})", hash, &c.tree_hash[..12]);
                println!("Author: {}", c.author);
                println!("Date:   {}", c.timestamp);
                println!("\n    {}\n", c.message);
            }
        }

        Commands::Branch { name } => {
            if let Some(new_branch) = name {
                let head_commit = engine.create_branch(&new_branch)?;
                println!("Created branch '{}' at {}", new_branch, short(&head_commit));
            } else {
                let active = engine.cas.current_branch()?;
                if cli.json {
                    let out: Vec<serde_json::Value> = engine
                        .cas
                        .list_branches()?
                        .into_iter()
                        .map(|(b, h)| {
                            serde_json::json!({
                                "name": b,
                                "commit": h,
                                "current": active.as_deref() == Some(b.as_str()),
                            })
                        })
                        .collect();
                    return print_json(&out);
                }
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

        Commands::Checkout { target, force } => {
            let commit_hash = engine.checkout(&target, force)?;
            println!("Switched to '{}' (commit {})", target, short(&commit_hash));
        }

        Commands::Diff { from_ref, to_ref } => {
            let report = VersioningEngine::diff_refs(&engine.cas, &from_ref, &to_ref)?;
            if cli.json {
                return print_json(&report);
            }
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

        Commands::Merge {
            branch,
            author,
            ours,
            theirs,
        } => {
            let strategy = if ours {
                MergeStrategy::Ours
            } else if theirs {
                MergeStrategy::Theirs
            } else {
                MergeStrategy::Abort
            };
            let outcome = engine.merge(&branch, author.as_deref(), strategy)?;
            if cli.json {
                return print_json(&outcome);
            }
            match outcome.kind {
                MergeKind::UpToDate => println!("Already up to date with '{}'.", branch),
                MergeKind::FastForward => println!(
                    "Fast-forwarded to '{}' (commit {})",
                    branch,
                    outcome.commit.as_deref().map(short).unwrap_or("-")
                ),
                MergeKind::Merged => {
                    println!(
                        "Merged branch '{}' -> merge commit {}",
                        branch,
                        outcome.commit.as_deref().map(short).unwrap_or("-")
                    );
                    for (old, new) in &outcome.renumbered {
                        println!("  Renumbered incoming task {} -> {} (same ID on both branches)", old, new);
                    }
                    for c in &outcome.resolved_conflicts {
                        println!(
                            "  Resolved conflict ({}): {} {} - {}",
                            if ours { "ours" } else { "theirs" },
                            c.entity_type,
                            c.entity_id,
                            c.reason
                        );
                    }
                }
            }
        }

        Commands::CatObject { hash } => {
            let obj = engine.cas.get_raw_object(&hash)?;
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }

        Commands::Fsck => {
            let report = engine.cas.fsck()?;
            if cli.json {
                print_json(&report)?;
            } else {
                println!(
                    "BLAKE3 CAS Integrity Check: {}/{} objects verified.",
                    report.verified_objects, report.total_objects
                );
                if !report.legacy_hashes.is_empty() {
                    println!(
                        "{} object(s) use the old clibm-web/clibm-tui hash scheme (content intact): {:?}",
                        report.legacy_hashes.len(),
                        report.legacy_hashes
                    );
                }
                if !report.corrupted_hashes.is_empty() {
                    println!("Corrupted hashes: {:?}", report.corrupted_hashes);
                }
            }
            if !report.corrupted_hashes.is_empty() {
                std::process::exit(1);
            }
        }

        Commands::Plugin { action } => match action {
            PluginCommands::List => {
                let plugins = PluginManager::load_registry(&engine.cas)?;
                if cli.json {
                    return print_json(&plugins);
                }
                if plugins.is_empty() {
                    println!(
                        "No plugins installed. Install one with `clibm plugin install <name>` (e.g. `clibm plugin install mcp`) or its install script."
                    );
                } else {
                    for p in plugins {
                        println!(
                            "{:<22} v{:<7} [{}] hooks={:?} - {}",
                            p.name,
                            p.version,
                            if p.enabled { "ENABLED" } else { "DISABLED" },
                            p.hooks,
                            p.description
                        );
                        println!("{:<22}   {}", "", p.executable.as_deref().unwrap_or("-"));
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
                let report = PluginManager::uninstall_plugin(&engine.cas, &name)?;
                if cli.json {
                    return print_json(&report);
                }
                for path in &report.removed {
                    println!("==> Removed binary: {}", path);
                }
                for path in &report.rotated {
                    println!(
                        "==> Binary was locked; renamed to {} (it is deleted on the next install/uninstall)",
                        path
                    );
                }
                for path in &report.failed {
                    eprintln!("==> Could not remove {} (close any program using it and retry)", path);
                }
                if report.registry_entry_removed {
                    println!("==> Removed '{}' from the plugin registry", report.name);
                }
                if report.removed.is_empty()
                    && report.rotated.is_empty()
                    && report.failed.is_empty()
                    && !report.registry_entry_removed
                {
                    println!("Plugin '{}' is not installed; nothing to do.", report.name);
                } else {
                    println!("Uninstalled plugin '{}'.", report.name);
                    if report.name == "clibm-mcp" {
                        if let Some(cfg_file) = clibm_home().map(|h| h.join(".clibm").join("mcp-config.json")) {
                            if cfg_file.exists() {
                                println!("  (Its settings file was kept: {})", cfg_file.display());
                            }
                        }
                    }
                    println!("  The user PATH was not changed; `clibm uninstall` lists remaining entries.");
                }
                if !report.failed.is_empty() {
                    std::process::exit(1);
                }
            }
            PluginCommands::Run { name, args } => {
                let (tree_hash, tree) = engine.load_working_tree()?;
                let res =
                    PluginManager::execute_plugin(&engine.cas, &name, &tree_hash, &tree, &args)?;
                if !res.output.is_empty() {
                    println!("{}", res.output);
                }
                if !res.ok {
                    std::process::exit(1);
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

        Commands::Uninstall { purge, yes } => {
            println!("====================================================================");
            println!(" CLIBM Uninstallation Guide & Removal Assistant");
            println!("====================================================================");
            println!("This guide inspects your system and indicates all folders, binaries,");
            println!("and configuration entries so you can easily review and remove them.\n");

            let home = clibm_home();
            let global_clibm = home.as_ref().map(|h| h.join(".clibm"));

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

            // Only the repository in the current directory is ever a purge target; a
            // repository found in a parent folder is reported but never deleted.
            let local_clibm = cwd.join(".clibm");
            let local_clibm_exists = local_clibm.join("objects").exists();
            let discovered = engine.cas.clibm_dir().to_path_buf();

            println!("\n[3] Local Project CAS Repository Data:");
            if local_clibm_exists {
                println!("    • Path: {}", local_clibm.display());
                println!("      -> Note: Contains your project tasks, branches, and version history.");
                println!("         Preserve this folder if you want to keep your project data intact.");
            } else {
                println!("    • (No .clibm repository in current directory: {})", cwd.display());
                if discovered != local_clibm && discovered.exists() {
                    println!(
                        "      -> A parent folder has one ({}); --purge never deletes it. Run the command there instead.",
                        discovered.display()
                    );
                }
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
                    let repo = ClibmEngine::open_exact(&cwd);
                    let objects = repo.cas.count_objects();
                    let branches = repo.cas.list_branches().map(|b| b.len()).unwrap_or(0);
                    println!(
                        "    This permanently deletes {} ({} objects, {} branch(es)). It cannot be undone.",
                        local_clibm.display(),
                        objects,
                        branches
                    );
                    let approved = yes || confirm("    Delete it? [y/N]:");
                    if approved {
                        fs::remove_dir_all(&local_clibm)?;
                        println!("    [OK] Deleted local repository directory: {}", local_clibm.display());
                    } else {
                        println!(
                            "    Not deleted. Re-run with --yes to confirm (required when not running in a terminal)."
                        );
                    }
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
            if let Some(tree_hash) = bundle.staged_tree_hash.as_deref() {
                engine.fire_hook(
                    "on-export",
                    tree_hash,
                    serde_json::json!({ "output": output, "objects": bundle.objects.len() }),
                )?;
            }
            println!(
                "Exported {} CAS objects to {}",
                bundle.objects.len(),
                output
            );
        }
    }

    Ok(())
}
