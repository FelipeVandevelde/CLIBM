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
    /// Initialize a new .clibm repository in the current directory (Pure Core by default).
    /// Refuses to run if the current directory already has a .clibm repository.
    Init {
        #[arg(long, default_value = "default-workspace")]
        workspace: String,
        #[arg(long, default_value = "CLIBM")]
        code: String,
        #[arg(long, default_value = "CLIBM Core Project")]
        name: String,
        /// Register the 3 default plugins (clibm-dep-audit, clibm-export-md, clibm-workflow-lint) instead of Pure Core
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

    /// Show the branch, HEAD commit, uncommitted changes, active workspace/project and task counts
    Status,

    /// Manage workspaces, switch active workspace, and coordinate across workspaces
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCommands,
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

    /// Show commit history log
    Log {
        /// Optional branch or commit hash to start from
        reference: Option<String>,
    },

    /// List or create branches
    Branch {
        /// Branch name to create (if omitted, lists all branches)
        name: Option<String>,
    },

    /// Checkout a branch or historical commit hash
    Checkout {
        /// Branch name or commit hash prefix
        target: String,
        /// Discard uncommitted changes instead of refusing
        #[arg(long)]
        force: bool,
    },

    /// Compare two commits or branches structurally
    Diff {
        from_ref: String,
        to_ref: String,
    },

    /// Three-way merge of a branch into HEAD. Conflicts stop the merge unless --ours/--theirs is given.
    Merge {
        branch: String,
        /// Merge commit author (defaults to `author_name` in .clibm/config.json)
        #[arg(short, long)]
        author: Option<String>,
        /// Resolve conflicts by keeping the current branch's version
        #[arg(long, conflicts_with = "theirs")]
        ours: bool,
        /// Resolve conflicts by taking the merged branch's version
        #[arg(long)]
        theirs: bool,
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

    /// Update CLIBM from the configured GitHub repository (via git pull or cargo install --git)
    Update {
        /// Override GitHub repository URL (defaults to .clibm/config.json `github_repo_url`)
        #[arg(long)]
        repo: Option<String>,
        /// Target branch or tag to pull from (defaults to .clibm/config.json `update_branch`)
        #[arg(long)]
        branch: Option<String>,
        /// Also rebuild and update standalone plugins in ./plugins/
        #[arg(long)]
        plugins: bool,
        /// Only check for updates against the GitHub remote without installing
        #[arg(long)]
        check: bool,
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
        /// Column ID or name
        #[arg(short, long, default_value = "todo")]
        column: String,
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
    /// Transfer or copy a task to another workspace
    Transfer {
        /// Task ID (CLIBM-101), number (101) or hash prefix from `board show`
        task_id: String,
        target_workspace: String,
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
        /// For clibm-mcp: select transport mode during installation ('both' [default], 'stdio', or 'http')
        #[arg(long, default_value = "both")]
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
