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

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new .clibm repository in the current directory (Pure Core by default)
    Init {
        #[arg(long, default_value = "default-workspace")]
        workspace: String,
        #[arg(long, default_value = "CLIBM")]
        code: String,
        #[arg(long, default_value = "CLIBM Core Project")]
        name: String,
        /// Install the 3 default plugins (clibm-dep-audit, clibm-export-md, clibm-workflow-lint) instead of Pure Core
        #[arg(long)]
        with_defaults: bool,
        /// Configure repository interface & AI documentation language (en, pt-BR, es)
        #[arg(long, default_value = "en")]
        lang: String,
        /// Enable or disable automatic per-modification commits (true by default)
        #[arg(long)]
        auto_commit: Option<bool>,
        /// Specify machine installation directory where clibm binaries are stored
        #[arg(long, default_value = "~/.local/bin")]
        install_path: String,
    },

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
    Commit {
        #[arg(short, long)]
        message: String,
        #[arg(short, long, default_value = "Local Maintainer")]
        author: String,
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
    },

    /// Compare two commits or branches structurally
    Diff {
        from_ref: String,
        to_ref: String,
    },

    /// Merge a branch into the current HEAD branch
    Merge {
        branch: String,
        #[arg(short, long, default_value = "Local Maintainer")]
        author: String,
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
        /// Also delete the local .clibm repository directory in the current working directory
        #[arg(long)]
        purge: bool,
        /// Skip interactive confirmation prompt
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
        #[arg(short, long, default_value = "medium")]
        priority: String,
        #[arg(short, long, default_value = "todo")]
        column: String,
        #[arg(short, long)]
        assignee: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        depends_on: Vec<String>,
    },
    /// Show full details of a task
    Show {
        id: String,
    },
    /// Add a comment to a task
    Comment {
        id: String,
        body: String,
        #[arg(short, long, default_value = "Local Maintainer")]
        author: String,
    },
    /// Add a blocking dependency between tasks (supports cross-workspace dependencies)
    Depend {
        task_id: String,
        depends_on_id: String,
    },
    /// Transfer or copy a task to another workspace
    Transfer {
        task_id: String,
        target_workspace: String,
        #[arg(long)]
        copy: bool,
    },
    /// Show individual change history of a single task across commits
    History {
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum BoardCommands {
    /// Render the active workspace's Kanban board in the terminal
    Show,
    /// Move a task to a target column (validates dependencies & WIP limits)
    Move {
        task_id: String,
        column: String,
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
    /// Display uninstallation guidance and paths to remove an installed plugin
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
