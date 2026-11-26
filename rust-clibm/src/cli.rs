//! Clap CLI Definitions for CLIBM v0.1.0

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "clibm",
    version = "0.1.0",
    about = "CLIBM — Local-first CLI Project Manager with BLAKE3 Content-Addressable Storage",
    long_about = "A lightweight, portable, offline-first project management CLI written in Rust.\nUses RFC 8785 Canonical JSON + BLAKE3 Content-Addressable Storage (CAS), automatic per-modification commits, and multi-workspace coordination."
)]
pub struct Cli {
    /// Output machine-readable JSON for interoperability with external tools
    #[arg(long, global = true)]
    pub json: bool,

    /// Do not run plugin lifecycle hooks for this command (same as CLIBM_NO_HOOKS=1)
    #[arg(long, global = true)]
    pub no_hooks: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new .clibm repository in the current directory. The install profile
    /// defaults to the installer's choice (pure, with-defaults or custom), else pure.
    /// Refuses to run if the current directory already has a .clibm repository.
    Init {
        #[arg(long, default_value = "default-workspace")]
        workspace: String,
        #[arg(long, default_value = "CLIBM")]
        code: String,
        #[arg(long, default_value = "CLIBM Core Project")]
        name: String,
        /// Record the "with-defaults" profile and point out which of the default plugins
        /// (clibm-dep-audit, clibm-export-md, clibm-workflow-lint) still need `clibm plugin install`
        #[arg(long)]
        with_defaults: bool,
        /// Repository interface language (en, pt-BR, es). Defaults to the installer's choice, else "en".
        #[arg(long)]
        lang: Option<String>,
        /// Enable or disable automatic per-modification commits. Defaults to the installer's choice, else true.
        #[arg(long)]
        auto_commit: Option<bool>,
        /// Machine directory where clibm binaries are stored. Defaults to the installer's choice.
        #[arg(long)]
        install_path: Option<String>,
    },

    /// Show the HEAD commit, history size, uncommitted changes, active workspace/project and task counts
    Status,

    /// Manage workspaces, switch active workspace, and coordinate across workspaces
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCommands,
    },

    /// Manage the projects of a workspace: list, create, switch, edit, delete
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
    },

    /// Manage repository configuration (.clibm/config.json) including auto_commit
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Manage tasks, priorities, dependencies, cross-workspace transfers, tags, and comments
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },

    /// Manage Kanban boards, columns, and task movement
    Board {
        #[command(subcommand)]
        action: BoardCommands,
    },

    /// Create an immutable commit snapshot of the current staged tree
    /// (does nothing when there are no uncommitted changes)
    Commit {
        #[arg(short, long)]
        message: String,
        /// Commit author (defaults to `author_name` in .clibm/config.json)
        #[arg(short, long)]
        author: Option<String>,
    },

    /// Show the commit history (newest first)
    Log {
        /// Start from this commit instead of HEAD (hash prefix, HEAD or HEAD~N)
        reference: Option<String>,
    },

    /// Bring back the state of an earlier commit as a new commit (history is kept).
    /// `clibm restore HEAD --force` discards uncommitted changes.
    Restore {
        /// Commit to restore: hash prefix from `clibm log`, HEAD or HEAD~N
        target: String,
        /// Discard uncommitted changes instead of refusing
        #[arg(long)]
        force: bool,
        /// Commit message (defaults to "restore state of <hash>")
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Compare two commits structurally (hash prefix, HEAD or HEAD~N)
    Diff {
        from_ref: String,
        to_ref: String,
    },

    /// Inspect a raw CAS object by its BLAKE3 hash or prefix
    CatObject {
        hash: String,
    },

    /// Verify BLAKE3 cryptographic integrity of all objects in .clibm/objects
    Fsck,

    /// Manage, scaffold, install, and run extensible plugins in ongoing projects
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },

    /// Rebuild CLIBM from GitHub (or a local clone) and replace the installed binary
    Update {
        /// Repository URL (defaults to the one recorded by the installer, then the official repository).
        /// GitHub browser links such as .../CLIBM/tree/main/rust-clibm are accepted.
        #[arg(long)]
        repo: Option<String>,
        /// Branch to build (defaults to the installer's branch, then "main")
        #[arg(long)]
        branch: Option<String>,
        /// Build from a local clone instead of GitHub (repository root or its rust-clibm folder)
        #[arg(long, conflicts_with_all = ["repo", "branch"])]
        path: Option<String>,
        /// Also rebuild every installed official plugin
        #[arg(long)]
        plugins: bool,
        /// Only compare the installed build with the latest remote commit
        #[arg(long)]
        check: bool,
        /// Skip the core binary (used with --plugins by the update scripts)
        #[arg(long, hide = true)]
        skip_core: bool,
    },

    /// Uninstallation assistant: scans and displays all installed binary paths, configuration folders,
    /// and user PATH entries with copy-pasteable removal commands so the user can easily remove them.
    Uninstall {
        /// Also delete the .clibm repository in the current directory (never a parent folder's).
        /// Asks for confirmation unless --yes is given.
        #[arg(long)]
        purge: bool,
        /// Skip the confirmation prompt of --purge
        #[arg(short, long)]
        yes: bool,
    },

    /// Export the entire CAS repository as a portable JSON bundle
    Export {
        #[arg(short, long, default_value = "clibm-bundle.json")]
        output: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceCommands {
    /// List all workspaces in the repository
    List,
    /// Create a new workspace with its own default project and Kanban board
    Create {
        name: String,
        #[arg(short, long)]
        code: Option<String>,
        #[arg(short, long, default_value = "")]
        description: String,
    },
    /// Switch active workspace by ID or name (if omitted, cycles to next workspace in alphabetical order)
    Switch {
        /// Optional workspace ID or name. If omitted, selects the next workspace alphabetically.
        target: Option<String>,
    },
    /// Rename a workspace or change its description (names must be unique)
    Edit {
        /// Workspace ID (WS-02) or name
        workspace: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a workspace (refuses while its projects have tasks, unless --move-to)
    Delete {
        /// Workspace ID (WS-02) or name
        workspace: String,
        /// Workspace that receives the deleted workspace's projects, whole (boards and tasks keep their IDs)
        #[arg(long)]
        move_to: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// List the projects of the active workspace (or of all workspaces)
    List {
        #[arg(long)]
        all_workspaces: bool,
    },
    /// Create a project with its own Kanban board in the active workspace
    Create {
        name: String,
        /// Project code, used as the task ID prefix (e.g. API -> API-101); must be unused
        #[arg(short, long)]
        code: String,
        #[arg(short, long, default_value = "")]
        description: String,
        /// Workspace (ID or name) to create the project in; defaults to the active one
        #[arg(short, long)]
        workspace: Option<String>,
        /// Make the new project (and its workspace) the active one
        #[arg(long)]
        switch: bool,
    },
    /// Make a project active by ID, code or name (switches workspace too if needed)
    Switch {
        project: String,
    },
    /// Rename a project or change its description (the code cannot change)
    Edit {
        /// Project ID (PRJ-02), code or name
        project: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a project and its board (refuses while it has tasks, unless --move-to)
    Delete {
        /// Project ID (PRJ-02), code or name
        project: String,
        /// Project that receives the deleted project's tasks (they keep their IDs)
        #[arg(long)]
        move_to: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show all configuration settings in .clibm/config.json
    Show,
    /// Get the value of a single configuration key
    Get { key: String },
    /// Set a configuration key (e.g. `clibm config set auto_commit false`)
    Set { key: String, value: String },
    /// Reset configuration to defaults (`auto_commit = true`)
    Reset,
}

#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    /// List tasks in the active workspace (or across all workspaces with --all-workspaces)
    List {
        #[arg(long)]
        column: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        all_workspaces: bool,
    },
    /// Create a new task in the active workspace
    Add {
        title: String,
        #[arg(short, long, default_value = "")]
        description: String,
        /// low | medium | high | critical (defaults to `default_priority` in .clibm/config.json)
        #[arg(short, long)]
        priority: Option<String>,
        /// Column ID or name (defaults to the board's default column, normally todo)
        #[arg(short, long)]
        column: Option<String>,
        #[arg(short, long)]
        assignee: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Comma-separated task references (ID, number or hash prefix)
        #[arg(long, value_delimiter = ',')]
        depends_on: Vec<String>,
        /// Ignore dependency, WIP-limit and assignee rules of the target column
        #[arg(long)]
        force: bool,
    },
    /// Show full details of a task
    Show {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        id: String,
    },
    /// Edit task fields (use `board move` to change its column)
    Edit {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        priority: Option<String>,
        /// New assignee ("" clears it)
        #[arg(short, long)]
        assignee: Option<String>,
        /// New due date ("" clears it)
        #[arg(long)]
        due: Option<String>,
        /// Replace all tags (comma-separated; "" clears them)
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        add_tag: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        remove_tag: Vec<String>,
    },
    /// Remove a task from the current state (it stays in history)
    Delete {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        id: String,
        /// Delete even if other tasks depend on it (those links are removed)
        #[arg(long)]
        force: bool,
    },
    /// Add a comment to a task
    Comment {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        id: String,
        body: String,
        /// Comment author (defaults to `author_name` in .clibm/config.json)
        #[arg(short, long)]
        author: Option<String>,
    },
    /// Add a blocking dependency between tasks (supports cross-workspace dependencies)
    Depend {
        /// Task that will be blocked (ID, number or hash prefix)
        task_id: String,
        /// Task it depends on (ID, number or hash prefix)
        depends_on_id: String,
    },
    /// Transfer or copy a task to another workspace (or another project of it)
    Transfer {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        task_id: String,
        target_workspace: String,
        /// Project of the target workspace (ID, code or name); defaults to its first project
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        copy: bool,
    },
    /// Show individual change history of a single task across commits
    History {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum BoardCommands {
    /// Render the active workspace's Kanban board in the terminal
    Show,
    /// Move a task to a target column (validates dependencies & WIP limits)
    Move {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        task_id: String,
        /// Column ID or name
        column: String,
        /// Ignore dependency, WIP-limit and assignee rules
        #[arg(long)]
        force: bool,
    },
    /// Rename the active project's board
    Rename { name: String },
    /// List, add, edit, remove or reorder the board's columns
    Column {
        #[command(subcommand)]
        action: ColumnCommands,
    },
    /// Show or change the board's workflow rules (no flags: show them)
    Rules {
        /// Moves past the default column need finished dependencies
        #[arg(long)]
        enforce_deps: Option<String>,
        /// Columns refuse tasks beyond their WIP limit
        #[arg(long)]
        enforce_wip: Option<String>,
        /// Tasks need an assignee to enter in-progress
        #[arg(long)]
        require_assignee: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ColumnCommands {
    /// List the columns with their IDs, WIP limits and task counts
    List,
    /// Add a column (by default before the first completed column)
    Add {
        name: String,
        /// Column ID (defaults to the name in lower case with dashes, e.g. qa-check)
        #[arg(long)]
        id: Option<String>,
        /// WIP limit (maximum number of tasks)
        #[arg(long)]
        wip: Option<usize>,
        /// Tasks in this column count as finished
        #[arg(long)]
        done: bool,
        /// Insert after this column (ID or name)
        #[arg(long, conflicts_with = "position")]
        after: Option<String>,
        /// Insert at this 0-based position
        #[arg(long)]
        position: Option<usize>,
    },
    /// Change a column's name, WIP limit or completed flag (its ID never changes)
    Edit {
        /// Column ID or name
        column: String,
        #[arg(long)]
        name: Option<String>,
        /// New WIP limit, or "none" to remove it
        #[arg(long)]
        wip: Option<String>,
        /// Whether tasks in this column count as finished (true/false)
        #[arg(long)]
        done: Option<String>,
        /// Allow a WIP limit below the column's current task count
        #[arg(long)]
        force: bool,
    },
    /// Remove a column (refuses while it has tasks, unless --move-to)
    Remove {
        /// Column ID or name
        column: String,
        /// Column that receives the removed column's tasks
        #[arg(long)]
        move_to: Option<String>,
    },
    /// Move a column to a 0-based position
    Move {
        /// Column ID or name
        column: String,
        position: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// List registered and installed plugins
    List,
    /// Scaffold a new standalone plugin crate from the official CLIBM plugin template
    New {
        /// Name of the new plugin (e.g. 'sprint-velocity' or 'slack-sync')
        name: String,
        /// Optional description of the plugin
        #[arg(short, long, default_value = "Custom CLIBM plugin scaffolded from official template")]
        description: String,
        /// Comma-separated lifecycle hooks (e.g. pre-commit,post-commit,on-task-move)
        #[arg(long, value_delimiter = ',', default_value = "pre-commit,on-task-move")]
        hooks: Vec<String>,
    },
    /// Build, install to PATH, and register a standalone plugin from its folder
    Install {
        /// Path to the plugin folder containing Cargo.toml (defaults to current directory '.')
        #[arg(default_value = ".")]
        path: String,
        /// For clibm-mcp: transport to build in ('stdio' [default], 'both' or 'http')
        #[arg(long, default_value = "stdio")]
        transport: String,
    },
    /// Enable a plugin
    Enable { name: String },
    /// Disable a plugin
    Disable { name: String },
    /// Uninstall a plugin: stop it, delete its binary and remove it from the plugin registries
    Uninstall {
        /// Plugin name (e.g. 'clibm-mcp', 'mcp', 'clibm-tui', 'tui', etc.)
        name: String,
    },
    /// Execute an installed standalone plugin inside the current ongoing project
    Run {
        /// Plugin name (e.g. 'clibm-mcp', 'mcp', 'clibm-tui', 'clibm-web', 'clibm-dep-audit')
        name: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}
